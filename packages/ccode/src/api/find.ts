/**
 * File Find API
 *
 * Simple file search using glob patterns.
 *
 * @module api/find
 */

import { Log } from "@/util/log"
import { glob } from "glob"

const log = Log.create({ service: "api.find" })

export namespace LocalFind {
  export interface FilesInput {
    query?: string
  }

  export async function files(input: FilesInput): Promise<string[]> {
    const directory = process.cwd()

    // Common ignore patterns
    const ignorePatterns = [
      "**/node_modules/**",
      "**/.git/**",
      "**/dist/**",
      "**/build/**",
      "**/.next/**",
      "**/coverage/**",
      "**/target/**",
      "**/.turbo/**",
    ]

    // If no query, return recent/common files
    if (!input.query || input.query.trim() === "") {
      try {
        const defaultFiles = await glob("**/*", {
          cwd: directory,
          absolute: false,
          nodir: true,
          ignore: ignorePatterns,
        })
        return defaultFiles.slice(0, 100)
      } catch {
        return []
      }
    }

    const query = input.query.trim()

    // Try to use glob pattern matching
    const patterns = [
      query, // direct pattern
      `**/*${query}*`, // contains pattern
      `**/${query}*`, // starts with pattern
    ]

    try {
      for (const pattern of patterns) {
        const matches = await glob(pattern, {
          cwd: directory,
          absolute: false,
          nodir: true,
          ignore: ignorePatterns,
        })

        if (matches.length > 0) {
          return matches.slice(0, 50)
        }
      }
    } catch (error) {
      log.warn("glob search failed", { error })
    }

    return []
  }
}
