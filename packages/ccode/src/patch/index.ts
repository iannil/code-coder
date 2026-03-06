/**
 * Patch Module
 *
 * Provides patch parsing and application using native Rust bindings.
 * Requires @codecoder-ai/core native implementation.
 */

import z from "zod"
import * as path from "path"
import * as fs from "fs/promises"
import { Log } from "@/util/log"
import {
  createPatchApplicator,
  applyPatchNative,
  type NapiPatchHunk,
  type NapiApplyPatchOptions,
  type NapiApplyPatchResult,
  type PatchApplicatorHandle,
} from "./native"

export namespace Patch {
  const log = Log.create({ service: "patch" })

  // ============================================================================
  // Schema & Types
  // ============================================================================

  export const PatchSchema = z.object({
    patchText: z.string().describe("The full patch text that describes all changes to be made"),
  })

  export type PatchParams = z.infer<typeof PatchSchema>

  export interface ApplyPatchArgs {
    patch: string
    hunks: Hunk[]
    workdir?: string
  }

  export type Hunk =
    | { type: "add"; path: string; contents: string }
    | { type: "delete"; path: string }
    | { type: "update"; path: string; move_path?: string; chunks: UpdateFileChunk[] }

  export interface UpdateFileChunk {
    old_lines: string[]
    new_lines: string[]
    change_context?: string
    is_end_of_file?: boolean
  }

  export interface AffectedPaths {
    added: string[]
    modified: string[]
    deleted: string[]
  }

  export enum ApplyPatchError {
    ParseError = "ParseError",
    IoError = "IoError",
    ComputeReplacements = "ComputeReplacements",
    ImplicitInvocation = "ImplicitInvocation",
  }

  export enum MaybeApplyPatch {
    Body = "Body",
    ShellParseError = "ShellParseError",
    PatchParseError = "PatchParseError",
    NotApplyPatch = "NotApplyPatch",
  }

  // ============================================================================
  // Native Integration
  // ============================================================================

  let applicator: PatchApplicatorHandle | null = null

  async function getApplicator(): Promise<PatchApplicatorHandle> {
    if (applicator === null) {
      applicator = await createPatchApplicator()
    }
    if (!applicator) {
      throw new Error("Native bindings required: @codecoder-ai/core PatchApplicator not available")
    }
    return applicator
  }

  /**
   * Convert NAPI hunk to TypeScript Hunk type
   */
  function convertNapiHunk(napiHunk: NapiPatchHunk): Hunk {
    switch (napiHunk.patchType) {
      case "add":
        return {
          type: "add",
          path: napiHunk.path,
          contents: napiHunk.content ?? "",
        }
      case "delete":
        return {
          type: "delete",
          path: napiHunk.path,
        }
      case "update":
      case "move":
        return {
          type: "update",
          path: napiHunk.path,
          move_path: napiHunk.movePath,
          chunks: napiHunk.chunks.map((c) => ({
            old_lines: [...c.contextBefore, ...c.removals],
            new_lines: [...c.contextBefore, ...c.additions],
            change_context: c.contextBefore[0],
            is_end_of_file: false,
          })),
        }
    }
  }

  /**
   * Derive new file contents from update chunks.
   * This is used for preview/verification before applying.
   */
  export function deriveNewContentsFromChunks(
    filePath: string,
    chunks: UpdateFileChunk[],
  ): { unified_diff: string; content: string } {
    const fs = require("fs")
    let originalContent: string
    try {
      originalContent = fs.readFileSync(filePath, "utf-8")
    } catch (error) {
      throw new Error(`Failed to read file ${filePath}: ${error}`)
    }

    let lines = originalContent.split("\n")
    // Remove trailing empty line if present
    if (lines.length > 0 && lines[lines.length - 1] === "") {
      lines.pop()
    }

    // Apply each chunk's changes
    for (const chunk of chunks) {
      const oldPattern = chunk.old_lines.join("\n")
      const newPattern = chunk.new_lines.join("\n")
      const content = lines.join("\n")

      // Try exact match first
      if (content.includes(oldPattern)) {
        lines = content.replace(oldPattern, newPattern).split("\n")
      } else {
        // Try line-by-line trimmed matching
        let found = false
        for (let i = 0; i <= lines.length - chunk.old_lines.length; i++) {
          let matches = true
          for (let j = 0; j < chunk.old_lines.length; j++) {
            if (lines[i + j].trim() !== chunk.old_lines[j].trim()) {
              matches = false
              break
            }
          }
          if (matches) {
            lines.splice(i, chunk.old_lines.length, ...chunk.new_lines)
            found = true
            break
          }
        }
        if (!found) {
          throw new Error(`Failed to find expected lines in ${filePath}:\n${chunk.old_lines.join("\n")}`)
        }
      }
    }

    // Ensure trailing newline
    if (lines.length === 0 || lines[lines.length - 1] !== "") {
      lines.push("")
    }

    const newContent = lines.join("\n")
    return { unified_diff: "", content: newContent }
  }

  // ============================================================================
  // Public API
  // ============================================================================

  /**
   * Parse a patch text and return hunks.
   * Uses native Rust implementation.
   * @throws Error if native bindings unavailable
   */
  export async function parsePatchAsync(patchText: string): Promise<{ hunks: Hunk[] }> {
    const app = await getApplicator()
    const napiHunks = app.parsePatch(patchText)
    return { hunks: napiHunks.map(convertNapiHunk) }
  }

  /**
   * Parse a patch text synchronously.
   * Uses native Rust implementation.
   * @throws Error if native bindings unavailable
   */
  export function parsePatch(patchText: string): { hunks: Hunk[] } {
    if (!applicator) {
      throw new Error("Native bindings required: call parsePatchAsync first to initialize, or use applyPatchNativeWrapper")
    }
    const napiHunks = applicator.parsePatch(patchText)
    return { hunks: napiHunks.map(convertNapiHunk) }
  }

  /**
   * Apply a patch using native implementation.
   * @throws Error if native bindings unavailable
   */
  export async function applyPatchNativeWrapper(
    patchText: string,
    options?: NapiApplyPatchOptions,
  ): Promise<NapiApplyPatchResult> {
    const result = await applyPatchNative(patchText, options)
    if (!result) {
      throw new Error("Native bindings required: @codecoder-ai/core applyPatch not available")
    }
    return result
  }

  /**
   * Detect if a command is an apply_patch invocation.
   */
  export async function maybeParseApplyPatch(
    argv: string[],
  ): Promise<
    | { type: MaybeApplyPatch.Body; args: ApplyPatchArgs }
    | { type: MaybeApplyPatch.PatchParseError; error: Error }
    | { type: MaybeApplyPatch.NotApplyPatch }
  > {
    const APPLY_PATCH_COMMANDS = ["apply_patch", "applypatch"]

    if (argv.length === 2 && APPLY_PATCH_COMMANDS.includes(argv[0])) {
      try {
        const { hunks } = await parsePatchAsync(argv[1])
        return { type: MaybeApplyPatch.Body, args: { patch: argv[1], hunks } }
      } catch (error) {
        return { type: MaybeApplyPatch.PatchParseError, error: error as Error }
      }
    }

    if (argv.length === 3 && argv[0] === "bash" && argv[1] === "-lc") {
      const script = argv[2]
      const heredocMatch = script.match(/apply_patch\s*<<['"](\w+)['"]\s*\n([\s\S]*?)\n\1/)

      if (heredocMatch) {
        const patchContent = heredocMatch[2]
        try {
          const { hunks } = await parsePatchAsync(patchContent)
          return { type: MaybeApplyPatch.Body, args: { patch: patchContent, hunks } }
        } catch (error) {
          return { type: MaybeApplyPatch.PatchParseError, error: error as Error }
        }
      }
    }

    return { type: MaybeApplyPatch.NotApplyPatch }
  }

  /**
   * Apply hunks directly to files using native implementation.
   * Uses native applyPatch for the actual file operations.
   */
  export async function applyHunksToFiles(hunks: Hunk[]): Promise<AffectedPaths> {
    if (hunks.length === 0) {
      throw new Error("No files were modified.")
    }

    const added: string[] = []
    const modified: string[] = []
    const deleted: string[] = []

    for (const hunk of hunks) {
      switch (hunk.type) {
        case "add": {
          const addDir = path.dirname(hunk.path)
          if (addDir !== "." && addDir !== "/") {
            await fs.mkdir(addDir, { recursive: true })
          }
          await fs.writeFile(hunk.path, hunk.contents, "utf-8")
          added.push(hunk.path)
          log.info(`Added file: ${hunk.path}`)
          break
        }

        case "delete": {
          await fs.unlink(hunk.path)
          deleted.push(hunk.path)
          log.info(`Deleted file: ${hunk.path}`)
          break
        }

        case "update": {
          // For updates, use native patch application
          // Reconstruct a minimal patch for this single file
          const app = await getApplicator()

          // Read current content
          const currentContent = await fs.readFile(hunk.path, "utf-8")

          // Apply chunks manually using line replacement
          let lines = currentContent.split("\n")
          for (const chunk of hunk.chunks) {
            // Find and replace the old lines with new lines
            const oldPattern = chunk.old_lines.join("\n")
            const newPattern = chunk.new_lines.join("\n")
            const content = lines.join("\n")
            lines = content.replace(oldPattern, newPattern).split("\n")
          }

          const newContent = lines.join("\n")

          if (hunk.move_path) {
            const moveDir = path.dirname(hunk.move_path)
            if (moveDir !== "." && moveDir !== "/") {
              await fs.mkdir(moveDir, { recursive: true })
            }
            await fs.writeFile(hunk.move_path, newContent, "utf-8")
            await fs.unlink(hunk.path)
            modified.push(hunk.move_path)
            log.info(`Moved file: ${hunk.path} -> ${hunk.move_path}`)
          } else {
            await fs.writeFile(hunk.path, newContent, "utf-8")
            modified.push(hunk.path)
            log.info(`Updated file: ${hunk.path}`)
          }
          break
        }
      }
    }

    return { added, modified, deleted }
  }
}
