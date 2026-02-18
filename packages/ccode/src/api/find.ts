import { Log } from "@/util/log"
import { Cache } from "@/context/cache"
import { glob } from "glob"
import path from "path"

const log = Log.create({ service: "api.find" })

export namespace LocalFind {
  export interface FilesInput {
    query?: string
  }

  export async function files(input: FilesInput): Promise<string[]> {
    const directory = process.cwd()

    // If no query, return recent/common files
    if (!input.query || input.query.trim() === "") {
      try {
        const defaultFiles = await glob("**/*", {
          cwd: directory,
          absolute: false,
          nodir: true,
          ignore: ["**/node_modules/**", "**/.git/**", "**/dist/**", "**/build/**", "**/.next/**", "**/coverage/**"],
        })
        return defaultFiles.slice(0, 100)
      } catch {
        return []
      }
    }

    const query = input.query.trim()

    // Try to use glob pattern matching first
    const patterns = [
      query, // direct pattern
      `**/*${query}*`, // contains pattern
      `**/${query}*`, // starts with pattern
    ]

    try {
      // Use glob for pattern-based file search
      for (const pattern of patterns) {
        const matches = await glob(pattern, {
          cwd: directory,
          absolute: false,
          nodir: true,
          ignore: ["**/node_modules/**", "**/.git/**", "**/dist/**", "**/build/**"],
        })

        if (matches.length > 0) {
          // Return relative paths
          return matches.slice(0, 50) // Limit results
        }
      }
    } catch (error) {
      log.warn("glob search failed, falling back to cache", { error })
    }

    // Fallback to cache search
    const cache = await Cache.get()
    if (!cache) return []

    const results = Object.values(cache.entries)
      .filter((entry) => {
        // Skip non-file entries
        if (entry.type === "route" || entry.type === "component") {
          return entry.path.toLowerCase().includes(query.toLowerCase())
        }
        return true
      })
      .map((entry) => entry.path)
      .filter((p) => p.toLowerCase().includes(query.toLowerCase()))

    return results.slice(0, 50)
  }
}
