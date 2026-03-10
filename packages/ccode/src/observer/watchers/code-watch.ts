/**
 * Code Watcher (CodeWatch)
 *
 * Observes the codebase for changes including:
 * - Git commits and changes
 * - Build status
 * - Test coverage
 * - Technical debt indicators
 * - File changes
 *
 * @deprecated This TypeScript implementation is deprecated in favor of the Rust
 * implementation in services/zero-cli/src/observer/watchers/code_watch.rs. The Rust
 * implementation provides better performance with native git2 integration.
 * Migration was completed in Phase 6-7 of the architecture refactoring.
 *
 * @module observer/watchers/code-watch
 */

import { Log } from "@/util/log"
import { BaseWatcher, type WatcherOptions } from "./base-watcher"
import type { CodeObservation, CodeObservationType } from "../types"
import { Bus } from "@/bus"

const log = Log.create({ service: "observer.code-watch" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface CodeWatchOptions extends WatcherOptions {
  /** Watch specific paths */
  watchPaths?: string[]
  /** Git repository root */
  gitRoot?: string
  /** Track build status */
  trackBuild?: boolean
  /** Track test coverage */
  trackTests?: boolean
  /** Enable periodic typecheck (default: false) */
  enableTypecheck?: boolean
  /** Typecheck interval in ms (default: 60000) */
  typecheckIntervalMs?: number
  /** Typecheck timeout in ms (default: 30000) */
  typecheckTimeoutMs?: number
}

interface GitChange {
  hash: string
  message: string
  author: string
  date: Date
  files: Array<{
    path: string
    action: "add" | "modify" | "delete"
    additions?: number
    deletions?: number
  }>
}

// ─────────────────────────────────────────────────────────────────────────────
// CodeWatch Implementation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Watcher that observes codebase changes.
 */
export class CodeWatch extends BaseWatcher<CodeObservation> {
  private lastCommitHash: string | null = null
  private gitRoot: string | null = null
  private fileWatchSubscription: (() => void) | null = null
  private bashSubscription: (() => void) | null = null
  private typecheckTimer: ReturnType<typeof setInterval> | null = null
  private options_: CodeWatchOptions

  constructor(options: CodeWatchOptions = {}) {
    super("code", {
      intervalMs: 30000, // Check every 30 seconds by default
      ...options,
    })
    this.gitRoot = options.gitRoot ?? null
    this.options_ = {
      enableTypecheck: false,
      typecheckIntervalMs: 60000, // 1 minute
      typecheckTimeoutMs: 30000, // 30 seconds
      ...options,
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────────

  protected override async onStart(): Promise<void> {
    // Initialize git root if not set
    if (!this.gitRoot) {
      this.gitRoot = await this.findGitRoot()
    }

    // Get current HEAD
    this.lastCommitHash = await this.getCurrentCommitHash()

    // Subscribe to file change events
    this.fileWatchSubscription = Bus.subscribe(
      { type: "file.changed", properties: {} as any },
      async (event) => {
        const filePath = (event.properties as any)?.path
        if (filePath && this.matchesFilters(filePath)) {
          await this.observeFileChange(filePath)
        }
      },
    )

    // Subscribe to Bash tool events to detect Git operations
    this.bashSubscription = Bus.subscribe(
      { type: "tool.bash.executed", properties: {} as any },
      async (event) => {
        const command = (event.properties as any)?.command ?? ""
        // Detect git commit, push, or merge operations
        if (
          command.includes("git commit") ||
          command.includes("git push") ||
          command.includes("git merge") ||
          command.includes("git rebase")
        ) {
          // Trigger immediate observation to detect changes
          await this.triggerObservation()
        }
      },
    )

    // Start periodic typecheck if enabled
    if (this.options_.enableTypecheck) {
      this.typecheckTimer = setInterval(async () => {
        await this.runTypeCheck()
      }, this.options_.typecheckIntervalMs ?? 60000)

      // Run initial typecheck
      void this.runTypeCheck()
    }

    log.info("CodeWatch initialized", {
      gitRoot: this.gitRoot,
      lastCommit: this.lastCommitHash,
      typecheckEnabled: this.options_.enableTypecheck,
    })
  }

  protected override async onStop(): Promise<void> {
    if (this.fileWatchSubscription) {
      this.fileWatchSubscription()
      this.fileWatchSubscription = null
    }
    if (this.bashSubscription) {
      this.bashSubscription()
      this.bashSubscription = null
    }
    if (this.typecheckTimer) {
      clearInterval(this.typecheckTimer)
      this.typecheckTimer = null
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Observation
  // ─────────────────────────────────────────────────────────────────────────────

  protected async observe(): Promise<CodeObservation | null> {
    // Check for git changes
    const currentHash = await this.getCurrentCommitHash()

    if (currentHash && currentHash !== this.lastCommitHash) {
      const changes = await this.getGitChanges(this.lastCommitHash, currentHash)
      this.lastCommitHash = currentHash

      if (changes.length > 0) {
        return this.createGitChangeObservation(changes)
      }
    }

    return null
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Observe a file change event.
   */
  async observeFileChange(filePath: string): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation("file_change", filePath, {
      action: "modify",
    })

    await this.emit(observation)
  }

  /**
   * Observe build status.
   */
  async observeBuildStatus(
    status: "passing" | "failing",
    details?: { errors?: string[]; warnings?: string[] },
  ): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation("build_status", "build", {
      action: status === "passing" ? "modify" : "modify",
      after: { status, ...details },
    })

    observation.impact.severity = status === "failing" ? "high" : "low"
    observation.confidence = 1.0

    await this.emit(observation)
  }

  /**
   * Observe test results.
   */
  async observeTestResults(results: {
    passed: number
    failed: number
    skipped: number
    coverage?: number
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation("test_coverage", "tests", {
      action: "modify",
      after: results,
    })

    const failureRate = results.failed / (results.passed + results.failed + 1)
    observation.impact.severity =
      failureRate > 0.1 ? "high" : failureRate > 0 ? "medium" : "low"
    observation.confidence = 1.0

    await this.emit(observation)
  }

  /**
   * Observe type errors.
   */
  async observeTypeErrors(errors: Array<{
    file: string
    line: number
    message: string
  }>): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation("type_error", "typescript", {
      action: "modify",
      after: { errors, count: errors.length },
    })

    observation.impact.severity = errors.length > 10 ? "high" : errors.length > 0 ? "medium" : "low"
    observation.impact.affectedFiles = [...new Set(errors.map((e) => e.file))]
    observation.confidence = 1.0

    await this.emit(observation)
  }

  /**
   * Observe lint issues.
   */
  async observeLintIssues(issues: Array<{
    file: string
    rule: string
    severity: "error" | "warning"
    message: string
  }>): Promise<void> {
    if (!this.isRunning()) return

    const errorCount = issues.filter((i) => i.severity === "error").length

    const observation = this.createObservation("lint_issue", "eslint", {
      action: "modify",
      after: { issues, errorCount, warningCount: issues.length - errorCount },
    })

    observation.impact.severity = errorCount > 0 ? "medium" : "low"
    observation.impact.affectedFiles = [...new Set(issues.map((i) => i.file))]
    observation.confidence = 1.0

    await this.emit(observation)
  }

  /**
   * Run typecheck and observe results.
   */
  async runTypeCheck(): Promise<void> {
    if (!this.isRunning()) return

    const cwd = this.gitRoot ?? process.cwd()

    try {
      const proc = Bun.spawn(["bun", "turbo", "typecheck", "--quiet"], {
        cwd,
        stdout: "pipe",
        stderr: "pipe",
      })

      // Wait for process with timeout
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error("Typecheck timeout")), this.options_.typecheckTimeoutMs ?? 30000)
      })

      const exitCode = await Promise.race([proc.exited, timeoutPromise])
      const stderr = await new Response(proc.stderr).text()
      const stdout = await new Response(proc.stdout).text()

      if (exitCode !== 0) {
        // Parse type errors from output
        const errors = this.parseTypeErrors(stderr + stdout)
        if (errors.length > 0) {
          await this.observeTypeErrors(errors)
        } else {
          // General build failure
          await this.observeBuildStatus("failing", {
            errors: [stderr || "Build failed with unknown error"],
          })
        }
      } else {
        await this.observeBuildStatus("passing")
      }
    } catch (error) {
      log.warn("Typecheck failed", {
        error: error instanceof Error ? error.message : String(error),
      })
      // Don't emit on timeout/error - just log
    }
  }

  /**
   * Parse TypeScript errors from compiler output.
   */
  private parseTypeErrors(output: string): Array<{
    file: string
    line: number
    message: string
  }> {
    const errors: Array<{ file: string; line: number; message: string }> = []

    // Match TypeScript error format: path/file.ts(line,col): error TSxxxx: message
    // Or: path/file.ts:line:col - error TSxxxx: message
    const patterns = [
      /([^:\s]+\.tsx?)\((\d+),\d+\):\s*error\s+TS\d+:\s*(.+)/g,
      /([^:\s]+\.tsx?):(\d+):\d+\s*-\s*error\s+TS\d+:\s*(.+)/g,
    ]

    for (const pattern of patterns) {
      let match
      while ((match = pattern.exec(output)) !== null) {
        errors.push({
          file: match[1],
          line: parseInt(match[2], 10),
          message: match[3].trim(),
        })
      }
    }

    return errors
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private createObservation(
    type: CodeObservationType,
    source: string,
    change: CodeObservation["change"],
  ): CodeObservation {
    const base = this.createBaseObservation()

    return {
      ...base,
      watcherType: "code" as const,
      type,
      source,
      change,
      impact: {
        scope: this.determineScope(source),
        severity: "low",
        affectedFiles: [],
      },
    }
  }

  private createGitChangeObservation(changes: GitChange[]): CodeObservation {
    const latestChange = changes[0]
    const allFiles = changes.flatMap((c) => c.files.map((f) => f.path))

    return this.createObservation("git_change", this.gitRoot ?? ".", {
      action: "modify",
      before: this.lastCommitHash,
      after: latestChange.hash,
      diff: changes.map((c) => `${c.hash.slice(0, 7)}: ${c.message}`).join("\n"),
    })
  }

  private determineScope(
    source: string,
  ): CodeObservation["impact"]["scope"] {
    if (source.includes("package.json") || source.includes("tsconfig")) {
      return "project"
    }
    if (source.includes("/")) {
      const parts = source.split("/")
      if (parts.length > 2) return "module"
      return "package"
    }
    return "file"
  }

  private async findGitRoot(): Promise<string | null> {
    try {
      const { execSync } = await import("child_process")
      const result = execSync("git rev-parse --show-toplevel", {
        encoding: "utf-8",
        timeout: 5000,
      }).trim()
      return result
    } catch {
      return null
    }
  }

  private async getCurrentCommitHash(): Promise<string | null> {
    if (!this.gitRoot) return null

    try {
      const { execSync } = await import("child_process")
      const result = execSync("git rev-parse HEAD", {
        encoding: "utf-8",
        cwd: this.gitRoot,
        timeout: 5000,
      }).trim()
      return result
    } catch {
      return null
    }
  }

  private async getGitChanges(
    fromHash: string | null,
    toHash: string,
  ): Promise<GitChange[]> {
    if (!this.gitRoot) return []

    try {
      const { execSync } = await import("child_process")
      const range = fromHash ? `${fromHash}..${toHash}` : `-1 ${toHash}`

      const logOutput = execSync(
        `git log --pretty=format:"%H|%s|%an|%aI" ${range}`,
        {
          encoding: "utf-8",
          cwd: this.gitRoot,
          timeout: 10000,
        },
      ).trim()

      if (!logOutput) return []

      const changes: GitChange[] = []

      for (const line of logOutput.split("\n")) {
        const [hash, message, author, dateStr] = line.split("|")
        if (!hash) continue

        // Get files changed in this commit
        const filesOutput = execSync(
          `git diff-tree --no-commit-id --name-status -r ${hash}`,
          {
            encoding: "utf-8",
            cwd: this.gitRoot,
            timeout: 5000,
          },
        ).trim()

        const files = filesOutput
          .split("\n")
          .filter(Boolean)
          .map((fileLine) => {
            const [status, path] = fileLine.split("\t")
            return {
              path,
              action: (status === "A"
                ? "add"
                : status === "D"
                  ? "delete"
                  : "modify") as "add" | "modify" | "delete",
            }
          })

        changes.push({
          hash,
          message,
          author,
          date: new Date(dateStr),
          files,
        })
      }

      return changes
    } catch (error) {
      log.warn("Failed to get git changes", {
        error: error instanceof Error ? error.message : String(error),
      })
      return []
    }
  }
}

/**
 * Create a CodeWatch instance.
 */
export function createCodeWatch(options?: CodeWatchOptions): CodeWatch {
  return new CodeWatch(options)
}
