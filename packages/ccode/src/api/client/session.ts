/**
 * Session API Client - Manage sessions via the zero-api service
 */

import z from "zod"
import { getClient, type ZeroClient } from "./index"

// ============================================================================
// Session Types
// ============================================================================

/** Session info */
export const SessionInfo = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.number(),
  updated_at: z.number(),
  message_count: z.number(),
  parent_id: z.string().optional(),
})

export type SessionInfo = z.infer<typeof SessionInfo>

/** Session message */
export const SessionMessage = z.object({
  id: z.string(),
  role: z.enum(["user", "assistant", "system"]),
  content: z.string(),
  timestamp: z.number(),
  metadata: z.record(z.string(), z.unknown()).optional(),
})

export type SessionMessage = z.infer<typeof SessionMessage>

/** Create session request */
export const CreateSessionRequest = z.object({
  title: z.string().optional(),
  parent_id: z.string().optional(),
})

export type CreateSessionRequest = z.infer<typeof CreateSessionRequest>

/** Add message request */
export const AddMessageRequest = z.object({
  role: z.enum(["user", "assistant", "system"]),
  content: z.string(),
  metadata: z.record(z.string(), z.unknown()).optional(),
})

export type AddMessageRequest = z.infer<typeof AddMessageRequest>

/** List sessions response */
export const ListSessionsResponse = z.object({
  sessions: z.array(SessionInfo),
  total: z.number(),
})

export type ListSessionsResponse = z.infer<typeof ListSessionsResponse>

// ============================================================================
// Session Client
// ============================================================================

/**
 * Session API client
 */
export class SessionClient {
  constructor(private client: ZeroClient = getClient()) {}

  /**
   * List all sessions
   */
  async list(): Promise<ListSessionsResponse> {
    return this.client.get<ListSessionsResponse>("/api/v1/session")
  }

  /**
   * Create a new session
   */
  async create(request: CreateSessionRequest = {}): Promise<SessionInfo> {
    return this.client.post<SessionInfo>("/api/v1/session", request)
  }

  /**
   * Get a session by ID
   */
  async get(sessionId: string): Promise<SessionInfo> {
    return this.client.get<SessionInfo>(`/api/v1/session/${sessionId}`)
  }

  /**
   * Delete a session
   */
  async delete(sessionId: string): Promise<void> {
    await this.client.delete(`/api/v1/session/${sessionId}`)
  }

  /**
   * Get messages in a session
   */
  async getMessages(sessionId: string): Promise<SessionMessage[]> {
    return this.client.get<SessionMessage[]>(`/api/v1/session/${sessionId}/messages`)
  }

  /**
   * Add a message to a session
   */
  async addMessage(sessionId: string, request: AddMessageRequest): Promise<SessionMessage> {
    return this.client.post<SessionMessage>(`/api/v1/session/${sessionId}/messages`, request)
  }
}

/**
 * Singleton session client
 */
let sessionClient: SessionClient | null = null

/**
 * Get the session client
 */
export function getSessionClient(): SessionClient {
  if (!sessionClient) {
    sessionClient = new SessionClient()
  }
  return sessionClient
}

/**
 * Namespace for session API
 */
export namespace ZeroSession {
  export const list = () => getSessionClient().list()
  export const create = (request?: CreateSessionRequest) => getSessionClient().create(request)
  export const get = (sessionId: string) => getSessionClient().get(sessionId)
  export const remove = (sessionId: string) => getSessionClient().delete(sessionId)
  export const messages = (sessionId: string) => getSessionClient().getMessages(sessionId)
  export const addMessage = (sessionId: string, request: AddMessageRequest) =>
    getSessionClient().addMessage(sessionId, request)
}
