/**
 * Session management - MessageStore and SessionStore wrappers
 *
 * These classes provide a TypeScript interface to the Rust-native session
 * storage using high-performance Rust implementations via NAPI.
 *
 * @example
 * ```typescript
 * import { MessageStore, SessionStore } from '@codecoder-ai/core'
 *
 * // In-memory message storage
 * const store = new MessageStore()
 * store.push({ role: 'user', content: 'Hello!' })
 *
 * // Persistent session storage
 * const sessions = new SessionStore('/path/to/sessions.db')
 * sessions.save({ id: 'abc', cwd: '/project', messages: store.messages() })
 * ```
 */

import type { Message, SessionData, IMessageStore, ISessionStore } from './types.js'
import {
  createMessageStore as createMessageStoreNative,
  openSessionStore as openSessionStoreNative,
  type MessageStoreHandle,
  type SessionStoreHandle,
  type NapiMessage,
  type NapiSessionData,
} from './binding.js'

// Conversion utilities
function toNativeMessage(msg: Message): NapiMessage {
  return {
    id: msg.id,
    role: msg.role,
    content: msg.content,
    timestamp: msg.timestamp,
    tokens: msg.tokens,
    toolCallId: msg.toolCallId,
    toolName: msg.toolName,
    compacted: msg.compacted,
  }
}

function fromNativeMessage(msg: NapiMessage): Message {
  return {
    id: msg.id,
    role: msg.role as Message['role'],
    content: msg.content,
    timestamp: msg.timestamp,
    tokens: msg.tokens ?? undefined,
    toolCallId: msg.toolCallId ?? undefined,
    toolName: msg.toolName ?? undefined,
    compacted: msg.compacted,
  }
}

function toNativeSessionData(session: SessionData): NapiSessionData {
  return {
    id: session.id,
    name: session.name,
    cwd: session.cwd,
    createdAt: session.createdAt,
    updatedAt: session.updatedAt,
    messages: session.messages.map(toNativeMessage),
  }
}

function fromNativeSessionData(session: NapiSessionData): SessionData {
  return {
    id: session.id,
    name: session.name ?? undefined,
    cwd: session.cwd,
    createdAt: session.createdAt,
    updatedAt: session.updatedAt,
    messages: session.messages.map(fromNativeMessage),
  }
}

/**
 * In-memory message store
 *
 * Stores conversation messages in memory with efficient token counting.
 * Uses native Rust implementation via NAPI.
 */
export class MessageStore implements IMessageStore {
  private handle: MessageStoreHandle

  constructor() {
    this.handle = createMessageStoreNative()
  }

  /** Add a message to the store */
  push(message: Message): void {
    this.handle.push(toNativeMessage(message))
  }

  /** Get all messages */
  messages(): Message[] {
    return this.handle.messages().map(fromNativeMessage)
  }

  /** Get the last N messages */
  lastN(n: number): Message[] {
    return this.handle.lastN(n).map(fromNativeMessage)
  }

  /** Get total token count (estimated) */
  totalTokens(): number {
    return this.handle.totalTokens()
  }

  /** Clear all messages */
  clear(): void {
    this.handle.clear()
  }

  /** Get message count */
  len(): number {
    return this.handle.len()
  }

  /** Check if store is empty */
  isEmpty(): boolean {
    return this.handle.isEmpty()
  }
}

/**
 * Persistent session store using SQLite
 *
 * Stores sessions with their messages persistently.
 * Uses native Rust implementation via NAPI.
 */
export class SessionStore implements ISessionStore {
  private handle: SessionStoreHandle
  private dbPath: string

  /**
   * Open or create a session store
   * @param path - Path to the SQLite database file
   */
  constructor(path: string) {
    this.dbPath = path
    this.handle = openSessionStoreNative(path)
  }

  /** Save a session */
  save(session: SessionData): void {
    this.handle.save(toNativeSessionData(session))
  }

  /** Load a session by ID */
  load(id: string): SessionData | null {
    const result = this.handle.load(id)
    return result ? fromNativeSessionData(result) : null
  }

  /** List all sessions (without messages for efficiency) */
  list(): SessionData[] {
    return this.handle.list().map(fromNativeSessionData)
  }

  /** Delete a session */
  delete(id: string): boolean {
    return this.handle.delete(id)
  }

  /** Get the database path */
  get path(): string {
    return this.dbPath
  }
}

// Native bindings are always required
export const isSessionNative = true

// ============================================================================
// Message/Session creation utilities (pure TypeScript helpers)
// ============================================================================

/** Generate a UUID v4 */
function generateUuid(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0
    const v = c === 'x' ? r : (r & 0x3) | 0x8
    return v.toString(16)
  })
}

/** Create a new user message */
export function createUserMessage(content: string): Message {
  return {
    id: generateUuid(),
    role: 'user',
    content,
    timestamp: new Date().toISOString(),
    compacted: false,
  }
}

/** Create a new assistant message */
export function createAssistantMessage(content: string): Message {
  return {
    id: generateUuid(),
    role: 'assistant',
    content,
    timestamp: new Date().toISOString(),
    compacted: false,
  }
}

/** Create a new system message */
export function createSystemMessage(content: string): Message {
  return {
    id: generateUuid(),
    role: 'system',
    content,
    timestamp: new Date().toISOString(),
    compacted: false,
  }
}

/** Create a new tool message */
export function createToolMessage(
  toolCallId: string,
  toolName: string,
  content: string
): Message {
  return {
    id: generateUuid(),
    role: 'tool',
    content,
    timestamp: new Date().toISOString(),
    toolCallId,
    toolName,
    compacted: false,
  }
}

/** Create a new session */
export function createSession(cwd: string, name?: string): SessionData {
  const now = new Date().toISOString()
  return {
    id: generateUuid(),
    name,
    cwd,
    createdAt: now,
    updatedAt: now,
    messages: [],
  }
}

/** Estimate token count for a message (rough approximation: ~4 chars per token) */
export function estimateTokens(content: string): number {
  return Math.ceil(content.length / 4)
}
