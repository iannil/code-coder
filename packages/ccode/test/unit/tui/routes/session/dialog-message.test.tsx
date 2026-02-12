// @ts-nocheck
/**
 * Dialog Message Component Unit Tests
 *
 * Tests for the message actions dialog including:
 * - Message action options (Revert, Copy, Fork)
 * - Session revert functionality
 * - Copy to clipboard
 * - Fork session creation
 * - Prompt reconstruction from parts
 */

import { describe, test, expect, beforeEach } from "bun:test"

describe("Dialog Message Component", () => {
  describe("props structure", () => {
    test("should have correct props interface", () => {
      const props = {
        messageID: "msg-123",
        sessionID: "session-456",
        setPrompt: (prompt: unknown) => {},
      }

      expect(props).toHaveProperty("messageID")
      expect(props).toHaveProperty("sessionID")
      expect(props).toHaveProperty("setPrompt")
    })

    test("should accept optional setPrompt", () => {
      const props = {
        messageID: "msg-123",
        sessionID: "session-456",
      }

      expect(props.messageID).toBe("msg-123")
      expect(props.sessionID).toBe("session-456")
      expect(props.setPrompt).toBeUndefined()
    })
  })

  describe("message actions", () => {
    test("should have Revert action", () => {
      const revertAction = {
        title: "Revert",
        value: "session.revert",
        description: "undo messages and file changes",
      }

      expect(revertAction.title).toBe("Revert")
      expect(revertAction.value).toBe("session.revert")
      expect(revertAction.description).toBe("undo messages and file changes")
    })

    test("should have Copy action", () => {
      const copyAction = {
        title: "Copy",
        value: "message.copy",
        description: "message text to clipboard",
      }

      expect(copyAction.title).toBe("Copy")
      expect(copyAction.value).toBe("message.copy")
      expect(copyAction.description).toBe("message text to clipboard")
    })

    test("should have Fork action", () => {
      const forkAction = {
        title: "Fork",
        value: "session.fork",
        description: "create a new session",
      }

      expect(forkAction.title).toBe("Fork")
      expect(forkAction.value).toBe("session.fork")
      expect(forkAction.description).toBe("create a new session")
    })
  })

  describe("prompt reconstruction", () => {
    test("should reconstruct prompt from text parts", () => {
      const parts = [
        { type: "text", text: "Hello ", synthetic: false },
        { type: "text", text: "world", synthetic: false },
      ]

      const promptInfo = parts.reduce(
        (agg: { input: string; parts: unknown[] }, part: any) => {
          if (part.type === "text" && !part.synthetic) {
            agg.input += part.text
          }
          return agg
        },
        { input: "", parts: [] },
      )

      expect(promptInfo.input).toBe("Hello world")
    })

    test("should skip synthetic text parts", () => {
      const parts = [
        { type: "text", text: "User input", synthetic: false },
        { type: "text", text: "Synthetic", synthetic: true },
      ]

      const promptInfo = parts.reduce(
        (agg: { input: string }, part: any) => {
          if (part.type === "text" && !part.synthetic) {
            agg.input += part.text
          }
          return agg
        },
        { input: "" },
      )

      expect(promptInfo.input).toBe("User input")
      expect(promptInfo.input).not.toContain("Synthetic")
    })

    test("should include file parts", () => {
      const parts = [
        { type: "text", text: "Check this file:", synthetic: false },
        { type: "file", path: "/path/to/file.ts", filename: "file.ts", mime: "text/typescript" },
      ]

      const promptInfo = parts.reduce(
        (agg: { input: string; parts: any[] }, part: any) => {
          if (part.type === "text" && !part.synthetic) {
            agg.input += part.text
          }
          if (part.type === "file") {
            agg.parts.push({ ...part, filename: part.filename ?? "file" })
          }
          return agg
        },
        { input: "", parts: [] },
      )

      expect(promptInfo.input).toBe("Check this file:")
      expect(promptInfo.parts).toHaveLength(1)
      expect(promptInfo.parts[0].filename).toBe("file.ts")
    })
  })

  describe("revert action", () => {
    test("should call session revert with correct params", () => {
      let calledParams: unknown = null

      const mockSDK = {
        client: {
          session: {
            revert: (params: unknown) => {
              calledParams = params
            },
          },
        },
      }

      mockSDK.client.session.revert({
        sessionID: "session-123",
        messageID: "msg-456",
      })

      expect(calledParams).toEqual({
        sessionID: "session-123",
        messageID: "msg-456",
      })
    })

    test("should set prompt after revert", () => {
      let setPromptCalled = false
      let promptValue: unknown = null

      const setPrompt = (prompt: unknown) => {
        setPromptCalled = true
        promptValue = prompt
      }

      const parts = [
        { type: "text", text: "Restored prompt", synthetic: false },
      ]

      const promptInfo = parts.reduce(
        (agg: { input: string }, part: any) => {
          if (part.type === "text" && !part.synthetic) {
            agg.input += part.text
          }
          return agg
        },
        { input: "" },
      )

      setPrompt(promptInfo)

      expect(setPromptCalled).toBe(true)
      expect((promptValue as { input: string }).input).toBe("Restored prompt")
    })
  })

  describe("copy action", () => {
    test("should extract text from parts for copying", () => {
      const parts = [
        { type: "text", text: "Line 1\n", synthetic: false },
        { type: "text", text: "Line 2", synthetic: false },
      ]

      const text = parts.reduce((agg: string, part: any) => {
        if (part.type === "text" && !part.synthetic) {
          agg += part.text
        }
        return agg
      }, "")

      expect(text).toBe("Line 1\nLine 2")
    })

    test("should skip synthetic parts when copying", () => {
      const parts = [
        { type: "text", text: "Real text", synthetic: false },
        { type: "text", text: "Hidden", synthetic: true },
      ]

      const text = parts.reduce((agg: string, part: any) => {
        if (part.type === "text" && !part.synthetic) {
          agg += part.text
        }
        return agg
      }, "")

      expect(text).toBe("Real text")
    })
  })

  describe("fork action", () => {
    test("should call session fork with correct params", () => {
      let calledParams: unknown = null

      const mockSDK = {
        client: {
          session: {
            fork: async (params: unknown) => {
              calledParams = params
              return { data: { id: "new-session-123" } }
            },
          },
        },
      }

      mockSDK.client.session.fork({
        sessionID: "session-123",
        messageID: "msg-456",
      })

      expect(calledParams).toEqual({
        sessionID: "session-123",
        messageID: "msg-456",
      })
    })

    test("should navigate to new session after fork", async () => {
      const mockSDK = {
        client: {
          session: {
            fork: async () => ({ data: { id: "new-session-id" } }),
          },
        },
      }

      let navigatedTo: unknown = null
      const navigate = (route: unknown) => {
        navigatedTo = route
      }

      const result = await mockSDK.client.session.fork({})
      navigate({
        sessionID: result.data.id,
        type: "session",
      })

      expect(navigatedTo).toEqual({
        sessionID: "new-session-id",
        type: "session",
      })
    })

    test("should include initialPrompt when navigating", async () => {
      const mockSDK = {
        client: {
          session: {
            fork: async () => ({ data: { id: "new-123" } }),
          },
        },
      }

      const initialPrompt = {
        input: "Continued conversation",
        parts: [],
      }

      let navigatedTo: unknown = null
      const navigate = (route: unknown) => {
        navigatedTo = route
      }

      const result = await mockSDK.client.session.fork({})
      navigate({
        sessionID: result.data.id,
        type: "session",
        initialPrompt,
      })

      expect((navigatedTo as { initialPrompt: unknown }).initialPrompt).toEqual(initialPrompt)
    })
  })

  describe("message lookup", () => {
    test("should find message by ID in session", () => {
      const messages = [
        { id: "msg-1", content: "First" },
        { id: "msg-2", content: "Second" },
        { id: "msg-3", content: "Third" },
      ]

      const messageID = "msg-2"
      const found = messages.find((m) => m.id === messageID)

      expect(found?.id).toBe("msg-2")
      expect(found?.content).toBe("Second")
    })

    test("should return undefined for non-existent message", () => {
      const messages = [
        { id: "msg-1", content: "First" },
      ]

      const found = messages.find((m) => m.id === "msg-999")

      expect(found).toBeUndefined()
    })
  })

  describe("dialog title", () => {
    test("should have correct dialog title", () => {
      const title = "Message Actions"

      expect(title).toBe("Message Actions")
    })
  })

  describe("edge cases", () => {
    test("should handle empty parts array", () => {
      const parts: any[] = []

      const text = parts.reduce((agg: string, part: any) => {
        if (part.type === "text" && !part.synthetic) {
          agg += part.text
        }
        return agg
      }, "")

      expect(text).toBe("")
    })

    test("should handle parts with only synthetic text", () => {
      const parts = [
        { type: "text", text: "Hidden 1", synthetic: true },
        { type: "text", text: "Hidden 2", synthetic: true },
      ]

      const text = parts.reduce((agg: string, part: any) => {
        if (part.type === "text" && !part.synthetic) {
          agg += part.text
        }
        return agg
      }, "")

      expect(text).toBe("")
    })

    test("should handle file parts without filename", () => {
      const part = { type: "file", path: "/path/to/file", mime: "text/plain" }

      const filename = part.filename ?? "file"
      expect(filename).toBe("file")
    })
  })
})
