import z from "zod"
import { Tool } from "./tool"
import { grep as nativeGrep } from "@codecoder-ai/core"

import DESCRIPTION from "./grep.txt"
import { Instance } from "../project/instance"
import path from "path"
import { assertExternalDirectory } from "./external-directory"
import {
  runWithChildSpanAsync,
  functionStart,
  functionEnd,
} from "@/observability"

const MAX_LINE_LENGTH = 2000

/** Native grep match entry */
interface GrepMatch {
  path: string
  lineNumber: number
  lineContent: string
}

/** Native grep result - can be array directly or wrapped object */
type GrepResult = GrepMatch[] | { matches: GrepMatch[]; truncated?: boolean }

/** Type guard: check if result is wrapped object */
function isWrappedResult(result: GrepResult): result is { matches: GrepMatch[]; truncated?: boolean } {
  return !Array.isArray(result) && typeof result === "object" && "matches" in result
}

export const GrepTool = Tool.define("grep", {
  description: DESCRIPTION,
  parameters: z.object({
    pattern: z.string().describe("The regex pattern to search for in file contents"),
    path: z.string().optional().describe("The directory to search in. Defaults to the current working directory."),
    include: z.string().optional().describe('File pattern to include in the search (e.g. "*.js", "*.{ts,tsx}")'),
  }),
  async execute(params, ctx) {
    return runWithChildSpanAsync(async () => {
      const startTime = Date.now()
      functionStart("GrepTool.execute", {
        pattern: params.pattern,
        path: params.path,
        include: params.include,
      })

    if (!params.pattern) {
      throw new Error("pattern is required")
    }

    if (!nativeGrep) {
      throw new Error("Native bindings required: @codecoder-ai/core grep not available")
    }

    await ctx.ask({
      permission: "grep",
      patterns: [params.pattern],
      always: ["*"],
      metadata: {
        pattern: params.pattern,
        path: params.path,
        include: params.include,
      },
    })

    let searchPath = params.path ?? Instance.directory
    searchPath = path.isAbsolute(searchPath) ? searchPath : path.resolve(Instance.directory, searchPath)
    await assertExternalDirectory(ctx, searchPath, { kind: "directory" })

    // Native API: grep(pattern, path, options?)
    // Cast through unknown: nativeGrep has overloaded signatures, we use the async variant
    const grepFn = nativeGrep as unknown as (
      pattern: string,
      path: string,
      options?: Record<string, unknown>,
    ) => Promise<GrepResult>
    const result = await grepFn(params.pattern, searchPath, {
      glob: params.include,
      outputMode: "content",
      lineNumbers: true,
      limit: 100,
    })

    // Handle result as array or wrapped object
    const rawMatches = isWrappedResult(result) ? result.matches : result
    if (rawMatches.length === 0) {
      return {
        title: params.pattern,
        metadata: { matches: 0, truncated: false },
        output: "No files found",
      }
    }

    // Get modification times for sorting
    const matches: Array<{
      path: string
      modTime: number
      lineNum: number
      lineText: string
    }> = []

    for (const match of rawMatches) {
      const file = Bun.file(match.path)
      const stats = await file.stat().catch(() => null)
      if (!stats) continue

      matches.push({
        path: match.path,
        modTime: stats.mtime.getTime(),
        lineNum: match.lineNumber,
        lineText: match.lineContent,
      })
    }

    matches.sort((a, b) => b.modTime - a.modTime)

    const truncated = isWrappedResult(result) && result.truncated === true
    const outputLines = [`Found ${matches.length} matches`]

    let currentFile = ""
    for (const match of matches) {
      if (currentFile !== match.path) {
        if (currentFile !== "") {
          outputLines.push("")
        }
        currentFile = match.path
        outputLines.push(`${match.path}:`)
      }
      const truncatedLineText =
        match.lineText.length > MAX_LINE_LENGTH ? match.lineText.substring(0, MAX_LINE_LENGTH) + "..." : match.lineText
      outputLines.push(`  Line ${match.lineNum}: ${truncatedLineText}`)
    }

    if (truncated) {
      outputLines.push("")
      outputLines.push("(Results are truncated. Consider using a more specific path or pattern.)")
    }

    functionEnd("GrepTool.execute", {
      matches: matches.length,
      truncated,
    }, Date.now() - startTime)

    return {
      title: params.pattern,
      metadata: {
        matches: matches.length,
        truncated,
      },
      output: outputLines.join("\n"),
    }
    }) // end runWithChildSpanAsync
  },
})
