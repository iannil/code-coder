/**
 * SDK Adapter - Converts SDK types to TypeScript module types
 *
 * This module provides adapter functions that convert responses from the Rust daemon
 * (via SDK) to the format expected by the TypeScript modules (Session.Info, etc.).
 *
 * This enables gradual migration from direct TypeScript module usage to SDK-based
 * communication with the Rust daemon.
 *
 * @module sdk/adapter
 */

import type { SessionInfo as SdkSessionInfo, SessionSummary, RevertInfo, PermissionRuleset } from "./types"
import { Instance } from "@/project/instance"

/**
 * Session.Info type (matching @/session module)
 * Defined here to avoid circular imports during migration
 */
export interface SessionInfoLegacy {
  id: string
  slug: string
  projectID: string
  directory: string
  parentID?: string
  summary?: {
    additions: number
    deletions: number
    files: number
    diffs?: Array<{
      file: string
      before: string
      after: string
      additions: number
      deletions: number
    }>
  }
  title: string
  version: string
  time: {
    created: number
    updated: number
    compacting?: number
    archived?: number
  }
  permission?: Record<string, unknown>
  revert?: {
    messageID: string
    partID?: string
    snapshot?: string
    diff?: string
  }
}

/**
 * Generate a slug from session ID
 * Extracts the UUID portion and takes first 8 characters
 */
function generateSlug(id: string): string {
  // Handle format like "cli:uuid" or just "uuid"
  const parts = id.split(":")
  const uuid = parts.length > 1 ? parts[1] : parts[0]
  return uuid.substring(0, 8)
}

/**
 * Generate a version string
 * Format: "v1.0.0" for new sessions
 */
function generateVersion(): string {
  return "v1.0.0"
}

/**
 * Convert SDK SessionInfo to legacy Session.Info format
 *
 * @param sdk - SessionInfo from SDK
 * @param options - Additional options for conversion
 * @returns Session.Info compatible object
 */
export function adaptSessionInfo(
  sdk: SdkSessionInfo,
  options: {
    directory?: string
  } = {}
): SessionInfoLegacy {
  // Convert SDK summary to legacy format (ensure diffs have required fields)
  const summary = sdk.summary
    ? {
        additions: sdk.summary.additions,
        deletions: sdk.summary.deletions,
        files: sdk.summary.files,
        diffs: sdk.summary.diffs?.map((d) => ({
          file: d.file,
          before: d.before ?? "",
          after: d.after ?? "",
          additions: d.additions,
          deletions: d.deletions,
        })),
      }
    : undefined

  // Convert SDK permission to legacy format (Record<string, unknown>)
  const permission = sdk.permission ? (sdk.permission as Record<string, unknown>) : undefined

  // Convert SDK revert to legacy format
  const revert = sdk.revert
    ? {
        messageID: sdk.revert.messageID,
        partID: sdk.revert.partID,
        snapshot: sdk.revert.snapshot,
        diff: sdk.revert.diff,
      }
    : undefined

  return {
    id: sdk.id,
    slug: generateSlug(sdk.id),
    projectID: sdk.project_id ?? "",
    directory: sdk.directory ?? options.directory ?? Instance.directory ?? process.cwd(),
    parentID: sdk.parent_id,
    title: sdk.title ?? `New session - ${new Date(sdk.time.created).toISOString()}`,
    version: generateVersion(),
    time: {
      created: sdk.time.created,
      updated: sdk.time.updated,
      compacting: sdk.time.compacting,
      archived: sdk.time.archived,
    },
    summary,
    permission,
    revert,
  }
}

/**
 * Convert an array of SDK SessionInfo to legacy format
 */
export function adaptSessionList(
  sessions: SdkSessionInfo[],
  options: {
    directory?: string
  } = {}
): SessionInfoLegacy[] {
  return sessions.map((s) => adaptSessionInfo(s, options))
}

/**
 * Check if SDK mode is enabled
 * Can be controlled via environment variable or configuration
 */
export function isSdkModeEnabled(): boolean {
  // SDK mode is enabled by default
  // Set CODECODER_SDK_MODE=0 to disable and use local TypeScript API
  return process.env.CODECODER_SDK_MODE !== "0"
}

/**
 * Adapter configuration
 */
export interface AdapterConfig {
  /** Default directory for sessions */
  defaultDirectory?: string
  /** Whether to fall back to local API on SDK errors */
  fallbackToLocal?: boolean
}

let adapterConfig: AdapterConfig = {
  fallbackToLocal: true,
}

/**
 * Configure the adapter
 */
export function configureAdapter(config: Partial<AdapterConfig>): void {
  adapterConfig = { ...adapterConfig, ...config }
}

/**
 * Get current adapter configuration
 */
export function getAdapterConfig(): AdapterConfig {
  return { ...adapterConfig }
}
