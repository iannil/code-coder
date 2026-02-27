import { spawn } from "child_process"
import { Log } from "@/util/log"
import { Shell } from "@/shell/shell"
import { ReachConfigManager } from "./config"

/**
 * Agent Reach - Utility Functions
 *
 * Shared utilities for command execution, URL parsing, and data formatting
 */

const log = Log.create({ service: "reach.utils" })

const DEFAULT_TIMEOUT = 60_000 // 60 seconds for media operations

export interface ExecResult {
  stdout: string
  stderr: string
  exitCode: number | null
  timedOut: boolean
}

export interface ExecOptions {
  timeout?: number
  cwd?: string
  env?: Record<string, string>
  abort?: AbortSignal
}

/**
 * Execute a command and return structured result
 */
export async function exec(command: string, args: string[], options: ExecOptions = {}): Promise<ExecResult> {
  const { timeout = DEFAULT_TIMEOUT, cwd, env, abort } = options
  const shell = Shell.acceptable()

  // Build full command for shell execution
  const fullCommand = [command, ...args].join(" ")

  return new Promise((resolve) => {
    let stdout = ""
    let stderr = ""
    let timedOut = false
    let resolved = false

    const proc = spawn(fullCommand, {
      shell,
      cwd,
      env: {
        ...process.env,
        ...env,
      },
      stdio: ["ignore", "pipe", "pipe"],
    })

    proc.stdout?.on("data", (chunk: Buffer) => {
      stdout += chunk.toString()
    })

    proc.stderr?.on("data", (chunk: Buffer) => {
      stderr += chunk.toString()
    })

    const timeoutId = setTimeout(() => {
      timedOut = true
      Shell.killTree(proc, { exited: () => resolved })
    }, timeout)

    const abortHandler = () => {
      timedOut = true
      Shell.killTree(proc, { exited: () => resolved })
    }

    abort?.addEventListener("abort", abortHandler, { once: true })

    proc.once("exit", (code) => {
      resolved = true
      clearTimeout(timeoutId)
      abort?.removeEventListener("abort", abortHandler)
      resolve({ stdout, stderr, exitCode: code, timedOut })
    })

    proc.once("error", (error) => {
      resolved = true
      clearTimeout(timeoutId)
      abort?.removeEventListener("abort", abortHandler)
      log.error("command execution failed", { command, error: error.message })
      resolve({ stdout, stderr: error.message, exitCode: 1, timedOut: false })
    })
  })
}

/**
 * Check if a command exists on the system
 */
export async function commandExists(command: string): Promise<boolean> {
  const whichCmd = process.platform === "win32" ? "where" : "which"
  const result = await exec(whichCmd, [command], { timeout: 5000 })
  return result.exitCode === 0
}

/**
 * Get command path if it exists
 */
export async function commandPath(command: string): Promise<string | null> {
  const whichCmd = process.platform === "win32" ? "where" : "which"
  const result = await exec(whichCmd, [command], { timeout: 5000 })
  return result.exitCode === 0 ? result.stdout.trim().split("\n")[0] : null
}

/**
 * Extract video ID from various platform URLs
 */
export function extractVideoId(url: string, platform: "youtube" | "bilibili"): string | null {
  try {
    const parsed = new URL(url)

    if (platform === "youtube") {
      // youtube.com/watch?v=VIDEO_ID
      if (parsed.hostname.includes("youtube.com")) {
        return parsed.searchParams.get("v")
      }
      // youtu.be/VIDEO_ID
      if (parsed.hostname === "youtu.be") {
        return parsed.pathname.slice(1)
      }
    }

    if (platform === "bilibili") {
      // bilibili.com/video/BV1xxxxx or av12345
      const match = parsed.pathname.match(/\/video\/(BV[\w]+|av\d+)/)
      return match ? match[1] : null
    }

    return null
  } catch {
    return null
  }
}

/**
 * Parse Reddit URL to extract subreddit and post info
 */
export function parseRedditUrl(url: string): { subreddit?: string; postId?: string } | null {
  try {
    const parsed = new URL(url)

    if (!parsed.hostname.includes("reddit.com")) {
      return null
    }

    // /r/subreddit/comments/postId/...
    const match = parsed.pathname.match(/\/r\/([^/]+)(?:\/comments\/([^/]+))?/)
    if (match) {
      return { subreddit: match[1], postId: match[2] }
    }

    return null
  } catch {
    return null
  }
}

/**
 * Parse Twitter/X URL to extract tweet ID
 */
export function parseTweetUrl(url: string): string | null {
  try {
    const parsed = new URL(url)

    if (!parsed.hostname.includes("twitter.com") && !parsed.hostname.includes("x.com")) {
      return null
    }

    // /user/status/tweetId
    const match = parsed.pathname.match(/\/\w+\/status\/(\d+)/)
    return match ? match[1] : null
  } catch {
    return null
  }
}

/**
 * Format duration in seconds to human-readable string
 */
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`
  }
  return `${minutes}:${secs.toString().padStart(2, "0")}`
}

/**
 * Format large numbers with K/M suffix
 */
export function formatCount(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}K`
  }
  return count.toString()
}

/**
 * Truncate text to maximum length
 */
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text
  return text.slice(0, maxLength - 3) + "..."
}

/**
 * Build proxy environment variables
 */
export async function getProxyEnv(): Promise<Record<string, string>> {
  const proxy = await ReachConfigManager.getProxy()
  if (!proxy) return {}

  return {
    HTTP_PROXY: proxy,
    HTTPS_PROXY: proxy,
    http_proxy: proxy,
    https_proxy: proxy,
  }
}

/**
 * Safe JSON parse with error handling
 */
export function safeJsonParse<T>(text: string): T | null {
  try {
    return JSON.parse(text) as T
  } catch {
    return null
  }
}

/**
 * Extract JSON from text that may contain other content
 */
export function extractJson(text: string): string | null {
  // Try to find JSON object or array
  const objectMatch = text.match(/\{[\s\S]*\}/)
  const arrayMatch = text.match(/\[[\s\S]*\]/)

  // Return the longer match (more likely to be complete)
  if (objectMatch && arrayMatch) {
    return objectMatch[0].length > arrayMatch[0].length ? objectMatch[0] : arrayMatch[0]
  }

  return objectMatch?.[0] ?? arrayMatch?.[0] ?? null
}

/**
 * Create installation instructions for missing dependencies
 */
export function getInstallInstructions(command: string): string {
  const instructions: Record<string, string> = {
    "yt-dlp": "pip install yt-dlp",
    bird: "npm install -g @anthropics/bird-cli",
  }

  return instructions[command] ?? `Please install ${command}`
}
