/**
 * ULC-ANL-* Tests: Decision Analyst Lifecycle
 *
 * Tests for decision analysts using CodeCoder for macro analysis,
 * trading decisions, product selection, and AI engineering consultation.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Provider } from "../../../src/provider/provider"
import { Env } from "../../../src/env"
import { Session } from "../../../src/session"
import { Config } from "../../../src/config/config"
import { Agent } from "../../../src/agent/agent"
import { ReadTool } from "../../../src/tool/read"

const SKIP_E2E = process.env.SKIP_E2E !== "false"

const toolCtx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "decision",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe.skipIf(SKIP_E2E)("ULC-ANL: Decision Analyst Lifecycle", () => {
  describe("ULC-ANL-INIT: Initialization Phase", () => {
    test("ULC-ANL-INIT-001: should initialize for analysis workflow", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const config = await Config.get()
          expect(config).toBeDefined()
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("ULC-ANL-INIT-002: should have decision agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const decisionAgent = await Agent.get("decision")

          expect(decisionAgent).toBeDefined()
          expect(decisionAgent.mode).toBe("subagent")
          expect(decisionAgent.description).toContain("CLOSE")
        },
      })
    })
  })

  describe("ULC-ANL-ANLZ: Analysis Workflow", () => {
    test("ULC-ANL-ANLZ-001: should have observer agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const observerAgent = await Agent.get("observer")

          expect(observerAgent).toBeDefined()
          expect(observerAgent.name).toBe("observer")
          expect(observerAgent.mode).toBe("subagent")
          expect(observerAgent.description).toContain("祝融说")
          expect(observerAgent.description).toContain("观察者")
        },
      })
    })

    test("ULC-ANL-ANLZ-002: should have decision agent with CLOSE framework", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const decisionAgent = await Agent.get("decision")

          expect(decisionAgent).toBeDefined()
          expect(decisionAgent.name).toBe("decision")
          expect(decisionAgent.mode).toBe("subagent")
          expect(decisionAgent.description).toContain("CLOSE")
          expect(decisionAgent.description).toContain("五维评估")
          expect(decisionAgent.temperature).toBe(0.6)
        },
      })
    })

    test("ULC-ANL-ANLZ-003: should have macro agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const macroAgent = await Agent.get("macro")

          expect(macroAgent).toBeDefined()
          expect(macroAgent.name).toBe("macro")
          expect(macroAgent.mode).toBe("subagent")
          expect(macroAgent.description).toContain("宏观经济")
          expect(macroAgent.description).toContain("18章")
        },
      })
    })

    test("ULC-ANL-ANLZ-004: should have trader agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const traderAgent = await Agent.get("trader")

          expect(traderAgent).toBeDefined()
          expect(traderAgent.name).toBe("trader")
          expect(traderAgent.mode).toBe("subagent")
          expect(traderAgent.description).toContain("超短线")
          expect(traderAgent.description).toContain("不构成投资建议")
        },
      })
    })

    test("ULC-ANL-ANLZ-005: should have picker agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const pickerAgent = await Agent.get("picker")

          expect(pickerAgent).toBeDefined()
          expect(pickerAgent.name).toBe("picker")
          expect(pickerAgent.mode).toBe("subagent")
          expect(pickerAgent.description).toContain("选品")
          expect(pickerAgent.description).toContain("七宗罪")
        },
      })
    })

    test("ULC-ANL-ANLZ-006: should have miniproduct agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const miniproductAgent = await Agent.get("miniproduct")

          expect(miniproductAgent).toBeDefined()
          expect(miniproductAgent.name).toBe("miniproduct")
          expect(miniproductAgent.mode).toBe("subagent")
          expect(miniproductAgent.description).toContain("极小产品")
          expect(miniproductAgent.description).toContain("0到1")
        },
      })
    })

    test("ULC-ANL-ANLZ-007: should have ai-engineer agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const aiEngineerAgent = await Agent.get("ai-engineer")

          expect(aiEngineerAgent).toBeDefined()
          expect(aiEngineerAgent.name).toBe("ai-engineer")
          expect(aiEngineerAgent.mode).toBe("subagent")
          expect(aiEngineerAgent.description).toContain("AI工程师")
          expect(aiEngineerAgent.description).toContain("RAG")
        },
      })
    })

    test("ULC-ANL-ANLZ-008: should list all ZRS (Zhurong-Say) agents", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const agents = await Agent.list()
          const zrsAgents = ["observer", "decision", "macro", "trader", "picker", "miniproduct", "ai-engineer"]

          const agentNames = agents.map((a) => a.name)
          for (const agentName of zrsAgents) {
            expect(agentNames).toContain(agentName)
          }
        },
      })
    })
  })

  describe("ULC-ANL-SESS: Session Management for Analysts", () => {
    test("ULC-ANL-SESS-001: should create analysis session", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const session = await Session.create({
            title: "Q1 Macro Analysis",
          })

          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("Q1 Macro Analysis")

          await Session.remove(session.id)
        },
      })
    })

    test("ULC-ANL-SESS-002: should manage multiple analysis projects", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const macroSession = await Session.create({ title: "Macro Analysis" })
          const tradeSession = await Session.create({ title: "Trading Strategy" })
          const productSession = await Session.create({ title: "Product Selection" })

          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }

          expect(sessions.length).toBeGreaterThanOrEqual(3)

          await Session.remove(macroSession.id)
          await Session.remove(tradeSession.id)
          await Session.remove(productSession.id)
        },
      })
    })
  })

  describe("ULC-ANL-MEMO: Memory System", () => {
    test("ULC-ANL-MEMO-001: should read MEMORY.md when present", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "memory", "MEMORY.md"),
            `# Long-term Memory

## User Preferences
- Risk tolerance: Conservative
- Investment horizon: Long-term

## Key Decisions
- 2026-01: Reduced equity exposure
- 2026-02: Increased cash reserves

## Lessons Learned
- Always verify data sources
- Document decision rationale`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "memory", "MEMORY.md") }, toolCtx)

          expect(result.output).toContain("Long-term Memory")
          expect(result.output).toContain("User Preferences")
          expect(result.output).toContain("Key Decisions")
          expect(result.output).toContain("Lessons Learned")
        },
      })
    })

    test("ULC-ANL-MEMO-002: should read daily notes when present", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "memory", "daily", "2026-02-16.md"),
            `# 2026-02-16 Daily Notes

## Morning Review
- PMI data released: 52.3
- Market sentiment: Cautiously optimistic

## Analysis Tasks
- Review macro indicators
- Update trading signals

## Evening Summary
- Completed macro analysis
- Updated decision framework`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "memory", "daily", "2026-02-16.md") }, toolCtx)

          expect(result.output).toContain("2026-02-16")
          expect(result.output).toContain("Morning Review")
          expect(result.output).toContain("PMI data")
          expect(result.output).toContain("Evening Summary")
        },
      })
    })
  })

  describe("ULC-ANL-ADVN: Advanced Features for Analysts", () => {
    test("ULC-ANL-ADVN-001: observer agent should have moderate temperature", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const observer = await Agent.get("observer")

          // Observer should have moderate temperature for balanced analysis
          expect(observer.temperature).toBe(0.7)
        },
      })
    })

    test("ULC-ANL-ADVN-002: macro and trader agents should have lower temperature", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const macro = await Agent.get("macro")
          const trader = await Agent.get("trader")

          // Financial analysis agents should have lower temperature for consistency
          expect(macro.temperature).toBe(0.5)
          expect(trader.temperature).toBe(0.5)
        },
      })
    })

    test("ULC-ANL-ADVN-003: all analysis agents should be subagents", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const analysisAgents = ["observer", "decision", "macro", "trader", "picker", "miniproduct", "ai-engineer"]

          for (const agentName of analysisAgents) {
            const agent = await Agent.get(agentName)
            expect(agent.mode).toBe("subagent")
          }
        },
      })
    })

    test("ULC-ANL-ADVN-004: synton-assistant should be available for memory operations", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const syntonAgent = await Agent.get("synton-assistant")

          expect(syntonAgent).toBeDefined()
          expect(syntonAgent.name).toBe("synton-assistant")
          expect(syntonAgent.description).toContain("SYNTON-DB")
          expect(syntonAgent.description).toContain("记忆数据库")
        },
      })
    })
  })

  describe("ULC-ANL-ERR: Error Handling for Analysts", () => {
    test("ULC-ANL-ERR-001: should handle missing memory file gracefully", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          const read = await ReadTool.init()

          await expect(read.execute({ filePath: path.join(tmp.path, "memory", "MEMORY.md") }, toolCtx)).rejects.toThrow(
            /not found|ENOENT|no such file/i,
          )
        },
      })
    })

    test("ULC-ANL-ERR-002: should handle non-existent session gracefully", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-analyst-key")
        },
        fn: async () => {
          await expect(Session.get("ses_nonexistent_analyst_123")).rejects.toThrow("NotFoundError")
        },
      })
    })
  })
})
