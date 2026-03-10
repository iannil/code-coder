// Ripgrep utility functions - Native Rust implementation only
import path from "path"
import z from "zod"

// Native implementations from @codecoder-ai/core
import { grep as nativeGrep, glob as nativeGlob } from "@codecoder-ai/core"

import { Log } from "@/util/log"

/** FileInfo from native glob result */
interface FileInfo {
  path: string
  size: number
  isDir: boolean
  isSymlink: boolean
  modified?: number
  extension?: string
}

/** Native glob result structure matching binding.d.ts */
interface NativeGlobResult {
  files: FileInfo[]
  totalMatches: number
  truncated: boolean
  durationMs: number
}

/** Native grep match entry */
interface NativeGrepMatch {
  path: string
  lineNumber: number
  lineContent: string
}

/** Native grep result - can be array directly or wrapped object */
type NativeGrepResult = NativeGrepMatch[] | { matches: NativeGrepMatch[]; truncated?: boolean }

/** Check if grep result is wrapped object */
function isWrappedGrepResult(result: NativeGrepResult): result is { matches: NativeGrepMatch[] } {
  return !Array.isArray(result) && typeof result === "object" && "matches" in result
}

export namespace Ripgrep {
  const log = Log.create({ service: "ripgrep" })

  export const Match = z.object({
    type: z.literal("match"),
    data: z.object({
      path: z.object({
        text: z.string(),
      }),
      lines: z.object({
        text: z.string(),
      }),
      line_number: z.number(),
      absolute_offset: z.number(),
      submatches: z.array(
        z.object({
          match: z.object({
            text: z.string(),
          }),
          start: z.number(),
          end: z.number(),
        }),
      ),
    }),
  })

  export type Match = z.infer<typeof Match>

  /**
   * List files using native Rust implementation.
   * Uses @codecoder-ai/core glob for SIMD-accelerated file traversal.
   * @throws Error if native bindings unavailable
   */
  export async function* files(input: {
    cwd: string
    glob?: string[]
    hidden?: boolean
    follow?: boolean
    maxDepth?: number
  }) {
    if (!nativeGlob) {
      throw new Error("Native bindings required: @codecoder-ai/core glob not available")
    }

    const pattern = input.glob?.length ? input.glob.join(",") : "**/*"
    // Native API: glob(options: GlobOptions) => Promise<GlobResult>
    const globFn = nativeGlob as unknown as (options: Record<string, unknown>) => Promise<NativeGlobResult>
    const result = await globFn({
      pattern,
      path: input.cwd,
      includeHidden: input.hidden !== false,
      respectGitignore: true,
      maxDepth: input.maxDepth,
      filesOnly: true,
      followSymlinks: input.follow !== false,
    })

    // Result contains FileInfo objects with { path: string, ... }
    for (const fileInfo of result.files) {
      // Return relative path from absolute FileInfo.path
      const relativePath = path.relative(input.cwd, fileInfo.path)
      if (relativePath && !relativePath.startsWith("..")) {
        yield relativePath
      }
    }
  }

  export async function tree(input: { cwd: string; limit?: number }) {
    log.info("tree", input)
    const files = await Array.fromAsync(Ripgrep.files({ cwd: input.cwd }))
    interface Node {
      path: string[]
      children: Node[]
    }

    function getPath(node: Node, parts: string[], create: boolean) {
      if (parts.length === 0) return node
      let current = node
      for (const part of parts) {
        let existing = current.children.find((x) => x.path.at(-1) === part)
        if (!existing) {
          if (!create) return
          existing = {
            path: current.path.concat(part),
            children: [],
          }
          current.children.push(existing)
        }
        current = existing
      }
      return current
    }

    const root: Node = {
      path: [],
      children: [],
    }
    for (const file of files) {
      if (file.includes(".codecoder")) continue
      const parts = file.split(path.sep)
      getPath(root, parts, true)
    }

    function sort(node: Node) {
      node.children.sort((a, b) => {
        if (!a.children.length && b.children.length) return 1
        if (!b.children.length && a.children.length) return -1
        return a.path.at(-1)!.localeCompare(b.path.at(-1)!)
      })
      for (const child of node.children) {
        sort(child)
      }
    }
    sort(root)

    let current = [root]
    const result: Node = {
      path: [],
      children: [],
    }

    let processed = 0
    const limit = input.limit ?? 50
    while (current.length > 0) {
      const next = []
      for (const node of current) {
        if (node.children.length) next.push(...node.children)
      }
      const max = Math.max(...current.map((x) => x.children.length))
      for (let i = 0; i < max && processed < limit; i++) {
        for (const node of current) {
          const child = node.children[i]
          if (!child) continue
          getPath(result, child.path, true)
          processed++
          if (processed >= limit) break
        }
      }
      if (processed >= limit) {
        for (const node of [...current, ...next]) {
          const compare = getPath(result, node.path, false)
          if (!compare) continue
          if (compare?.children.length !== node.children.length) {
            const diff = node.children.length - compare.children.length
            compare.children.push({
              path: compare.path.concat(`[${diff} truncated]`),
              children: [],
            })
          }
        }
        break
      }
      current = next
    }

    const lines: string[] = []

    function render(node: Node, depth: number) {
      const indent = "\t".repeat(depth)
      lines.push(indent + node.path.at(-1) + (node.children.length ? "/" : ""))
      for (const child of node.children) {
        render(child, depth + 1)
      }
    }
    result.children.map((x) => render(x, 0))

    return lines.join("\n")
  }

  /**
   * Search for patterns using native Rust implementation.
   * Uses @codecoder-ai/core grep for parallel, SIMD-accelerated search.
   * @throws Error if native bindings unavailable
   */
  export async function search(input: {
    cwd: string
    pattern: string
    glob?: string[]
    limit?: number
    follow?: boolean
  }) {
    if (!nativeGrep) {
      throw new Error("Native bindings required: @codecoder-ai/core grep not available")
    }

    // Native API: grep(pattern, path, options?)
    // Cast through unknown: nativeGrep has overloaded signatures, we use the async variant
    const grepFn = nativeGrep as unknown as (
      pattern: string,
      path: string,
      options?: Record<string, unknown>,
    ) => Promise<NativeGrepResult>
    const result = await grepFn(input.pattern, input.cwd, {
      glob: input.glob?.join(","),
      limit: input.limit,
      outputMode: "content",
      lineNumbers: true,
    })

    // Handle result as array or wrapped object
    const matches = isWrappedGrepResult(result) ? result.matches : result
    return matches.map((m) => ({
      path: { text: m.path },
      lines: { text: m.lineContent },
      line_number: m.lineNumber,
      absolute_offset: 0,
      submatches: [
        {
          match: { text: input.pattern },
          start: 0,
          end: input.pattern.length,
        },
      ],
    }))
  }
}
