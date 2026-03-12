import fs from "fs/promises"
import path from "path"
import { Global } from "@/util/global"
import { Identifier } from "@/util/id/id"
import { PermissionNext } from "@/security/permission/next"
import type { AgentInfoType } from "@/sdk/agent-bridge"
import { Scheduler } from "@/infrastructure/scheduler"

// Try to import native bindings
let nativeTruncate: typeof import("@codecoder-ai/core").truncatePreview | undefined
let nativeAvailable = false

try {
  const core = await import("@codecoder-ai/core")
  nativeTruncate = core.truncatePreview
  nativeAvailable = typeof nativeTruncate === "function"
} catch {
  // Native bindings not available
}

export namespace Truncate {
  export const MAX_LINES = 2000
  export const MAX_BYTES = 50 * 1024
  export const DIR = path.join(Global.Path.data, "tool-output")
  export const GLOB = path.join(DIR, "*")
  const RETENTION_MS = 7 * 24 * 60 * 60 * 1000 // 7 days
  const HOUR_MS = 60 * 60 * 1000

  export type Result = { content: string; truncated: false } | { content: string; truncated: true; outputPath: string }

  export interface Options {
    maxLines?: number
    maxBytes?: number
    direction?: "head" | "tail"
  }

  /** Whether native truncation is available */
  export const isNative = nativeAvailable

  export function init() {
    Scheduler.register({
      id: "tool.truncation.cleanup",
      interval: HOUR_MS,
      run: cleanup,
      scope: "global",
    })
  }

  export async function cleanup() {
    const cutoff = Identifier.timestamp(Identifier.create("tool", false, Date.now() - RETENTION_MS))
    const glob = new Bun.Glob("tool_*")
    const entries = await Array.fromAsync(glob.scan({ cwd: DIR, onlyFiles: true })).catch(() => [] as string[])
    for (const entry of entries) {
      if (Identifier.timestamp(entry) >= cutoff) continue
      await fs.unlink(path.join(DIR, entry)).catch(() => {})
    }
  }

  function hasTaskTool(agent?: AgentInfoType): boolean {
    if (!agent?.permission) return false
    const rule = PermissionNext.evaluate("task", "*", agent.permission)
    return rule.action !== "deny"
  }

  /**
   * Truncate output using native Rust implementation when available.
   * Falls back to TypeScript implementation if native bindings are not loaded.
   */
  export async function output(text: string, options: Options = {}, agent?: AgentInfoType): Promise<Result> {
    const maxLines = options.maxLines ?? MAX_LINES
    const maxBytes = options.maxBytes ?? MAX_BYTES
    const direction = options.direction ?? "head"

    // Quick check if truncation is needed
    const lines = text.split("\n")
    const totalBytes = Buffer.byteLength(text, "utf-8")

    if (lines.length <= maxLines && totalBytes <= maxBytes) {
      return { content: text, truncated: false }
    }

    // Use native truncation for the core logic when available
    let preview: string
    let removed: number
    let unit: string

    if (nativeAvailable && nativeTruncate) {
      const result = nativeTruncate(text, maxLines, maxBytes, direction)
      preview = result.content
      // Extract the preview content (before the truncation message)
      const truncatedMatch = result.content.match(/\n\n\.\.\.\d+ (bytes|lines) truncated/)
      if (truncatedMatch) {
        preview = result.content.substring(0, truncatedMatch.index)
      }
      removed = result.bytesRemoved > 0 ? result.bytesRemoved : result.linesRemoved
      unit = result.bytesRemoved > 0 ? "bytes" : "lines"
    } else {
      // Fallback to TypeScript implementation
      const { content, removedCount, hitBytes } = truncateTs(text, maxLines, maxBytes, direction)
      preview = content
      removed = removedCount
      unit = hitBytes ? "bytes" : "lines"
    }

    // Save full output to file
    const id = Identifier.ascending("tool")
    const filepath = path.join(DIR, id)
    await Bun.write(Bun.file(filepath), text)

    // Generate context-aware hint
    const hint = hasTaskTool(agent)
      ? `The tool call succeeded but the output was truncated. Full output saved to: ${filepath}\nUse the Task tool to have explore agent process this file with Grep and Read (with offset/limit). Do NOT read the full file yourself - delegate to save context.`
      : `The tool call succeeded but the output was truncated. Full output saved to: ${filepath}\nUse Grep to search the full content or Read with offset/limit to view specific sections.`

    const message =
      direction === "head"
        ? `${preview}\n\n...${removed} ${unit} truncated...\n\n${hint}`
        : `...${removed} ${unit} truncated...\n\n${hint}\n\n${preview}`

    return { content: message, truncated: true, outputPath: filepath }
  }

  /**
   * TypeScript fallback implementation for truncation.
   * Used when native bindings are not available.
   */
  function truncateTs(
    text: string,
    maxLines: number,
    maxBytes: number,
    direction: "head" | "tail",
  ): { content: string; removedCount: number; hitBytes: boolean } {
    const lines = text.split("\n")
    const totalBytes = Buffer.byteLength(text, "utf-8")
    const out: string[] = []
    let bytes = 0
    let hitBytes = false

    if (direction === "head") {
      for (let i = 0; i < lines.length && i < maxLines; i++) {
        const size = Buffer.byteLength(lines[i], "utf-8") + (i > 0 ? 1 : 0)
        if (bytes + size > maxBytes) {
          hitBytes = true
          break
        }
        out.push(lines[i])
        bytes += size
      }
    } else {
      for (let i = lines.length - 1; i >= 0 && out.length < maxLines; i--) {
        const size = Buffer.byteLength(lines[i], "utf-8") + (out.length > 0 ? 1 : 0)
        if (bytes + size > maxBytes) {
          hitBytes = true
          break
        }
        out.unshift(lines[i])
        bytes += size
      }
    }

    const removedCount = hitBytes ? totalBytes - bytes : lines.length - out.length

    return {
      content: out.join("\n"),
      removedCount,
      hitBytes,
    }
  }
}
