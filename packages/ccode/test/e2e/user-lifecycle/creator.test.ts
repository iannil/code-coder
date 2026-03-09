/**
 * ULC-CRT-* Tests: Content Creator Lifecycle
 *
 * Tests for content creators using CodeCoder for long-form writing,
 * proofreading, and content expansion workflows.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Provider } from "../../../src/provider/provider"
import { Env } from "../../../src/util/env"
import { Session } from "../../../src/session"
import { Config } from "../../../src/config/config"
import { Agent } from "../../../src/agent/agent"
import { ReadTool } from "../../../src/tool/read"

const SKIP_E2E = process.env.SKIP_E2E !== "false"

const toolCtx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "writer",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe.skipIf(SKIP_E2E)("ULC-CRT: Content Creator Lifecycle", () => {
  describe("ULC-CRT-INIT: Initialization Phase", () => {
    test("ULC-CRT-INIT-001: should initialize for content creation workflow", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const config = await Config.get()
          expect(config).toBeDefined()
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("ULC-CRT-INIT-002: should have writer agent as option", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const writerAgent = await Agent.get("writer")

          expect(writerAgent).toBeDefined()
          expect(writerAgent.mode).toBe("primary")
          expect(writerAgent.description).toContain("long-form")
        },
      })
    })
  })

  describe("ULC-CRT-WRTR: Writing Workflow", () => {
    test("ULC-CRT-WRTR-001: should have writer agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const writerAgent = await Agent.get("writer")

          expect(writerAgent).toBeDefined()
          expect(writerAgent.name).toBe("writer")
          expect(writerAgent.mode).toBe("primary")
          expect(writerAgent.native).toBe(true)
          expect(writerAgent.description).toContain("20k+ words")
        },
      })
    })

    test("ULC-CRT-WRTR-002: should have writer agent with correct options", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const writerAgent = await Agent.get("writer")

          expect(writerAgent.options.maxOutputTokens).toBe(128_000)
          expect(writerAgent.temperature).toBe(0.7)
        },
      })
    })

    test("ULC-CRT-WRTR-003: should have proofreader agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const proofreaderAgent = await Agent.get("proofreader")

          expect(proofreaderAgent).toBeDefined()
          expect(proofreaderAgent.name).toBe("proofreader")
          expect(proofreaderAgent.mode).toBe("subagent")
          expect(proofreaderAgent.description).toContain("proofreading")
          expect(proofreaderAgent.description).toContain("PROOF")
        },
      })
    })

    test("ULC-CRT-WRTR-004: should have expander agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const expanderAgent = await Agent.get("expander")

          expect(expanderAgent).toBeDefined()
          expect(expanderAgent.name).toBe("expander")
          expect(expanderAgent.mode).toBe("subagent")
          expect(expanderAgent.description).toContain("expansion")
        },
      })
    })

    test("ULC-CRT-WRTR-005: unified expander should support fiction via domain detection", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const expanderAgent = await Agent.get("expander")

          expect(expanderAgent).toBeDefined()
          expect(expanderAgent.name).toBe("expander")
          expect(expanderAgent.mode).toBe("subagent")
          // Unified expander supports both fiction and nonfiction via domain tags
          expect(expanderAgent.description).toContain("fiction")
          expect(expanderAgent.description).toContain("nonfiction")
        },
      })
    })

    test("ULC-CRT-WRTR-006: unified expander prompt should include domain detection", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const expanderAgent = await Agent.get("expander")

          expect(expanderAgent).toBeDefined()
          expect(expanderAgent.name).toBe("expander")
          expect(expanderAgent.mode).toBe("subagent")
          // Prompt should include domain detection capability
          expect(expanderAgent.prompt).toContain("Domain Detection")
          expect(expanderAgent.prompt).toContain("[DOMAIN:fiction]")
          expect(expanderAgent.prompt).toContain("[DOMAIN:nonfiction]")
        },
      })
    })

    test("ULC-CRT-WRTR-007: should list all content creation agents", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const agents = await Agent.list()
          // expander-fiction and expander-nonfiction merged into unified expander
          const creatorAgents = ["writer", "proofreader", "expander"]

          const agentNames = agents.map((a) => a.name)
          for (const agentName of creatorAgents) {
            expect(agentNames).toContain(agentName)
          }
        },
      })
    })
  })

  describe("ULC-CRT-SESS: Session Management for Creators", () => {
    test("ULC-CRT-SESS-001: should create writing session", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const session = await Session.create({
            title: "My Novel Draft",
          })

          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("My Novel Draft")

          await Session.remove(session.id)
        },
      })
    })

    test("ULC-CRT-SESS-002: should manage multiple writing projects", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const novel = await Session.create({ title: "Fiction Novel" })
          const blog = await Session.create({ title: "Blog Series" })
          const essay = await Session.create({ title: "Academic Essay" })

          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }

          expect(sessions.length).toBeGreaterThanOrEqual(3)

          await Session.remove(novel.id)
          await Session.remove(blog.id)
          await Session.remove(essay.id)
        },
      })
    })
  })

  describe("ULC-CRT-DOCS: Document Management", () => {
    test("ULC-CRT-DOCS-001: should read manuscript file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "manuscript.md"),
            `# Chapter 1: The Beginning

It was a dark and stormy night. The protagonist sat by the window, contemplating the events that led to this moment.

## Scene 1

The rain pattered against the glass, creating a symphony of nature's melancholy.`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "manuscript.md") }, toolCtx)

          expect(result.output).toContain("Chapter 1")
          expect(result.output).toContain("The Beginning")
          expect(result.output).toContain("Scene 1")
        },
      })
    })

    test("ULC-CRT-DOCS-002: should read outline file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "outline.md"),
            `# Book Outline

## Part I: Introduction
- Chapter 1: Setting the Stage
- Chapter 2: Meeting the Characters

## Part II: Rising Action
- Chapter 3: The Conflict
- Chapter 4: Escalation

## Part III: Resolution
- Chapter 5: Climax
- Chapter 6: Conclusion`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "outline.md") }, toolCtx)

          expect(result.output).toContain("Book Outline")
          expect(result.output).toContain("Part I")
          expect(result.output).toContain("Part II")
          expect(result.output).toContain("Part III")
        },
      })
    })
  })

  describe("ULC-CRT-ADVN: Advanced Features for Creators", () => {
    test("ULC-CRT-ADVN-001: should support writer as default agent", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              default_agent: "writer",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const defaultAgent = await Agent.defaultAgent()
          expect(defaultAgent).toBe("writer")
        },
      })
    })

    test("ULC-CRT-ADVN-002: expander agents should have high output limits", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const expander = await Agent.get("expander")

          // Unified expander should have high output limits for long-form content
          // (expander-fiction and expander-nonfiction merged into unified expander)
          expect(expander.options.maxOutputTokens).toBe(128_000)
        },
      })
    })

    test("ULC-CRT-ADVN-003: proofreader should have low temperature for accuracy", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const proofreader = await Agent.get("proofreader")

          // Proofreader should have low temperature for consistent, accurate corrections
          expect(proofreader.temperature).toBe(0.3)
        },
      })
    })
  })

  describe("ULC-CRT-ERR: Error Handling for Creators", () => {
    test("ULC-CRT-ERR-001: should handle missing manuscript gracefully", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          const read = await ReadTool.init()

          await expect(read.execute({ filePath: path.join(tmp.path, "missing-manuscript.md") }, toolCtx)).rejects.toThrow(
            "not found",
          )
        },
      })
    })

    test("ULC-CRT-ERR-002: should handle session deletion gracefully", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-creator-key")
        },
        fn: async () => {
          // Removing non-existent session should not throw
          await Session.remove("ses_nonexistent_creator_123")
        },
      })
    })
  })
})
