import z from "zod"
import { Tool } from "../tool"
import { exec, extractVideoId, formatDuration, formatCount, getProxyEnv, safeJsonParse } from "./utils"
import { checkChannel } from "./doctor"
import type { VideoInfo } from "./types"
import { Log } from "@/util/log"

/**
 * Agent Reach - Bilibili Tool
 *
 * Extract B站 video info and transcripts using yt-dlp
 */

const log = Log.create({ service: "reach.bilibili" })

const DESCRIPTION = `Extract Bilibili (B站) video information and transcripts.

Actions:
- info: Get video metadata (title, description, duration, views, etc.)
- transcript: Get video subtitles/captions

Examples:
- Get info: { "url": "https://www.bilibili.com/video/BV1xx411c7mD", "action": "info" }
- Get transcript: { "url": "https://www.bilibili.com/video/BV1xx411c7mD", "action": "transcript" }

Requires yt-dlp to be installed (pip install yt-dlp).`

export const BilibiliTool = Tool.define("reach_bilibili", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["info", "transcript"]).default("info").describe("Action to perform"),
    url: z.string().describe("Bilibili video URL"),
    language: z.string().optional().describe("Subtitle language preference"),
  }),
  async execute(params, ctx) {
    // Check if yt-dlp is available
    const channelStatus = await checkChannel("bilibili")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "Bilibili - Unavailable",
        metadata: { error: true },
        output: `Bilibili tool unavailable: ${channelStatus.message}`,
      }
    }

    await ctx.ask({
      permission: "reach_bilibili",
      patterns: [params.url],
      always: ["*"],
      metadata: { action: params.action },
    })

    const proxyEnv = await getProxyEnv()

    switch (params.action) {
      case "info":
        return await getVideoInfo(params.url, proxyEnv, ctx.abort)
      case "transcript":
        return await getTranscript(params.url, params.language, proxyEnv, ctx.abort)
      default:
        return {
          title: "Bilibili - Invalid Action",
          metadata: { error: true },
          output: `Invalid action: ${params.action}`,
        }
    }
  },
})

async function getVideoInfo(
  url: string,
  env: Record<string, string>,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const videoId = extractVideoId(url, "bilibili")
  if (!videoId) {
    return {
      title: "Bilibili - Invalid URL",
      metadata: { error: true },
      output: `Could not extract video ID from URL: ${url}`,
    }
  }

  const result = await exec(
    "yt-dlp",
    [
      "--dump-json",
      "--no-download",
      "--no-warnings",
      url,
    ],
    { env, abort, timeout: 30_000 },
  )

  if (result.exitCode !== 0) {
    log.error("yt-dlp failed", { url, stderr: result.stderr })
    return {
      title: "Bilibili - Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to get video info: ${result.stderr || "Unknown error"}`,
    }
  }

  const data = safeJsonParse<Record<string, unknown>>(result.stdout)
  if (!data) {
    return {
      title: "Bilibili - Parse Error",
      metadata: { error: true },
      output: "Failed to parse video info from yt-dlp output",
    }
  }

  const info: VideoInfo = {
    id: String(data.id ?? videoId),
    title: String(data.title ?? "Unknown"),
    description: data.description ? String(data.description) : undefined,
    duration: typeof data.duration === "number" ? data.duration : undefined,
    uploadDate: data.upload_date ? String(data.upload_date) : undefined,
    uploader: data.uploader ? String(data.uploader) : undefined,
    viewCount: typeof data.view_count === "number" ? data.view_count : undefined,
    likeCount: typeof data.like_count === "number" ? data.like_count : undefined,
    thumbnailUrl: data.thumbnail ? String(data.thumbnail) : undefined,
    tags: Array.isArray(data.tags) ? data.tags.map(String) : undefined,
  }

  const output = formatVideoInfo(info, url)

  return {
    title: `Bilibili - ${info.title}`,
    metadata: { videoId: info.id, ...info },
    output,
  }
}

async function getTranscript(
  url: string,
  language: string | undefined,
  env: Record<string, string>,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const videoId = extractVideoId(url, "bilibili")
  if (!videoId) {
    return {
      title: "Bilibili - Invalid URL",
      metadata: { error: true },
      output: `Could not extract video ID from URL: ${url}`,
    }
  }

  // First, get available subtitles
  const result = await exec(
    "yt-dlp",
    [
      "--list-subs",
      "--no-download",
      "--no-warnings",
      url,
    ],
    { env, abort, timeout: 30_000 },
  )

  if (result.exitCode !== 0) {
    log.error("yt-dlp list-subs failed", { url, stderr: result.stderr })
    return {
      title: "Bilibili - Transcript Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to list subtitles: ${result.stderr || "No subtitles available"}`,
    }
  }

  // Check if subtitles are available
  if (result.stdout.includes("has no subtitles") || !result.stdout.includes("subtitle")) {
    return {
      title: "Bilibili - No Transcript",
      metadata: { error: true, videoId },
      output: "No subtitles available for this video",
    }
  }

  // Download subtitles
  const langArgs = language ? ["--sub-langs", language] : ["--sub-langs", "zh.*,en.*,all"]

  const subResult = await exec(
    "yt-dlp",
    [
      "--write-sub",
      "--write-auto-sub",
      "--skip-download",
      "--sub-format", "vtt",
      ...langArgs,
      "-o", "-",
      "--print", "%(requested_subtitles)j",
      "--no-warnings",
      url,
    ],
    { env, abort, timeout: 60_000 },
  )

  if (subResult.exitCode !== 0 || !subResult.stdout.trim()) {
    // Try alternative approach - get subtitles from video info
    const infoResult = await exec(
      "yt-dlp",
      [
        "--dump-json",
        "--no-download",
        "--no-warnings",
        url,
      ],
      { env, abort, timeout: 30_000 },
    )

    if (infoResult.exitCode === 0) {
      const data = safeJsonParse<Record<string, unknown>>(infoResult.stdout)
      if (data?.subtitles || data?.automatic_captions) {
        const subs = (data.subtitles ?? data.automatic_captions) as Record<string, unknown[]>
        const langs = Object.keys(subs)

        if (langs.length > 0) {
          return {
            title: "Bilibili - Transcript Available",
            metadata: { videoId, availableLanguages: langs },
            output: `Subtitles available in: ${langs.join(", ")}\n\nUse yt-dlp directly to download the full subtitles.`,
          }
        }
      }
    }

    return {
      title: "Bilibili - No Transcript",
      metadata: { error: true, videoId },
      output: "Could not extract transcript content",
    }
  }

  // Parse subtitle info
  const subData = safeJsonParse<Record<string, unknown>>(subResult.stdout)
  const output = formatSubtitleInfo(subData, videoId)

  return {
    title: `Bilibili - Transcript Info`,
    metadata: { videoId, subtitles: subData },
    output,
  }
}

function formatVideoInfo(info: VideoInfo, url: string): string {
  const lines: string[] = [
    `# ${info.title}`,
    "",
    `**ID:** ${info.id}`,
    `**URL:** ${url}`,
  ]

  if (info.uploader) lines.push(`**UP主:** ${info.uploader}`)
  if (info.duration) lines.push(`**时长:** ${formatDuration(info.duration)}`)
  if (info.viewCount) lines.push(`**播放:** ${formatCount(info.viewCount)}`)
  if (info.likeCount) lines.push(`**点赞:** ${formatCount(info.likeCount)}`)
  if (info.uploadDate) {
    const date = `${info.uploadDate.slice(0, 4)}-${info.uploadDate.slice(4, 6)}-${info.uploadDate.slice(6, 8)}`
    lines.push(`**发布日期:** ${date}`)
  }

  if (info.tags && info.tags.length > 0) {
    lines.push(`**标签:** ${info.tags.slice(0, 10).join(", ")}`)
  }

  if (info.description) {
    lines.push("", "## 简介", "", info.description)
  }

  return lines.join("\n")
}

function formatSubtitleInfo(subData: Record<string, unknown> | null, videoId: string): string {
  const lines: string[] = [
    `# 字幕信息`,
    `视频 ID: ${videoId}`,
    "",
  ]

  if (!subData || Object.keys(subData).length === 0) {
    lines.push("暂无字幕信息")
  } else {
    lines.push("可用字幕:")
    for (const [lang, info] of Object.entries(subData)) {
      const langInfo = info as { ext?: string; url?: string }
      lines.push(`- ${lang} (${langInfo.ext ?? "unknown"})`)
    }
  }

  return lines.join("\n")
}
