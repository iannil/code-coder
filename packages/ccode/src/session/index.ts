/**
 * Session Type Stubs
 * @deprecated This module has been moved to Rust. Use @/api/session instead.
 */

export namespace Session {
  export interface Info {
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

  export interface Message {
    id: string
    sessionID: string
    role: "user" | "assistant"
    time: { created: number }
    agent?: string
    model?: { providerID: string; modelID: string }
  }

  export interface Part {
    id: string
    sessionID: string
    messageID: string
    type: string
    text?: string
    time: { start: number; end?: number }
  }

  export async function* list(): AsyncGenerator<Info> {
    // Stub - use LocalSession.list() instead
  }

  export async function get(_id: string): Promise<Info> {
    throw new Error("Deprecated: use LocalSession.get() from @/api")
  }

  export async function create(_input: { title?: string }): Promise<Info> {
    throw new Error("Deprecated: use LocalSession.create() from @/api")
  }

  export async function updateMessage(_message: Message): Promise<void> {
    throw new Error("Deprecated: use LocalSession API from @/api")
  }

  export async function updatePart(_part: Part): Promise<void> {
    throw new Error("Deprecated: use LocalSession API from @/api")
  }

  export const Event = {
    Created: { type: "session.created" as const },
    Updated: { type: "session.updated" as const },
    Deleted: { type: "session.deleted" as const },
    Error: { type: "session.error" as const },
  }
}
