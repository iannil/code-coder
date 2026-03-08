/**
 * Channels Client Tests
 *
 * Tests for the Observer Network channels integration client.
 *
 * @module test/observer/integration/channels-client.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, spyOn } from "bun:test"
import {
  ChannelsClient,
  createChannelsClient,
  getChannelsClient,
  resetChannelsClient,
} from "@/observer/integration/channels-client"

describe("ChannelsClient", () => {
  let client: ChannelsClient
  let fetchSpy: ReturnType<typeof spyOn>

  beforeEach(() => {
    resetChannelsClient()
    client = createChannelsClient({
      baseUrl: "http://localhost:4402",
      defaultChannel: "telegram",
      defaultChannelId: "123456",
    })

    // Mock global fetch
    fetchSpy = spyOn(globalThis, "fetch")
  })

  afterEach(() => {
    fetchSpy.mockRestore()
  })

  describe("send", () => {
    it("should send a text message successfully", async () => {
      const mockResponse = {
        success: true,
        messageId: "msg_123",
      }

      fetchSpy.mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      } as Response)

      const result = await client.send({
        channelType: "telegram",
        channelId: "123456",
        content: { type: "text", text: "Hello, World!" },
      })

      expect(result.success).toBe(true)
      expect(result.messageId).toBe("msg_123")
      expect(fetchSpy).toHaveBeenCalledTimes(1)
    })

    it("should handle HTTP error response", async () => {
      fetchSpy.mockResolvedValueOnce({
        ok: false,
        status: 500,
        json: async () => ({ error: "Internal Server Error" }),
      } as Response)

      const result = await client.send({
        channelType: "telegram",
        channelId: "123456",
        content: { type: "text", text: "Hello" },
      })

      expect(result.success).toBe(false)
      expect(result.error).toBe("Internal Server Error")
    })

    it("should handle network error", async () => {
      fetchSpy.mockRejectedValueOnce(new Error("Connection refused"))

      const result = await client.send({
        channelType: "telegram",
        channelId: "123456",
        content: { type: "text", text: "Hello" },
      })

      expect(result.success).toBe(false)
      expect(result.error).toBe("Connection refused")
    })
  })

  describe("sendText", () => {
    it("should send text to default channel", async () => {
      fetchSpy.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true, messageId: "msg_456" }),
      } as Response)

      const result = await client.sendText("Hello from default channel")

      expect(result.success).toBe(true)
      expect(result.messageId).toBe("msg_456")
    })

    it("should fail if default channel not configured", async () => {
      const noDefaultClient = createChannelsClient({ baseUrl: "http://localhost:4402" })
      const result = await noDefaultClient.sendText("Hello")

      expect(result.success).toBe(false)
      expect(result.error).toBe("Default channel not configured")
    })
  })

  describe("sendMarkdown", () => {
    it("should send markdown to default channel", async () => {
      fetchSpy.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      } as Response)

      const result = await client.sendMarkdown("**Bold** and _italic_")

      expect(result.success).toBe(true)
    })
  })

  describe("sendWithButtons", () => {
    it("should send message with inline buttons", async () => {
      fetchSpy.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      } as Response)

      const result = await client.sendWithButtons({
        channelType: "telegram",
        channelId: "123456",
        text: "Choose an option:",
        buttons: [[{ text: "Yes", callbackData: "yes" }, { text: "No", callbackData: "no" }]],
      })

      expect(result.success).toBe(true)
    })
  })

  describe("isAvailable", () => {
    it("should return true when service is available", async () => {
      fetchSpy.mockResolvedValueOnce({
        ok: true,
      } as Response)

      const available = await client.isAvailable()
      expect(available).toBe(true)
    })

    it("should return false when service is unavailable", async () => {
      fetchSpy.mockRejectedValueOnce(new Error("Connection refused"))

      const available = await client.isAvailable()
      expect(available).toBe(false)
    })
  })

  describe("singleton management", () => {
    it("should return same instance on multiple calls", () => {
      const instance1 = getChannelsClient()
      const instance2 = getChannelsClient()
      expect(instance1).toBe(instance2)
    })

    it("should reset instance", () => {
      const instance1 = getChannelsClient()
      resetChannelsClient()
      const instance2 = getChannelsClient()
      expect(instance1).not.toBe(instance2)
    })
  })
})
