import { describe, expect, it, beforeEach } from 'bun:test'
import {
  MessageStore,
  SessionStore,
  isSessionNative,
} from '../src/session.js'
import {
  FallbackMessageStore,
  FallbackSessionStore,
  createUserMessage,
  createAssistantMessage,
  createSystemMessage,
  createToolMessage,
  createSession,
  estimateTokens,
} from '../src/fallback.js'

describe('Session Module', () => {
  describe('isSessionNative', () => {
    it('should be a boolean', () => {
      expect(typeof isSessionNative).toBe('boolean')
    })
  })

  describe('MessageStore', () => {
    let store: MessageStore

    beforeEach(() => {
      store = new MessageStore()
    })

    it('should start empty', () => {
      expect(store.isEmpty()).toBe(true)
      expect(store.len()).toBe(0)
    })

    it('should add messages', () => {
      const msg = createUserMessage('Hello, world!')
      store.push(msg)

      expect(store.isEmpty()).toBe(false)
      expect(store.len()).toBe(1)
    })

    it('should retrieve all messages', () => {
      const msg1 = createUserMessage('First')
      const msg2 = createAssistantMessage('Second')

      store.push(msg1)
      store.push(msg2)

      const messages = store.messages()
      expect(messages.length).toBe(2)
      expect(messages[0]?.content).toBe('First')
      expect(messages[1]?.content).toBe('Second')
    })

    it('should retrieve last N messages', () => {
      store.push(createUserMessage('1'))
      store.push(createAssistantMessage('2'))
      store.push(createUserMessage('3'))

      const lastTwo = store.lastN(2)
      expect(lastTwo.length).toBe(2)
      expect(lastTwo[0]?.content).toBe('2')
      expect(lastTwo[1]?.content).toBe('3')
    })

    it('should clear messages', () => {
      store.push(createUserMessage('Test'))
      expect(store.len()).toBe(1)

      store.clear()
      expect(store.isEmpty()).toBe(true)
    })

    it('should calculate total tokens', () => {
      const msg1 = createUserMessage('Hello')
      msg1.tokens = 10
      const msg2 = createAssistantMessage('World')
      msg2.tokens = 15

      store.push(msg1)
      store.push(msg2)

      expect(store.totalTokens()).toBe(25)
    })
  })

  describe('FallbackMessageStore', () => {
    it('should work the same as MessageStore', () => {
      const store = new FallbackMessageStore()

      expect(store.isEmpty()).toBe(true)

      store.push(createUserMessage('Test'))
      expect(store.len()).toBe(1)

      store.clear()
      expect(store.isEmpty()).toBe(true)
    })
  })

  describe('SessionStore', () => {
    let store: SessionStore
    const testDbPath = '/tmp/test-session-store.db'

    beforeEach(() => {
      store = new SessionStore(testDbPath)
    })

    it('should have correct path', () => {
      expect(store.path).toBe(testDbPath)
    })

    it('should save and load sessions', () => {
      const session = createSession('/tmp/test', 'Test Session')
      session.messages.push(createUserMessage('Hello'))

      store.save(session)

      const loaded = store.load(session.id)
      expect(loaded).not.toBeNull()
      expect(loaded?.id).toBe(session.id)
      expect(loaded?.name).toBe('Test Session')
      expect(loaded?.cwd).toBe('/tmp/test')
    })

    it('should return null for non-existent sessions', () => {
      const loaded = store.load('non-existent-id')
      expect(loaded).toBeNull()
    })

    it('should list sessions', () => {
      const session1 = createSession('/project1', 'Session 1')
      const session2 = createSession('/project2', 'Session 2')

      store.save(session1)
      store.save(session2)

      const sessions = store.list()
      expect(sessions.length).toBeGreaterThanOrEqual(2)
    })

    it('should delete sessions', () => {
      const session = createSession('/tmp/delete-test')
      store.save(session)

      const deleted = store.delete(session.id)
      expect(deleted).toBe(true)

      const loaded = store.load(session.id)
      expect(loaded).toBeNull()
    })
  })

  describe('FallbackSessionStore', () => {
    it('should work as an in-memory store', () => {
      const store = new FallbackSessionStore('/fake/path.db')

      const session = createSession('/project')
      store.save(session)

      const loaded = store.load(session.id)
      expect(loaded).not.toBeNull()
      expect(loaded?.id).toBe(session.id)

      store.delete(session.id)
      expect(store.load(session.id)).toBeNull()
    })
  })

  describe('Message creation utilities', () => {
    it('should create user messages', () => {
      const msg = createUserMessage('Hello')
      expect(msg.role).toBe('user')
      expect(msg.content).toBe('Hello')
      expect(msg.id).toBeTruthy()
      expect(msg.timestamp).toBeTruthy()
    })

    it('should create assistant messages', () => {
      const msg = createAssistantMessage('Response')
      expect(msg.role).toBe('assistant')
      expect(msg.content).toBe('Response')
    })

    it('should create system messages', () => {
      const msg = createSystemMessage('You are a helpful assistant')
      expect(msg.role).toBe('system')
    })

    it('should create tool messages', () => {
      const msg = createToolMessage('call_123', 'calculator', '42')
      expect(msg.role).toBe('tool')
      expect(msg.toolCallId).toBe('call_123')
      expect(msg.toolName).toBe('calculator')
      expect(msg.content).toBe('42')
    })
  })

  describe('Session creation utility', () => {
    it('should create sessions', () => {
      const session = createSession('/project', 'My Session')
      expect(session.id).toBeTruthy()
      expect(session.cwd).toBe('/project')
      expect(session.name).toBe('My Session')
      expect(session.messages).toEqual([])
      expect(session.createdAt).toBeTruthy()
      expect(session.updatedAt).toBeTruthy()
    })

    it('should create sessions without name', () => {
      const session = createSession('/project')
      expect(session.name).toBeUndefined()
    })
  })

  describe('Token estimation', () => {
    it('should estimate tokens based on character count', () => {
      // Roughly 4 chars per token
      expect(estimateTokens('Hello')).toBe(2) // 5 chars / 4 = 1.25 -> ceil = 2
      expect(estimateTokens('Hello, World!')).toBe(4) // 13 chars / 4 = 3.25 -> ceil = 4
      expect(estimateTokens('')).toBe(0)
    })
  })
})
