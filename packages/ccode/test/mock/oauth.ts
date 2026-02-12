/**
 * OAuth/Browser Mock for Testing
 *
 * Provides mocking utilities for OAuth flows and browser launches
 * Used by lifecycle tests that need to simulate OAuth authentication
 */

import { mock } from "bun:test"
import { EventEmitter } from "events"

export interface OAuthMockState {
  openedUrl: string | undefined
  shouldFail: boolean
  callCount: number
}

const state: OAuthMockState = {
  openedUrl: undefined,
  shouldFail: false,
  callCount: 0,
}

/**
 * Sets up the OAuth/browser mock
 * Must be called before importing modules that use 'open'
 */
export function setupOAuthMock() {
  mock.module("open", () => ({
    default: async (url: string) => {
      state.openedUrl = url
      state.callCount++

      const subprocess = new EventEmitter()

      if (state.shouldFail) {
        setTimeout(() => {
          subprocess.emit("error", new Error("spawn xdg-open ENOENT"))
        }, 10)
      }

      return subprocess
    },
  }))

  return {
    /**
     * Get the URL that was passed to open()
     */
    getOpenedUrl: () => state.openedUrl,

    /**
     * Get the number of times open() was called
     */
    getCallCount: () => state.callCount,

    /**
     * Configure whether open() should fail
     */
    setShouldFail: (shouldFail: boolean) => {
      state.shouldFail = shouldFail
    },

    /**
     * Simulate OAuth callback with authorization code
     */
    simulateCallback: async (code: string, callbackUrl?: string) => {
      const url = callbackUrl ?? `http://localhost:3847/oauth/callback?code=${code}`
      try {
        await fetch(url)
      } catch {
        // Callback server may not be running in tests
      }
    },

    /**
     * Reset mock state
     */
    reset: () => {
      state.openedUrl = undefined
      state.shouldFail = false
      state.callCount = 0
    },
  }
}

/**
 * Create mock MCP transport classes for OAuth testing
 */
export function setupMcpTransportMock() {
  const transportCalls: Array<{
    type: "streamable" | "sse"
    url: string
    options: { authProvider?: unknown }
  }> = []

  class MockUnauthorizedError extends Error {
    constructor() {
      super("Unauthorized")
      this.name = "UnauthorizedError"
    }
  }

  mock.module("@modelcontextprotocol/sdk/client/streamableHttp.js", () => ({
    StreamableHTTPClientTransport: class MockStreamableHTTP {
      url: string
      authProvider: { redirectToAuthorization?: (url: URL) => Promise<void> } | undefined

      constructor(url: URL, options?: { authProvider?: { redirectToAuthorization?: (url: URL) => Promise<void> } }) {
        this.url = url.toString()
        this.authProvider = options?.authProvider
        transportCalls.push({
          type: "streamable",
          url: url.toString(),
          options: options ?? {},
        })
      }

      async start() {
        if (this.authProvider?.redirectToAuthorization) {
          await this.authProvider.redirectToAuthorization(
            new URL("https://auth.example.com/authorize?client_id=test"),
          )
        }
        throw new MockUnauthorizedError()
      }

      async finishAuth(_code: string) {}
    },
  }))

  mock.module("@modelcontextprotocol/sdk/client/sse.js", () => ({
    SSEClientTransport: class MockSSE {
      constructor(url: URL) {
        transportCalls.push({
          type: "sse",
          url: url.toString(),
          options: {},
        })
      }

      async start() {
        throw new Error("Mock SSE transport cannot connect")
      }
    },
  }))

  mock.module("@modelcontextprotocol/sdk/client/index.js", () => ({
    Client: class MockClient {
      async connect(transport: { start: () => Promise<void> }) {
        await transport.start()
      }
    },
  }))

  mock.module("@modelcontextprotocol/sdk/client/auth.js", () => ({
    UnauthorizedError: MockUnauthorizedError,
  }))

  return {
    getTransportCalls: () => transportCalls,
    reset: () => {
      transportCalls.length = 0
    },
    MockUnauthorizedError,
  }
}
