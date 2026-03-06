import { describe, expect, it, beforeEach, afterEach, mock } from 'bun:test'
import {
  McpClientManager,
  isMcpNative,
} from '../src/mcp.js'
import type { McpConfig, McpConnectionStatus, McpTool } from '../src/protocol.js'

describe('MCP Module', () => {
  describe('isMcpNative', () => {
    it('should be a boolean', () => {
      expect(typeof isMcpNative).toBe('boolean')
    })

    it('should be true when native bindings are available', () => {
      // This test verifies the native bindings are loaded correctly
      // In a properly built environment, this should be true
      expect(isMcpNative).toBe(true)
    })
  })

  describe('McpClientManager', () => {
    let manager: McpClientManager

    beforeEach(() => {
      manager = new McpClientManager()
    })

    afterEach(async () => {
      await manager.closeAll().catch(() => {})
    })

    describe('constructor', () => {
      it('should create a manager instance', () => {
        expect(manager).toBeInstanceOf(McpClientManager)
      })

      it('should report native status', () => {
        expect(typeof manager.isNative).toBe('boolean')
        expect(manager.isNative).toBe(true)
      })
    })

    describe('status', () => {
      it('should return empty object when no clients added', async () => {
        const statuses = await manager.status()
        expect(statuses).toEqual({})
      })
    })

    describe('listTools', () => {
      it('should return empty object when no clients connected', async () => {
        const tools = await manager.listTools()
        expect(tools).toEqual({})
      })
    })

    describe('add - error handling', () => {
      it('should return failed status for invalid stdio config', async () => {
        const config: McpConfig = {
          name: 'test-invalid',
          transport: 'stdio',
          command: ['nonexistent-command-that-does-not-exist'],
        }

        const status = await manager.add('test-invalid', config)

        // Should return failed status, not throw
        expect(status.status).toBe('failed')
        expect(status.error).toBeDefined()
      })

      it('should return failed status for invalid http URL', async () => {
        const config: McpConfig = {
          name: 'test-http-invalid',
          transport: 'http',
          url: 'http://127.0.0.1:1', // Invalid port
          timeoutMs: 1000, // Short timeout for test
        }

        const status = await manager.add('test-http-invalid', config)

        expect(status.status).toBe('failed')
      })
    })

    describe('remove', () => {
      it('should not throw when removing non-existent client', async () => {
        // Should not throw
        await manager.remove('nonexistent')
      })
    })

    describe('closeAll', () => {
      it('should not throw when no clients exist', async () => {
        // Should not throw
        await manager.closeAll()
      })
    })

    // NOTE: OAuth methods are defined in Rust NAPI but not yet compiled into the current native module.
    // These tests are skipped until the native module is rebuilt with OAuth support.
    // See: services/zero-core/src/napi/protocol.rs lines 289-362
    describe.skip('OAuth methods (pending native rebuild)', () => {
      it('should load OAuth without error', async () => {
        await manager.loadOAuth()
      })

      it('should return not_authenticated for unknown server', async () => {
        const status = await manager.getOAuthStatus('nonexistent-server')
        expect(status).toBe('not_authenticated')
      })

      it('should return false for hasOAuthCredentials on unknown server', async () => {
        const has = await manager.hasOAuthCredentials('nonexistent-server')
        expect(has).toBe(false)
      })

      it('should not throw when canceling OAuth on unknown server', async () => {
        await manager.cancelOAuth('nonexistent-server')
      })

      it('should not throw when removing OAuth on unknown server', async () => {
        await manager.removeOAuth('nonexistent-server')
      })
    })
  })

  describe('Config Conversion', () => {
    it('should accept all transport types', () => {
      const manager = new McpClientManager()

      // These configs should be accepted without throwing
      const configs: McpConfig[] = [
        { name: 'stdio', transport: 'stdio', command: ['test'] },
        { name: 'http', transport: 'http', url: 'http://example.com/mcp' },
        { name: 'sse', transport: 'sse', url: 'http://example.com/mcp' },
      ]

      for (const config of configs) {
        expect(() => {
          // Just verify the config is accepted - we won't actually connect
          void manager.add(config.name, config)
        }).not.toThrow()
      }
    })

    it('should handle optional OAuth config', async () => {
      const manager = new McpClientManager()

      const config: McpConfig = {
        name: 'oauth-test',
        transport: 'http',
        url: 'http://127.0.0.1:1', // Invalid to fail fast
        timeoutMs: 500,
        oauth: {
          clientId: 'test-client',
          clientSecret: 'test-secret',
          scope: 'read write',
        },
      }

      // Should process OAuth config without throwing
      const status = await manager.add('oauth-test', config)
      expect(status.status).toBe('failed') // Expected to fail due to invalid URL
    })

    it('should handle oauthDisabled flag', async () => {
      const manager = new McpClientManager()

      const config: McpConfig = {
        name: 'no-oauth',
        transport: 'http',
        url: 'http://127.0.0.1:1',
        timeoutMs: 500,
        oauthDisabled: true,
      }

      // Should process config without OAuth
      const status = await manager.add('no-oauth', config)
      expect(status.status).toBe('failed')
    })
  })

  describe('Type Exports', () => {
    it('should export McpConfig type', () => {
      const config: McpConfig = {
        name: 'test',
        transport: 'stdio',
      }
      expect(config.name).toBe('test')
    })

    it('should export McpConnectionStatus type', () => {
      const status: McpConnectionStatus = {
        status: 'connected',
      }
      expect(status.status).toBe('connected')
    })

    it('should export McpTool type', () => {
      const tool: McpTool = {
        name: 'test-tool',
        description: 'A test tool',
        inputSchema: { type: 'object' },
      }
      expect(tool.name).toBe('test-tool')
    })
  })
})
