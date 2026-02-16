/**
 * Remote Security Policy
 * Defines which operations require human approval when invoked remotely
 */

import { Log } from "@/util/log"
import type { TaskContext } from "@/api/task/types"

export namespace RemotePolicy {
  const log = Log.create({ service: "remote-policy" })

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

  /**
   * Add an operation to a user's allowlist
   */
  export function allowForUser(userID: string, tool: string): void {
    const existing = userAllowlists.get(userID) ?? new Set()
    existing.add(tool.toLowerCase())
    userAllowlists.set(userID, existing)
    log.info("added tool to user allowlist", { userID, tool })
  }

  /**
   * Remove an operation from a user's allowlist
   */
  export function revokeForUser(userID: string, tool: string): void {
    const existing = userAllowlists.get(userID)
    if (existing) {
      existing.delete(tool.toLowerCase())
      log.info("removed tool from user allowlist", { userID, tool })
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
   */
  export function clearUserAllowlist(userID: string): void {
    userAllowlists.delete(userID)
    log.info("cleared user allowlist", { userID })
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

      default:
        return `Operation "${tool}" requested`
    }
  }
}

// ============================================================================
// Convenience Export
// ============================================================================

export const { shouldRequireApproval, isDangerous, isSafe, riskLevel, describeApprovalReason } = RemotePolicy
