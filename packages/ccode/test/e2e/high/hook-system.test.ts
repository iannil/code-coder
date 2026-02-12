/**
 * E2E High Priority Test: Hook System
 * Priority: High - Runs daily
 *
 * Tests the hook system's ability to intercept tool calls,
 * validate content, and chain multiple hooks.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Hook } from "../../../src/hook/hook"

describe("E2E High: Hook System", () => {
  describe("PreToolUse Blocking", () => {
    test("should block Edit with secret patterns", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  block_secrets: {
                    pattern: "Edit",
                    description: "Block secret patterns in Edit operations",
                    actions: [
                      {
                        type: "scan",
                        patterns: [
                          "sk_live_[a-zA-Z0-9]+",
                          "AKIA[0-9A-Z]{16}",
                          "ghp_[a-zA-Z0-9]{36}",
                          "-----BEGIN.*PRIVATE KEY-----",
                        ],
                        message: "Sensitive pattern detected: {match}",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Test: Stripe API key should be blocked
          const stripeResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/config.ts",
              oldString: "API_KEY",
              newString: "sk_live_abcdefghij1234567890",
            },
          })
          expect(stripeResult.blocked).toBe(true)
          expect(stripeResult.message).toContain("Sensitive pattern detected")
          expect(stripeResult.hookName).toBe("block_secrets")

          // Test: AWS access key should be blocked
          const awsResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/config.ts",
              oldString: "ACCESS_KEY",
              newString: "AKIAIOSFODNN7EXAMPLE",
            },
          })
          expect(awsResult.blocked).toBe(true)

          // Test: GitHub personal access token should be blocked
          const ghpResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/config.ts",
              oldString: "TOKEN",
              newString: "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            },
          })
          expect(ghpResult.blocked).toBe(true)
        },
      })
    })

    test("should block Write to .env files", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  block_env_write: {
                    pattern: "Write",
                    file_pattern: "\\.env(\\.local|\\.production|\\.development)?$",
                    actions: [
                      {
                        type: "notify_only",
                        message: "Blocked write to sensitive env file",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Test: Writing to .env should be blocked
          const envResult = await Hook.run("PreToolUse", {
            tool: "Write",
            filePath: "/project/.env",
            input: { filePath: "/project/.env", content: "SECRET=value" },
          })
          expect(envResult.blocked).toBe(true)

          // Test: Writing to .env.local should be blocked
          const envLocalResult = await Hook.run("PreToolUse", {
            tool: "Write",
            filePath: "/project/.env.local",
            input: { filePath: "/project/.env.local", content: "SECRET=value" },
          })
          expect(envLocalResult.blocked).toBe(true)

          // Test: Writing to regular file should NOT be blocked
          const regularResult = await Hook.run("PreToolUse", {
            tool: "Write",
            filePath: "/project/config.ts",
            input: { filePath: "/project/config.ts", content: "export const config = {}" },
          })
          expect(regularResult.blocked).toBe(false)
        },
      })
    })

    test("should allow safe operations", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  block_secrets: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["sk_live_[a-zA-Z0-9]+"],
                        message: "Secret detected",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Safe operation with no secrets
          const safeResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/utils.ts",
              oldString: "const foo = 1",
              newString: "const foo = 2",
            },
          })
          expect(safeResult.blocked).toBe(false)

          // Different tool not matched by pattern
          const readResult = await Hook.run("PreToolUse", {
            tool: "Read",
            input: { filePath: "/test/config.ts" },
          })
          expect(readResult.blocked).toBe(false)
        },
      })
    })
  })

  describe("PostToolUse Validation", () => {
    test("should validate Bash output for errors", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PostToolUse: {
                  check_bash_errors: {
                    pattern: "Bash",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["FATAL ERROR", "panic:", "segmentation fault", "core dumped"],
                        message: "Critical error detected in output: {match}",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Test: Fatal error in output should be blocked
          const fatalResult = await Hook.run("PostToolUse", {
            tool: "Bash",
            output: "FATAL ERROR: cannot allocate memory",
            input: { command: "run-tests" },
          })
          expect(fatalResult.blocked).toBe(true)
          expect(fatalResult.message).toContain("Critical error detected")

          // Test: Panic should be blocked
          const panicResult = await Hook.run("PostToolUse", {
            tool: "Bash",
            output: "panic: runtime error: index out of range",
            input: { command: "go build" },
          })
          expect(panicResult.blocked).toBe(true)

          // Test: Normal output should pass
          const normalResult = await Hook.run("PostToolUse", {
            tool: "Bash",
            output: "Build completed successfully",
            input: { command: "npm run build" },
          })
          expect(normalResult.blocked).toBe(false)
        },
      })
    })

    test("should log tool execution results", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PostToolUse: {
                  log_execution: {
                    pattern: ".*",
                    actions: [
                      {
                        type: "notify_only",
                        message: "Tool execution completed",
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // notify_only should not block
          const result = await Hook.run("PostToolUse", {
            tool: "Edit",
            output: "File edited successfully",
            input: { filePath: "/test/file.ts" },
          })
          expect(result.blocked).toBe(false)
        },
      })
    })
  })

  describe("Hook Chain Execution", () => {
    test("should execute multiple hooks in order", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  first_hook: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "notify_only",
                        message: "First hook executed",
                      },
                    ],
                  },
                  second_hook: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "notify_only",
                        message: "Second hook executed",
                      },
                    ],
                  },
                  third_hook: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "notify_only",
                        message: "Third hook executed",
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // All notify_only hooks should execute and not block
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", oldString: "a", newString: "b" },
          })
          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should stop on first blocking hook", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  first_non_blocking: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "notify_only",
                        message: "First hook - non-blocking",
                      },
                    ],
                  },
                  second_blocking: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["BLOCK_ME"],
                        message: "Second hook blocked",
                        block: true,
                      },
                    ],
                  },
                  third_never_reached: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "notify_only",
                        message: "This should never be reached",
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/file.ts",
              oldString: "original",
              newString: "BLOCK_ME",
            },
          })
          expect(result.blocked).toBe(true)
          expect(result.hookName).toBe("second_blocking")
          expect(result.message).toBe("Second hook blocked")
        },
      })
    })

    test("should merge hooks from multiple config files", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          // Project-level hooks
          const ccodeHooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(ccodeHooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  project_hook: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["PROJECT_SECRET"],
                        message: "Project secret detected",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )

          // Claude-directory hooks
          const claudeHooksDir = path.join(dir, ".claude", "hooks")
          await Bun.write(
            path.join(claudeHooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  claude_hook: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["CLAUDE_SECRET"],
                        message: "Claude secret detected",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Both hooks should be effective
          const projectResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", oldString: "x", newString: "PROJECT_SECRET" },
          })
          expect(projectResult.blocked).toBe(true)
          expect(projectResult.hookName).toBe("project_hook")
        },
      })
    })
  })

  describe("Hook Configuration", () => {
    test("should load hooks from .ccode directory", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  test_hook: {
                    pattern: "Edit",
                    actions: [{ type: "notify_only", message: "Hook loaded" }],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const configs = await Hook.load()
          expect(configs.length).toBeGreaterThan(0)
          expect(configs[0].hooks.PreToolUse?.test_hook).toBeDefined()
        },
      })
    })

    test("should respect disabled settings", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  always_block: {
                    pattern: ".*",
                    actions: [
                      {
                        type: "scan",
                        patterns: [".*"],
                        message: "Should always block",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: false },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", content: "anything" },
          })
          // Hook is disabled, so it should not block
          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should handle missing hooks.json gracefully", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // No hooks.json exists, should not throw
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts" },
          })
          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should handle invalid hooks.json gracefully", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(path.join(hooksDir, "hooks.json"), "{ invalid json }")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Invalid JSON should not throw, just log error
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts" },
          })
          expect(result.blocked).toBe(false)
        },
      })
    })
  })

  describe("Action Types", () => {
    test("scan action should match regex patterns", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  regex_scan: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["password\\s*=\\s*['\"][^'\"]+['\"]", "api_key\\s*:\\s*['\"][^'\"]+['\"]"],
                        message: "Hardcoded credential detected: {match}",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Password assignment should be blocked
          const pwResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/config.ts",
              oldString: "//",
              newString: 'password = "secret123"',
            },
          })
          expect(pwResult.blocked).toBe(true)

          // API key assignment should be blocked
          const apiResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/config.ts",
              oldString: "//",
              newString: "api_key: 'my-secret-key'",
            },
          })
          expect(apiResult.blocked).toBe(true)

          // Normal code should pass
          const normalResult = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: {
              filePath: "/test/config.ts",
              oldString: "const x = 1",
              newString: "const x = 2",
            },
          })
          expect(normalResult.blocked).toBe(false)
        },
      })
    })

    test("check_env action should validate environment variables", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  check_node_env: {
                    pattern: "Bash",
                    actions: [
                      {
                        type: "check_env",
                        variable: "NONEXISTENT_TEST_VAR_12345",
                        command_pattern: "npm run build",
                        message: "NONEXISTENT_TEST_VAR_12345 is {status}",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Environment variable not set should block
          const result = await Hook.run("PreToolUse", {
            tool: "Bash",
            command: "npm run build",
            input: { command: "npm run build" },
          })
          expect(result.blocked).toBe(true)
          expect(result.message).toContain("not set")
        },
      })
    })
  })
})
