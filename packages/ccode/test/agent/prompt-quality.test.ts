import { test, expect, describe } from "bun:test"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Agent } from "../../src/agent/agent"

// Expected keywords mapping for each agent's prompt
const EXPECTED_KEYWORDS: Record<string, string[]> = {
  // ZRS Agents - Chinese keywords
  observer: ["可能性", "观察", "收敛"],
  decision: ["决策", "选择权", "余量"],
  macro: ["GDP", "货币", "经济"],
  trader: ["涨停", "情绪", "风险"],
  picker: ["选品", "需求", "痛点"],
  miniproduct: ["MVP", "独立开发", "变现"],
  "synton-assistant": ["SYNTON", "PaQL", "Graph-RAG"],
  "ai-engineer": ["Python", "LLM", "RAG"],

  // Professional Agents - English keywords
  "code-reviewer": ["code", "review", "quality"],
  "security-reviewer": ["security", "vulnerability", "injection"],
  "tdd-guide": ["test", "TDD", "coverage"],
  architect: ["architecture", "design", "pattern"],
  writer: ["write", "chapter", "outline"],
  proofreader: ["Punctuation", "spelling", "PROOF"],
  "code-reverse": ["reverse", "engineering", "Technology"],
  "jar-code-reverse": ["JAR", "Java", "bytecode"],

  // Core Agents - English keywords
  explore: ["search", "file", "codebase"],
  compaction: ["summarize", "context", "conversation"],
  title: ["title", "generator", "conversation"],
  summary: ["Summarize", "changes", "conversation"],
}

describe("Prompt Quality Tests", () => {
  for (const [agentName, keywords] of Object.entries(EXPECTED_KEYWORDS)) {
    test(`${agentName} prompt contains expected keywords`, async () => {
      await using tmp = await tmpdir()
      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const agent = await Agent.get(agentName)
          expect(agent).toBeDefined()

          const prompt = agent?.prompt ?? ""

          // Prompt should have meaningful length
          expect(prompt.length).toBeGreaterThan(100)

          // Check each expected keyword
          for (const keyword of keywords) {
            const found = prompt.toLowerCase().includes(keyword.toLowerCase())
            if (!found) {
              console.log(`Agent ${agentName} missing keyword: ${keyword}`)
            }
            expect(found).toBe(true)
          }
        },
      })
    })
  }
})

describe("Prompt Structure Tests", () => {
  test("all visible agents have non-empty prompts", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agents = await Agent.list()
        for (const agent of agents) {
          // Skip hidden agents that might not need prompts
          if (agent.hidden) continue
          // build and plan don't have custom prompts (they use system defaults)
          if (agent.name === "build" || agent.name === "plan" || agent.name === "general") continue

          expect(agent.prompt).toBeDefined()
          expect(agent.prompt?.length).toBeGreaterThan(0)
        }
      },
    })
  })

  test("ZRS agent prompts contain markdown headers", async () => {
    const zrsAgents = [
      "observer",
      "decision",
      "macro",
      "trader",
      "picker",
      "miniproduct",
      "synton-assistant",
      "ai-engineer",
    ]

    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of zrsAgents) {
          const agent = await Agent.get(name)
          const prompt = agent?.prompt ?? ""
          // Check for markdown headers
          expect(prompt).toMatch(/^#/m)
        }
      },
    })
  })

  test("professional agent prompts contain markdown headers", async () => {
    const professionalAgents = [
      "code-reviewer",
      "security-reviewer",
      "tdd-guide",
      "architect",
      "writer",
      "proofreader",
      "code-reverse",
      "jar-code-reverse",
    ]

    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of professionalAgents) {
          const agent = await Agent.get(name)
          const prompt = agent?.prompt ?? ""
          // Check for markdown headers
          expect(prompt).toMatch(/^#/m)
        }
      },
    })
  })
})

describe("Agent Completeness Tests", () => {
  test("total agent count is at least 23", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agents = await Agent.list()
        expect(agents.length).toBeGreaterThanOrEqual(23)
      },
    })
  })

  test("all expected agent categories are present", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agents = await Agent.list()
        const names = agents.map((a) => a.name)

        // Core agents (7)
        const coreAgents = ["build", "plan", "general", "explore", "compaction", "title", "summary"]
        for (const name of coreAgents) {
          expect(names).toContain(name)
        }

        // Professional agents (9)
        const professionalAgents = [
          "code-reviewer",
          "security-reviewer",
          "tdd-guide",
          "architect",
          "writer",
          "proofreader",
          "code-reverse",
          "jar-code-reverse",
        ]
        for (const name of professionalAgents) {
          expect(names).toContain(name)
        }

        // ZRS agents (8)
        const zrsAgents = [
          "observer",
          "decision",
          "macro",
          "trader",
          "picker",
          "miniproduct",
          "synton-assistant",
          "ai-engineer",
        ]
        for (const name of zrsAgents) {
          expect(names).toContain(name)
        }
      },
    })
  })

  test("agent mode distribution is correct", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agents = await Agent.list()

        const primary = agents.filter((a) => a.mode === "primary")
        const subagent = agents.filter((a) => a.mode === "subagent")
        const hidden = agents.filter((a) => a.hidden === true)

        // Expected: 4-5 primary (build, plan, code-reverse, jar-code-reverse, and hidden ones)
        expect(primary.length).toBeGreaterThanOrEqual(4)

        // Expected: 16+ subagents (general, explore, 6 professional subagents, 8 ZRS agents)
        expect(subagent.length).toBeGreaterThanOrEqual(16)

        // Expected: 3 hidden (compaction, title, summary)
        expect(hidden.length).toBe(3)
      },
    })
  })
})
