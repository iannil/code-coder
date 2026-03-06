/**
 * Git Operations - Native Rust bindings via @codecoder-ai/core
 *
 * Provides high-performance git operations using libgit2 instead of shell calls.
 * Phase 8.1: Migrated from child_process.execSync to native NAPI bindings.
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import {
  openGitRepo,
  initGitRepo,
  cloneGitRepo,
  isGitRepo as nativeIsGitRepo,
  type GitOpsHandleType,
  type NapiGitStatus,
  type NapiCommitInfo,
  type NapiInitOptions,
  type NapiCloneOptions,
} from "@codecoder-ai/core"

const log = Log.create({ service: "autonomous.execution.git-ops" })

/**
 * Git checkpoint data
 */
export interface GitCheckpoint {
  id: string
  sessionId: string
  commitHash: string
  message: string
  createdAt: number
  files: string[]
  metadata: Record<string, unknown>
}

/**
 * Git status result
 */
export interface GitStatus {
  modified: string[]
  added: string[]
  deleted: string[]
  renamed: Map<string, string>
  untracked: string[]
  branch: string
  ahead: number
  behind: number
}

/**
 * Git commit result
 */
export interface GitCommitResult {
  success: boolean
  commitHash?: string
  error?: string
}

// Convert native status to our GitStatus interface
function convertStatus(native: NapiGitStatus): GitStatus {
  return {
    modified: native.modified,
    added: native.added,
    deleted: native.deleted,
    renamed: new Map(Object.entries(native.renamed)),
    untracked: native.untracked,
    branch: native.branch,
    ahead: native.ahead,
    behind: native.behind,
  }
}

// Get or create repository handle for the current worktree
function getHandle(directory?: string): GitOpsHandleType {
  const path = directory ?? Instance.worktree
  if (!openGitRepo) {
    throw new Error("Native git bindings not available. Ensure @codecoder-ai/core is built.")
  }
  return openGitRepo(path)
}

/**
 * Git operations utility
 *
 * Provides git operations for Autonomous Mode checkpoints
 */
export namespace GitOps {
  /**
   * Get current git status
   */
  export function getStatus(): GitStatus {
    try {
      const handle = getHandle()
      const status = handle.status()
      return convertStatus(status)
    } catch (error) {
      log.error("Failed to get git status", {
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        modified: [],
        added: [],
        deleted: [],
        renamed: new Map(),
        untracked: [],
        branch: "main",
        ahead: 0,
        behind: 0,
      }
    }
  }

  /**
   * Create a git commit
   */
  export function createCommit(
    message: string,
    options: {
      addAll?: boolean
      allowEmpty?: boolean
    } = {},
  ): GitCommitResult {
    try {
      const handle = getHandle()
      const fullMessage = `[autonomous-mode] ${message}`
      const result = handle.commit(fullMessage, options.addAll ?? false, options.allowEmpty ?? false)

      if (result.success) {
        log.info("Git commit created", { commitHash: result.commitHash, message })
      }

      return {
        success: result.success,
        commitHash: result.commitHash ?? undefined,
        error: result.error ?? undefined,
      }
    } catch (error) {
      log.error("Failed to create git commit", {
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Get list of commits
   */
  export function getCommits(limit = 10): Array<{ hash: string; message: string; date: number }> {
    try {
      const handle = getHandle()
      const commits = handle.commits(limit)
      return commits.map((c: NapiCommitInfo) => ({
        hash: c.hash,
        message: c.message,
        date: c.date * 1000, // Convert to milliseconds
      }))
    } catch {
      return []
    }
  }

  /**
   * Get current commit hash
   */
  export function getCurrentCommit(): string | undefined {
    try {
      const handle = getHandle()
      return handle.currentCommit() ?? undefined
    } catch {
      return undefined
    }
  }

  /**
   * Reset to a commit
   */
  export function resetToCommit(
    commitHash: string,
    hard = true,
  ): { success: boolean; error?: string } {
    try {
      const handle = getHandle()
      const result = handle.reset(commitHash, hard)

      if (result.success) {
        log.info("Git reset successful", { commitHash, mode: hard ? "hard" : "soft" })
      }

      return {
        success: result.success,
        error: result.error ?? undefined,
      }
    } catch (error) {
      log.error("Failed to reset git", {
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Get changed files
   */
  export function getChangedFiles(
    sinceCommit?: string,
  ): { modified: string[]; added: string[]; deleted: string[] } {
    try {
      const handle = getHandle()
      const files = handle.changedFiles(sinceCommit)
      return {
        modified: files,
        added: [],
        deleted: [],
      }
    } catch {
      return {
        modified: [],
        added: [],
        deleted: [],
      }
    }
  }

  /**
   * Check if repo is clean (no uncommitted changes)
   */
  export function isClean(): boolean {
    try {
      const handle = getHandle()
      return handle.isClean()
    } catch {
      return true
    }
  }

  /**
   * Stash changes
   */
  export function stash(message?: string): boolean {
    try {
      const handle = getHandle()
      const result = handle.stash(message)

      if (result.success) {
        log.info("Git stash created", { message })
      }

      return result.success
    } catch (error) {
      log.error("Failed to stash", {
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Unstash changes (pop stash)
   */
  export function unstash(): boolean {
    try {
      const handle = getHandle()
      const result = handle.stashPop()

      if (result.success) {
        log.info("Git stash applied")
      }

      return result.success
    } catch (error) {
      log.error("Failed to unstash", {
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  // ============================================================================
  // Project Creation Operations
  // ============================================================================

  /**
   * Result of git init operation
   */
  export interface GitInitResult {
    success: boolean
    directory: string
    error?: string
  }

  /**
   * Initialize a new git repository
   * @param directory - Directory to initialize (must exist)
   * @param options - Optional settings
   */
  export function init(
    directory: string,
    options: {
      defaultBranch?: string
      initialCommit?: boolean
      commitMessage?: string
    } = {},
  ): GitInitResult {
    const { defaultBranch = "main", initialCommit = true, commitMessage = "Initial commit" } = options

    try {
      if (!initGitRepo) {
        throw new Error("Native git bindings not available")
      }

      const initOptions: NapiInitOptions = {
        defaultBranch,
        initialCommit,
        commitMessage,
      }

      initGitRepo(directory, initOptions)
      log.info("Git repository initialized", { directory, branch: defaultBranch })

      return {
        success: true,
        directory,
      }
    } catch (error) {
      log.error("Failed to initialize git repository", {
        directory,
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        directory,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Result of git clone operation
   */
  export interface GitCloneResult {
    success: boolean
    directory: string
    clonedFrom: string
    error?: string
  }

  /**
   * Clone a repository
   * @param url - Repository URL to clone
   * @param directory - Target directory
   * @param options - Clone options
   */
  export function clone(
    url: string,
    directory: string,
    options: {
      depth?: number
      branch?: string
      reinitialize?: boolean
    } = {},
  ): GitCloneResult {
    const { depth = 1, branch, reinitialize = true } = options

    try {
      if (!cloneGitRepo) {
        throw new Error("Native git bindings not available")
      }

      const cloneOptions: NapiCloneOptions = {
        depth: depth > 0 ? depth : undefined,
        branch,
        reinitialize,
      }

      cloneGitRepo(url, directory, cloneOptions)
      log.info("Repository cloned", { url, directory, depth })

      return {
        success: true,
        directory,
        clonedFrom: url,
      }
    } catch (error) {
      log.error("Failed to clone repository", {
        url,
        directory,
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        directory,
        clonedFrom: url,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Remove a remote
   * @param directory - Repository directory
   * @param name - Remote name (default: "origin")
   */
  export function removeRemote(directory: string, name = "origin"): { success: boolean; error?: string } {
    try {
      const handle = getHandle(directory)
      const result = handle.removeRemote(name)

      if (result.success) {
        log.info("Remote removed", { directory, name })
      }

      return {
        success: result.success,
        error: result.error ?? undefined,
      }
    } catch (error) {
      log.error("Failed to remove remote", {
        directory,
        name,
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Add a remote
   * @param directory - Repository directory
   * @param name - Remote name
   * @param url - Remote URL
   */
  export function addRemote(
    directory: string,
    name: string,
    url: string,
  ): { success: boolean; error?: string } {
    try {
      const handle = getHandle(directory)
      const result = handle.addRemote(name, url)

      if (result.success) {
        log.info("Remote added", { directory, name, url })
      }

      return {
        success: result.success,
        error: result.error ?? undefined,
      }
    } catch (error) {
      log.error("Failed to add remote", {
        directory,
        name,
        url,
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Push to a remote
   * @param directory - Repository directory
   * @param remote - Remote name
   * @param branch - Branch to push
   * @param setUpstream - Set upstream tracking (default: true)
   */
  export function push(
    directory: string,
    remote = "origin",
    branch = "main",
    setUpstream = true,
  ): { success: boolean; error?: string } {
    try {
      const handle = getHandle(directory)
      const result = handle.push(remote, branch, setUpstream)

      if (result.success) {
        log.info("Pushed to remote", { directory, remote, branch })
      }

      return {
        success: result.success,
        error: result.error ?? undefined,
      }
    } catch (error) {
      log.error("Failed to push", {
        directory,
        remote,
        branch,
        error: error instanceof Error ? error.message : String(error),
      })
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /**
   * Check if a directory is a git repository
   */
  export function isGitRepo(directory: string): boolean {
    try {
      if (!nativeIsGitRepo) {
        throw new Error("Native git bindings not available")
      }
      return nativeIsGitRepo(directory)
    } catch {
      return false
    }
  }

  /**
   * Get the remote URL
   */
  export function getRemoteUrl(directory: string, remote = "origin"): string | null {
    try {
      const handle = getHandle(directory)
      return handle.remoteUrl(remote)
    } catch {
      return null
    }
  }
}
