/**
 * ULC-DEV-* Tests: Software Developer Lifecycle
 *
 * Tests for software developers using CodeCoder for code development,
 * quality review, and architecture design workflows.
 */

import { describe, test, expect, beforeEach } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Provider } from "../../../src/provider/provider"
import { Env } from "../../../src/env"
import { Session } from "../../../src/session"
import { Config } from "../../../src/config/config"
import { Agent } from "../../../src/agent/agent"
import { ReadTool } from "../../../src/tool/read"
import { GlobTool } from "../../../src/tool/glob"
import { GrepTool } from "../../../src/tool/grep"

const SKIP_E2E = process.env.SKIP_E2E !== "false"

const toolCtx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

// Check if ripgrep is available (required for glob/grep tools)
const rgAvailable = await (async () => {
  const proc = Bun.spawn(["rg", "--version"], {
    stdout: "pipe",
    stderr: "pipe",
  })
  proc.unref()
  return proc.exitCode === 0 || proc.exitCode === null
})().catch(() => false)

describe.skipIf(SKIP_E2E)("ULC-DEV: Software Developer Lifecycle", () => {
  describe("ULC-DEV-INIT: Initialization Phase", () => {
    test("ULC-DEV-INIT-001: should load configuration without errors", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          expect(config).toBeDefined()
          expect(Instance.project).toBeDefined()
          expect(Instance.project.id).toBeDefined()
        },
      })
    })

    test("ULC-DEV-INIT-002: should configure API key from environment", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          expect(["env", "custom", "config"]).toContain(providers["anthropic"].source)
        },
      })
    })

    test("ULC-DEV-INIT-003: should list available models", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          const models = Object.keys(providers["anthropic"].models)
          expect(models.length).toBeGreaterThan(0)
          expect(models).toContain("claude-sonnet-4-20250514")
        },
      })
    })
  })

  describe("ULC-DEV-SESS: Session Management", () => {
    test("ULC-DEV-SESS-001: should create new session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const session = await Session.create({
            title: "Developer Test Session",
          })

          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("Developer Test Session")
          expect(session.projectID).toBeDefined()

          await Session.remove(session.id)
        },
      })
    })

    test("ULC-DEV-SESS-002: should list sessions", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const session1 = await Session.create({ title: "Session 1" })
          const session2 = await Session.create({ title: "Session 2" })

          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }

          expect(sessions.length).toBeGreaterThanOrEqual(2)
          expect(sessions.some((s) => s.id === session1.id)).toBe(true)
          expect(sessions.some((s) => s.id === session2.id)).toBe(true)

          await Session.remove(session1.id)
          await Session.remove(session2.id)
        },
      })
    })

    test("ULC-DEV-SESS-003: should switch sessions", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const session1 = await Session.create({ title: "Switch Test 1" })
          const session2 = await Session.create({ title: "Switch Test 2" })

          // Get each session individually
          const retrieved1 = await Session.get(session1.id)
          const retrieved2 = await Session.get(session2.id)

          expect(retrieved1?.title).toBe("Switch Test 1")
          expect(retrieved2?.title).toBe("Switch Test 2")

          await Session.remove(session1.id)
          await Session.remove(session2.id)
        },
      })
    })

    test("ULC-DEV-SESS-004: should delete session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Delete Test" })
          const sessionId = session.id

          // Verify session exists
          const retrieved = await Session.get(sessionId)
          expect(retrieved).toBeDefined()

          // Delete session
          await Session.remove(sessionId)

          // Verify session is deleted
          await expect(Session.get(sessionId)).rejects.toThrow("NotFoundError")
        },
      })
    })
  })

  describe("ULC-DEV-AGNT: Agent Workflow", () => {
    test("ULC-DEV-AGNT-001: should have build agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const buildAgent = await Agent.get("build")

          expect(buildAgent).toBeDefined()
          expect(buildAgent.name).toBe("build")
          expect(buildAgent.mode).toBe("primary")
          expect(buildAgent.native).toBe(true)
        },
      })
    })

    test("ULC-DEV-AGNT-002: should have plan agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const planAgent = await Agent.get("plan")

          expect(planAgent).toBeDefined()
          expect(planAgent.name).toBe("plan")
          expect(planAgent.mode).toBe("primary")
        },
      })
    })

    test("ULC-DEV-AGNT-003: should have code-reviewer agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const codeReviewerAgent = await Agent.get("code-reviewer")

          expect(codeReviewerAgent).toBeDefined()
          expect(codeReviewerAgent.name).toBe("code-reviewer")
          expect(codeReviewerAgent.mode).toBe("subagent")
          expect(codeReviewerAgent.description).toContain("code quality")
        },
      })
    })

    test("ULC-DEV-AGNT-004: should have security-reviewer agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const securityReviewerAgent = await Agent.get("security-reviewer")

          expect(securityReviewerAgent).toBeDefined()
          expect(securityReviewerAgent.name).toBe("security-reviewer")
          expect(securityReviewerAgent.mode).toBe("subagent")
          expect(securityReviewerAgent.description).toContain("security")
        },
      })
    })

    test("ULC-DEV-AGNT-005: should have tdd-guide agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const tddGuideAgent = await Agent.get("tdd-guide")

          expect(tddGuideAgent).toBeDefined()
          expect(tddGuideAgent.name).toBe("tdd-guide")
          expect(tddGuideAgent.mode).toBe("subagent")
          expect(tddGuideAgent.description).toContain("test-driven")
        },
      })
    })

    test("ULC-DEV-AGNT-006: should have architect agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const architectAgent = await Agent.get("architect")

          expect(architectAgent).toBeDefined()
          expect(architectAgent.name).toBe("architect")
          expect(architectAgent.mode).toBe("subagent")
          expect(architectAgent.description).toContain("architecture")
        },
      })
    })

    test("ULC-DEV-AGNT-007: should have explore agent available", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const exploreAgent = await Agent.get("explore")

          expect(exploreAgent).toBeDefined()
          expect(exploreAgent.name).toBe("explore")
          expect(exploreAgent.mode).toBe("subagent")
          expect(exploreAgent.description).toContain("exploring")
        },
      })
    })

    test("ULC-DEV-AGNT-008: should list all developer agents", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const agents = await Agent.list()
          const developerAgents = ["build", "plan", "code-reviewer", "security-reviewer", "tdd-guide", "architect", "explore"]

          const agentNames = agents.map((a) => a.name)
          for (const agentName of developerAgents) {
            expect(agentNames).toContain(agentName)
          }
        },
      })
    })
  })

  describe("ULC-DEV-FILE: File Operations", () => {
    test("ULC-DEV-FILE-001: should read file content", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "test-file.ts"),
            `export function add(a: number, b: number): number {
  return a + b
}`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "test-file.ts") }, toolCtx)

          expect(result.output).toContain("function add")
          expect(result.output).toContain("return a + b")
        },
      })
    })

    test.skipIf(!rgAvailable)("ULC-DEV-FILE-002: should find files by pattern", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(path.join(dir, "src", "index.ts"), "export {}")
          await Bun.write(path.join(dir, "src", "utils.ts"), "export {}")
          await Bun.write(path.join(dir, "src", "helper.js"), "module.exports = {}")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const glob = await GlobTool.init()
          const result = await glob.execute({ pattern: "**/*.ts" }, toolCtx)

          expect(result.output).toContain("index.ts")
          expect(result.output).toContain("utils.ts")
          expect(result.output).not.toContain("helper.js")
        },
      })
    })

    test.skipIf(!rgAvailable)("ULC-DEV-FILE-003: should search code content", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "src", "api.ts"),
            `export async function fetchData() {
  const response = await fetch('/api/data')
  return response.json()
}`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const grep = await GrepTool.init()
          const result = await grep.execute({ pattern: "fetchData", path: tmp.path }, toolCtx)

          expect(result.output).toContain("api.ts")
        },
      })
    })

    test("ULC-DEV-FILE-004: should handle non-existent file gracefully", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const read = await ReadTool.init()

          await expect(read.execute({ filePath: path.join(tmp.path, "nonexistent.ts") }, toolCtx)).rejects.toThrow(
            "not found",
          )
        },
      })
    })
  })

  describe("ULC-DEV-ADVN: Advanced Features", () => {
    test("ULC-DEV-ADVN-001: should support default agent configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              default_agent: "build",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const defaultAgent = await Agent.defaultAgent()
          expect(defaultAgent).toBe("build")
        },
      })
    })

    test("ULC-DEV-ADVN-002: should support custom agent configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              agent: {
                "custom-dev": {
                  description: "Custom developer agent",
                  mode: "subagent",
                  prompt: "You are a custom developer assistant.",
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          const customAgent = await Agent.get("custom-dev")

          expect(customAgent).toBeDefined()
          expect(customAgent.name).toBe("custom-dev")
          expect(customAgent.description).toBe("Custom developer agent")
          expect(customAgent.mode).toBe("subagent")
        },
      })
    })

    test("ULC-DEV-ADVN-003: should support multiple concurrent sessions", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          // Create multiple sessions concurrently
          const [session1, session2, session3] = await Promise.all([
            Session.create({ title: "Concurrent Dev 1" }),
            Session.create({ title: "Concurrent Dev 2" }),
            Session.create({ title: "Concurrent Dev 3" }),
          ])

          expect(session1.id).toBeDefined()
          expect(session2.id).toBeDefined()
          expect(session3.id).toBeDefined()

          // All sessions should be unique
          expect(new Set([session1.id, session2.id, session3.id]).size).toBe(3)

          // Cleanup
          await Promise.all([Session.remove(session1.id), Session.remove(session2.id), Session.remove(session3.id)])
        },
      })
    })
  })

  describe("ULC-DEV-ERR: Error Handling", () => {
    test("ULC-DEV-ERR-001: should handle missing config gracefully", async () => {
      await using tmp = await tmpdir({})

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Should work without config file
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("ULC-DEV-ERR-002: should handle invalid config gracefully", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Write invalid JSON
          await Bun.write(path.join(dir, "codecoder.json"), "{ invalid json }")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Should handle invalid config gracefully
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("ULC-DEV-ERR-003: should handle non-existent session gracefully", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-developer-key")
        },
        fn: async () => {
          await expect(Session.get("ses_nonexistent_dev_123")).rejects.toThrow("NotFoundError")
        },
      })
    })
  })
})
