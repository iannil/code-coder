/**
 * E2E Medium Priority Test: Model Configuration
 * Priority: Medium - Runs weekly
 *
 * Tests advanced model configuration options
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Provider } from "../../../src/provider/provider"
import { ModelsDev } from "../../../src/provider/models"
import { Env } from "../../../src/env"

describe("E2E Medium: Model Configuration", () => {
  describe("Model Refresh", () => {
    test("should have refresh capability", async () => {
      expect(ModelsDev.refresh).toBeDefined()
      expect(typeof ModelsDev.refresh).toBe("function")
    })
  })

  describe("Model Filtering", () => {
    test("should whitelist specific models", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              provider: {
                anthropic: {
                  whitelist: ["claude-sonnet-4-20250514"],
                },
              },
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
          const providers = await Provider.list()
          const models = Object.keys(providers["anthropic"].models)
          expect(models).toContain("claude-sonnet-4-20250514")
          expect(models.length).toBe(1)
        },
      })
    })

    test("should blacklist specific models", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              provider: {
                anthropic: {
                  blacklist: ["claude-haiku-4-20250514"],
                },
              },
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
          const providers = await Provider.list()
          const models = Object.keys(providers["anthropic"].models)
          expect(models).not.toContain("claude-haiku-4-20250514")
        },
      })
    })
  })

  describe("Custom Models", () => {
    test("should add custom model alias", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              provider: {
                anthropic: {
                  models: {
                    fast: {
                      id: "claude-haiku-4-20250514",
                      name: "Fast Model",
                    },
                  },
                },
              },
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
          const providers = await Provider.list()
          const model = providers["anthropic"].models["fast"]
          expect(model).toBeDefined()
          expect(model.name).toBe("Fast Model")
        },
      })
    })

    test("should override model properties", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              provider: {
                anthropic: {
                  models: {
                    "claude-sonnet-4-20250514": {
                      name: "Custom Sonnet Name",
                      cost: { input: 1, output: 2 },
                    },
                  },
                },
              },
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
          const providers = await Provider.list()
          const model = providers["anthropic"].models["claude-sonnet-4-20250514"]
          expect(model.name).toBe("Custom Sonnet Name")
          expect(model.cost.input).toBe(1)
          expect(model.cost.output).toBe(2)
        },
      })
    })
  })

  describe("Custom Providers", () => {
    test("should add custom OpenAI-compatible provider", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              provider: {
                "custom-openai": {
                  name: "Custom OpenAI",
                  npm: "@ai-sdk/openai-compatible",
                  env: [],
                  api: "https://custom.openai.com/v1",
                  models: {
                    "custom-gpt4": {
                      name: "Custom GPT-4",
                      tool_call: true,
                      limit: { context: 128000, output: 4096 },
                    },
                  },
                  options: {
                    apiKey: "custom-key",
                  },
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["custom-openai"]).toBeDefined()
          expect(providers["custom-openai"].name).toBe("Custom OpenAI")
          expect(providers["custom-openai"].models["custom-gpt4"]).toBeDefined()
        },
      })
    })

    test("should configure provider timeout", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              provider: {
                anthropic: {
                  options: {
                    timeout: 120000,
                  },
                },
              },
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
          const providers = await Provider.list()
          expect(providers["anthropic"].options.timeout).toBe(120000)
        },
      })
    })
  })

  describe("Provider Filtering", () => {
    test("should disable specific providers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              disabled_providers: ["openai"],
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
          Env.set("OPENAI_API_KEY", "sk-openai-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          expect(providers["openai"]).toBeUndefined()
        },
      })
    })

    test("should enable only specific providers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              enabled_providers: ["anthropic"],
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
          Env.set("OPENAI_API_KEY", "sk-openai-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          expect(providers["openai"]).toBeUndefined()
        },
      })
    })
  })
})
