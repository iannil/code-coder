/**
 * Remote Security Policy
 * Defines which operations require human approval when invoked remotely
 */

import { Log } from "@/util/log"
import { Global } from "@/global"
import path from "path"
import type { TaskContext } from "@/api/task/types"

export namespace RemotePolicy {
  const log = Log.create({ service: "remote-policy" })

  /** File path for persisting user allowlists */
  const ALLOWLIST_FILE = path.join(Global.Path.config, "remote-allowlists.json")

  // ============================================================================
  // Dangerous Operations
  // ============================================================================

  /**
   * Operations that require explicit human approval when invoked remotely
   *
   * Categories:
   * - File system mutations (write, edit, delete)
   * - Shell command execution
   * - Git operations that modify history
   * - Network operations (fetch, curl)
   * - MCP browser operations that can modify state or navigate
   */
  const DANGEROUS_OPERATIONS = new Set([
    // File mutations
    "write",
    "edit",
    "patch",
    "multiedit",
    "delete",
    "move",
    "rename",

    // Shell execution
    "bash",
    "shell",
    "exec",
    "run",

    // Git mutations
    "git_push",
    "git_commit",
    "git_reset",
    "git_checkout",
    "git_branch_delete",
    "git_force_push",

    // Network operations
    "fetch",
    "curl",
    "http",

    // MCP Playwright browser operations (mutating/navigating)
    "mcp__playwright__browser_navigate",
    "mcp__playwright__browser_click",
    "mcp__playwright__browser_type",
    "mcp__playwright__browser_fill_form",
    "mcp__playwright__browser_file_upload",
    "mcp__playwright__browser_evaluate",
    "mcp__playwright__browser_run_code",
    "mcp__playwright__browser_select_option",
    "mcp__playwright__browser_drag",
    "mcp__playwright__browser_press_key",
    "mcp__playwright__browser_handle_dialog",

    // MCP Puppeteer browser operations (mutating/navigating)
    "mcp__puppeteer__puppeteer_navigate",
    "mcp__puppeteer__puppeteer_click",
    "mcp__puppeteer__puppeteer_fill",
    "mcp__puppeteer__puppeteer_evaluate",
  ])

  /**
   * Operations that are always safe (read-only)
   */
  const SAFE_OPERATIONS = new Set([
    "read",
    "view",
    "search",
    "grep",
    "find",
    "list",
    "git_status",
    "git_log",
    "git_diff",

    // MCP Playwright read-only operations
    "mcp__playwright__browser_snapshot",
    "mcp__playwright__browser_take_screenshot",
    "mcp__playwright__browser_console_messages",
    "mcp__playwright__browser_network_requests",
    "mcp__playwright__browser_tabs",
    "mcp__playwright__browser_wait_for",
    "mcp__playwright__browser_navigate_back",
    "mcp__playwright__browser_resize",
    "mcp__playwright__browser_hover",
    "mcp__playwright__browser_close",
    "mcp__playwright__browser_install",

    // MCP Puppeteer read-only operations
    "mcp__puppeteer__puppeteer_screenshot",
  ])

  /**
   * User-specific allowlists
   * Maps userID to set of allowed operations
   */
  const userAllowlists: Map<string, Set<string>> = new Map()

  // ============================================================================
  // Policy Evaluation
  // ============================================================================

  /**
   * Check if an operation requires human approval
   */
  export function shouldRequireApproval(tool: string, context: TaskContext): boolean {
    // Non-remote contexts don't need extra approval
    if (context.source !== "remote") {
      return false
    }

    // Normalize tool name
    const normalizedTool = tool.toLowerCase()

    // Check user-specific allowlist
    const userAllowed = userAllowlists.get(context.userID)
    if (userAllowed?.has(normalizedTool)) {
      log.info("tool allowed by user allowlist", { tool, userID: context.userID })
      return false
    }

    // Safe operations never need approval
    if (SAFE_OPERATIONS.has(normalizedTool)) {
      return false
    }

    // Dangerous operations always need approval for remote calls
    if (DANGEROUS_OPERATIONS.has(normalizedTool)) {
      log.info("dangerous operation requires approval", { tool, userID: context.userID })
      return true
    }

    // MCP tools (prefixed with mcp__) need approval by default unless explicitly safe
    if (normalizedTool.startsWith("mcp__")) {
      log.info("MCP operation requires approval", { tool, userID: context.userID })
      return true
    }

    // Unknown operations default to requiring approval for remote calls
    log.warn("unknown operation, requiring approval by default", { tool })
    return true
  }

  /**
   * Check if an operation is explicitly dangerous
   */
  export function isDangerous(tool: string): boolean {
    return DANGEROUS_OPERATIONS.has(tool.toLowerCase())
  }

  /**
   * Check if an operation is explicitly safe
   */
  export function isSafe(tool: string): boolean {
    return SAFE_OPERATIONS.has(tool.toLowerCase())
  }

  /**
   * Get the risk level for an operation
   */
  export function riskLevel(tool: string): "safe" | "moderate" | "dangerous" {
    const normalizedTool = tool.toLowerCase()

    if (SAFE_OPERATIONS.has(normalizedTool)) {
      return "safe"
    }

    if (DANGEROUS_OPERATIONS.has(normalizedTool)) {
      return "dangerous"
    }

    return "moderate"
  }

  // ============================================================================
  // User Allowlist Management
  // ============================================================================

  /** Whether allowlists have been loaded from disk */
  let allowlistsLoaded = false

  /**
   * Load user allowlists from persistent storage
   * Called automatically on first use, can also be called explicitly
   */
  export async function loadAllowlists(): Promise<void> {
    if (allowlistsLoaded) return

    try {
      const file = Bun.file(ALLOWLIST_FILE)
      if (await file.exists()) {
        const data = (await file.json()) as Record<string, string[]>
        for (const [userID, tools] of Object.entries(data)) {
          userAllowlists.set(userID, new Set(tools))
        }
        log.info("loaded user allowlists from disk", {
          userCount: userAllowlists.size,
          file: ALLOWLIST_FILE,
        })
      }
    } catch (error) {
      log.warn("failed to load user allowlists", { error })
    }

    allowlistsLoaded = true
  }

  /**
   * Save user allowlists to persistent storage
   */
  export async function saveAllowlists(): Promise<void> {
    try {
      const data: Record<string, string[]> = {}
      for (const [userID, tools] of userAllowlists) {
        data[userID] = [...tools]
      }
      await Bun.write(ALLOWLIST_FILE, JSON.stringify(data, null, 2))
      log.info("saved user allowlists to disk", {
        userCount: userAllowlists.size,
        file: ALLOWLIST_FILE,
      })
    } catch (error) {
      log.error("failed to save user allowlists", { error })
    }
  }

  /**
   * Add an operation to a user's allowlist
   * Also persists the allowlist to disk
   */
  export async function allowForUser(userID: string, tool: string): Promise<void> {
    const existing = userAllowlists.get(userID) ?? new Set()
    existing.add(tool.toLowerCase())
    userAllowlists.set(userID, existing)
    log.info("added tool to user allowlist", { userID, tool })
    await saveAllowlists()
  }

  /**
   * Remove an operation from a user's allowlist
   * Also persists the allowlist to disk
   */
  export async function revokeForUser(userID: string, tool: string): Promise<void> {
    const existing = userAllowlists.get(userID)
    if (existing) {
      existing.delete(tool.toLowerCase())
      log.info("removed tool from user allowlist", { userID, tool })
      await saveAllowlists()
    }
  }

  /**
   * Get a user's allowlist
   */
  export function getUserAllowlist(userID: string): string[] {
    return [...(userAllowlists.get(userID) ?? [])]
  }

  /**
   * Clear a user's allowlist
   * Also persists the change to disk
   */
  export async function clearUserAllowlist(userID: string): Promise<void> {
    userAllowlists.delete(userID)
    log.info("cleared user allowlist", { userID })
    await saveAllowlists()
  }

  // ============================================================================
  // Description Generation
  // ============================================================================

  /**
   * Generate a human-readable description of why approval is needed
   */
  export function describeApprovalReason(tool: string, args: unknown): string {
    const normalizedTool = tool.toLowerCase()

    switch (normalizedTool) {
      case "write":
      case "edit":
      case "patch":
        return `File modification: ${(args as { path?: string })?.path ?? "unknown path"}`

      case "bash":
      case "shell":
      case "exec":
        return `Shell command: ${String((args as { command?: string })?.command ?? args).slice(0, 100)}`

      case "git_push":
        return "Git push operation will modify remote repository"

      case "git_commit":
        return "Git commit will modify repository history"

      case "delete":
        return `Delete file: ${(args as { path?: string })?.path ?? "unknown path"}`

      // MCP Playwright operations
      case "mcp__playwright__browser_navigate":
      case "mcp__puppeteer__puppeteer_navigate":
        return `Navigate browser to: ${(args as { url?: string })?.url ?? "unknown URL"}`

      case "mcp__playwright__browser_click":
      case "mcp__puppeteer__puppeteer_click":
        return `Click element: ${(args as { element?: string })?.element ?? (args as { ref?: string })?.ref ?? (args as { selector?: string })?.selector ?? "unknown"}`

      case "mcp__playwright__browser_type":
      case "mcp__puppeteer__puppeteer_fill":
        return `Type text into: ${(args as { element?: string })?.element ?? (args as { ref?: string })?.ref ?? (args as { selector?: string })?.selector ?? "unknown"}`

      case "mcp__playwright__browser_fill_form":
        return `Fill form with ${((args as { fields?: unknown[] })?.fields?.length ?? 0)} fields`

      case "mcp__playwright__browser_file_upload":
        return `Upload files: ${((args as { paths?: string[] })?.paths ?? []).join(", ") || "unknown"}`

      case "mcp__playwright__browser_evaluate":
      case "mcp__playwright__browser_run_code":
      case "mcp__puppeteer__puppeteer_evaluate":
        return `Execute JavaScript in browser`

      default:
        // Handle other MCP tools generically
        if (normalizedTool.startsWith("mcp__playwright__")) {
          const action = normalizedTool.replace("mcp__playwright__browser_", "")
          return `Browser ${action} operation`
        }
        if (normalizedTool.startsWith("mcp__puppeteer__")) {
          const action = normalizedTool.replace("mcp__puppeteer__puppeteer_", "")
          return `Browser ${action} operation`
        }
        if (normalizedTool.startsWith("mcp__")) {
          return `MCP operation: ${tool}`
        }
        return `Operation "${tool}" requested`
    }
  }
}

// ============================================================================
// Convenience Export
// ============================================================================

export const {
  shouldRequireApproval,
  isDangerous,
  isSafe,
  riskLevel,
  describeApprovalReason,
  loadAllowlists,
  saveAllowlists,
  allowForUser,
  revokeForUser,
  getUserAllowlist,
  clearUserAllowlist,
} = RemotePolicy
