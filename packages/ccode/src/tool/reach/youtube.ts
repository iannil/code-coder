import z from "zod"
import { Tool } from "../tool"
import { exec, extractVideoId, formatDuration, formatCount, getProxyEnv, safeJsonParse } from "./utils"
import { checkChannel } from "./doctor"
import type { VideoInfo, TranscriptSegment } from "./types"
import { Log } from "@/util/log"

/**
 * Agent Reach - YouTube Tool
 *
 * Extract video info, transcripts, and search using yt-dlp
 */

const log = Log.create({ service: "reach.youtube" })

const DESCRIPTION = `Extract YouTube video information and transcripts.

Actions:
- info: Get video metadata (title, description, duration, views, etc.)
- transcript: Get video transcript/subtitles in specified language
- search: Search YouTube for videos (returns top results)

Examples:
- Get info: { "url": "https://youtube.com/watch?v=dQw4w9WgXcQ", "action": "info" }
- Get transcript: { "url": "https://youtube.com/watch?v=dQw4w9WgXcQ", "action": "transcript", "language": "en" }
- Search: { "query": "rust programming tutorial", "action": "search", "limit": 5 }

Requires yt-dlp to be installed (pip install yt-dlp).`

export const YouTubeTool = Tool.define("reach_youtube", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["info", "transcript", "search"]).default("info").describe("Action to perform"),
    url: z.string().optional().describe("YouTube video URL (required for info/transcript)"),
    query: z.string().optional().describe("Search query (required for search action)"),
    language: z.string().optional().describe("Subtitle language code (e.g., en, zh, auto)"),
    limit: z.number().optional().default(5).describe("Number of search results to return"),
  }),
  async execute(params, ctx) {
    // Check if yt-dlp is available
    const channelStatus = await checkChannel("youtube")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "YouTube - Unavailable",
        metadata: { error: true },
        output: `YouTube tool unavailable: ${channelStatus.message}`,
      }
    }

    await ctx.ask({
      permission: "reach_youtube",
      patterns: params.url ? [params.url] : params.query ? [`search:${params.query}`] : ["youtube"],
      always: ["*"],
      metadata: { action: params.action },
    })

    const proxyEnv = await getProxyEnv()

    switch (params.action) {
      case "info":
        return await getVideoInfo(params.url!, proxyEnv, ctx.abort)
      case "transcript":
        return await getTranscript(params.url!, params.language, proxyEnv, ctx.abort)
      case "search":
        return await searchVideos(params.query!, params.limit ?? 5, proxyEnv, ctx.abort)
      default:
        return {
          title: "YouTube - Invalid Action",
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
  const videoId = extractVideoId(url, "youtube")
  if (!videoId) {
    return {
      title: "YouTube - Invalid URL",
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
      "--no-playlist",
      url,
    ],
    { env, abort, timeout: 30_000 },
  )

  if (result.exitCode !== 0) {
    log.error("yt-dlp failed", { url, stderr: result.stderr })
    return {
      title: "YouTube - Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to get video info: ${result.stderr || "Unknown error"}`,
    }
  }

  const data = safeJsonParse<Record<string, unknown>>(result.stdout)
  if (!data) {
    return {
      title: "YouTube - Parse Error",
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

  const output = formatVideoInfo(info)

  return {
    title: `YouTube - ${info.title}`,
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
  const videoId = extractVideoId(url, "youtube")
  if (!videoId) {
    return {
      title: "YouTube - Invalid URL",
      metadata: { error: true },
      output: `Could not extract video ID from URL: ${url}`,
    }
  }

  // Use yt-dlp to download subtitles
  const langArgs = language ? ["--sub-langs", language] : ["--sub-langs", "en.*,zh.*"]

  const result = await exec(
    "yt-dlp",
    [
      "--write-sub",
      "--write-auto-sub",
      "--skip-download",
      "--sub-format", "json3",
      ...langArgs,
      "--print", "%(subtitles)j",
      "--no-warnings",
      url,
    ],
    { env, abort, timeout: 60_000 },
  )

  if (result.exitCode !== 0) {
    log.error("yt-dlp transcript failed", { url, stderr: result.stderr })
    return {
      title: "YouTube - Transcript Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to get transcript: ${result.stderr || "No subtitles available"}`,
    }
  }

  // Parse subtitle data
  const subtitleData = safeJsonParse<Record<string, unknown>>(result.stdout)
  if (!subtitleData || Object.keys(subtitleData).length === 0) {
    return {
      title: "YouTube - No Transcript",
      metadata: { error: true, videoId },
      output: "No transcript available for this video",
    }
  }

  // Extract transcript text
  const segments: TranscriptSegment[] = []
  const firstLang = Object.keys(subtitleData)[0]
  const langData = subtitleData[firstLang]

  if (Array.isArray(langData)) {
    for (const sub of langData) {
      if (sub && typeof sub === "object" && "url" in sub) {
        // Need to fetch the actual subtitle content
        try {
          const subResponse = await fetch(String(sub.url), { signal: abort })
          const subContent = await subResponse.json()

          if (subContent.events) {
            for (const event of subContent.events) {
              if (event.segs) {
                const text = event.segs.map((s: { utf8?: string }) => s.utf8 ?? "").join("")
                if (text.trim()) {
                  segments.push({
                    start: (event.tStartMs ?? 0) / 1000,
                    end: ((event.tStartMs ?? 0) + (event.dDurationMs ?? 0)) / 1000,
                    text: text.trim(),
                  })
                }
              }
            }
          }
        } catch (err) {
          log.warn("failed to fetch subtitle content", { url: sub.url, error: err })
        }
      }
    }
  }

  if (segments.length === 0) {
    return {
      title: "YouTube - No Transcript",
      metadata: { error: true, videoId },
      output: "Could not extract transcript content",
    }
  }

  const output = formatTranscript(segments, videoId)

  return {
    title: `YouTube - Transcript (${segments.length} segments)`,
    metadata: { videoId, language: firstLang, segmentCount: segments.length },
    output,
  }
}

async function searchVideos(
  query: string,
  limit: number,
  env: Record<string, string>,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const result = await exec(
    "yt-dlp",
    [
      "--dump-json",
      "--no-download",
      "--no-warnings",
      "--flat-playlist",
      `ytsearch${limit}:${query}`,
    ],
    { env, abort, timeout: 30_000 },
  )

  if (result.exitCode !== 0) {
    log.error("yt-dlp search failed", { query, stderr: result.stderr })
    return {
      title: "YouTube - Search Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to search: ${result.stderr || "Unknown error"}`,
    }
  }

  // Parse NDJSON output (one JSON object per line)
  const videos: VideoInfo[] = []
  for (const line of result.stdout.split("\n")) {
    if (!line.trim()) continue
    const data = safeJsonParse<Record<string, unknown>>(line)
    if (!data) continue

    videos.push({
      id: String(data.id ?? ""),
      title: String(data.title ?? "Unknown"),
      description: data.description ? String(data.description) : undefined,
      duration: typeof data.duration === "number" ? data.duration : undefined,
      uploader: data.uploader ? String(data.uploader) : undefined,
      viewCount: typeof data.view_count === "number" ? data.view_count : undefined,
    })
  }

  const output = formatSearchResults(videos, query)

  return {
    title: `YouTube - Search: ${query} (${videos.length} results)`,
    metadata: { query, resultCount: videos.length },
    output,
  }
}

function formatVideoInfo(info: VideoInfo): string {
  const lines: string[] = [
    `# ${info.title}`,
    "",
    `**ID:** ${info.id}`,
    `**URL:** https://youtube.com/watch?v=${info.id}`,
  ]

  if (info.uploader) lines.push(`**Channel:** ${info.uploader}`)
  if (info.duration) lines.push(`**Duration:** ${formatDuration(info.duration)}`)
  if (info.viewCount) lines.push(`**Views:** ${formatCount(info.viewCount)}`)
  if (info.likeCount) lines.push(`**Likes:** ${formatCount(info.likeCount)}`)
  if (info.uploadDate) {
    const date = `${info.uploadDate.slice(0, 4)}-${info.uploadDate.slice(4, 6)}-${info.uploadDate.slice(6, 8)}`
    lines.push(`**Uploaded:** ${date}`)
  }

  if (info.tags && info.tags.length > 0) {
    lines.push(`**Tags:** ${info.tags.slice(0, 10).join(", ")}`)
  }

  if (info.description) {
    lines.push("", "## Description", "", info.description)
  }

  return lines.join("\n")
}

function formatTranscript(segments: TranscriptSegment[], videoId: string): string {
  const lines: string[] = [
    `# Transcript`,
    `Video: https://youtube.com/watch?v=${videoId}`,
    "",
  ]

  for (const seg of segments) {
    const timestamp = formatDuration(seg.start)
    lines.push(`[${timestamp}] ${seg.text}`)
  }

  return lines.join("\n")
}

function formatSearchResults(videos: VideoInfo[], query: string): string {
  const lines: string[] = [
    `# YouTube Search: "${query}"`,
    `Found ${videos.length} results`,
    "",
  ]

  for (let i = 0; i < videos.length; i++) {
    const v = videos[i]
    lines.push(`## ${i + 1}. ${v.title}`)
    lines.push(`- URL: https://youtube.com/watch?v=${v.id}`)
    if (v.uploader) lines.push(`- Channel: ${v.uploader}`)
    if (v.duration) lines.push(`- Duration: ${formatDuration(v.duration)}`)
    if (v.viewCount) lines.push(`- Views: ${formatCount(v.viewCount)}`)
    lines.push("")
  }

  return lines.join("\n")
}
