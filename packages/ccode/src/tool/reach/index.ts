/**
 * Agent Reach - Internet Access Capabilities
 *
 * Provides tools for accessing various internet platforms:
 *
 * Tier 0 (Zero Config):
 * - YouTube: Video info, transcripts, search (yt-dlp)
 * - Bilibili: B站 video info and transcripts (yt-dlp)
 * - RSS: Read RSS/Atom feeds (built-in parser)
 *
 * Tier 1 (Needs Config):
 * - Twitter/X: Read tweets, search, timeline (bird CLI + cookies)
 * - Reddit: Read posts and comments (JSON API + optional proxy)
 *
 * Tier 2 (Needs MCP):
 * - 小红书: Notes and search via MCP
 * - 抖音: Videos and search via MCP
 * - LinkedIn: Profiles and posts via MCP
 * - Boss直聘: Job listings via MCP
 *
 * @module reach
 */

// Types
export * from "./types"

// Configuration
export { ReachConfigManager } from "./config"

// Diagnostics
export { checkAll, checkChannel, formatReport, getAvailableChannels } from "./doctor"

// Tier 0 Tools (Zero Config)
export { YouTubeTool } from "./youtube"
export { BilibiliTool } from "./bilibili"
export { RssTool } from "./rss"

// Tier 1 Tools (Needs Config)
export { TwitterTool } from "./twitter"
export { RedditTool } from "./reddit"

// Tier 2 Tools (Needs MCP)
export { XiaohongshuTool } from "./xiaohongshu"
export { DouyinTool } from "./douyin"
export { LinkedInTool } from "./linkedin"
export { BossZhipinTool } from "./bosszhipin"

// Utilities
export { commandExists, exec, formatDuration, formatCount } from "./utils"

import { YouTubeTool } from "./youtube"
import { BilibiliTool } from "./bilibili"
import { RssTool } from "./rss"
import { TwitterTool } from "./twitter"
import { RedditTool } from "./reddit"
import { XiaohongshuTool } from "./xiaohongshu"
import { DouyinTool } from "./douyin"
import { LinkedInTool } from "./linkedin"
import { BossZhipinTool } from "./bosszhipin"
import type { Tool } from "../tool"

/**
 * All Reach tools as an array for registration
 */
export const ReachTools: Tool.Info[] = [
  // Tier 0 - Zero Config
  YouTubeTool,
  BilibiliTool,
  RssTool,

  // Tier 1 - Needs Config
  TwitterTool,
  RedditTool,

  // Tier 2 - Needs MCP
  XiaohongshuTool,
  DouyinTool,
  LinkedInTool,
  BossZhipinTool,
]
