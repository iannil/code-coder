/**
 * ULC-REV-* Tests: Reverse Engineer Lifecycle
 *
 * Tests for reverse engineers using CodeCoder for website
 * reverse engineering and JAR package analysis workflows.
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
  agent: "code-reverse",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe.skipIf(SKIP_E2E)("ULC-REV: Reverse Engineer Lifecycle", () => {
  describe("ULC-REV-INIT: Initialization Phase", () => {
    test("ULC-REV-INIT-001: should initialize for reverse engineering workflow", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const config = await Config.get()
          expect(config).toBeDefined()
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("ULC-REV-INIT-002: should have code-reverse agent available", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const codeReverseAgent = await Agent.get("code-reverse")

          expect(codeReverseAgent).toBeDefined()
          expect(codeReverseAgent.mode).toBe("subagent")
          expect(codeReverseAgent.description).toContain("reverse engineering")
        },
      })
    })
  })

  describe("ULC-REV-REVS: Reverse Engineering Workflow", () => {
    test("ULC-REV-REVS-001: should have code-reverse agent with correct configuration", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const codeReverseAgent = await Agent.get("code-reverse")

          expect(codeReverseAgent).toBeDefined()
          expect(codeReverseAgent.name).toBe("code-reverse")
          expect(codeReverseAgent.mode).toBe("subagent")
          expect(codeReverseAgent.native).toBe(true)
          expect(codeReverseAgent.description).toContain("Website")
          expect(codeReverseAgent.description).toContain("pixel-perfect")
          expect(codeReverseAgent.temperature).toBe(0.3)
          expect(codeReverseAgent.color).toBe("cyan")
        },
      })
    })

    test("ULC-REV-REVS-002: should have jar-code-reverse agent with correct configuration", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const jarReverseAgent = await Agent.get("jar-code-reverse")

          expect(jarReverseAgent).toBeDefined()
          expect(jarReverseAgent.name).toBe("jar-code-reverse")
          expect(jarReverseAgent.mode).toBe("subagent")
          expect(jarReverseAgent.native).toBe(true)
          expect(jarReverseAgent.description).toContain("JAR")
          expect(jarReverseAgent.description).toContain("Java")
          expect(jarReverseAgent.temperature).toBe(0.3)
          expect(jarReverseAgent.color).toBe("magenta")
        },
      })
    })

    test("ULC-REV-REVS-003: code-reverse should have plan permissions", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const codeReverseAgent = await Agent.get("code-reverse")

          // code-reverse should have plan_enter and plan_exit permissions
          const hasQuestion = codeReverseAgent.permission.some((p) => p.permission === "question" && p.action === "allow")
          const hasPlanEnter = codeReverseAgent.permission.some((p) => p.permission === "plan_enter" && p.action === "allow")
          const hasPlanExit = codeReverseAgent.permission.some((p) => p.permission === "plan_exit" && p.action === "allow")

          expect(hasQuestion).toBe(true)
          expect(hasPlanEnter).toBe(true)
          expect(hasPlanExit).toBe(true)
        },
      })
    })

    test("ULC-REV-REVS-004: jar-code-reverse should have JAR read permissions", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const jarReverseAgent = await Agent.get("jar-code-reverse")

          // jar-code-reverse should have read permissions for JAR files
          const hasJarRead = jarReverseAgent.permission.some(
            (p) => p.permission === "read" && p.pattern === "*.jar" && p.action === "allow",
          )
          const hasMetaInfRead = jarReverseAgent.permission.some(
            (p) => p.permission === "read" && p.pattern === "META-INF/*" && p.action === "allow",
          )

          expect(hasJarRead).toBe(true)
          expect(hasMetaInfRead).toBe(true)
        },
      })
    })

    test("ULC-REV-REVS-005: should list all reverse engineering agents", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const agents = await Agent.list()
          const reverseAgents = ["code-reverse", "jar-code-reverse"]

          const agentNames = agents.map((a) => a.name)
          for (const agentName of reverseAgents) {
            expect(agentNames).toContain(agentName)
          }
        },
      })
    })
  })

  describe("ULC-REV-SESS: Session Management for Reverse Engineers", () => {
    test("ULC-REV-SESS-001: should create reverse engineering session", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const session = await Session.create({
            title: "Website Clone: example.com",
          })

          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("Website Clone: example.com")

          await Session.remove(session.id)
        },
      })
    })

    test("ULC-REV-SESS-002: should manage multiple reverse engineering projects", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const webSession = await Session.create({ title: "Web Reverse: site-a.com" })
          const jarSession = await Session.create({ title: "JAR Reverse: app.jar" })

          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }

          expect(sessions.length).toBeGreaterThanOrEqual(2)

          await Session.remove(webSession.id)
          await Session.remove(jarSession.id)
        },
      })
    })
  })

  describe("ULC-REV-FILE: File Operations for Reverse Engineering", () => {
    test("ULC-REV-FILE-001: should read analysis plan file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "reverse-plan.md"),
            `# Reverse Engineering Plan: example.com

## Technology Stack Analysis
- Frontend: React 18
- CSS: Tailwind CSS
- State Management: Redux

## Design System
- Primary Color: #3B82F6
- Font: Inter
- Spacing: 8px grid

## Component Structure
1. Header
2. Hero Section
3. Feature Cards
4. Footer

## Implementation Steps
1. Setup project with Next.js
2. Configure Tailwind CSS
3. Implement base components
4. Add responsive design`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "reverse-plan.md") }, toolCtx)

          expect(result.output).toContain("Reverse Engineering Plan")
          expect(result.output).toContain("Technology Stack")
          expect(result.output).toContain("Design System")
          expect(result.output).toContain("Implementation Steps")
        },
      })
    })

    test("ULC-REV-FILE-002: should read JAR analysis file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(
            path.join(dir, "jar-analysis.md"),
            `# JAR Analysis Report: app.jar

## Package Structure
- com.example.app
  - controllers/
  - services/
  - repositories/
  - models/

## Dependencies
- Spring Boot 3.2
- Spring Data JPA
- PostgreSQL Driver
- Lombok

## Key Classes
- MainApplication.java
- UserController.java
- UserService.java
- UserRepository.java

## Reconstruction Plan
1. Setup Spring Boot project
2. Create entity classes
3. Implement repository layer
4. Add service layer
5. Create REST controllers`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "jar-analysis.md") }, toolCtx)

          expect(result.output).toContain("JAR Analysis Report")
          expect(result.output).toContain("Package Structure")
          expect(result.output).toContain("Dependencies")
          expect(result.output).toContain("Reconstruction Plan")
        },
      })
    })
  })

  describe("ULC-REV-ADVN: Advanced Features for Reverse Engineers", () => {
    test("ULC-REV-ADVN-001: reverse agents should have low temperature for precision", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const codeReverse = await Agent.get("code-reverse")
          const jarReverse = await Agent.get("jar-code-reverse")

          // Reverse engineering agents should have low temperature for precise analysis
          expect(codeReverse.temperature).toBe(0.3)
          expect(jarReverse.temperature).toBe(0.3)
        },
      })
    })

    test("ULC-REV-ADVN-002: reverse agents should have distinct colors", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const codeReverse = await Agent.get("code-reverse")
          const jarReverse = await Agent.get("jar-code-reverse")

          // Each reverse agent should have a distinct color
          expect(codeReverse.color).toBe("cyan")
          expect(jarReverse.color).toBe("magenta")
          expect(codeReverse.color).not.toBe(jarReverse.color)
        },
      })
    })

    test("ULC-REV-ADVN-003: reverse agents should be able to access explore agent", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const agents = await Agent.list()

          // Explore agent should be available for codebase analysis during reverse engineering
          const exploreAgent = agents.find((a) => a.name === "explore")
          expect(exploreAgent).toBeDefined()
          expect(exploreAgent?.mode).toBe("subagent")
        },
      })
    })

    test("ULC-REV-ADVN-004: verifier agent should be available for validation", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const verifierAgent = await Agent.get("verifier")

          expect(verifierAgent).toBeDefined()
          expect(verifierAgent.name).toBe("verifier")
          expect(verifierAgent.description).toContain("Verification")
          expect(verifierAgent.temperature).toBe(0.1) // Very low for accurate validation
        },
      })
    })
  })

  describe("ULC-REV-ERR: Error Handling for Reverse Engineers", () => {
    test("ULC-REV-ERR-001: should handle missing analysis file gracefully", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          const read = await ReadTool.init()

          await expect(read.execute({ filePath: path.join(tmp.path, "missing-analysis.md") }, toolCtx)).rejects.toThrow(
            "not found",
          )
        },
      })
    })

    test("ULC-REV-ERR-002: should handle non-existent session gracefully", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          await expect(Session.get("ses_nonexistent_reverse_123")).rejects.toThrow("NotFoundError")
        },
      })
    })

    test("ULC-REV-ERR-003: should handle concurrent reverse sessions", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-reverse-key")
        },
        fn: async () => {
          // Create multiple reverse engineering sessions concurrently
          const [web1, web2, jar1] = await Promise.all([
            Session.create({ title: "Web Reverse 1" }),
            Session.create({ title: "Web Reverse 2" }),
            Session.create({ title: "JAR Reverse 1" }),
          ])

          expect(web1.id).toBeDefined()
          expect(web2.id).toBeDefined()
          expect(jar1.id).toBeDefined()

          // All sessions should be unique
          expect(new Set([web1.id, web2.id, jar1.id]).size).toBe(3)

          // Cleanup
          await Promise.all([Session.remove(web1.id), Session.remove(web2.id), Session.remove(jar1.id)])
        },
      })
    })
  })
})
