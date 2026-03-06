// Edit Tool - Native Rust implementation only
// Based on approaches from:
// https://github.com/cline/cline/blob/main/evals/diff-edits/diff-apply/diff-06-23-25.ts
// https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/utils/editCorrector.ts
// https://github.com/cline/cline/blob/main/evals/diff-edits/diff-apply/diff-06-26-25.ts

import z from "zod"
import * as path from "path"
import { Tool } from "./tool"
import { LSP } from "../lsp"
import { createTwoFilesPatch, diffLines } from "diff"
import DESCRIPTION from "./edit.txt"
import { File } from "../file"
import { Bus } from "../bus"
import { FileTime } from "../file/time"
import { Filesystem } from "@/util/filesystem"
import { Instance } from "../project/instance"
import { Snapshot } from "@/snapshot"
import { assertExternalDirectory } from "./external-directory"
import { point, runWithChildSpanAsync, functionStart, functionEnd } from "@/observability"
import { replaceWithFuzzyMatch as replaceWithFuzzyMatchNative } from "@codecoder-ai/core"

const MAX_DIAGNOSTICS_PER_FILE = 20

function normalizeLineEndings(text: string): string {
  return text.replaceAll("\r\n", "\n")
}

export const EditTool = Tool.define("edit", {
  description: DESCRIPTION,
  parameters: z.object({
    filePath: z.string().describe("The absolute path to the file to modify"),
    oldString: z.string().describe("The text to replace"),
    newString: z.string().describe("The text to replace it with (must be different from oldString)"),
    replaceAll: z.boolean().optional().describe("Replace all occurrences of oldString (default false)"),
  }),
  async execute(params, ctx) {
    return runWithChildSpanAsync(async () => {
      const startTime = Date.now()
      functionStart("EditTool.execute", {
        filePath: params.filePath,
        oldStringLength: params.oldString.length,
        newStringLength: params.newString.length,
        replaceAll: params.replaceAll,
      })
      point("edit_execute", {
        filePath: params.filePath,
        oldStringLength: params.oldString.length,
        newStringLength: params.newString.length,
        replaceAll: params.replaceAll,
      })

      if (!params.filePath) {
        throw new Error("filePath is required")
      }

      if (params.oldString === params.newString) {
        throw new Error("oldString and newString must be different")
      }

      const filePath = path.isAbsolute(params.filePath) ? params.filePath : path.join(Instance.directory, params.filePath)
      await assertExternalDirectory(ctx, filePath)

      let diff = ""
      let contentOld = ""
      let contentNew = ""
      await FileTime.withLock(filePath, async () => {
        if (params.oldString === "") {
          contentNew = params.newString
          diff = trimDiff(createTwoFilesPatch(filePath, filePath, contentOld, contentNew))
          await ctx.ask({
            permission: "edit",
            patterns: [path.relative(Instance.worktree, filePath)],
            always: ["*"],
            metadata: {
              filepath: filePath,
              diff,
            },
          })
          await Bun.write(filePath, params.newString)
          await Bus.publish(File.Event.Edited, {
            file: filePath,
          })
          FileTime.read(ctx.sessionID, filePath)
          return
        }

        const file = Bun.file(filePath)
        const stats = await file.stat().catch(() => {})
        if (!stats) throw new Error(`File ${filePath} not found`)
        if (stats.isDirectory()) throw new Error(`Path is a directory, not a file: ${filePath}`)
        await FileTime.assert(ctx.sessionID, filePath)
        contentOld = await file.text()
        contentNew = replace(contentOld, params.oldString, params.newString, params.replaceAll)

        diff = trimDiff(
          createTwoFilesPatch(filePath, filePath, normalizeLineEndings(contentOld), normalizeLineEndings(contentNew)),
        )
        await ctx.ask({
          permission: "edit",
          patterns: [path.relative(Instance.worktree, filePath)],
          always: ["*"],
          metadata: {
            filepath: filePath,
            diff,
          },
        })

        await file.write(contentNew)
        await Bus.publish(File.Event.Edited, {
          file: filePath,
        })
        contentNew = await file.text()
        diff = trimDiff(
          createTwoFilesPatch(filePath, filePath, normalizeLineEndings(contentOld), normalizeLineEndings(contentNew)),
        )
        FileTime.read(ctx.sessionID, filePath)
      })

      const filediff: Snapshot.FileDiff = {
        file: filePath,
        before: contentOld,
        after: contentNew,
        additions: 0,
        deletions: 0,
      }
      for (const change of diffLines(contentOld, contentNew)) {
        if (change.added) filediff.additions += change.count || 0
        if (change.removed) filediff.deletions += change.count || 0
      }

      ctx.metadata({
        metadata: {
          diff,
          filediff,
          diagnostics: {},
        },
      })

      let output = "Edit applied successfully."
      await LSP.touchFile(filePath, true)
      const diagnostics = await LSP.diagnostics()
      const normalizedFilePath = Filesystem.normalizePath(filePath)
      const issues = diagnostics[normalizedFilePath] ?? []
      const errors = issues.filter((item) => item.severity === 1)
      if (errors.length > 0) {
        const limited = errors.slice(0, MAX_DIAGNOSTICS_PER_FILE)
        const suffix =
          errors.length > MAX_DIAGNOSTICS_PER_FILE ? `\n... and ${errors.length - MAX_DIAGNOSTICS_PER_FILE} more` : ""
        output += `\n\nLSP errors detected in this file, please fix:\n<diagnostics file="${filePath}">\n${limited.map(LSP.Diagnostic.pretty).join("\n")}${suffix}\n</diagnostics>`
      }

      functionEnd("EditTool.execute", {
        filepath: filePath,
        hasErrors: errors.length > 0,
      }, Date.now() - startTime)

      return {
        metadata: {
          diagnostics,
          diff,
          filediff,
        },
        title: `${path.relative(Instance.worktree, filePath)}`,
        output,
      }
    }) // end runWithChildSpanAsync
  },
})

export function trimDiff(diff: string): string {
  const lines = diff.split("\n")
  const contentLines = lines.filter(
    (line) =>
      (line.startsWith("+") || line.startsWith("-") || line.startsWith(" ")) &&
      !line.startsWith("---") &&
      !line.startsWith("+++"),
  )

  if (contentLines.length === 0) return diff

  let min = Infinity
  for (const line of contentLines) {
    const content = line.slice(1)
    if (content.trim().length > 0) {
      const match = content.match(/^(\s*)/)
      if (match) min = Math.min(min, match[1].length)
    }
  }
  if (min === Infinity || min === 0) return diff
  const trimmedLines = lines.map((line) => {
    if (
      (line.startsWith("+") || line.startsWith("-") || line.startsWith(" ")) &&
      !line.startsWith("---") &&
      !line.startsWith("+++")
    ) {
      const prefix = line[0]
      const content = line.slice(1)
      return prefix + content.slice(min)
    }
    return line
  })

  return trimmedLines.join("\n")
}

/**
 * Replace content using native Rust fuzzy matching implementation.
 * Uses multiple strategies: exact, line-trimmed, block-anchor, whitespace-normalized, etc.
 * @throws Error if native bindings unavailable or no match found
 */
export function replace(content: string, oldString: string, newString: string, replaceAll = false): string {
  if (oldString === newString) {
    throw new Error("oldString and newString must be different")
  }

  if (!replaceWithFuzzyMatchNative) {
    throw new Error("Native bindings required: @codecoder-ai/core replaceWithFuzzyMatch not available")
  }

  // Note: Native replaceWithFuzzyMatch doesn't support replaceAll - it always replaces first match
  // For replaceAll, we implement a loop
  if (replaceAll) {
    let result = content
    let changed = true
    while (changed) {
      const fuzzyResult = replaceWithFuzzyMatchNative(result, oldString, newString)
      if (fuzzyResult.found) {
        result = fuzzyResult.content
      } else {
        changed = false
      }
    }
    if (result === content) {
      throw new Error("oldString not found in content")
    }
    point("replace_native_success", { strategy: "replaceAll" })
    return result
  }

  const result = replaceWithFuzzyMatchNative(content, oldString, newString)

  if (result.found) {
    point("replace_native_success", { strategy: result.strategy })
    return result.content
  }

  // Native failed, throw appropriate error
  if (result.error) {
    throw new Error(result.error)
  }

  throw new Error("oldString not found in content")
}
