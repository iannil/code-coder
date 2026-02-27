import z from "zod"

/**
 * Agent Reach - Types for internet access capabilities
 *
 * Channel tiers:
 * - Tier 0: Zero configuration (YouTube, Bilibili, RSS)
 * - Tier 1: Requires configuration (Twitter, Reddit)
 * - Tier 2: Requires MCP server (Xiaohongshu, Douyin, LinkedIn, BossZhipin)
 */

export const ChannelStatus = z.enum(["ok", "warn", "off", "error"])
export type ChannelStatus = z.infer<typeof ChannelStatus>

export const ChannelTier = z.union([z.literal(0), z.literal(1), z.literal(2)])
export type ChannelTier = z.infer<typeof ChannelTier>

export const ChannelInfo = z.object({
  name: z.string().describe("Channel display name"),
  description: z.string().describe("Brief description of the channel"),
  status: ChannelStatus,
  message: z.string().describe("Status message or installation instructions"),
  tier: ChannelTier.describe("0=zero-config, 1=needs-config, 2=needs-MCP"),
  backends: z.array(z.string()).describe("Required external tools or services"),
})
export type ChannelInfo = z.infer<typeof ChannelInfo>

export const TwitterConfig = z.object({
  cookies: z.string().optional().describe("Path to cookies file or cookie string"),
})
export type TwitterConfig = z.infer<typeof TwitterConfig>

export const McpChannelConfig = z.object({
  mcpName: z.string().describe("Name of the MCP server in config"),
})
export type McpChannelConfig = z.infer<typeof McpChannelConfig>

export const ReachConfig = z.object({
  proxy: z.string().optional().describe("HTTP/SOCKS proxy URL"),
  twitter: TwitterConfig.optional(),
  xiaohongshu: McpChannelConfig.optional(),
  douyin: McpChannelConfig.optional(),
  linkedin: McpChannelConfig.optional(),
  bosszhipin: McpChannelConfig.optional(),
})
export type ReachConfig = z.infer<typeof ReachConfig>

// Channel identifiers
export const ChannelId = z.enum([
  "youtube",
  "bilibili",
  "rss",
  "twitter",
  "reddit",
  "xiaohongshu",
  "douyin",
  "linkedin",
  "bosszhipin",
])
export type ChannelId = z.infer<typeof ChannelId>

// Tool output types
export const VideoInfo = z.object({
  id: z.string(),
  title: z.string(),
  description: z.string().optional(),
  duration: z.number().optional().describe("Duration in seconds"),
  uploadDate: z.string().optional(),
  uploader: z.string().optional(),
  viewCount: z.number().optional(),
  likeCount: z.number().optional(),
  thumbnailUrl: z.string().optional(),
  tags: z.array(z.string()).optional(),
})
export type VideoInfo = z.infer<typeof VideoInfo>

export const TranscriptSegment = z.object({
  start: z.number().describe("Start time in seconds"),
  end: z.number().describe("End time in seconds"),
  text: z.string(),
})
export type TranscriptSegment = z.infer<typeof TranscriptSegment>

export const RssItem = z.object({
  title: z.string(),
  link: z.string().optional(),
  description: z.string().optional(),
  pubDate: z.string().optional(),
  author: z.string().optional(),
  guid: z.string().optional(),
  content: z.string().optional(),
})
export type RssItem = z.infer<typeof RssItem>

export const RssFeed = z.object({
  title: z.string(),
  description: z.string().optional(),
  link: z.string().optional(),
  items: z.array(RssItem),
})
export type RssFeed = z.infer<typeof RssFeed>

export const Tweet = z.object({
  id: z.string(),
  text: z.string(),
  author: z.string(),
  authorHandle: z.string().optional(),
  createdAt: z.string().optional(),
  likeCount: z.number().optional(),
  retweetCount: z.number().optional(),
  replyCount: z.number().optional(),
  mediaUrls: z.array(z.string()).optional(),
})
export type Tweet = z.infer<typeof Tweet>

export const RedditPost = z.object({
  id: z.string(),
  title: z.string(),
  selftext: z.string().optional(),
  author: z.string(),
  subreddit: z.string(),
  score: z.number().optional(),
  numComments: z.number().optional(),
  url: z.string().optional(),
  createdUtc: z.number().optional(),
  isVideo: z.boolean().optional(),
  isSelf: z.boolean().optional(),
})
export type RedditPost = z.infer<typeof RedditPost>

export const RedditComment: z.ZodType<{
  id: string
  author: string
  body: string
  score?: number
  createdUtc?: number
  replies?: RedditComment[]
}> = z.object({
  id: z.string(),
  author: z.string(),
  body: z.string(),
  score: z.number().optional(),
  createdUtc: z.number().optional(),
  replies: z.lazy(() => z.array(RedditComment)).optional(),
})
export type RedditComment = z.infer<typeof RedditComment>
