/**
 * Tests for Compare API Handler
 */

import { describe, it, expect, mock, beforeAll, afterAll } from "bun:test"
import type { HttpRequest } from "../../src/api/server/types"

// Mock request helper
function createMockRequest(body: object): HttpRequest {
  const bodyStr = JSON.stringify(body)
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(bodyStr))
      controller.close()
    },
  })

  return {
    method: "POST",
    url: new URL("http://localhost:4400/api/v1/compare"),
    headers: new Map([["content-type", "application/json"]]),
    body: stream,
  }
}

describe("Compare API", () => {
  describe("compare handler", () => {
    it("should reject empty models array", async () => {
      const { compare } = await import("../../src/api/server/handlers/compare")

      const request = createMockRequest({
        models: [],
        prompt: "Hello",
      })

      const response = await compare(request, {})

      expect(response.status).toBe(400)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(false)
      expect(body.error).toContain("models array is required")
    })

    it("should reject missing prompt", async () => {
      const { compare } = await import("../../src/api/server/handlers/compare")

      const request = createMockRequest({
        models: ["anthropic/claude-sonnet-4"],
      })

      const response = await compare(request, {})

      expect(response.status).toBe(400)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(false)
      expect(body.error).toContain("prompt is required")
    })

    it("should reject more than 5 models", async () => {
      const { compare } = await import("../../src/api/server/handlers/compare")

      const request = createMockRequest({
        models: [
          "anthropic/claude-sonnet-4",
          "openai/gpt-4o",
          "google/gemini-pro",
          "mistral/mistral-large",
          "groq/llama-3",
          "xai/grok-2",
        ],
        prompt: "Hello",
      })

      const response = await compare(request, {})

      expect(response.status).toBe(400)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(false)
      expect(body.error).toContain("Maximum 5 models allowed")
    })

    it("should reject invalid model format", async () => {
      const { compare } = await import("../../src/api/server/handlers/compare")

      const request = createMockRequest({
        models: ["invalid-model-without-slash"],
        prompt: "Hello",
      })

      const response = await compare(request, {})

      expect(response.status).toBe(400)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(false)
      expect(body.error).toContain("Invalid model format")
    })
  })

  describe("compareHealth handler", () => {
    it("should return healthy status", async () => {
      const { compareHealth } = await import("../../src/api/server/handlers/compare")

      const request: HttpRequest = {
        method: "GET",
        url: new URL("http://localhost:4400/api/v1/compare/health"),
        headers: new Map(),
        body: null,
      }

      const response = await compareHealth(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(body.data.status).toBe("healthy")
      expect(body.data.max_models).toBe(5)
    })
  })

  describe("listCompareModels handler", () => {
    it("should list available models", async () => {
      // Skip if no providers configured
      const { Provider } = await import("../../src/provider/provider")
      const providers = await Provider.list()

      if (Object.keys(providers).length === 0) {
        console.log("Skipping test - no providers configured")
        return
      }

      const { listCompareModels } = await import("../../src/api/server/handlers/compare")

      const request: HttpRequest = {
        method: "GET",
        url: new URL("http://localhost:4400/api/v1/compare/models"),
        headers: new Map(),
        body: null,
      }

      const response = await listCompareModels(request, {})

      expect(response.status).toBe(200)
      const body = JSON.parse(response.body as string)
      expect(body.success).toBe(true)
      expect(Array.isArray(body.data.models)).toBe(true)
      expect(typeof body.data.total).toBe("number")
    })
  })
})
