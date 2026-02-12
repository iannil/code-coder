/**
 * Project Detection Utilities
 *
 * Automatically detects project identifiers from:
 * 1. package.json name field
 * 2. Git repository name
 * 3. Current directory name (fallback)
 */

import path from "path"
import fs from "fs"
import { Log } from "@/util/log"

const log = Log.create({ service: "memory-markdown.project" })

/**
 * Detect project identifier from the current working directory
 *
 * Priority:
 * 1. package.json name (with scope conversion: @scope/name -> scope-name)
 * 2. Git repository name
 * 3. Current directory basename
 */
export async function detectProjectId(cwd = process.cwd()): Promise<string> {
  // Try package.json first
  const pkgName = tryReadPackageName(cwd)
  if (pkgName) {
    log.debug("detected project ID from package.json", { projectId: pkgName })
    return pkgName
  }

  // Try git repo name
  const gitName = await tryGetGitRepoName(cwd)
  if (gitName) {
    log.debug("detected project ID from git", { projectId: gitName })
    return gitName
  }

  // Fallback to directory name
  const dirName = path.basename(cwd)
  log.debug("detected project ID from directory name", { projectId: dirName })
  return dirName
}

/**
 * Detect project identifier synchronously
 *
 * Same as detectProjectId but uses sync operations.
 * Used when async detection is not possible.
 */
export function detectProjectIdSync(cwd = process.cwd()): string {
  // Try package.json first
  const pkgName = tryReadPackageNameSync(cwd)
  if (pkgName) {
    return pkgName
  }

  // Fallback to directory name (git is async-only)
  const dirName = path.basename(cwd)
  return dirName
}

/**
 * Read project name from package.json
 *
 * Converts scoped packages (@scope/name) to unscoped (scope-name)
 */
function tryReadPackageName(cwd: string): string | null {
  return tryReadPackageNameSync(cwd)
}

/**
 * Read project name from package.json (sync)
 *
 * Converts scoped packages (@scope/name) to unscoped (scope-name)
 */
function tryReadPackageNameSync(cwd: string): string | null {
  try {
    const pkgPath = path.join(cwd, "package.json")
    if (!fs.existsSync(pkgPath)) return null

    const text = fs.readFileSync(pkgPath, "utf-8")
    const pkg = JSON.parse(text) as { name?: string }

    if (pkg.name) {
      // Convert scoped packages: @scope/name -> scope-name
      return pkg.name.replace("@", "").replace("/", "-")
    }
  } catch {
    // package.json not found or invalid
  }
  return null
}

/**
 * Get git repository name
 *
 * Returns the basename of the git root directory.
 */
async function tryGetGitRepoName(cwd: string): Promise<string | null> {
  try {
    const proc = Bun.spawn({
      cmd: ["git", "rev-parse", "--show-toplevel"],
      cwd,
      stdout: "pipe",
      stderr: "pipe",
    })

    // Wait for process completion and get exit code
    const exitCode = await proc.exited
    if (exitCode !== 0) return null

    const output = await new Response(proc.stdout).text()
    const gitRoot = output.trim()

    if (gitRoot) {
      return path.basename(gitRoot)
    }
  } catch {
    // git not available or not in a git repo
  }
  return null
}

/**
 * Sanitize project ID for use as a filename or directory name
 *
 * Removes or replaces characters that may cause issues.
 */
export function sanitizeProjectId(projectId: string): string {
  return projectId
    .toLowerCase()
    .replace(/[^a-z0-9-_]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "")
}

/**
 * Get full project context including path and ID
 */
export interface ProjectContext {
  id: string
  sanitizedId: string
  path: string
  gitRoot?: string
}

/**
 * Get full project context
 *
 * Returns project ID, sanitized ID, and path information.
 */
export async function getProjectContext(cwd = process.cwd()): Promise<ProjectContext> {
  const id = await detectProjectId(cwd)
  const sanitizedId = sanitizeProjectId(id)

  const context: ProjectContext = {
    id,
    sanitizedId,
    path: cwd,
  }

  // Try to get git root
  try {
    const proc = Bun.spawn({
      cmd: ["git", "rev-parse", "--show-toplevel"],
      cwd,
      stdout: "pipe",
      stderr: "pipe",
    })

    const exitCode = await proc.exited
    if (exitCode === 0) {
      const output = await new Response(proc.stdout).text()
      const gitRoot = output.trim()
      if (gitRoot) {
        context.gitRoot = gitRoot
      }
    }
  } catch {
    // Not a git repo
  }

  return context
}
