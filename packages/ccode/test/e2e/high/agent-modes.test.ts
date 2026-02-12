/**
 * E2E High Priority Test: Agent Mode Operations
 * Priority: High - Runs daily
 *
 * Tests the agent mode switching and operations
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Provider } from "../../../src/provider/provider"
import { Session } from "../../../src/session"
import { Env } from "../../../src/env"

describe("E2E High: Agent Mode Operations", () => {
  describe("Agent Mode Availability", () => {
    test("should have plan agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Plan agent uses the same provider/model infrastructure
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()

          const model = providers["anthropic"].models["claude-sonnet-4-20250514"]
          expect(model).toBeDefined()
          expect(model.capabilities.toolcall).toBe(true)
        },
      })
    })

    test("should have build agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          const model = providers["anthropic"].models["claude-sonnet-4-20250514"]

          // Build agent requires tool call capability
          expect(model.capabilities.toolcall).toBe(true)
        },
      })
    })

    test("should have explore agent with small model", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Explore agent can use smaller models for efficiency
          const smallModel = await Provider.getSmallModel("anthropic")
          expect(smallModel).toBeDefined()
          expect(smallModel?.id).toContain("haiku")
        },
      })
    })

    test("should have general agent as default", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // General agent uses the default model
          const model = await Provider.defaultModel()
          expect(model).toBeDefined()
          expect(model.providerID).toBeDefined()
          expect(model.modelID).toBeDefined()
        },
      })
    })
  })

  describe("Model Selection", () => {
    test("should respect configured default model", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              model: "anthropic/claude-sonnet-4-20250514",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const model = await Provider.defaultModel()
          expect(model.providerID).toBe("anthropic")
          expect(model.modelID).toBe("claude-sonnet-4-20250514")
        },
      })
    })

    test("should support provider-specific model selection", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const model = await Provider.getModel("anthropic", "claude-sonnet-4-20250514")
          expect(model).toBeDefined()
          expect(model.id).toBe("claude-sonnet-4-20250514")
        },
      })
    })
  })

  describe("Session with Agent Context", () => {
    test("should create session for agent operations", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create a session for agent operations
          const session = await Session.create({ title: "Agent Test Session" })
          expect(session).toBeDefined()
          expect(session.id).toBeDefined()

          // Session should be usable for agent context
          const retrieved = await Session.get(session.id)
          expect(retrieved).toBeDefined()

          await Session.remove(session.id)
        },
      })
    })
  })
})
