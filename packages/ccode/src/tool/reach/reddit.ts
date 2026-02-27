import z from "zod"
import { Tool } from "../tool"
import { parseRedditUrl, formatCount, truncate } from "./utils"
import { ReachConfigManager } from "./config"
import type { RedditPost, RedditComment } from "./types"
import { Log } from "@/util/log"

/**
 * Agent Reach - Reddit Tool
 *
 * Read Reddit posts and comments using JSON API
 */

const log = Log.create({ service: "reach.reddit" })

const DESCRIPTION = `Read Reddit posts, comments, and search.

Actions:
- read: Read a specific post and its comments
- hot: Get hot posts from a subreddit
- search: Search posts in a subreddit or across Reddit

Examples:
- Read post: { "url": "https://reddit.com/r/programming/comments/abc123/...", "action": "read" }
- Hot posts: { "subreddit": "programming", "action": "hot", "limit": 10 }
- Search: { "query": "rust vs go", "subreddit": "programming", "action": "search" }

Uses Reddit's public JSON API. May require proxy in some regions.`

export const RedditTool = Tool.define("reach_reddit", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["read", "hot", "search"]).default("read").describe("Action to perform"),
    url: z.string().optional().describe("Reddit post URL (required for read action)"),
    subreddit: z.string().optional().describe("Subreddit name (without r/)"),
    query: z.string().optional().describe("Search query (required for search action)"),
    limit: z.number().optional().default(10).describe("Maximum number of posts to return"),
  }),
  async execute(params, ctx) {
    const pattern = params.url ?? params.subreddit ?? params.query ?? "reddit"
    await ctx.ask({
      permission: "reach_reddit",
      patterns: [pattern],
      always: ["*"],
      metadata: { action: params.action },
    })

    // Get proxy if configured
    const proxy = await ReachConfigManager.getProxy()

    switch (params.action) {
      case "read":
        if (!params.url) {
          return {
            title: "Reddit - Missing URL",
            metadata: { error: true },
            output: "URL is required for read action",
          }
        }
        return await readPost(params.url, proxy, ctx.abort)
      case "hot":
        if (!params.subreddit) {
          return {
            title: "Reddit - Missing Subreddit",
            metadata: { error: true },
            output: "Subreddit is required for hot action",
          }
        }
        return await getHotPosts(params.subreddit, params.limit ?? 10, proxy, ctx.abort)
      case "search":
        if (!params.query) {
          return {
            title: "Reddit - Missing Query",
            metadata: { error: true },
            output: "Query is required for search action",
          }
        }
        return await searchPosts(params.query, params.subreddit, params.limit ?? 10, proxy, ctx.abort)
      default:
        return {
          title: "Reddit - Invalid Action",
          metadata: { error: true },
          output: `Invalid action: ${params.action}`,
        }
    }
  },
})

async function fetchJson(url: string, proxy: string | undefined, abort?: AbortSignal): Promise<unknown> {
  const headers = {
    "User-Agent": "CodeCoder/1.0 (Reddit Reader)",
    Accept: "application/json",
  }

  // Note: Native fetch doesn't support proxy directly
  // For proxy support, users would need to configure system-level proxy or use a proxy agent
  const response = await fetch(url, {
    signal: abort,
    headers,
  })

  if (!response.ok) {
    throw new Error(`Reddit API error: HTTP ${response.status}`)
  }

  return response.json()
}

async function readPost(
  url: string,
  proxy: string | undefined,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const parsed = parseRedditUrl(url)
  if (!parsed || !parsed.postId) {
    return {
      title: "Reddit - Invalid URL",
      metadata: { error: true },
      output: `Could not parse Reddit URL: ${url}`,
    }
  }

  try {
    // Append .json to the URL to get JSON response
    const jsonUrl = url.replace(/\/?$/, ".json")
    const data = await fetchJson(jsonUrl, proxy, abort)

    if (!Array.isArray(data) || data.length < 1) {
      return {
        title: "Reddit - Parse Error",
        metadata: { error: true },
        output: "Unexpected API response format",
      }
    }

    // First element is the post, second is comments
    const postData = data[0]?.data?.children?.[0]?.data
    const commentsData = data[1]?.data?.children ?? []

    if (!postData) {
      return {
        title: "Reddit - Not Found",
        metadata: { error: true },
        output: "Post not found",
      }
    }

    const post = parsePostData(postData)
    const comments = commentsData
      .filter((c: { kind: string }) => c.kind === "t1")
      .slice(0, 20)
      .map((c: { data: Record<string, unknown> }) => parseCommentData(c.data))

    const output = formatPost(post, comments)

    return {
      title: `Reddit - ${truncate(post.title, 50)}`,
      metadata: { postId: post.id, subreddit: post.subreddit, commentCount: comments.length },
      output,
    }
  } catch (error) {
    log.error("reddit read failed", { url, error })
    return {
      title: "Reddit - Error",
      metadata: { error: true },
      output: `Failed to read post: ${error instanceof Error ? error.message : "Unknown error"}`,
    }
  }
}

async function getHotPosts(
  subreddit: string,
  limit: number,
  proxy: string | undefined,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  try {
    const url = `https://www.reddit.com/r/${subreddit}/hot.json?limit=${limit}`
    const data = await fetchJson(url, proxy, abort) as { data?: { children?: { data: Record<string, unknown> }[] } }

    const posts = (data.data?.children ?? []).map((c) => parsePostData(c.data))
    const output = formatPostList(posts, `r/${subreddit} - Hot`)

    return {
      title: `Reddit - r/${subreddit} Hot (${posts.length} posts)`,
      metadata: { subreddit, postCount: posts.length },
      output,
    }
  } catch (error) {
    log.error("reddit hot failed", { subreddit, error })
    return {
      title: "Reddit - Error",
      metadata: { error: true },
      output: `Failed to get hot posts: ${error instanceof Error ? error.message : "Unknown error"}`,
    }
  }
}

async function searchPosts(
  query: string,
  subreddit: string | undefined,
  limit: number,
  proxy: string | undefined,
  abort?: AbortSignal,
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  try {
    const baseUrl = subreddit
      ? `https://www.reddit.com/r/${subreddit}/search.json`
      : "https://www.reddit.com/search.json"

    const params = new URLSearchParams({
      q: query,
      limit: String(limit),
      sort: "relevance",
      ...(subreddit ? { restrict_sr: "1" } : {}),
    })

    const url = `${baseUrl}?${params}`
    const data = await fetchJson(url, proxy, abort) as { data?: { children?: { data: Record<string, unknown> }[] } }

    const posts = (data.data?.children ?? []).map((c) => parsePostData(c.data))
    const title = subreddit ? `r/${subreddit} Search: "${query}"` : `Reddit Search: "${query}"`
    const output = formatPostList(posts, title)

    return {
      title: `Reddit - ${title} (${posts.length} results)`,
      metadata: { query, subreddit, resultCount: posts.length },
      output,
    }
  } catch (error) {
    log.error("reddit search failed", { query, subreddit, error })
    return {
      title: "Reddit - Error",
      metadata: { error: true },
      output: `Failed to search: ${error instanceof Error ? error.message : "Unknown error"}`,
    }
  }
}

function parsePostData(data: Record<string, unknown>): RedditPost {
  return {
    id: String(data.id ?? ""),
    title: String(data.title ?? "Untitled"),
    selftext: data.selftext ? String(data.selftext) : undefined,
    author: String(data.author ?? "[deleted]"),
    subreddit: String(data.subreddit ?? ""),
    score: typeof data.score === "number" ? data.score : undefined,
    numComments: typeof data.num_comments === "number" ? data.num_comments : undefined,
    url: data.url ? String(data.url) : undefined,
    createdUtc: typeof data.created_utc === "number" ? data.created_utc : undefined,
    isVideo: Boolean(data.is_video),
    isSelf: Boolean(data.is_self),
  }
}

function parseCommentData(data: Record<string, unknown>): RedditComment {
  const replies = data.replies as { data?: { children?: { kind: string; data: Record<string, unknown> }[] } } | undefined

  return {
    id: String(data.id ?? ""),
    author: String(data.author ?? "[deleted]"),
    body: String(data.body ?? ""),
    score: typeof data.score === "number" ? data.score : undefined,
    createdUtc: typeof data.created_utc === "number" ? data.created_utc : undefined,
    replies:
      replies?.data?.children
        ?.filter((c) => c.kind === "t1")
        .slice(0, 3)
        .map((c) => parseCommentData(c.data)) ?? undefined,
  }
}

function formatPost(post: RedditPost, comments: RedditComment[]): string {
  const lines: string[] = [
    `# ${post.title}`,
    "",
    `**Subreddit:** r/${post.subreddit}`,
    `**Author:** u/${post.author}`,
  ]

  if (post.score !== undefined) {
    lines.push(`**Score:** ${formatCount(post.score)}`)
  }

  if (post.numComments !== undefined) {
    lines.push(`**Comments:** ${formatCount(post.numComments)}`)
  }

  if (post.createdUtc) {
    const date = new Date(post.createdUtc * 1000)
    lines.push(`**Posted:** ${date.toISOString().split("T")[0]}`)
  }

  if (!post.isSelf && post.url) {
    lines.push(`**Link:** ${post.url}`)
  }

  if (post.selftext) {
    lines.push("", "---", "", post.selftext)
  }

  if (comments.length > 0) {
    lines.push("", "---", "", "## Top Comments", "")

    for (const comment of comments.slice(0, 10)) {
      lines.push(formatComment(comment, 0))
    }
  }

  return lines.join("\n")
}

function formatComment(comment: RedditComment, depth: number): string {
  const indent = "  ".repeat(depth)
  const lines: string[] = []

  const score = comment.score !== undefined ? ` (${formatCount(comment.score)} points)` : ""
  lines.push(`${indent}**u/${comment.author}**${score}`)
  lines.push(`${indent}${comment.body.split("\n").join(`\n${indent}`)}`)

  if (comment.replies && depth < 2) {
    for (const reply of comment.replies.slice(0, 2)) {
      lines.push("")
      lines.push(formatComment(reply, depth + 1))
    }
  }

  lines.push("")
  return lines.join("\n")
}

function formatPostList(posts: RedditPost[], title: string): string {
  const lines: string[] = [
    `# ${title}`,
    `${posts.length} posts`,
    "",
  ]

  for (let i = 0; i < posts.length; i++) {
    const p = posts[i]
    lines.push(`## ${i + 1}. ${p.title}`)
    lines.push(`- **Subreddit:** r/${p.subreddit}`)
    lines.push(`- **Author:** u/${p.author}`)

    if (p.score !== undefined) {
      lines.push(`- **Score:** ${formatCount(p.score)}`)
    }

    if (p.numComments !== undefined) {
      lines.push(`- **Comments:** ${formatCount(p.numComments)}`)
    }

    if (!p.isSelf && p.url) {
      lines.push(`- **Link:** ${p.url}`)
    } else {
      lines.push(`- **Link:** https://reddit.com/r/${p.subreddit}/comments/${p.id}`)
    }

    if (p.selftext) {
      lines.push("", truncate(p.selftext, 300))
    }

    lines.push("")
  }

  return lines.join("\n")
}
