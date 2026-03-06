import { describe, expect, it, beforeEach } from 'bun:test'
import {
  Compactor,
  estimateTokenCount,
  createDefaultCompactor,
  isCompactionNative,
  type CompactionStrategy,
} from '../src/compaction.js'
import type { Message } from '../src/types.js'

// Helper to create test messages
function createMessage(
  role: Message['role'],
  content: string,
  tokens?: number
): Message {
  return {
    id: `msg_${Math.random().toString(36).slice(2, 9)}`,
    role,
    content,
    timestamp: new Date().toISOString(),
    tokens,
    compacted: false,
  }
}

function createMessages(count: number, tokensEach: number): Message[] {
  return Array.from({ length: count }, (_, i) =>
    createMessage('user', `Message ${i}`, tokensEach)
  )
}

describe('Compaction Module', () => {
  describe('isCompactionNative', () => {
    it('should be a boolean', () => {
      expect(typeof isCompactionNative).toBe('boolean')
    })
  })

  describe('estimateTokenCount', () => {
    it('should estimate tokens based on text length', () => {
      // ~4 chars per token
      expect(estimateTokenCount('Hello')).toBeGreaterThan(0)
      expect(estimateTokenCount('')).toBe(0)
    })

    it('should return larger count for longer text', () => {
      const short = estimateTokenCount('Hi')
      const long = estimateTokenCount('This is a much longer message with many words')
      expect(long).toBeGreaterThan(short)
    })
  })

  describe('createDefaultCompactor', () => {
    it('should create a compactor with default settings', () => {
      const compactor = createDefaultCompactor()
      expect(compactor).toBeInstanceOf(Compactor)
    })
  })

  describe('Compactor', () => {
    describe('constructor', () => {
      it('should create with default settings', () => {
        const compactor = new Compactor()
        expect(compactor).toBeInstanceOf(Compactor)
      })

      it('should create with custom limits', () => {
        const compactor = new Compactor(10000, 8000)
        expect(compactor).toBeInstanceOf(Compactor)
      })
    })

    describe('isNative', () => {
      it('should report native status', () => {
        const compactor = new Compactor()
        expect(typeof compactor.isNative).toBe('boolean')
      })
    })

    describe('needsCompaction', () => {
      it('should return false for small message sets', () => {
        const compactor = new Compactor(10000, 8000)
        const messages = createMessages(5, 100)
        expect(compactor.needsCompaction(messages)).toBe(false)
      })

      it('should return true when over max tokens', () => {
        const compactor = new Compactor(1000, 800) // Small limits for testing
        const messages = createMessages(20, 100) // 2000 tokens > 1000 max
        expect(compactor.needsCompaction(messages)).toBe(true)
      })
    })

    describe('compact', () => {
      it('should return unchanged messages when not over limit', () => {
        const compactor = new Compactor(10000, 8000)
        const messages = createMessages(5, 100)
        const result = compactor.compact(messages)

        expect(result.metrics.messagesBefore).toBe(5)
        expect(result.metrics.messagesAfter).toBe(5)
        expect(result.messages.length).toBe(5)
      })

      it('should reduce messages when over limit', () => {
        const compactor = new Compactor(1000, 500) // Small limits
        const messages = createMessages(20, 100) // 2000 tokens > 1000 max
        const result = compactor.compact(messages)

        expect(result.metrics.messagesAfter).toBeLessThan(result.metrics.messagesBefore)
        expect(result.metrics.tokensAfter).toBeLessThanOrEqual(1000)
      })

      it('should preserve system messages', () => {
        const compactor = new Compactor(500, 300)
        const messages = [
          createMessage('system', 'System prompt', 100),
          ...createMessages(10, 100),
        ]
        const result = compactor.compact(messages)

        // System message should still be present
        const hasSystem = result.messages.some((m) => m.role === 'system')
        expect(hasSystem).toBe(true)
      })

      it('should return valid metrics', () => {
        const compactor = new Compactor(1000, 500)
        const messages = createMessages(20, 100)
        const result = compactor.compact(messages)

        expect(result.metrics.messagesBefore).toBe(20)
        expect(result.metrics.tokensAfter).toBeLessThanOrEqual(result.metrics.tokensBefore)
      })
    })

    describe('setStrategy', () => {
      it('should accept strategy changes', () => {
        const compactor = new Compactor()
        // Should not throw
        compactor.setStrategy('remove_oldest')
        compactor.setStrategy('summarize')
        compactor.setStrategy('hybrid')
      })
    })
  })

  describe('Integration with Messages', () => {
    it('should handle mixed message roles', () => {
      const compactor = new Compactor(500, 300)
      const messages: Message[] = [
        createMessage('system', 'You are a helpful assistant', 50),
        createMessage('user', 'Hello!', 10),
        createMessage('assistant', 'Hi there! How can I help?', 20),
        createMessage('user', 'Can you explain X?', 15),
        createMessage('assistant', 'Of course! X is...', 200),
        createMessage('user', 'Thanks!', 5),
        createMessage('assistant', 'You\'re welcome!', 10),
      ]

      const result = compactor.compact(messages)

      // Should preserve message structure
      expect(result.messages.every((m) => m.id && m.role && m.content)).toBe(true)
    })

    it('should handle tool messages', () => {
      const compactor = new Compactor(1000, 800)
      const toolMessage: Message = {
        id: 'tool_msg_1',
        role: 'tool',
        content: '{"result": 42}',
        timestamp: new Date().toISOString(),
        tokens: 20,
        toolCallId: 'call_123',
        toolName: 'calculator',
        compacted: false,
      }

      const messages = [
        createMessage('user', 'Calculate 6 * 7', 10),
        toolMessage,
        createMessage('assistant', 'The result is 42', 15),
      ]

      const result = compactor.compact(messages)
      expect(result.messages.length).toBeGreaterThan(0)
    })

    it('should handle empty messages array', () => {
      const compactor = new Compactor()
      const result = compactor.compact([])

      expect(result.metrics.messagesBefore).toBe(0)
      expect(result.metrics.messagesAfter).toBe(0)
      expect(result.messages.length).toBe(0)
    })
  })
})
