/**
 * Directory API Handler
 * Handles /api/directories/* endpoints for browsing directories
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import fs from "fs/promises"
import path from "path"
import os from "os"

// ============================================================================
// Types
// ============================================================================

interface DirectoryEntry {
  name: string
  path: string
  isDirectory: boolean
}

interface DirectoryListResponse {
  path: string
  directories: DirectoryEntry[]
  parent: string | null
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Expand home directory (~) in path
 */
function expandHomePath(inputPath: string): string {
  if (inputPath.startsWith("~")) {
    return path.join(os.homedir(), inputPath.slice(1))
  }
  return inputPath
}

/**
 * Get parent directory path, or null if at root
 */
function getParentPath(dirPath: string): string | null {
  const parent = path.dirname(dirPath)
  return parent !== dirPath ? parent : null
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/directories
 * List directories at a given path
 * Query params:
 *   - path: Directory path to list (defaults to home directory)
 */
export async function listDirectories(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const requestedPath = url.searchParams.get("path") ?? os.homedir()
    const dirPath = path.resolve(expandHomePath(requestedPath))

    // Verify the path exists and is a directory
    const stat = await fs.stat(dirPath).catch(() => null)
    if (!stat) {
      return errorResponse(`Path not found: ${dirPath}`, 404)
    }
    if (!stat.isDirectory()) {
      return errorResponse(`Path is not a directory: ${dirPath}`, 400)
    }

    // Read directory contents
    const entries = await fs.readdir(dirPath, { withFileTypes: true })

    // Filter to directories only and sort alphabetically
    const directories: DirectoryEntry[] = entries
      .filter((entry) => {
        // Skip hidden directories that start with .
        if (entry.name.startsWith(".")) return false
        return entry.isDirectory()
      })
      .map((entry) => ({
        name: entry.name,
        path: path.join(dirPath, entry.name),
        isDirectory: true,
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    const response: DirectoryListResponse = {
      path: dirPath,
      directories,
      parent: getParentPath(dirPath),
    }

    return jsonResponse({
      success: true,
      data: response,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
