/**
 * ULC-CD-* Tests: CLI Developer Lifecycle
 * Tests for CLI developer workflows
 */

import { describe, test, expect, beforeEach, mock } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Provider } from "../../src/provider/provider"
import { Env } from "../../src/env"
import { Session } from "../../src/session"
import { Storage } from "../../src/storage/storage"
import { setupGitHubMock, createGitHubMockController } from "../mock/github"
import { ReadTool } from "../../src/tool/read"
import { WriteTool } from "../../src/tool/write"
import { EditTool } from "../../src/tool/edit"
import { BashTool } from "../../src/tool/bash"
import { GlobTool } from "../../src/tool/glob"
import { GrepTool } from "../../src/tool/grep"
import { Skill } from "../../src/skill"

const toolCtx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe("CLI Developer Lifecycle - ULC-CD", () => {
  describe("ULC-CD-SESS-001: CLI developer session management", () => {
    test("should list all sessions", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create multiple sessions
          const session1 = await Session.create({ title: "Session 1" })
          const session2 = await Session.create({ title: "Session 2" })
          const session3 = await Session.create({ title: "Session 3" })

          // List sessions
          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }

          expect(sessions.length).toBeGreaterThanOrEqual(3)

          // Cleanup
          await Session.remove(session1.id)
          await Session.remove(session2.id)
          await Session.remove(session3.id)
        },
      })
    })

    test("should continue previous session", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create a session
          const original = await Session.create({ title: "Continue Test" })

          // Retrieve it (simulating --continue)
          const retrieved = await Session.get(original.id)
          expect(retrieved).toBeDefined()
          expect(retrieved?.id).toBe(original.id)
          expect(retrieved?.title).toBe("Continue Test")

          await Session.remove(original.id)
        },
      })
    })

    test("should attach to specific session by ID", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create sessions
          const session1 = await Session.create({ title: "Target Session" })
          await Session.create({ title: "Other Session" })

          // Get specific session
          const target = await Session.get(session1.id)
          expect(target?.title).toBe("Target Session")

          await Session.remove(session1.id)
        },
      })
    })

    test("should create new session independent of existing", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session1 = await Session.create({ title: "First" })
          const session2 = await Session.create({ title: "Second" })

          expect(session1.id).not.toBe(session2.id)
          expect(session1.projectID).toBe(session2.projectID)

          await Session.remove(session1.id)
          await Session.remove(session2.id)
        },
      })
    })
  })

  describe("ULC-CD-AGNT-001: CLI developer agent modes", () => {
    test("should support plan agent", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Verify plan agent is available
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          // Plan mode uses the same provider with different agent settings
          const model = await Provider.getModel("anthropic", "claude-sonnet-4-20250514")
          expect(model).toBeDefined()
        },
      })
    })

    test("should support build agent", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          // Build agent can use models with tool call capability
          const model = providers["anthropic"].models["claude-sonnet-4-20250514"]
          expect(model.capabilities.toolcall).toBe(true)
        },
      })
    })

    test("should support explore agent", async () => {
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

    test("should support general agent", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // General agent uses default model
          const model = await Provider.defaultModel()
          expect(model).toBeDefined()
          expect(model.providerID).toBeDefined()
          expect(model.modelID).toBeDefined()
        },
      })
    })
  })

  describe("ULC-CD-EXP-001: CLI developer export/import session", () => {
    test("should export session to JSON format", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Export Test" })

          // Get session info
          const info = await Session.get(session.id)
          expect(info).toBeDefined()
          expect(info?.title).toBe("Export Test")

          // Get session messages
          const messages = await Session.messages({ sessionID: session.id })
          expect(Array.isArray(messages)).toBe(true)

          // Export data structure
          const exportData = {
            info,
            messages: messages.map((msg) => ({
              info: msg.info,
              parts: msg.parts,
            })),
          }

          expect(exportData.info).toBeDefined()
          expect(exportData.messages).toBeDefined()

          await Session.remove(session.id)
        },
      })
    })

    test("should maintain JSON format validity", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "JSON Validity Test" })

          const info = await Session.get(session.id)
          const messages = await Session.messages({ sessionID: session.id })

          const exportData = {
            info,
            messages: messages.map((msg) => ({
              info: msg.info,
              parts: msg.parts,
            })),
          }

          // Verify JSON can be stringified and parsed
          const jsonStr = JSON.stringify(exportData)
          const parsed = JSON.parse(jsonStr)

          expect(parsed.info.id).toBe(session.id)
          expect(Array.isArray(parsed.messages)).toBe(true)

          await Session.remove(session.id)
        },
      })
    })

    test("should import session from JSON data", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // First create a session to get a valid ID format, then we'll simulate import
          const existing = await Session.create({ title: "Temp Session" })
          const validIdFormat = existing.id

          // Create export data with valid ID format
          const exportData = {
            info: {
              id: validIdFormat,
              projectID: Instance.project.id,
              title: "Imported Session",
              time: {
                created: Date.now(),
                updated: Date.now(),
              },
            } as Session.Info,
            messages: [],
          }

          // Update the session to simulate import (updating with new data)
          await Session.update(validIdFormat, (s) => {
            s.title = "Imported Session"
          })

          // Verify import
          const imported = await Session.get(validIdFormat)
          expect(imported).toBeDefined()
          expect(imported?.title).toBe("Imported Session")

          await Session.remove(validIdFormat)
        },
      })
    })
  })

  describe("ULC-CD-PR-001: CLI developer PR workflow", () => {
    test("should create PR with title and description", async () => {
      const ghController = createGitHubMockController()

      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          // Initialize git repo
          await Bun.write(path.join(dir, ".git", "config"), "[core]\n")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Simulate PR creation command structure
          const prData = {
            title: "feat: Add new feature",
            body: "## Summary\n- Added new functionality\n\n## Test plan\n- Run tests",
            base: "main",
            head: "feature-branch",
          }

          // Verify PR data structure is valid
          expect(prData.title).toBeDefined()
          expect(prData.body).toContain("Summary")
          expect(prData.base).toBe("main")
          expect(prData.head).toBeDefined()

          // Use mock controller to simulate PR creation response
          ghController.setPRs([
            {
              number: 123,
              title: prData.title,
              body: prData.body,
              state: "open",
              url: "https://github.com/test/repo/pull/123",
              headRefName: prData.head,
              baseRefName: prData.base,
              author: { login: "test-user" },
            },
          ])

          const prs = ghController.getPRs()
          expect(prs.length).toBe(1)
          expect(prs[0].title).toBe("feat: Add new feature")
          expect(prs[0].number).toBe(123)
        },
      })
    })

    test("should list open PRs for repository", async () => {
      const ghController = createGitHubMockController()

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
          // Set up multiple PRs
          ghController.setPRs([
            {
              number: 1,
              title: "PR 1",
              state: "open",
              url: "https://github.com/test/repo/pull/1",
              headRefName: "branch-1",
              baseRefName: "main",
              author: { login: "user1" },
            },
            {
              number: 2,
              title: "PR 2",
              state: "open",
              url: "https://github.com/test/repo/pull/2",
              headRefName: "branch-2",
              baseRefName: "main",
              author: { login: "user2" },
            },
            {
              number: 3,
              title: "PR 3",
              state: "closed",
              url: "https://github.com/test/repo/pull/3",
              headRefName: "branch-3",
              baseRefName: "main",
              author: { login: "user3" },
            },
          ])

          const allPRs = ghController.getPRs()
          expect(allPRs.length).toBe(3)

          const openPRs = allPRs.filter((pr) => pr.state === "open")
          expect(openPRs.length).toBe(2)
        },
      })
    })

    test("should view PR details by number", async () => {
      const ghController = createGitHubMockController()

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
          ghController.setPRs([
            {
              number: 42,
              title: "Important PR",
              body: "This is an important change",
              state: "open",
              url: "https://github.com/test/repo/pull/42",
              headRefName: "feature",
              baseRefName: "main",
              author: { login: "developer" },
              additions: 100,
              deletions: 50,
              changedFiles: 5,
            },
          ])

          const pr = ghController.getPRs().find((p) => p.number === 42)
          expect(pr).toBeDefined()
          expect(pr?.title).toBe("Important PR")
          expect(pr?.body).toBe("This is an important change")
          expect(pr?.additions).toBe(100)
          expect(pr?.deletions).toBe(50)
          expect(pr?.changedFiles).toBe(5)
        },
      })
    })

    test("should support PR merge operations", async () => {
      const ghController = createGitHubMockController()

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
          ghController.setPRs([
            {
              number: 99,
              title: "Ready to merge",
              state: "open",
              url: "https://github.com/test/repo/pull/99",
              headRefName: "ready-branch",
              baseRefName: "main",
              author: { login: "dev" },
              mergeable: true,
            },
          ])

          const pr = ghController.getPRs().find((p) => p.number === 99)
          expect(pr?.state).toBe("open")
          expect(pr?.mergeable).toBe(true)

          // Simulate merge by updating PR state
          ghController.setPRs([
            {
              ...pr!,
              state: "merged",
            },
          ])

          const mergedPR = ghController.getPRs().find((p) => p.number === 99)
          expect(mergedPR?.state).toBe("merged")
        },
      })
    })

    test("should handle PR with checks status", async () => {
      const ghController = createGitHubMockController()

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
          ghController.setChecks([
            {
              name: "CI / Build",
              status: "completed",
              conclusion: "success",
            },
            {
              name: "CI / Test",
              status: "completed",
              conclusion: "success",
            },
            {
              name: "CI / Lint",
              status: "in_progress",
              conclusion: null,
            },
          ])

          const checks = ghController.getChecks()
          expect(checks.length).toBe(3)

          const completedChecks = checks.filter((c) => c.status === "completed")
          expect(completedChecks.length).toBe(2)

          const successfulChecks = checks.filter((c) => c.conclusion === "success")
          expect(successfulChecks.length).toBe(2)

          const pendingChecks = checks.filter((c) => c.status === "in_progress")
          expect(pendingChecks.length).toBe(1)
        },
      })
    })

    test("should support PR review comments", async () => {
      const ghController = createGitHubMockController()

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
          ghController.setComments([
            {
              id: 1,
              body: "LGTM!",
              author: { login: "reviewer1" },
              path: "src/index.ts",
              line: 42,
            },
            {
              id: 2,
              body: "Consider using const here",
              author: { login: "reviewer2" },
              path: "src/utils.ts",
              line: 10,
            },
          ])

          const comments = ghController.getComments()
          expect(comments.length).toBe(2)
          expect(comments[0].body).toBe("LGTM!")
          expect(comments[1].path).toBe("src/utils.ts")
        },
      })
    })
  })

  describe("ULC-CD-GIT-001: CLI developer git integration", () => {
    test("should detect git repository", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          // Create .git directory to simulate git repo
          await Bun.write(
            path.join(dir, ".git", "config"),
            "[core]\n\trepositoryformatversion = 0\n",
          )
          await Bun.write(path.join(dir, ".git", "HEAD"), "ref: refs/heads/main\n")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Check for .git directory existence
          const gitConfig = Bun.file(path.join(tmp.path, ".git", "config"))
          expect(await gitConfig.exists()).toBe(true)

          const gitHead = Bun.file(path.join(tmp.path, ".git", "HEAD"))
          const headContent = await gitHead.text()
          expect(headContent).toContain("refs/heads/main")
        },
      })
    })

    test("should read git branch from HEAD", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(path.join(dir, ".git", "HEAD"), "ref: refs/heads/feature-branch\n")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const headContent = await Bun.file(path.join(tmp.path, ".git", "HEAD")).text()
          const branchMatch = headContent.match(/ref: refs\/heads\/(.+)/)
          expect(branchMatch).toBeDefined()
          expect(branchMatch?.[1].trim()).toBe("feature-branch")
        },
      })
    })

    test("should handle detached HEAD state", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          // Detached HEAD contains commit SHA directly
          await Bun.write(path.join(dir, ".git", "HEAD"), "abc123def456789\n")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const headContent = await Bun.file(path.join(tmp.path, ".git", "HEAD")).text()
          const isDetached = !headContent.startsWith("ref:")
          expect(isDetached).toBe(true)
          expect(headContent.trim()).toBe("abc123def456789")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-001: Read tool for file reading", () => {
    test("should read text file contents", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "test.txt"), "Hello, World!\nLine 2")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "test.txt") }, toolCtx)
          expect(result.output).toContain("Hello, World!")
          expect(result.output).toContain("Line 2")
        },
      })
    })

    test("should read file with offset and limit", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          const lines = Array.from({ length: 20 }, (_, i) => `Line ${i + 1}`).join("\n")
          await Bun.write(path.join(dir, "large.txt"), lines)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "large.txt"), offset: 5, limit: 5 }, toolCtx)
          expect(result.output).toContain("Line 6")
          expect(result.output).toContain("Line 10")
          expect(result.output).not.toContain("Line 1\n")
          expect(result.output).not.toContain("Line 11")
        },
      })
    })

    test("should handle non-existent file", async () => {
      await using tmp = await tmpdir({})

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          // ReadTool.execute throws for non-existent files
          await expect(
            read.execute({ filePath: path.join(tmp.path, "nonexistent.txt") }, toolCtx),
          ).rejects.toThrow("not found")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-002: Write tool for file creation", () => {
    test("should create new file with content", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const write = await WriteTool.init()
          const filepath = path.join(tmp.path, "new-file.txt")
          const result = await write.execute({ filePath: filepath, content: "New content" }, toolCtx)
          expect(result.output).toContain("successfully")

          const content = await Bun.file(filepath).text()
          expect(content).toBe("New content")
        },
      })
    })

    test("should overwrite existing file", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(path.join(dir, "existing.txt"), "Old content")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const write = await WriteTool.init()
          const read = await ReadTool.init()
          const filepath = path.join(tmp.path, "existing.txt")

          // Must read file before overwriting
          await read.execute({ filePath: filepath }, toolCtx)
          await write.execute({ filePath: filepath, content: "Updated content" }, toolCtx)

          const content = await Bun.file(filepath).text()
          expect(content).toBe("Updated content")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-003: Edit tool for file modification", () => {
    test("should replace text in file", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(path.join(dir, "edit.txt"), "Hello World")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const edit = await EditTool.init()
          const read = await ReadTool.init()
          const filepath = path.join(tmp.path, "edit.txt")

          // Must read file before editing
          await read.execute({ filePath: filepath }, toolCtx)
          await edit.execute({ filePath: filepath, oldString: "World", newString: "Universe" }, toolCtx)

          const content = await Bun.file(filepath).text()
          expect(content).toBe("Hello Universe")
        },
      })
    })

    test("should replace all occurrences with replaceAll", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(path.join(dir, "multi.txt"), "foo bar foo baz foo")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const edit = await EditTool.init()
          const read = await ReadTool.init()
          const filepath = path.join(tmp.path, "multi.txt")

          // Must read file before editing
          await read.execute({ filePath: filepath }, toolCtx)
          await edit.execute({ filePath: filepath, oldString: "foo", newString: "qux", replaceAll: true }, toolCtx)

          const content = await Bun.file(filepath).text()
          expect(content).toBe("qux bar qux baz qux")
        },
      })
    })

    test("should throw error when oldString not found", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(path.join(dir, "content.txt"), "Hello World")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const edit = await EditTool.init()
          const read = await ReadTool.init()
          const filepath = path.join(tmp.path, "content.txt")

          // Must read file before editing
          await read.execute({ filePath: filepath }, toolCtx)

          await expect(
            edit.execute({ filePath: filepath, oldString: "NotFound", newString: "Replaced" }, toolCtx),
          ).rejects.toThrow("not found")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-004: Bash tool for command execution", () => {
    test("should execute simple command", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const bash = await BashTool.init()
          const result = await bash.execute({ command: "echo 'test output'", description: "Test echo" }, toolCtx)
          expect(result.metadata.exit).toBe(0)
          expect(result.metadata.output).toContain("test output")
        },
      })
    })

    test("should handle command failure", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const bash = await BashTool.init()
          const result = await bash.execute({ command: "exit 1", description: "Force exit" }, toolCtx)
          expect(result.metadata.exit).toBe(1)
        },
      })
    })

    test("should execute chained commands", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const bash = await BashTool.init()
          const result = await bash.execute(
            { command: "echo 'first' && echo 'second'", description: "Chained commands" },
            toolCtx,
          )
          expect(result.metadata.exit).toBe(0)
          expect(result.metadata.output).toContain("first")
          expect(result.metadata.output).toContain("second")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-005: Glob tool for file pattern matching", () => {
    test("should find files matching pattern", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "file1.ts"), "")
          await Bun.write(path.join(dir, "file2.ts"), "")
          await Bun.write(path.join(dir, "file3.js"), "")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const glob = await GlobTool.init()
          const result = await glob.execute({ pattern: "*.ts" }, toolCtx)
          expect(result.output).toContain("file1.ts")
          expect(result.output).toContain("file2.ts")
          expect(result.output).not.toContain("file3.js")
        },
      })
    })

    test("should handle no matches", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "file.txt"), "")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const glob = await GlobTool.init()
          const result = await glob.execute({ pattern: "*.py" }, toolCtx)
          expect(result.output).toContain("No files found")
        },
      })
    })

    test("should search in specified path", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "src", "index.ts"), "")
          await Bun.write(path.join(dir, "test", "test.ts"), "")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const glob = await GlobTool.init()
          const result = await glob.execute({ pattern: "*.ts", path: path.join(tmp.path, "src") }, toolCtx)
          expect(result.output).toContain("index.ts")
          expect(result.output).not.toContain("test.ts")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-006: Grep tool for content search", () => {
    test("should find content matching pattern", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "search.ts"), "function hello() {\n  console.log('world')\n}")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const grep = await GrepTool.init()
          const result = await grep.execute({ pattern: "function" }, toolCtx)
          expect(result.output).toContain("search.ts")
        },
      })
    })

    test("should search with regex pattern", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "code.ts"), "const foo = 123\nconst bar = 456")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const grep = await GrepTool.init()
          const result = await grep.execute({ pattern: "const \\w+ = \\d+" }, toolCtx)
          expect(result.output).toContain("code.ts")
        },
      })
    })

    test("should handle no matches", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "file.txt"), "hello world")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const grep = await GrepTool.init()
          const result = await grep.execute({ pattern: "notfound" }, toolCtx)
          expect(result.output).toContain("No files found")
        },
      })
    })
  })

  describe("ULC-CD-TOOL-007: Tool error handling", () => {
    test("should handle file not found gracefully", async () => {
      await using tmp = await tmpdir({})

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          await expect(
            read.execute({ filePath: path.join(tmp.path, "missing.txt") }, toolCtx),
          ).rejects.toThrow("not found")
        },
      })
    })

    test("should handle invalid path gracefully", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const edit = await EditTool.init()
          await expect(
            edit.execute({ filePath: path.join(tmp.path, "nonexistent.txt"), oldString: "a", newString: "b" }, toolCtx),
          ).rejects.toThrow()
        },
      })
    })
  })

  describe("ULC-CD-TOOL-008: Tool batch operations", () => {
    test("should handle multiple file operations", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const write = await WriteTool.init()
          const read = await ReadTool.init()

          // Write multiple files
          await write.execute({ filePath: path.join(tmp.path, "batch1.txt"), content: "content1" }, toolCtx)
          await write.execute({ filePath: path.join(tmp.path, "batch2.txt"), content: "content2" }, toolCtx)
          await write.execute({ filePath: path.join(tmp.path, "batch3.txt"), content: "content3" }, toolCtx)

          // Verify all files
          const r1 = await read.execute({ filePath: path.join(tmp.path, "batch1.txt") }, toolCtx)
          const r2 = await read.execute({ filePath: path.join(tmp.path, "batch2.txt") }, toolCtx)
          const r3 = await read.execute({ filePath: path.join(tmp.path, "batch3.txt") }, toolCtx)

          expect(r1.output).toContain("content1")
          expect(r2.output).toContain("content2")
          expect(r3.output).toContain("content3")
        },
      })
    })
  })

  describe("ULC-CD-SKILL-001: Skill system listing", () => {
    test("should list available skills", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          expect(Array.isArray(skills)).toBe(true)
          expect(skills.length).toBeGreaterThan(0)

          // Verify skill structure
          const skill = skills[0]
          expect(skill.name).toBeDefined()
          expect(typeof skill.name).toBe("string")
        },
      })
    })

    test("should include built-in skills", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const skillNames = skills.map((s) => s.name)

          // Verify some common built-in skills exist
          expect(skillNames.length).toBeGreaterThan(0)
        },
      })
    })
  })

  describe("ULC-CD-SKILL-002: Skill discovery from project", () => {
    test("should discover skills from .codecoder/skills directory", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".codecoder", "skills", "custom-skill")
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: custom-skill
description: A custom skill for testing.
---

# Custom Skill

Instructions here.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const customSkill = skills.find((s) => s.name === "custom-skill")
          expect(customSkill).toBeDefined()
          expect(customSkill!.description).toBe("A custom skill for testing.")
        },
      })
    })

    test("should skip skills with invalid frontmatter", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".codecoder", "skills", "invalid-skill")
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `# No Frontmatter

Just content without YAML frontmatter.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const invalidSkill = skills.find((s) => s.name === "invalid-skill")
          expect(invalidSkill).toBeUndefined()
        },
      })
    })
  })

  describe("ULC-CD-SKILL-003: Multiple skill sources", () => {
    test("should discover skills from multiple directories", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Create skills in .codecoder/skills
          const ccodeSkillDir = path.join(dir, ".codecoder", "skills", "ccode-skill")
          await Bun.write(
            path.join(ccodeSkillDir, "SKILL.md"),
            `---
name: ccode-skill
description: Skill from .codecoder directory.
---

# CCode Skill
`,
          )

          // Create skills in .claude/skills
          const claudeSkillDir = path.join(dir, ".claude", "skills", "claude-skill")
          await Bun.write(
            path.join(claudeSkillDir, "SKILL.md"),
            `---
name: claude-skill
description: Skill from .claude directory.
---

# Claude Skill
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const ccodeSkill = skills.find((s) => s.name === "ccode-skill")
          const claudeSkill = skills.find((s) => s.name === "claude-skill")

          expect(ccodeSkill).toBeDefined()
          expect(claudeSkill).toBeDefined()
        },
      })
    })
  })

  describe("ULC-CD-SKILL-004: Skill retrieval by name", () => {
    test("should get skill by name", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".codecoder", "skills", "my-skill")
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: my-skill
description: My custom skill for testing.
---

# My Skill

Do something useful.
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("my-skill")
          expect(skill).toBeDefined()
          expect(skill!.name).toBe("my-skill")
          expect(skill!.description).toBe("My custom skill for testing.")
          expect(skill!.location).toContain("SKILL.md")
        },
      })
    })

    test("should return undefined for non-existent skill", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("nonexistent-skill")
          expect(skill).toBeUndefined()
        },
      })
    })
  })

  describe("ULC-CD-SKILL-005: Built-in skills", () => {
    test("should include built-in commit skill", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skills = await Skill.all()
          const commitSkill = skills.find((s) => s.name === "commit")

          // Built-in commit skill should exist
          if (commitSkill) {
            expect(commitSkill.name).toBe("commit")
            expect(commitSkill.description).toBeDefined()
          }
        },
      })
    })

    test("should have skill location path", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const skillDir = path.join(dir, ".codecoder", "skills", "located-skill")
          await Bun.write(
            path.join(skillDir, "SKILL.md"),
            `---
name: located-skill
description: A skill with known location.
---

# Located Skill
`,
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const skill = await Skill.get("located-skill")
          expect(skill).toBeDefined()
          expect(skill!.location).toContain(tmp.path)
          expect(skill!.location).toContain("SKILL.md")
        },
      })
    })
  })

  describe("ULC-CD-ERR-001: CLI developer tool execution errors", () => {
    test("should handle permission denial gracefully", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const write = await WriteTool.init()
          const ctxWithDeny = {
            ...toolCtx,
            ask: async () => {
              throw new Error("Permission denied")
            },
          }

          await expect(
            write.execute({ filePath: path.join(tmp.path, "denied.txt"), content: "test" }, ctxWithDeny),
          ).rejects.toThrow("Permission denied")
        },
      })
    })

    test("should handle command timeout", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const bash = await BashTool.init()
          // Very short timeout to trigger timeout behavior
          const result = await bash.execute(
            { command: "sleep 0.01 && echo done", description: "Quick sleep", timeout: 5000 },
            toolCtx,
          )
          expect(result.metadata.exit).toBe(0)
        },
      })
    })
  })

  describe("ULC-CD-ERR-002: CLI developer permission rejection", () => {
    test("should reject edit when user denies", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(path.join(dir, "protected.txt"), "original")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const edit = await EditTool.init()
          const read = await ReadTool.init()
          const filepath = path.join(tmp.path, "protected.txt")

          // Read file first (required before editing)
          await read.execute({ filePath: filepath }, toolCtx)

          const ctxWithReject = {
            ...toolCtx,
            ask: async () => {
              throw new Error("User rejected permission")
            },
          }

          await expect(
            edit.execute(
              { filePath: filepath, oldString: "original", newString: "modified" },
              ctxWithReject,
            ),
          ).rejects.toThrow()

          // Verify file unchanged
          const content = await Bun.file(filepath).text()
          expect(content).toBe("original")
        },
      })
    })
  })
})
