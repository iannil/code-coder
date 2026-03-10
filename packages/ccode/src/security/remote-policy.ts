/**
 * Remote Security Policy
 *
 * Defines which operations require human approval when invoked remotely.
 * This module wraps the Rust implementation via NAPI bindings.
 */

import {
  RemotePolicy as CoreRemotePolicy,
  getRemoteRiskLevel,
  isRemoteDangerous,
  isRemoteSafe,
  type RemoteTaskContext as CoreRemoteTaskContext,
} from "@codecoder-ai/core"
import { Log } from "@/util/log"
import type { TaskContext } from "@/api/task/types"

export namespace RemotePolicy {
  const log = Log.create({ service: "remote-policy" })

  // Singleton policy handle (initialized lazily)
  let policyHandle: CoreRemotePolicy | null = null

  /**
   * Get or create the policy handle
   */
  function getHandle(): CoreRemotePolicy {
    if (!policyHandle) {
      policyHandle = new CoreRemotePolicy()
      // Load allowlists on first use
      try {
        policyHandle.loadAllowlists()
        log.info("loaded remote policy allowlists")
      } catch (error) {
        log.warn("failed to load remote policy allowlists", { error })
      }
    }
    return policyHandle
  }

  // ============================================================================
  // Policy Evaluation
  // ============================================================================

  /**
   * Check if an operation requires human approval
   *
   * @param tool - The tool/operation name
   * @param context - The task context containing source and userID
   * @returns true if approval is required
   */
  export function shouldRequireApproval(tool: string, context: TaskContext): boolean {
    const handle = getHandle()
    const napiContext = {
      source: context.source ?? "cli",
      userId: context.userID ?? "local",
      sessionId: context.conversationId,
    }

    const result = handle.shouldRequireApproval(tool, napiContext)

    if (result) {
      log.info("operation requires approval", { tool, userID: context.userID, source: context.source })
    }

    return result
  }

  /**
   * Check if an operation is explicitly dangerous
   */
  export function isDangerous(tool: string): boolean {
    return isRemoteDangerous(tool)
  }

  /**
   * Check if an operation is explicitly safe
   */
  export function isSafe(tool: string): boolean {
    return isRemoteSafe(tool)
  }

  /**
   * Get the risk level for an operation
   */
  export function riskLevel(tool: string): "safe" | "moderate" | "dangerous" {
    const level = getRemoteRiskLevel(tool)
    return level.toLowerCase() as "safe" | "moderate" | "dangerous"
  }

  // ============================================================================
  // User Allowlist Management
  // ============================================================================

  /**
   * Load user allowlists from persistent storage
   * Called automatically on first use, can also be called explicitly
   */
  export async function loadAllowlists(): Promise<void> {
    const handle = getHandle()
    handle.loadAllowlists()
    log.info("loaded remote policy allowlists")
  }

  /**
   * Save user allowlists to persistent storage
   * Note: The Rust implementation auto-saves on each modification
   */
  export async function saveAllowlists(): Promise<void> {
    // No-op: Rust implementation auto-saves
    log.info("allowlists auto-saved by Rust implementation")
  }

  /**
   * Add an operation to a user's allowlist
   * Also persists the allowlist to disk
   */
  export async function allowForUser(userID: string, tool: string): Promise<void> {
    const handle = getHandle()
    handle.allowForUser(userID, tool)
    log.info("added tool to user allowlist", { userID, tool })
  }

  /**
   * Remove an operation from a user's allowlist
   * Also persists the allowlist to disk
   */
  export async function revokeForUser(userID: string, tool: string): Promise<void> {
    const handle = getHandle()
    handle.revokeForUser(userID, tool)
    log.info("removed tool from user allowlist", { userID, tool })
  }

  /**
   * Get a user's allowlist
   */
  export function getUserAllowlist(userID: string): string[] {
    const handle = getHandle()
    return handle.getUserAllowlist(userID)
  }

  /**
   * Clear a user's allowlist
   * Also persists the change to disk
   */
  export async function clearUserAllowlist(userID: string): Promise<void> {
    const handle = getHandle()
    handle.clearUserAllowlist(userID)
    log.info("cleared user allowlist", { userID })
  }

  // ============================================================================
  // Description Generation
  // ============================================================================

  /**
   * Generate a human-readable description of why approval is needed
   */
  export function describeApprovalReason(tool: string, args: unknown): string {
    const handle = getHandle()
    const argsJson = args !== undefined ? JSON.stringify(args) : undefined
    return handle.describeApprovalReason(tool, argsJson)
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
