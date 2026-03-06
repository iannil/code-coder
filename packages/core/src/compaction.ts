/**
 * Compaction module - Context window management
 *
 * This module provides TypeScript wrappers for the Rust-native compaction
 * functionality. It helps manage context window limits by intelligently
 * removing or summarizing older messages.
 *
 * @example
 * ```typescript
 * import { Compactor, CompactionStrategy } from '@codecoder-ai/core'
 *
 * // Create a compactor with custom limits
 * const compactor = new Compactor(128_000, 100_000)
 *
 * // Check if compaction is needed
 * if (compactor.needsCompaction(messages)) {
 *   const result = compactor.compact(messages)
 *   console.log(`Reduced from ${result.metrics.messagesBefore} to ${result.metrics.messagesAfter} messages`)
 * }
 * ```
 */

import type { Message } from './types.js'
import type {
  CompactorHandle,
  NapiMessage,
  NapiCompactResult,
  NapiCompactionResult,
  NapiCompactionStrategy,
} from './binding.d.ts'

// Native bindings loader
let createCompactorNative: (() => CompactorHandle) | null = null
let createCompactorWithLimitsNative: ((max: number, target: number) => CompactorHandle) | null = null
let estimateTokensNative: ((text: string) => number) | null = null

try {
  const bindings = await import('./binding.js')
  createCompactorNative = bindings.createCompactor
  createCompactorWithLimitsNative = bindings.createCompactorWithLimits
  estimateTokensNative = bindings.estimateTokens
} catch {
  // Native bindings not available
}

/**
 * Compaction strategy
 */
export type CompactionStrategy = 'remove_oldest' | 'summarize' | 'hybrid'

/**
 * Result of a compaction operation
 */
export interface CompactionResult {
  /** Number of messages before compaction */
  messagesBefore: number
  /** Number of messages after compaction */
  messagesAfter: number
  /** Tokens before compaction */
  tokensBefore: number
  /** Tokens after compaction */
  tokensAfter: number
  /** Summary generated (if any) */
  summary?: string
}

/**
 * Combined result with metrics and compacted messages
 */
export interface CompactResult {
  /** Compaction metrics */
  metrics: CompactionResult
  /** The compacted messages */
  messages: Message[]
}

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

function fromNativeResult(result: NapiCompactionResult): CompactionResult {
  return {
    messagesBefore: result.messagesBefore,
    messagesAfter: result.messagesAfter,
    tokensBefore: result.tokensBefore,
    tokensAfter: result.tokensAfter,
    summary: result.summary ?? undefined,
  }
}

function toNativeStrategy(strategy: CompactionStrategy): NapiCompactionStrategy {
  switch (strategy) {
    case 'remove_oldest':
      return 'RemoveOldest' as NapiCompactionStrategy
    case 'summarize':
      return 'Summarize' as NapiCompactionStrategy
    case 'hybrid':
      return 'Hybrid' as NapiCompactionStrategy
  }
}

/**
 * Compactor for managing context window
 *
 * Provides intelligent message compaction to stay within token limits.
 * Uses native Rust implementation for performance.
 */
export class Compactor {
  private handle: CompactorHandle | null = null
  private maxTokens: number
  private targetTokens: number

  /**
   * Create a new compactor
   * @param maxTokens - Maximum tokens before compaction is triggered (default: 128,000)
   * @param targetTokens - Target token count after compaction (default: 100,000)
   */
  constructor(maxTokens = 128_000, targetTokens = 100_000) {
    this.maxTokens = maxTokens
    this.targetTokens = targetTokens

    if (createCompactorWithLimitsNative) {
      this.handle = createCompactorWithLimitsNative(maxTokens, targetTokens)
    }
  }

  /** Check if native implementation is available */
  get isNative(): boolean {
    return this.handle !== null
  }

  /**
   * Check if compaction is needed for the given messages
   */
  needsCompaction(messages: Message[]): boolean {
    if (this.handle) {
      return this.handle.needsCompaction(messages.map(toNativeMessage))
    }

    // Fallback: simple token count check
    const totalTokens = messages.reduce((sum, m) => sum + (m.tokens ?? Math.ceil(m.content.length / 4)), 0)
    return totalTokens > this.maxTokens
  }

  /**
   * Compact messages to fit within token limit
   * @returns CompactResult with metrics and compacted messages
   */
  compact(messages: Message[]): CompactResult {
    if (this.handle) {
      const result = this.handle.compact(messages.map(toNativeMessage))
      return {
        metrics: fromNativeResult(result.result),
        messages: result.messages.map(fromNativeMessage),
      }
    }

    // Fallback: simple oldest-first removal
    return this.fallbackCompact(messages)
  }

  /**
   * Set the compaction strategy
   */
  setStrategy(strategy: CompactionStrategy): void {
    if (this.handle) {
      this.handle.setStrategy(toNativeStrategy(strategy))
    }
    // Note: fallback doesn't support strategy changes
  }

  /**
   * Fallback compaction when native bindings unavailable
   */
  private fallbackCompact(messages: Message[]): CompactResult {
    const tokensBefore = messages.reduce((sum, m) => sum + (m.tokens ?? Math.ceil(m.content.length / 4)), 0)
    const messagesBefore = messages.length

    if (tokensBefore <= this.maxTokens) {
      return {
        metrics: {
          messagesBefore,
          messagesAfter: messagesBefore,
          tokensBefore,
          tokensAfter: tokensBefore,
        },
        messages: [...messages],
      }
    }

    // Keep system messages and recent history
    const systemMessages = messages.filter((m) => m.role === 'system')
    const nonSystemMessages = messages.filter((m) => m.role !== 'system')

    let runningTokens = systemMessages.reduce((sum, m) => sum + (m.tokens ?? Math.ceil(m.content.length / 4)), 0)
    const keptMessages: Message[] = []

    // Work backwards, keeping messages until we hit target
    for (let i = nonSystemMessages.length - 1; i >= 0; i--) {
      const msg = nonSystemMessages[i]!
      const msgTokens = msg.tokens ?? Math.ceil(msg.content.length / 4)
      if (runningTokens + msgTokens > this.targetTokens) break
      runningTokens += msgTokens
      keptMessages.unshift(msg)
    }

    const result = [...systemMessages, ...keptMessages]
    const tokensAfter = result.reduce((sum, m) => sum + (m.tokens ?? Math.ceil(m.content.length / 4)), 0)

    return {
      metrics: {
        messagesBefore,
        messagesAfter: result.length,
        tokensBefore,
        tokensAfter,
      },
      messages: result,
    }
  }
}

/**
 * Estimate token count for text (fast, approximate)
 *
 * Uses native Rust implementation if available, otherwise falls back
 * to a simple character-based estimate (1 token ≈ 4 characters).
 */
export function estimateTokenCount(text: string): number {
  if (estimateTokensNative) {
    return estimateTokensNative(text)
  }
  // Fallback: ~4 chars per token
  return Math.ceil(text.length / 4)
}

/**
 * Create a compactor with default settings
 */
export function createDefaultCompactor(): Compactor {
  return new Compactor()
}

/**
 * Check if native compaction is available
 */
export const isCompactionNative = createCompactorNative !== null
