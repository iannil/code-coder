/**
 * Workspace path resolution utilities.
 *
 * Centralizes all runtime-generated data paths under a single workspace directory.
 * Supports configuration via ~/.codecoder/config.json and CODECODER_WORKSPACE env var.
 */

import os from "os"
import path from "path"
import { promises as fs } from "fs"

/**
 * Workspace path structure.
 * All paths are absolute after resolution.
 */
export interface WorkspacePaths {
  /** Root workspace directory */
  root: string
  /** Hands definitions and output directory */
  hands: string
  /** Persistent storage (sessions, messages, etc.) */
  storage: string
  /** Log files directory */
  log: string
  /** Tool execution output directory */
  toolOutput: string
  /** Knowledge base storage */
  knowledge: string
  /** Execution tracking data */
  tracking: string
  /** MCP authentication storage file */
  mcpAuth: string
  /** Cache directory */
  cache: string
}

/** Default workspace path */
const DEFAULT_WORKSPACE = "~/.codecoder/workspace"

/** Default subdirectory names */
const DEFAULT_SUBDIRS = {
  hands: "hands",
  storage: "storage",
  log: "log",
  tool_output: "tool-output",
  knowledge: "knowledge",
  tracking: "tracking",
  mcp_auth: "mcp-auth.json",
  cache: "cache",
} as const

/**
 * Expand tilde (~) to home directory.
 * @param filePath - Path possibly starting with ~
 * @returns Absolute path with ~ expanded
 */
export function expandTilde(filePath: string): string {
  if (filePath.startsWith("~/")) {
    return path.join(os.homedir(), filePath.slice(2))
  }
  return filePath
}

/**
 * Get the workspace root directory.
 * Priority: CODECODER_WORKSPACE env var > config.path > default
 * @param configWorkspace - Workspace config from config.json
 * @returns Absolute path to workspace root
 */
export function getWorkspaceRoot(configWorkspace?: { path?: string }): string {
  if (process.env.CODECODER_WORKSPACE) {
    return expandTilde(process.env.CODECODER_WORKSPACE)
  }
  if (configWorkspace?.path) {
    return expandTilde(configWorkspace.path)
  }
  return expandTilde(DEFAULT_WORKSPACE)
}

/**
 * Resolve all workspace paths from configuration.
 * @param configWorkspace - Workspace config from config.json
 * @returns Resolved workspace paths
 */
export function resolveWorkspacePaths(
  configWorkspace?: { path?: string; subdirs?: typeof DEFAULT_SUBDIRS },
): WorkspacePaths {
  const root = getWorkspaceRoot(configWorkspace)
  const subdirs = { ...DEFAULT_SUBDIRS, ...configWorkspace?.subdirs }

  return {
    root,
    hands: path.join(root, subdirs.hands),
    storage: path.join(root, subdirs.storage),
    log: path.join(root, subdirs.log),
    toolOutput: path.join(root, subdirs.tool_output),
    knowledge: path.join(root, subdirs.knowledge),
    tracking: path.join(root, subdirs.tracking),
    mcpAuth: path.join(root, subdirs.mcp_auth),
    cache: path.join(root, subdirs.cache),
  }
}

/**
 * Ensure workspace directories exist.
 * Creates all subdirectories if they don't exist.
 * @param paths - Optional workspace paths (defaults to resolved paths)
 */
export async function ensureWorkspace(paths?: WorkspacePaths): Promise<void> {
  const workspacePaths = paths || resolveWorkspacePaths()

  await fs.mkdir(workspacePaths.root, { recursive: true })
  await fs.mkdir(workspacePaths.hands, { recursive: true })
  await fs.mkdir(workspacePaths.storage, { recursive: true })
  await fs.mkdir(workspacePaths.log, { recursive: true })
  await fs.mkdir(workspacePaths.toolOutput, { recursive: true })
  await fs.mkdir(workspacePaths.knowledge, { recursive: true })
  await fs.mkdir(workspacePaths.tracking, { recursive: true })
  await fs.mkdir(workspacePaths.cache, { recursive: true })
  // Note: mcpAuth is a file, not a directory, so we don't create it
}

/**
 * Get workspace paths from environment variable only.
 * Useful for testing or when config is not loaded.
 * @returns Workspace paths using CODECODER_WORKSPACE env var or default
 */
export function getWorkspacePathsFromEnv(): WorkspacePaths {
  const root = process.env.CODECODER_WORKSPACE
    ? expandTilde(process.env.CODECODER_WORKSPACE)
    : expandTilde(DEFAULT_WORKSPACE)

  return {
    root,
    hands: path.join(root, DEFAULT_SUBDIRS.hands),
    storage: path.join(root, DEFAULT_SUBDIRS.storage),
    log: path.join(root, DEFAULT_SUBDIRS.log),
    toolOutput: path.join(root, DEFAULT_SUBDIRS.tool_output),
    knowledge: path.join(root, DEFAULT_SUBDIRS.knowledge),
    tracking: path.join(root, DEFAULT_SUBDIRS.tracking),
    mcpAuth: path.join(root, DEFAULT_SUBDIRS.mcp_auth),
    cache: path.join(root, DEFAULT_SUBDIRS.cache),
  }
}
