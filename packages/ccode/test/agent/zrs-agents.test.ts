import { test, expect, describe } from "bun:test"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Agent } from "../../src/agent/agent"

const ZRS_AGENTS = [
  "observer",
  "decision",
  "macro",
  "trader",
  "picker",
  "miniproduct",
  "synton-assistant",
  "ai-engineer",
] as const

describe("ZRS Agents Configuration", () => {
  test("all ZRS agents exist", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent).toBeDefined()
          expect(agent?.name).toBe(name)
        }
      },
    })
  })

  test("all ZRS agents are subagents", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent?.mode).toBe("subagent")
        }
      },
    })
  })

  test("all ZRS agents are native", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent?.native).toBe(true)
        }
      },
    })
  })

  test("all ZRS agents have prompts with sufficient length", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent?.prompt).toBeDefined()
          expect(agent?.prompt?.length).toBeGreaterThan(100)
        }
      },
    })
  })

  test("all ZRS agents have Chinese descriptions", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent?.description).toBeDefined()
          // Chinese character regex
          expect(agent?.description).toMatch(/[\u4e00-\u9fa5]/)
        }
      },
    })
  })

  test("all ZRS agents have temperature configured", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent?.temperature).toBeDefined()
          expect(agent?.temperature).toBeGreaterThan(0)
          expect(agent?.temperature).toBeLessThanOrEqual(1)
        }
      },
    })
  })

  test("all ZRS agents are not hidden", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        for (const name of ZRS_AGENTS) {
          const agent = await Agent.get(name)
          expect(agent?.hidden).toBeUndefined()
        }
      },
    })
  })

  test("all ZRS agents appear in agent list", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agents = await Agent.list()
        const names = agents.map((a) => a.name)
        for (const name of ZRS_AGENTS) {
          expect(names).toContain(name)
        }
      },
    })
  })
})

describe("ZRS Agents Individual Tests", () => {
  test("observer has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("observer")
        expect(agent?.temperature).toBe(0.7)
        expect(agent?.description).toContain("观察者")
      },
    })
  })

  test("decision has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("decision")
        expect(agent?.temperature).toBe(0.6)
        expect(agent?.description).toContain("决策")
      },
    })
  })

  test("macro has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("macro")
        expect(agent?.temperature).toBe(0.5)
        expect(agent?.description).toContain("宏观经济")
      },
    })
  })

  test("trader has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("trader")
        expect(agent?.temperature).toBe(0.5)
        expect(agent?.description).toContain("交易")
      },
    })
  })

  test("picker has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("picker")
        expect(agent?.temperature).toBe(0.6)
        expect(agent?.description).toContain("选品")
      },
    })
  })

  test("miniproduct has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("miniproduct")
        expect(agent?.temperature).toBe(0.6)
        expect(agent?.description).toContain("极小产品")
      },
    })
  })

  test("synton-assistant has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("synton-assistant")
        expect(agent?.temperature).toBe(0.5)
        expect(agent?.description).toContain("SYNTON")
      },
    })
  })

  test("ai-engineer has correct properties", async () => {
    await using tmp = await tmpdir()
    await Instance.provide({
      directory: tmp.path,
      fn: async () => {
        const agent = await Agent.get("ai-engineer")
        expect(agent?.temperature).toBe(0.5)
        expect(agent?.description).toContain("AI工程师")
      },
    })
  })
})
