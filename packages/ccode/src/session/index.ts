/**
 * Session Type Stubs
 * @deprecated This module has been moved to Rust. Use @/api/session instead.
 */

import { z } from "zod"
import { BusEvent } from "@/bus/bus-event"

export namespace Session {
  export interface Info {
    id: string
    title: string
    projectID?: string
    parentID?: string
    directory?: string
    permission?: Array<{ permission: string; pattern: string; action: "ask" | "allow" | "deny" }> | Record<string, unknown>
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

  // Message type that accepts both MessageV2.Assistant and MessageV2.User
  export type Message = {
    id: string
    sessionID: string
    role: "user" | "assistant"
    time?: { created: number }
    agent?: string
    model?: { providerID: string; modelID: string }
    parentID?: string
    modelID?: string
    providerID?: string
    mode?: string
    path?: { cwd: string; root: string }
    cost?: number
    tokens?: {
      input: number
      output: number
      reasoning: number
      cache: { read: number; write: number }
    }
    parts?: unknown[]
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
    Created: BusEvent.define("session.created", z.object({
      sessionId: z.string().optional(),
    })),
    Updated: BusEvent.define("session.updated", z.object({
      sessionId: z.string().optional(),
    })),
    Deleted: BusEvent.define("session.deleted", z.object({
      sessionId: z.string().optional(),
    })),
    Error: BusEvent.define("session.error", z.object({
      sessionId: z.string().optional(),
      error: z.unknown().optional(),
    })),
  }
}
