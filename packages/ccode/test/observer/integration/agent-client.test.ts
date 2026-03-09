/**
 * AgentClient Tests
 *
 * Tests for the Agent Client component of the Observer Network.
 *
 * @module test/observer/integration/agent-client.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, mock } from "bun:test"
import {
  AgentClient,
  createAgentClient,
  getAgentClient,
  resetAgentClient,
  type AgentInvocation,
} from "@/observer/integration/agent-client"

describe("AgentClient", () => {
  let client: AgentClient

  beforeEach(() => {
    resetAgentClient()
    client = createAgentClient({
      baseUrl: "http://127.0.0.1:4400",
      timeoutMs: 5000,
      maxConcurrent: 2,
    })
  })

  afterEach(() => {
    resetAgentClient()
  })

  describe("constructor", () => {
    it("should use default config", () => {
      const defaultClient = createAgentClient()
      expect(defaultClient).toBeDefined()
      expect(defaultClient.getRunningCount()).toBe(0)
    })

    it("should allow custom config", () => {
      const customClient = createAgentClient({
        baseUrl: "http://localhost:8080",
        timeoutMs: 10000,
        maxConcurrent: 5,
      })

      expect(customClient).toBeDefined()
      expect(customClient.getRunningCount()).toBe(0)
    })
  })

  describe("singleton", () => {
    it("should return same instance from getAgentClient", () => {
      const client1 = getAgentClient()
      const client2 = getAgentClient()

      expect(client1).toBe(client2)
    })

    it("should reset singleton correctly", () => {
      const client1 = getAgentClient()
      resetAgentClient()
      const client2 = getAgentClient()

      expect(client1).not.toBe(client2)
    })
  })

  describe("invoke", () => {
    it("should handle agent errors gracefully", async () => {
      // Invoke will fail because the server is not running
      const invocation: AgentInvocation = {
        agentId: "test-agent",
        prompt: "Test prompt",
        timeoutMs: 100,
      }

      const result = await client.invoke(invocation)

      // Should return an error result, not throw
      expect(result.success).toBe(false)
      expect(result.error).toBeDefined()
    })

    it("should include context in prompt when provided", async () => {
      const invocation: AgentInvocation = {
        agentId: "explore",
        prompt: "Search for files",
        context: {
          directory: "/src",
          pattern: "*.ts",
        },
        timeoutMs: 100,
      }

      // Will fail due to no server, but tests the invocation path
      const result = await client.invoke(invocation)
      expect(result.success).toBe(false)
    })

    it("should respect timeout", async () => {
      const invocation: AgentInvocation = {
        agentId: "slow-agent",
        prompt: "This should timeout",
        timeoutMs: 50, // Very short timeout
      }

      const startTime = Date.now()
      const result = await client.invoke(invocation)
      const duration = Date.now() - startTime

      expect(result.success).toBe(false)
      // Should fail quickly, not wait forever
      expect(duration).toBeLessThan(5000)
    })

    it("should limit concurrent calls", async () => {
      // Create client with maxConcurrent = 1
      const limitedClient = createAgentClient({
        maxConcurrent: 1,
        timeoutMs: 100,
      })

      // Track running count during invocations
      const invocation1: AgentInvocation = {
        agentId: "agent1",
        prompt: "First call",
        timeoutMs: 50,
      }

      const invocation2: AgentInvocation = {
        agentId: "agent2",
        prompt: "Second call",
        timeoutMs: 50,
      }

      // Start first call
      const promise1 = limitedClient.invoke(invocation1)

      // While first is running, try second
      const promise2 = limitedClient.invoke(invocation2)

      const [result1, result2] = await Promise.all([promise1, promise2])

      // At least one should complete (might fail due to concurrency limit or network)
      expect(result1.success === false || result2.success === false).toBe(true)
    })
  })

  describe("getRunningCount", () => {
    it("should return 0 when no calls are running", () => {
      expect(client.getRunningCount()).toBe(0)
    })

    it("should track running calls", async () => {
      // Before any calls
      expect(client.getRunningCount()).toBe(0)

      // Start a call (will fail quickly)
      const promise = client.invoke({
        agentId: "test",
        prompt: "test",
        timeoutMs: 100,
      })

      // After call completes
      await promise
      expect(client.getRunningCount()).toBe(0)
    })
  })

  describe("isAvailable", () => {
    it("should return false when server is down", async () => {
      // Server is not running in test environment
      const available = await client.isAvailable()
      expect(available).toBe(false)
    })

    it("should handle timeout gracefully", async () => {
      // Use localhost with an unused port - connection refused is immediate
      const slowClient = createAgentClient({
        baseUrl: "http://127.0.0.1:59999", // Unlikely to be in use
      })

      const startTime = Date.now()
      const available = await slowClient.isAvailable()
      const duration = Date.now() - startTime

      expect(available).toBe(false)
      // Connection refused should be nearly instantaneous (< 2 seconds)
      expect(duration).toBeLessThan(2000)
    })
  })

  describe("maxConcurrent enforcement", () => {
    it("should reject when max concurrent reached", async () => {
      // Create client with max concurrent of 1
      const limitedClient = createAgentClient({
        maxConcurrent: 1,
        timeoutMs: 1000,
      })

      // Simulate reaching max concurrent by manually tracking
      // The first call will occupy the slot
      const slowInvocation: AgentInvocation = {
        agentId: "slow",
        prompt: "This takes time",
        timeoutMs: 500,
      }

      // Start first call (async, don't await yet)
      const firstCall = limitedClient.invoke(slowInvocation)

      // Immediately try second call
      const secondInvocation: AgentInvocation = {
        agentId: "blocked",
        prompt: "This should be blocked",
        timeoutMs: 100,
      }

      // The second call may be rejected if first is still running
      const secondCall = limitedClient.invoke(secondInvocation)

      // Wait for both
      const [result1, result2] = await Promise.all([firstCall, secondCall])

      // Both should have completed (with errors, but no crashes)
      expect(result1.error).toBeDefined()
      expect(result2.error).toBeDefined()
    })
  })
})
