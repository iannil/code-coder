import { describe, test, expect, beforeEach } from "bun:test"
import {
  SandboxIntegrationManager,
  createSandboxManager,
  shouldSandboxTool,
  getToolSandboxPolicy,
  type ToolSandboxPolicy,
  type ToolExecutionConfig,
} from "@/tool/sandbox-integration"

describe("sandbox-integration", () => {
  describe("SandboxIntegrationManager", () => {
    let manager: SandboxIntegrationManager

    beforeEach(() => {
      manager = createSandboxManager()
    })

    describe("getPolicy", () => {
      test("returns policy for Read tool", () => {
        const policy = manager.getPolicy("Read")
        expect(policy).toBeDefined()
        expect(policy?.backend).toBe("process")
        expect(policy?.limits.fileSystemAccess).toBe("readonly")
      })

      test("returns policy for Bash tool", () => {
        const policy = manager.getPolicy("Bash")
        expect(policy).toBeDefined()
        expect(policy?.backend).toBe("docker")
        expect(policy?.limits.memoryMB).toBe(512)
      })

      test("returns policy for WebFetch tool", () => {
        const policy = manager.getPolicy("WebFetch")
        expect(policy).toBeDefined()
        expect(policy?.backend).toBe("wasm")
        expect(policy?.limits.networkAccess).toBe(true)
      })

      test("returns null for bypass tools", () => {
        expect(manager.getPolicy("TodoRead")).toBeNull()
        expect(manager.getPolicy("AskUserQuestion")).toBeNull()
        expect(manager.getPolicy("Skill")).toBeNull()
      })

      test("returns null for unknown tools", () => {
        expect(manager.getPolicy("UnknownTool")).toBeNull()
      })
    })

    describe("shouldSandbox", () => {
      test("returns false when disabled", () => {
        expect(manager.shouldSandbox("Bash")).toBe(false)
      })

      test("returns true when enabled for configured tools", async () => {
        await manager.enable()
        expect(manager.shouldSandbox("Bash")).toBe(true)
        expect(manager.shouldSandbox("Write")).toBe(true)
      })

      test("returns false for bypass tools even when enabled", async () => {
        await manager.enable()
        expect(manager.shouldSandbox("TodoRead")).toBe(false)
        expect(manager.shouldSandbox("AskUserQuestion")).toBe(false)
      })
    })

    describe("getRecommendedBackend", () => {
      test("returns process for read-only tools", () => {
        expect(manager.getRecommendedBackend("Read")).toBe("process")
        expect(manager.getRecommendedBackend("Glob")).toBe("process")
        expect(manager.getRecommendedBackend("Grep")).toBe("process")
      })

      test("returns docker for file modification tools", () => {
        expect(manager.getRecommendedBackend("Write")).toBe("docker")
        expect(manager.getRecommendedBackend("Edit")).toBe("docker")
        expect(manager.getRecommendedBackend("Bash")).toBe("docker")
      })

      test("returns wasm for network tools", () => {
        expect(manager.getRecommendedBackend("WebFetch")).toBe("wasm")
        expect(manager.getRecommendedBackend("WebSearch")).toBe("wasm")
      })

      test("returns default for unknown tools", () => {
        expect(manager.getRecommendedBackend("UnknownTool")).toBe("auto")
      })
    })

    describe("enable/disable", () => {
      test("can enable sandbox", async () => {
        expect(manager.isEnabled()).toBe(false)
        await manager.enable()
        expect(manager.isEnabled()).toBe(true)
      })

      test("can disable sandbox", async () => {
        await manager.enable()
        expect(manager.isEnabled()).toBe(true)
        manager.disable()
        expect(manager.isEnabled()).toBe(false)
      })
    })

    describe("updateConfig", () => {
      test("can update configuration", () => {
        manager.updateConfig({ enabled: true })
        expect(manager.isEnabled()).toBe(true)
      })

      test("can add custom policies", () => {
        const customPolicy: ToolSandboxPolicy = {
          backend: "docker",
          limits: { memoryMB: 1024, cpuTimeMs: 60000, networkAccess: true, fileSystemAccess: "full" },
          reason: "Custom tool",
        }

        manager.updateConfig({
          policies: {
            ...manager.getConfig().policies,
            CustomTool: customPolicy,
          },
        })

        const policy = manager.getPolicy("CustomTool")
        expect(policy).toEqual(customPolicy)
      })
    })

    describe("getConfig", () => {
      test("returns readonly configuration", () => {
        const config = manager.getConfig()
        expect(config.defaultBackend).toBe("auto")
        expect(config.enabled).toBe(false)
        expect(Object.keys(config.policies).length).toBeGreaterThan(0)
      })
    })
  })

  describe("convenience functions", () => {
    test("shouldSandboxTool works", () => {
      // Default is disabled
      expect(shouldSandboxTool("Bash")).toBe(false)
    })

    test("getToolSandboxPolicy works", () => {
      const policy = getToolSandboxPolicy("Bash")
      expect(policy).toBeDefined()
      expect(policy?.backend).toBe("docker")
    })
  })

  describe("policy rationale", () => {
    let manager: SandboxIntegrationManager

    beforeEach(() => {
      manager = createSandboxManager()
    })

    test("read-only tools have readonly filesystem access", () => {
      const readOnlyTools = ["Read", "Glob", "Grep", "LS"]

      for (const tool of readOnlyTools) {
        const policy = manager.getPolicy(tool)
        expect(policy?.limits.fileSystemAccess).toBe("readonly")
        expect(policy?.limits.networkAccess).toBe(false)
      }
    })

    test("network tools have network access enabled", () => {
      const networkTools = ["WebFetch", "WebSearch"]

      for (const tool of networkTools) {
        const policy = manager.getPolicy(tool)
        expect(policy?.limits.networkAccess).toBe(true)
        expect(policy?.limits.fileSystemAccess).toBe("none")
      }
    })

    test("file modification tools have restricted filesystem access", () => {
      const fileModTools = ["Write", "Edit", "NotebookEdit"]

      for (const tool of fileModTools) {
        const policy = manager.getPolicy(tool)
        expect(policy?.limits.fileSystemAccess).toBe("restricted")
        expect(policy?.backend).toBe("docker")
      }
    })

    test("Bash has highest resource limits", () => {
      const bashPolicy = manager.getPolicy("Bash")
      const readPolicy = manager.getPolicy("Read")

      expect(bashPolicy?.limits.memoryMB).toBeGreaterThan(readPolicy?.limits.memoryMB ?? 0)
      expect(bashPolicy?.limits.cpuTimeMs).toBeGreaterThan(readPolicy?.limits.cpuTimeMs ?? 0)
    })

    test("Task has highest timeout for long-running operations", () => {
      const taskPolicy = manager.getPolicy("Task")

      expect(taskPolicy?.limits.cpuTimeMs).toBe(600000) // 10 minutes
    })
  })

  describe("bypass tools", () => {
    test("internal tools are bypassed", () => {
      const manager = createSandboxManager()
      const bypassTools = [
        "TodoRead",
        "TodoWrite",
        "AskUserQuestion",
        "EnterPlanMode",
        "ExitPlanMode",
        "Skill",
        "TaskCreate",
        "TaskGet",
        "TaskUpdate",
        "TaskList",
      ]

      for (const tool of bypassTools) {
        expect(manager.getPolicy(tool)).toBeNull()
        expect(manager.shouldSandbox(tool)).toBe(false)
      }
    })
  })
})
