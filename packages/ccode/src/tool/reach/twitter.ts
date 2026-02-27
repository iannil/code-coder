import z from "zod"
import { Tool } from "../tool"
import { exec, parseTweetUrl, formatCount, getProxyEnv, safeJsonParse } from "./utils"
import { checkChannel } from "./doctor"
import { ReachConfigManager } from "./config"
import type { Tweet } from "./types"
import { Log } from "@/util/log"

/**
 * Agent Reach - Twitter/X Tool
 *
 * Read tweets, search, and timeline using bird CLI
 */

const log = Log.create({ service: "reach.twitter" })

const DESCRIPTION = `Read and search Twitter/X content.

Actions:
- read: Read a specific tweet by URL
- search: Search for tweets by keyword
- timeline: Get user timeline

Examples:
- Read tweet: { "url": "https://twitter.com/user/status/123", "action": "read" }
- Search: { "query": "AI news", "action": "search", "limit": 10 }
- Timeline: { "username": "elonmusk", "action": "timeline", "limit": 20 }

Requires bird CLI to be installed (npm i -g @anthropics/bird-cli).
Also requires cookies configuration for authentication.`

export const TwitterTool = Tool.define("reach_twitter", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["read", "search", "timeline"]).default("read").describe("Action to perform"),
    url: z.string().optional().describe("Tweet URL (required for read action)"),
    query: z.string().optional().describe("Search query (required for search action)"),
    username: z.string().optional().describe("Username for timeline (required for timeline action)"),
    limit: z.number().optional().default(10).describe("Maximum number of tweets to return"),
  }),
  async execute(params, ctx) {
    // Check if bird CLI and config are available
    const channelStatus = await checkChannel("twitter")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "Twitter - Unavailable",
        metadata: { error: true },
        output: `Twitter tool unavailable: ${channelStatus.message}`,
      }
    }

    if (channelStatus.status === "warn") {
      return {
        title: "Twitter - Configuration Needed",
        metadata: { error: true, needsConfig: true },
        output: channelStatus.message,
      }
    }

    const pattern = params.url ?? params.query ?? params.username ?? "twitter"
    await ctx.ask({
      permission: "reach_twitter",
      patterns: [pattern],
      always: ["*"],
      metadata: { action: params.action },
    })

    const proxyEnv = await getProxyEnv()
    const config = await ReachConfigManager.load()

    switch (params.action) {
      case "read":
        if (!params.url) {
          return {
            title: "Twitter - Missing URL",
            metadata: { error: true },
            output: "URL is required for read action",
          }
        }
        return await readTweet(params.url, config.twitter?.cookies, proxyEnv, ctx.abort)
      case "search":
        if (!params.query) {
          return {
            title: "Twitter - Missing Query",
            metadata: { error: true },
            output: "Query is required for search action",
          }
        }
        return await searchTweets(params.query, params.limit ?? 10, config.twitter?.cookies, proxyEnv, ctx.abort)
      case "timeline":
        if (!params.username) {
          return {
            title: "Twitter - Missing Username",
            metadata: { error: true },
            output: "Username is required for timeline action",
          }
        }
        return await getTimeline(params.username, params.limit ?? 10, config.twitter?.cookies, proxyEnv, ctx.abort)
      default:
        return {
          title: "Twitter - Invalid Action",
          metadata: { error: true },
          output: `Invalid action: ${params.action}`,
        }
    }
  },
})

async function readTweet(
  url: string,
  cookies: string | undefined,
  env: Record<string, string>,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const tweetId = parseTweetUrl(url)
  if (!tweetId) {
    return {
      title: "Twitter - Invalid URL",
      metadata: { error: true },
      output: `Could not extract tweet ID from URL: ${url}`,
    }
  }

  const args = ["read", "--json", url]
  if (cookies) {
    args.push("--cookies", cookies)
  }

  const result = await exec("bird", args, { env, abort, timeout: 30_000 })

  if (result.exitCode !== 0) {
    log.error("bird read failed", { url, stderr: result.stderr })
    return {
      title: "Twitter - Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to read tweet: ${result.stderr || "Unknown error"}`,
    }
  }

  const data = safeJsonParse<Record<string, unknown>>(result.stdout)
  if (!data) {
    return {
      title: "Twitter - Parse Error",
      metadata: { error: true },
      output: "Failed to parse tweet data",
    }
  }

  const tweet = parseTweetData(data)
  const output = formatTweet(tweet)

  return {
    title: `Twitter - @${tweet.authorHandle || tweet.author}`,
    metadata: { tweetId: tweet.id, ...tweet },
    output,
  }
}

async function searchTweets(
  query: string,
  limit: number,
  cookies: string | undefined,
  env: Record<string, string>,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const args = ["search", "--json", "--limit", String(limit), query]
  if (cookies) {
    args.push("--cookies", cookies)
  }

  const result = await exec("bird", args, { env, abort, timeout: 60_000 })

  if (result.exitCode !== 0) {
    log.error("bird search failed", { query, stderr: result.stderr })
    return {
      title: "Twitter - Search Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to search: ${result.stderr || "Unknown error"}`,
    }
  }

  // Parse NDJSON output
  const tweets: Tweet[] = []
  for (const line of result.stdout.split("\n")) {
    if (!line.trim()) continue
    const data = safeJsonParse<Record<string, unknown>>(line)
    if (data) {
      tweets.push(parseTweetData(data))
    }
  }

  const output = formatSearchResults(tweets, query)

  return {
    title: `Twitter - Search: "${query}" (${tweets.length} results)`,
    metadata: { query, resultCount: tweets.length },
    output,
  }
}

async function getTimeline(
  username: string,
  limit: number,
  cookies: string | undefined,
  env: Record<string, string>,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const args = ["timeline", "--json", "--limit", String(limit), username]
  if (cookies) {
    args.push("--cookies", cookies)
  }

  const result = await exec("bird", args, { env, abort, timeout: 60_000 })

  if (result.exitCode !== 0) {
    log.error("bird timeline failed", { username, stderr: result.stderr })
    return {
      title: "Twitter - Timeline Error",
      metadata: { error: true, exitCode: result.exitCode },
      output: `Failed to get timeline: ${result.stderr || "Unknown error"}`,
    }
  }

  // Parse NDJSON output
  const tweets: Tweet[] = []
  for (const line of result.stdout.split("\n")) {
    if (!line.trim()) continue
    const data = safeJsonParse<Record<string, unknown>>(line)
    if (data) {
      tweets.push(parseTweetData(data))
    }
  }

  const output = formatTimeline(tweets, username)

  return {
    title: `Twitter - @${username} Timeline (${tweets.length} tweets)`,
    metadata: { username, tweetCount: tweets.length },
    output,
  }
}

function parseTweetData(data: Record<string, unknown>): Tweet {
  const user = data.user as Record<string, unknown> | undefined
  return {
    id: String(data.id ?? data.id_str ?? ""),
    text: String(data.text ?? data.full_text ?? ""),
    author: String(user?.name ?? data.author ?? "Unknown"),
    authorHandle: user?.screen_name ? String(user.screen_name) : undefined,
    createdAt: data.created_at ? String(data.created_at) : undefined,
    likeCount: typeof data.favorite_count === "number" ? data.favorite_count : undefined,
    retweetCount: typeof data.retweet_count === "number" ? data.retweet_count : undefined,
    replyCount: typeof data.reply_count === "number" ? data.reply_count : undefined,
    mediaUrls: extractMediaUrls(data),
  }
}

function extractMediaUrls(data: Record<string, unknown>): string[] | undefined {
  const extendedEntities = data.extended_entities as Record<string, unknown> | undefined
  const entities = data.entities as Record<string, unknown> | undefined
  const media = extendedEntities?.media ?? entities?.media
  if (!Array.isArray(media)) return undefined

  return media
    .map((m: Record<string, unknown>) => m.media_url_https ?? m.url)
    .filter(Boolean)
    .map(String)
}

function formatTweet(tweet: Tweet): string {
  const lines: string[] = [
    `# @${tweet.authorHandle || tweet.author}`,
    "",
    tweet.text,
    "",
  ]

  const stats: string[] = []
  if (tweet.likeCount !== undefined) stats.push(`❤️ ${formatCount(tweet.likeCount)}`)
  if (tweet.retweetCount !== undefined) stats.push(`🔁 ${formatCount(tweet.retweetCount)}`)
  if (tweet.replyCount !== undefined) stats.push(`💬 ${formatCount(tweet.replyCount)}`)

  if (stats.length > 0) {
    lines.push(stats.join(" · "))
  }

  if (tweet.createdAt) {
    lines.push(`📅 ${tweet.createdAt}`)
  }

  if (tweet.id) {
    lines.push("", `🔗 https://twitter.com/i/status/${tweet.id}`)
  }

  if (tweet.mediaUrls && tweet.mediaUrls.length > 0) {
    lines.push("", "**Media:**")
    for (const url of tweet.mediaUrls) {
      lines.push(`- ${url}`)
    }
  }

  return lines.join("\n")
}

function formatSearchResults(tweets: Tweet[], query: string): string {
  const lines: string[] = [
    `# Twitter Search: "${query}"`,
    `Found ${tweets.length} results`,
    "",
  ]

  for (let i = 0; i < tweets.length; i++) {
    const t = tweets[i]
    lines.push(`## ${i + 1}. @${t.authorHandle || t.author}`)
    lines.push(t.text)

    const stats: string[] = []
    if (t.likeCount !== undefined) stats.push(`❤️ ${formatCount(t.likeCount)}`)
    if (t.retweetCount !== undefined) stats.push(`🔁 ${formatCount(t.retweetCount)}`)
    if (stats.length > 0) {
      lines.push(stats.join(" · "))
    }

    lines.push("")
  }

  return lines.join("\n")
}

function formatTimeline(tweets: Tweet[], username: string): string {
  const lines: string[] = [
    `# @${username} Timeline`,
    `${tweets.length} recent tweets`,
    "",
  ]

  for (const t of tweets) {
    lines.push(`---`)
    lines.push(t.text)

    const stats: string[] = []
    if (t.likeCount !== undefined) stats.push(`❤️ ${formatCount(t.likeCount)}`)
    if (t.retweetCount !== undefined) stats.push(`🔁 ${formatCount(t.retweetCount)}`)
    if (t.createdAt) stats.push(`📅 ${t.createdAt}`)
    if (stats.length > 0) {
      lines.push(stats.join(" · "))
    }

    lines.push("")
  }

  return lines.join("\n")
}
