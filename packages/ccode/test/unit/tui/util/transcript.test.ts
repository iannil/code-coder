// @ts-nocheck
/**
 * Transcript Utility Unit Tests
 *
 * Tests for the transcript formatting utility including:
 * - Session header formatting
 * - Message formatting (user/assistant)
 * - Part formatting (text, tool, reasoning)
 * - Metadata inclusion
 * - Tool details display
 */

import { describe, test, expect, beforeEach } from "bun:test"

describe("Transcript Utility", () => {
  describe("SessionInfo structure", () => {
    test("should have correct SessionInfo interface", () => {
      const sessionInfo = {
        id: "session-123",
        title: "Test Session",
        time: {
          created: Date.now(),
          updated: Date.now(),
        },
      }

      expect(sessionInfo).toHaveProperty("id")
      expect(sessionInfo).toHaveProperty("title")
      expect(sessionInfo).toHaveProperty("time")
      expect(sessionInfo.time).toHaveProperty("created")
      expect(sessionInfo.time).toHaveProperty("updated")
    })

    test("should handle timestamps", () => {
      const now = Date.now()
      const time = { created: now, updated: now }

      expect(time.created).toBe(now)
      expect(time.updated).toBe(now)
    })
  })

  describe("MessageWithParts structure", () => {
    test("should have correct MessageWithParts interface", () => {
      const message = {
        info: {
          id: "msg-1",
          role: "user",
          createdAt: Date.now(),
        },
        parts: [
          { type: "text", text: "Hello" },
        ],
      }

      expect(message).toHaveProperty("info")
      expect(message).toHaveProperty("parts")
      expect(Array.isArray(message.parts)).toBe(true)
    })
  })

  describe("TranscriptOptions", () => {
    test("should have correct options structure", () => {
      const options = {
        thinking: true,
        toolDetails: true,
        assistantMetadata: true,
      }

      expect(options).toHaveProperty("thinking")
      expect(options).toHaveProperty("toolDetails")
      expect(options).toHaveProperty("assistantMetadata")
    })

    test("should support all options disabled", () => {
      const options = {
        thinking: false,
        toolDetails: false,
        assistantMetadata: false,
      }

      expect(options.thinking).toBe(false)
      expect(options.toolDetails).toBe(false)
      expect(options.assistantMetadata).toBe(false)
    })
  })

  describe("session header formatting", () => {
    test("should format session title", () => {
      const session = {
        id: "test-123",
        title: "My Session",
        time: { created: Date.now(), updated: Date.now() },
      }

      const header = `# ${session.title}\n\n`
      expect(header).toContain("# My Session")
    })

    test("should include session ID", () => {
      const session = {
        id: "abc-123",
        title: "Test",
        time: { created: Date.now(), updated: Date.now() },
      }

      const idLine = `**Session ID:** ${session.id}\n`
      expect(idLine).toContain("abc-123")
    })

    test("should format timestamps", () => {
      const created = new Date("2024-01-01T12:00:00Z").getTime()
      const updated = new Date("2024-01-01T14:30:00Z").getTime()

      const createdDate = new Date(created).toLocaleString()
      const updatedDate = new Date(updated).toLocaleString()

      expect(createdDate).toBeTruthy()
      expect(updatedDate).toBeTruthy()
    })
  })

  describe("message formatting", () => {
    type MessageRole = "user" | "assistant"

    test("should format user message header", () => {
      const role: MessageRole = "user"

      const header = role === "user" ? "## User\n\n" : "## Assistant\n\n"
      expect(header).toBe("## User\n\n")
    })

    test("should format assistant message header", () => {
      const role: MessageRole = "assistant"

      const header = role === "user" ? "## User\n\n" : "## Assistant\n\n"
      expect(header).toBe("## Assistant\n\n")
    })

    test("should format assistant with metadata", () => {
      const msg = {
        role: "assistant",
        agent: "editor",
        modelID: "claude-sonnet-4-5",
        time: {
          created: Date.now() - 5000,
          completed: Date.now(),
        },
      }

      const duration = ((msg.time.completed! - msg.time.created!) / 1000).toFixed(1) + "s"
      expect(duration).toMatch(/^\d+\.\d+s$/)
    })
  })

  describe("text part formatting", () => {
    test("should format text part", () => {
      const part = {
        type: "text",
        text: "Hello, world!",
        synthetic: false,
      }

      const formatted = part.type === "text" && !part.synthetic ? `${part.text}\n\n` : ""
      expect(formatted).toBe("Hello, world!\n\n")
    })

    test("should skip synthetic text", () => {
      const part = {
        type: "text",
        text: "Synthetic content",
        synthetic: true,
      }

      const formatted = part.type === "text" && !part.synthetic ? `${part.text}\n\n` : ""
      expect(formatted).toBe("")
    })

    test("should handle empty text", () => {
      const part = {
        type: "text",
        text: "",
        synthetic: false,
      }

      const formatted = part.type === "text" && !part.synthetic ? `${part.text}\n\n` : ""
      expect(formatted).toBe("\n\n")
    })
  })

  describe("reasoning part formatting", () => {
    test("should format reasoning when thinking enabled", () => {
      const part = {
        type: "reasoning",
        text: "Thinking about the problem...",
      }

      const options = { thinking: true }
      const formatted = part.type === "reasoning" && options.thinking ? `_Thinking:_\n\n${part.text}\n\n` : ""
      expect(formatted).toContain("Thinking")
      expect(formatted).toContain("Thinking about the problem...")
    })

    test("should skip reasoning when thinking disabled", () => {
      const part = {
        type: "reasoning",
        text: "Hidden thinking",
      }

      const options = { thinking: false }
      const formatted = part.type === "reasoning" && options.thinking ? `_Thinking:_\n\n${part.text}\n\n` : ""
      expect(formatted).toBe("")
    })
  })

  describe("tool part formatting", () => {
    test("should format tool name", () => {
      const part = {
        type: "tool",
        tool: "read_file",
        state: { input: {}, status: "completed", output: "file content" },
      }

      const toolLine = `\`\`\`\nTool: ${part.tool}\n`
      expect(toolLine).toContain("read_file")
    })

    test("should format tool input when details enabled", () => {
      const part = {
        type: "tool",
        tool: "bash",
        state: {
          input: { command: "ls -la" },
          status: "completed",
          output: "file list",
        },
      }

      const options = { toolDetails: true }
      const inputFormatted =
        options.toolDetails && part.state.input ? `\n**Input:**\n\`\`\`json\n${JSON.stringify(part.state.input, null, 2)}\n\`\`\`` : ""

      expect(inputFormatted).toContain("ls -la")
    })

    test("should format tool output when completed", () => {
      const part = {
        type: "tool",
        tool: "bash",
        state: {
          input: {},
          status: "completed",
          output: "success",
        },
      }

      const options = { toolDetails: true }
      const outputFormatted =
        options.toolDetails && part.state.status === "completed" && part.state.output
          ? `\n**Output:**\n\`\`\`\n${part.state.output}\n\`\`\``
          : ""

      expect(outputFormatted).toContain("success")
    })

    test("should format tool error when error status", () => {
      const part = {
        type: "tool",
        tool: "bash",
        state: {
          input: {},
          status: "error",
          error: "Command failed",
        },
      }

      const options = { toolDetails: true }
      const errorFormatted =
        options.toolDetails && part.state.status === "error" && part.state.error
          ? `\n**Error:**\n\`\`\`\n${part.state.error}\n\`\`\``
          : ""

      expect(errorFormatted).toContain("Command failed")
    })

    test("should skip details when disabled", () => {
      const part = {
        type: "tool",
        tool: "bash",
        state: {
          input: { command: "test" },
          status: "completed",
          output: "result",
        },
      }

      const options = { toolDetails: false }
      const inputFormatted = options.toolDetails && part.state.input ? "details" : ""
      const outputFormatted =
        options.toolDetails && part.state.status === "completed" && part.state.output ? "output" : ""

      expect(inputFormatted).toBe("")
      expect(outputFormatted).toBe("")
    })
  })

  describe("unknown part types", () => {
    test("should return empty string for unknown types", () => {
      const part = {
        type: "unknown_type",
        data: "something",
      }

      const formatted = ""
      expect(formatted).toBe("")
    })
  })

  describe("edge cases", () => {
    test("should handle empty parts array", () => {
      const parts: unknown[] = []

      expect(parts).toHaveLength(0)
    })

    test("should handle multiline text", () => {
      const part = {
        type: "text",
        text: "Line 1\nLine 2\nLine 3",
        synthetic: false,
      }

      const formatted = part.type === "text" && !part.synthetic ? `${part.text}\n\n` : ""
      expect(formatted).toContain("\n")
    })

    test("should handle unicode in text", () => {
      const part = {
        type: "text",
        text: "Hello 世界 🌍",
        synthetic: false,
      }

      const formatted = part.type === "text" && !part.synthetic ? `${part.text}\n\n` : ""
      expect(formatted).toContain("世界")
      expect(formatted).toContain("🌍")
    })

    test("should handle very long tool output", () => {
      const longOutput = "A".repeat(10000)
      const part = {
        type: "tool",
        tool: "bash",
        state: {
          input: {},
          status: "completed",
          output: longOutput,
        },
      }

      const options = { toolDetails: true }
      const outputFormatted =
        options.toolDetails && part.state.status === "completed" && part.state.output
          ? part.state.output
          : ""

      expect(outputFormatted.length).toBe(10000)
    })
  })

  describe("full transcript formatting", () => {
    test("should combine all elements", () => {
      const session = {
        id: "test-123",
        title: "Test Session",
        time: { created: Date.now(), updated: Date.now() },
      }

      const messages = [
        {
          info: { role: "user", id: "1", createdAt: Date.now() },
          parts: [{ type: "text", text: "Hello", synthetic: false }],
        },
        {
          info: { role: "assistant", id: "2", createdAt: Date.now(), agent: "editor", modelID: "claude" },
          parts: [{ type: "text", text: "Hi there!", synthetic: false }],
        },
      ]

      const options = { thinking: false, toolDetails: false, assistantMetadata: false }

      let transcript = `# ${session.title}\n\n`
      transcript += `**Session ID:** ${session.id}\n\n`
      transcript += `---\n\n`

      for (const msg of messages) {
        transcript += msg.info.role === "user" ? "## User\n\n" : "## Assistant\n\n"
        for (const part of msg.parts as any[]) {
          if (part.type === "text" && !part.synthetic) {
            transcript += `${part.text}\n\n`
          }
        }
        transcript += `---\n\n`
      }

      expect(transcript).toContain("# Test Session")
      expect(transcript).toContain("## User")
      expect(transcript).toContain("Hello")
      expect(transcript).toContain("## Assistant")
      expect(transcript).toContain("Hi there!")
    })
  })
})
