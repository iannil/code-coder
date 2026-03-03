import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import type { Checkpoint, CheckpointType } from "../execution/checkpoint"

const log = Log.create({ service: "autonomous.execution.checkpoint" })

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

/**
 * Git operations utility
 *
 * Provides git operations for Autonomous Mode checkpoints
 */
export namespace GitOps {
  /**
   * Get current git status
   */
  export async function getStatus(): Promise<GitStatus> {
    try {
      const { execSync } = require("child_process")

      // Get current branch
      let branch = "main"
      let ahead = 0
      let behind = 0

      try {
        const branchOutput = execSync("git rev-parse --abbrev-ref HEAD", {
          cwd: Instance.worktree,
          encoding: "utf-8",
          stdio: ["pipe", "pipe", "pipe"],
        }).trim()
        branch = branchOutput
      } catch {
        // Ignore error
      }

      try {
        const aheadBehind = execSync("git rev-list --left-right --count origin/${branch}...HEAD", {
          cwd: Instance.worktree,
          encoding: "utf-8",
          stdio: ["pipe", "pipe", "pipe"],
          env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
        }).trim()
        const [behindStr, aheadStr] = aheadBehind.split("\t")
        ahead = parseInt(aheadStr || "0", 10)
        behind = parseInt(behindStr || "0", 10)
      } catch {
        // Ignore error for local branches
      }

      // Get status
      const statusOutput = execSync("git status --porcelain", {
        cwd: Instance.worktree,
        encoding: "utf-8",
      }).trim()

      const modified: string[] = []
      const added: string[] = []
      const deleted: string[] = []
      const renamed = new Map<string, string>()
      const untracked: string[] = []

      if (statusOutput) {
        for (const line of statusOutput.split("\n")) {
          if (!line) continue

          const status = line.slice(0, 2)
          const path = line.slice(3)

          if (path.includes(" -> ")) {
            // Renamed file
            const [oldPath, newPath] = path.split(" -> ")
            renamed.set(oldPath, newPath)
          }

          switch (status[0]) {
            case "M":
              modified.push(path)
              break
            case "A":
              added.push(path)
              break
            case "D":
              deleted.push(path)
              break
            case "R":
              renamed.set(path.split(" -> ")[0], path.split(" -> ")[1])
              break
            case "?":
              untracked.push(path)
              break
          }
        }
      }

      return {
        modified,
        added,
        deleted,
        renamed,
        untracked,
        branch,
        ahead,
        behind,
      }
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
  export async function createCommit(
    message: string,
    options: {
      addAll?: boolean
      allowEmpty?: boolean
    } = {},
  ): Promise<GitCommitResult> {
    try {
      const { execSync } = require("child_process")

      // Add all changes if requested
      if (options.addAll) {
        execSync("git add -A", {
          cwd: Instance.worktree,
          stdio: ["pipe", "pipe", "pipe"],
        })
      }

      // Check if there are changes to commit
      const status = await getStatus()
      const hasChanges = status.modified.length > 0 || status.added.length > 0 || status.deleted.length > 0 || status.renamed.size > 0

      if (!hasChanges && !options.allowEmpty) {
        return {
          success: false,
          error: "No changes to commit",
        }
      }

      // Create commit
      const fullMessage = `[autonomous-mode] ${message}`

      // Use --allow-empty for checkpoints that might not have changes
      const allowEmptyFlag = options.allowEmpty ? "--allow-empty" : ""

      const output = execSync(`git commit ${allowEmptyFlag} -m ${JSON.stringify(fullMessage)}`, {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      })

      // Extract commit hash
      const match = output.match(/\[([a-f0-9]+)\]/)
      const commitHash = match ? match[1] : ""

      log.info("Git commit created", { commitHash, message })

      return {
        success: true,
        commitHash,
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
  export async function getCommits(limit = 10): Promise<Array<{ hash: string; message: string; date: number }>> {
    try {
      const { execSync } = require("child_process")

      const output = execSync(`git log -${limit} --format="%H|%s|%ct"`, {
        cwd: Instance.worktree,
        encoding: "utf-8",
      }).trim()

      const commits: Array<{ hash: string; message: string; date: number }> = []

      for (const line of output.split("\n")) {
        const [hash, message, dateStr] = line.split("|")
        commits.push({
          hash,
          message,
          date: parseInt(dateStr, 10) * 1000,
        })
      }

      return commits
    } catch {
      return []
    }
  }

  /**
   * Get current commit hash
   */
  export async function getCurrentCommit(): Promise<string | undefined> {
    try {
      const { execSync } = require("child_process")

      const hash = execSync("git rev-parse HEAD", {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      }).trim()

      return hash
    } catch {
      return undefined
    }
  }

  /**
   * Reset to a commit
   */
  export async function resetToCommit(
    commitHash: string,
    hard = true,
  ): Promise<{ success: boolean; error?: string }> {
    try {
      const { execSync } = require("child_process")

      const mode = hard ? "--hard" : "--soft"
      execSync(`git reset ${mode} ${commitHash}`, {
        cwd: Instance.worktree,
        stdio: ["pipe", "pipe", "pipe"],
      })

      log.info("Git reset successful", { commitHash, mode })

      return { success: true }
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
  export async function getChangedFiles(
    sinceCommit?: string,
  ): Promise<{ modified: string[]; added: string[]; deleted: string[] }> {
    try {
      const { execSync } = require("child_process")

      let cmd = "git diff --name-only"
      if (sinceCommit) {
        cmd = `git diff --name-only ${sinceCommit}`
      }

      const output = execSync(cmd, {
        cwd: Instance.worktree,
        encoding: "utf-8",
      }).trim()

      const files = output ? output.split("\n").filter(Boolean) : []

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
  export async function isClean(): Promise<boolean> {
    const status = await getStatus()
    return (
      status.modified.length === 0 &&
      status.added.length === 0 &&
      status.deleted.length === 0
    )
  }

  /**
   * Stash changes
   */
  export async function stash(message?: string): Promise<boolean> {
    try {
      const { execSync } = require("child_process")

      const msgArg = message ? ` -m "${message}"` : ""
      execSync(`git stash${msgArg}`, {
        cwd: Instance.worktree,
        stdio: ["pipe", "pipe", "pipe"],
      })

      log.info("Git stash created", { message })
      return true
    } catch (error) {
      log.error("Failed to stash", {
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Unstash changes
   */
  export async function unstash(index = 0): Promise<boolean> {
    try {
      const { execSync } = require("child_process")

      const indexArg = index > 0 ? ` stash@{${index}}` : "stash"
      execSync(`git restore --source=${indexArg} --worktree .`, {
        cwd: Instance.worktree,
        stdio: ["pipe", "pipe", "pipe"],
      })
      execSync(`git stash drop ${indexArg}`, {
        cwd: Instance.worktree,
        stdio: ["pipe", "pipe", "pipe"],
      })

      log.info("Git stash applied", { index })
      return true
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
  export async function init(
    directory: string,
    options: {
      defaultBranch?: string
      initialCommit?: boolean
      commitMessage?: string
    } = {},
  ): Promise<GitInitResult> {
    const { defaultBranch = "main", initialCommit = true, commitMessage = "Initial commit" } = options

    try {
      const { execSync } = require("child_process")

      // Initialize repository
      execSync(`git init -b ${defaultBranch}`, {
        cwd: directory,
        stdio: ["pipe", "pipe", "pipe"],
      })

      log.info("Git repository initialized", { directory, branch: defaultBranch })

      // Create initial commit if requested
      if (initialCommit) {
        // Stage all files
        execSync("git add -A", {
          cwd: directory,
          stdio: ["pipe", "pipe", "pipe"],
        })

        // Check if there are files to commit
        try {
          execSync("git diff --cached --quiet", {
            cwd: directory,
            stdio: ["pipe", "pipe", "pipe"],
          })
          // No changes, skip commit
          log.info("No files to commit, skipping initial commit")
        } catch {
          // There are changes, create commit
          execSync(`git commit -m ${JSON.stringify(commitMessage)}`, {
            cwd: directory,
            stdio: ["pipe", "pipe", "pipe"],
            env: {
              ...process.env,
              GIT_AUTHOR_NAME: "CodeCoder",
              GIT_AUTHOR_EMAIL: "codecoder@local",
              GIT_COMMITTER_NAME: "CodeCoder",
              GIT_COMMITTER_EMAIL: "codecoder@local",
            },
          })
          log.info("Initial commit created", { directory })
        }
      }

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
  export async function clone(
    url: string,
    directory: string,
    options: {
      depth?: number
      branch?: string
      reinitialize?: boolean
    } = {},
  ): Promise<GitCloneResult> {
    const { depth = 1, branch, reinitialize = true } = options

    try {
      const { execSync } = require("child_process")

      // Build clone command
      const args = ["git", "clone"]
      if (depth > 0) args.push("--depth", String(depth))
      if (branch) args.push("--branch", branch)
      args.push(url, directory)

      execSync(args.join(" "), {
        stdio: ["pipe", "pipe", "pipe"],
        env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
      })

      log.info("Repository cloned", { url, directory, depth })

      // Reinitialize: remove origin and clean git history for template use
      if (reinitialize) {
        // Remove the origin remote
        try {
          execSync("git remote remove origin", {
            cwd: directory,
            stdio: ["pipe", "pipe", "pipe"],
          })
          log.info("Removed origin remote", { directory })
        } catch {
          // Origin might not exist, that's fine
        }

        // Remove .git directory and reinitialize for fresh history
        const { rm } = require("fs/promises")
        const path = require("path")
        await rm(path.join(directory, ".git"), { recursive: true, force: true })

        // Initialize fresh repository
        await init(directory, {
          initialCommit: true,
          commitMessage: "[project-scaffold] Initial commit from template",
        })

        log.info("Repository reinitialized for fresh start", { directory })
      }

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
  export async function removeRemote(directory: string, name = "origin"): Promise<{ success: boolean; error?: string }> {
    try {
      const { execSync } = require("child_process")

      execSync(`git remote remove ${name}`, {
        cwd: directory,
        stdio: ["pipe", "pipe", "pipe"],
      })

      log.info("Remote removed", { directory, name })
      return { success: true }
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
  export async function addRemote(
    directory: string,
    name: string,
    url: string,
  ): Promise<{ success: boolean; error?: string }> {
    try {
      const { execSync } = require("child_process")

      execSync(`git remote add ${name} ${url}`, {
        cwd: directory,
        stdio: ["pipe", "pipe", "pipe"],
      })

      log.info("Remote added", { directory, name, url })
      return { success: true }
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
  export async function push(
    directory: string,
    remote = "origin",
    branch = "main",
    setUpstream = true,
  ): Promise<{ success: boolean; error?: string }> {
    try {
      const { execSync } = require("child_process")

      const args = ["git", "push"]
      if (setUpstream) args.push("-u")
      args.push(remote, branch)

      execSync(args.join(" "), {
        cwd: directory,
        stdio: ["pipe", "pipe", "pipe"],
        env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
      })

      log.info("Pushed to remote", { directory, remote, branch })
      return { success: true }
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
  export async function isGitRepo(directory: string): Promise<boolean> {
    try {
      const { execSync } = require("child_process")

      execSync("git rev-parse --git-dir", {
        cwd: directory,
        stdio: ["pipe", "pipe", "pipe"],
      })

      return true
    } catch {
      return false
    }
  }

  /**
   * Get the remote URL
   */
  export async function getRemoteUrl(directory: string, remote = "origin"): Promise<string | null> {
    try {
      const { execSync } = require("child_process")

      const url = execSync(`git remote get-url ${remote}`, {
        cwd: directory,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      }).trim()

      return url
    } catch {
      return null
    }
  }
}
