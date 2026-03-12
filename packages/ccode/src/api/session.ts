/**
 * Session API - Rust API Client Wrapper
 *
 * This module provides a TypeScript interface to the Rust unified API.
 * All session operations are delegated to the Rust backend.
 *
 * @module api/session
 */

import { getRustClient, type SessionSummary, type SessionDetail } from "./rust-client"

// ─────────────────────────────────────────────────────────────────────────────
// Type Definitions (mirrors Rust API types)
// ─────────────────────────────────────────────────────────────────────────────

export interface SessionInfo {
  id: string
  title: string
  projectID?: string
  parentID?: string
  directory?: string
  permission?: Record<string, unknown>
  time: {
    created: number
    updated: number
  }
  summary?: {
    additions: number
    deletions: number
    files: number
  }
  revert?: {
    messageID: string
    snapshot?: string
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

function toSessionInfo(session: SessionSummary | SessionDetail): SessionInfo {
  return {
    id: session.id,
    title: session.title ?? "Untitled",
    projectID: session.project_id,
    parentID: session.parent_id,
    directory: session.directory,
    permission: session.permission,
    time: {
      created: session.created_at ?? session.time.created,
      updated: session.updated_at ?? session.time.updated,
    },
    summary: session.summary
      ? {
          additions: session.summary.additions,
          deletions: session.summary.deletions,
          files: session.summary.files,
        }
      : undefined,
    revert: session.revert
      ? {
          messageID: session.revert.messageID,
          snapshot: session.revert.snapshot,
        }
      : undefined,
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// LocalSession Namespace
// ─────────────────────────────────────────────────────────────────────────────

export namespace LocalSession {
  const client = getRustClient()

  export interface ListInput {
    directory?: string
    roots?: boolean
    start?: number
    search?: string
    limit?: number
  }

  /**
   * List sessions with optional filtering
   */
  export async function list(input?: ListInput): Promise<SessionInfo[]> {
    const response = await client.listSessions({
      limit: input?.limit,
      project_id: input?.directory,
    })

    if (!response.success || !response.data) {
      return []
    }

    let sessions = response.data.sessions.map(toSessionInfo)

    // Apply client-side filters not supported by Rust API
    if (input?.roots) {
      sessions = sessions.filter((s) => !s.parentID)
    }
    if (input?.start !== undefined) {
      sessions = sessions.filter((s) => s.time.updated >= input.start!)
    }
    if (input?.search) {
      const searchLower = input.search.toLowerCase()
      sessions = sessions.filter((s) => s.title.toLowerCase().includes(searchLower))
    }
    if (input?.limit) {
      sessions = sessions.slice(0, input.limit)
    }

    return sessions
  }

  /**
   * Get session by ID
   */
  export async function get(sessionID: string): Promise<SessionInfo> {
    const response = await client.getSession(sessionID)
    if (!response.success || !response.data) {
      throw new Error(`Session not found: ${sessionID}`)
    }
    return toSessionInfo(response.data.session)
  }

  export interface CreateInput {
    parentID?: string
    title?: string
    permission?: Record<string, unknown>
  }

  /**
   * Create a new session
   */
  export async function create(input?: CreateInput): Promise<SessionInfo> {
    const response = await client.createSession({
      title: input?.title,
      project_id: input?.parentID,
    })
    if (!response.success || !response.data) {
      throw new Error("Failed to create session")
    }
    return toSessionInfo(response.data.session)
  }

  /**
   * Get child sessions
   */
  export async function children(sessionID: string): Promise<SessionInfo[]> {
    const response = await client.listSessions()
    if (!response.success || !response.data) {
      return []
    }
    return response.data.sessions.filter((s) => s.parent_id === sessionID).map(toSessionInfo)
  }

  /**
   * Get session status (stub - returns empty array)
   * @deprecated Session status is not yet implemented in Rust API
   */
  export async function status(): Promise<unknown[]> {
    return []
  }

  /**
   * Get session summary
   */
  export async function summary(sessionID: string): Promise<SessionInfo["summary"]> {
    const session = await get(sessionID)
    return session.summary
  }

  /**
   * Get todo list for session (stub - returns empty array)
   * @deprecated Todo is not yet implemented in Rust API
   */
  export async function todo(_sessionID: string): Promise<unknown[]> {
    return []
  }

  export interface ForkInput {
    sessionID: string
    messageID?: string
  }

  /**
   * Fork a session
   */
  export async function fork(input: ForkInput): Promise<SessionInfo> {
    const response = await client.forkSession(input.sessionID, {
      from_message_id: input.messageID,
    })
    if (!response.success || !response.data) {
      throw new Error("Failed to fork session")
    }
    return toSessionInfo(response.data.session)
  }

  /**
   * Delete a session
   */
  export async function remove(sessionID: string): Promise<void> {
    const response = await client.deleteSession(sessionID)
    if (!response.success) {
      throw new Error(`Failed to delete session: ${sessionID}`)
    }
  }

  export interface RenameInput {
    sessionID: string
    title: string
  }

  /**
   * Rename a session
   */
  export async function rename(input: RenameInput): Promise<SessionInfo> {
    await client.updateSession(input.sessionID, { title: input.title })
    return get(input.sessionID)
  }

  /**
   * Compact session history
   */
  export async function compact(sessionID: string): Promise<boolean> {
    const response = await client.compactSession(sessionID)
    return response.success
  }

  /**
   * Revert session to a previous message (stub)
   * @deprecated Revert is not yet implemented in Rust API
   */
  export async function revert(_sessionID: string, _messageID: string): Promise<boolean> {
    return false
  }

  export interface MessagesInput {
    sessionID: string
    limit?: number
  }

  /**
   * Get session messages
   */
  export async function messages(input: MessagesInput) {
    const response = await client.getSessionMessages(input.sessionID)
    if (!response.success || !response.data) {
      return []
    }
    let msgs = response.data.messages
    if (input.limit) {
      msgs = msgs.slice(-input.limit)
    }
    return msgs
  }

  /**
   * Send a prompt to the session
   */
  export interface PromptInput {
    sessionID: string
    agent?: string
    model?: string
    variant?: string
    parts: Array<{ type: string; text?: string; url?: string; filename?: string; mime?: string }>
  }

  export async function prompt(input: PromptInput): Promise<{ messageID: string; content?: string }> {
    // Extract text from parts
    const textParts = input.parts.filter((p) => p.type === "text" && p.text)
    const message = textParts.map((p) => p.text).join("\n")

    // Use SSE chat
    let fullResponse = ""
    for await (const event of client.chat(input.sessionID, message, {
      agent: input.agent,
      model: input.model,
    })) {
      if (event.type === "text_delta") {
        fullResponse += event.content
      }
    }

    // Return a synthetic message ID (Rust API doesn't return one from SSE)
    return {
      messageID: `msg-${Date.now()}`,
      content: fullResponse,
    }
  }

  /**
   * Execute a command (slash command)
   */
  export interface CommandInput {
    sessionID: string
    agent?: string
    model?: string
    command: string
    arguments: string
    variant?: string
  }

  export async function command(input: CommandInput): Promise<{ messageID: string }> {
    // Get command template from prompts API
    const promptResponse = await client.getPrompt(input.command)
    if (!promptResponse.success || !promptResponse.data) {
      throw new Error(`Command "${input.command}" not found`)
    }

    // Substitute arguments into template
    let template = promptResponse.data.content
    if (input.arguments) {
      template = template.replace(/\$ARGUMENTS/g, input.arguments)
      // Replace positional args $1, $2, etc.
      const args = input.arguments.split(/\s+/)
      args.forEach((arg, i) => {
        template = template.replace(new RegExp(`\\$${i + 1}`, "g"), arg)
      })
    }

    // Execute as prompt
    const result = await prompt({
      sessionID: input.sessionID,
      agent: input.agent,
      model: input.model,
      variant: input.variant,
      parts: [{ type: "text", text: template }],
    })

    return { messageID: result.messageID }
  }

  /**
   * Cancel/abort an ongoing session prompt (stub)
   * @deprecated Abort is not yet implemented in Rust API
   */
  export function abort(_sessionID: string): boolean | undefined {
    return undefined
  }
}
