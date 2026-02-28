import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import {
  assessToolRisk,
  createAutoApproveHandler,
  createSafeOnlyConfig,
  createPermissiveConfig,
  clearAuditLog,
  getAuditLog,
  compareRisk,
  riskAtOrBelowThreshold,
  parseRiskLevel,
  getAutoApproveFromEnv,
  evaluateAdaptiveRisk,
  adaptiveRiskAllowsApproval,
  createAdaptiveAutoApproveHandler,
  ENV_VARS,
  type RiskLevel,
  type AutoApproveConfig,
  type ExecutionContext,
} from "@/permission/auto-approve"
import type { Permission } from "@/permission"

describe("auto-approve", () => {
  beforeEach(() => {
    clearAuditLog()
  })

  describe("assessToolRisk", () => {
    test("Read tool is safe", () => {
      const result = assessToolRisk("Read", { file_path: "/some/file.ts" })
      expect(result.risk).toBe("safe")
      expect(result.autoApprovable).toBe(true)
    })

    test("Glob tool is safe", () => {
      const result = assessToolRisk("Glob", { pattern: "**/*.ts" })
      expect(result.risk).toBe("safe")
    })

    test("WebFetch tool is low risk", () => {
      const result = assessToolRisk("WebFetch", { url: "https://example.com" })
      expect(result.risk).toBe("low")
    })

    test("Write tool is medium risk", () => {
      const result = assessToolRisk("Write", { file_path: "/some/file.ts" })
      expect(result.risk).toBe("medium")
    })

    test("Bash with echo is low risk", () => {
      const result = assessToolRisk("Bash", { command: "echo hello" })
      expect(result.risk).toBe("low")
    })

    test("Bash with unknown command is high risk by default", () => {
      const result = assessToolRisk("Bash", { command: "some-unknown-command --flag" })
      expect(result.risk).toBe("high")
    })

    test("Bash with sudo is critical", () => {
      const result = assessToolRisk("Bash", { command: "sudo apt-get install vim" })
      expect(result.risk).toBe("critical")
      expect(result.autoApprovable).toBe(false)
    })

    test("Bash with rm -rf is high risk for non-root paths", () => {
      const result = assessToolRisk("Bash", { command: "rm -rf /tmp" })
      expect(result.risk).toBe("high")
    })

    test("Bash with rm -rf / is critical", () => {
      const rootResult = assessToolRisk("Bash", { command: "rm -rf /" })
      expect(rootResult.risk).toBe("critical")
      expect(rootResult.autoApprovable).toBe(false)
    })

    test("Bash with git push is high risk", () => {
      const result = assessToolRisk("Bash", { command: "git push origin main" })
      expect(result.risk).toBe("high")
    })

    test("Bash with git push --force is critical", () => {
      const result = assessToolRisk("Bash", { command: "git push --force origin main" })
      expect(result.risk).toBe("critical")
    })

    test("Bash with git status is low risk", () => {
      const result = assessToolRisk("Bash", { command: "git status" })
      expect(result.risk).toBe("low")
    })

    test("Write to .env file is high risk", () => {
      const result = assessToolRisk("Write", { file_path: "/project/.env" })
      expect(result.risk).toBe("high")
    })

    test("Write to system directory is high risk", () => {
      const result = assessToolRisk("Write", { file_path: "/etc/passwd" })
      expect(result.risk).toBe("high")
    })

    test("Edit package.json is medium risk", () => {
      const result = assessToolRisk("Edit", { file_path: "/project/package.json" })
      expect(result.risk).toBe("medium")
    })

    test("Unknown tool defaults to medium", () => {
      const result = assessToolRisk("UnknownTool", {})
      expect(result.risk).toBe("medium")
    })
  })

  describe("risk comparison", () => {
    test("compareRisk orders correctly", () => {
      expect(compareRisk("safe", "low")).toBeLessThan(0)
      expect(compareRisk("low", "medium")).toBeLessThan(0)
      expect(compareRisk("medium", "high")).toBeLessThan(0)
      expect(compareRisk("high", "critical")).toBeLessThan(0)
      expect(compareRisk("critical", "safe")).toBeGreaterThan(0)
    })

    test("riskAtOrBelowThreshold works correctly", () => {
      expect(riskAtOrBelowThreshold("safe", "medium")).toBe(true)
      expect(riskAtOrBelowThreshold("low", "medium")).toBe(true)
      expect(riskAtOrBelowThreshold("medium", "medium")).toBe(true)
      expect(riskAtOrBelowThreshold("high", "medium")).toBe(false)
      expect(riskAtOrBelowThreshold("critical", "medium")).toBe(false)
    })

    test("parseRiskLevel handles valid levels", () => {
      expect(parseRiskLevel("safe")).toBe("safe")
      expect(parseRiskLevel("LOW")).toBe("low")
      expect(parseRiskLevel("Medium")).toBe("medium")
      expect(parseRiskLevel("HIGH")).toBe("high")
      expect(parseRiskLevel("CRITICAL")).toBe("critical")
    })

    test("parseRiskLevel defaults to medium for unknown", () => {
      expect(parseRiskLevel("unknown")).toBe("medium")
      expect(parseRiskLevel("")).toBe("medium")
    })
  })

  describe("createAutoApproveHandler", () => {
    const createMockPermissionInfo = (tool: string, metadata: Record<string, unknown> = {}): Permission.Info => ({
      id: "test-permission-id",
      type: tool,
      sessionID: "test-session",
      messageID: "test-message",
      message: `Permission for ${tool}`,
      metadata,
      time: { created: Date.now() },
    })

    test("disabled config always rejects", async () => {
      const config: AutoApproveConfig = {
        enabled: false,
        allowedTools: ["Read", "Glob"],
        riskThreshold: "medium",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      const result = await handler(createMockPermissionInfo("Read"))
      expect(result).toBe("reject")
      expect(getAuditLog()).toHaveLength(1)
      expect(getAuditLog()[0].reason).toBe("Auto-approve disabled")
    })

    test("critical operations always rejected", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: ["Bash"],
        riskThreshold: "high",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      const result = await handler(createMockPermissionInfo("Bash", { command: "sudo rm -rf /" }))
      expect(result).toBe("reject")
      expect(getAuditLog()[0].decision).toBe("rejected")
      expect(getAuditLog()[0].risk).toBe("critical")
    })

    test("whitelisted safe tools approved", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: ["Read", "Glob", "Grep"],
        riskThreshold: "low",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      const result = await handler(createMockPermissionInfo("Read", { file_path: "/some/file.ts" }))
      expect(result).toBe("once")
      expect(getAuditLog()[0].decision).toBe("approved")
    })

    test("non-whitelisted tools rejected", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: ["Read"],
        riskThreshold: "medium",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      const result = await handler(createMockPermissionInfo("Write", { file_path: "/some/file.ts" }))
      expect(result).toBe("reject")
    })

    test("empty whitelist uses risk-based evaluation only", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: [],
        riskThreshold: "medium",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      // Safe tool should be approved (empty whitelist = any tool)
      const readResult = await handler(createMockPermissionInfo("Read", { file_path: "/some/file.ts" }))
      expect(readResult).toBe("once")

      // High risk tool should be rejected (exceeds medium threshold)
      clearAuditLog()
      const bashResult = await handler(createMockPermissionInfo("Bash", { command: "git push" }))
      expect(bashResult).toBe("reject")
    })

    test("tools above risk threshold rejected", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: ["Bash"],
        riskThreshold: "low",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      // Bash with git push is high risk, exceeds low threshold
      const result = await handler(createMockPermissionInfo("Bash", { command: "git push origin main" }))
      expect(result).toBe("reject")
    })

    test("timeout auto-approval in unattended mode", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: [],
        riskThreshold: "low",
        timeoutMs: 50, // Short timeout for test
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      const start = Date.now()
      // Write is medium risk, exceeds low threshold, but timeout will approve
      const result = await handler(createMockPermissionInfo("Write", { file_path: "/some/file.ts" }))
      const elapsed = Date.now() - start

      expect(result).toBe("once")
      expect(elapsed).toBeGreaterThanOrEqual(50)
      expect(getAuditLog()[0].decision).toBe("timeout_approved")
    })

    test("no timeout approval when timeoutMs is 0", async () => {
      const config: AutoApproveConfig = {
        enabled: true,
        allowedTools: [],
        riskThreshold: "low",
        timeoutMs: 0,
        unattended: true,
      }
      const handler = createAutoApproveHandler(config)

      // Medium risk, exceeds threshold, no timeout
      const result = await handler(createMockPermissionInfo("Write", { file_path: "/some/file.ts" }))
      expect(result).toBe("reject")
    })
  })

  describe("preset configs", () => {
    test("createSafeOnlyConfig has correct defaults", () => {
      const config = createSafeOnlyConfig()
      expect(config.enabled).toBe(true)
      expect(config.allowedTools).toContain("Read")
      expect(config.allowedTools).toContain("Glob")
      expect(config.allowedTools).toContain("WebFetch")
      expect(config.riskThreshold).toBe("low")
      expect(config.timeoutMs).toBe(0)
      expect(config.unattended).toBe(true)
    })

    test("createPermissiveConfig has correct defaults", () => {
      const config = createPermissiveConfig()
      expect(config.enabled).toBe(true)
      expect(config.allowedTools).toHaveLength(0)
      expect(config.riskThreshold).toBe("medium")
      expect(config.timeoutMs).toBe(30000)
      expect(config.unattended).toBe(true)
    })
  })

  describe("audit log", () => {
    test("audit log records decisions", async () => {
      const config = createSafeOnlyConfig()
      const handler = createAutoApproveHandler(config)

      await handler({
        id: "perm-1",
        type: "Read",
        sessionID: "session-1",
        messageID: "msg-1",
        message: "Read file",
        metadata: { file_path: "/some/file.ts" },
        time: { created: Date.now() },
      })

      const log = getAuditLog()
      expect(log).toHaveLength(1)
      expect(log[0].permissionId).toBe("perm-1")
      expect(log[0].tool).toBe("Read")
      expect(log[0].decision).toBe("approved")
      expect(log[0].risk).toBe("safe")
    })

    test("clearAuditLog works", async () => {
      const config = createSafeOnlyConfig()
      const handler = createAutoApproveHandler(config)

      await handler({
        id: "perm-1",
        type: "Read",
        sessionID: "session-1",
        messageID: "msg-1",
        message: "Read file",
        metadata: {},
        time: { created: Date.now() },
      })

      expect(getAuditLog()).toHaveLength(1)
      clearAuditLog()
      expect(getAuditLog()).toHaveLength(0)
    })
  })

  describe("environment variables", () => {
    const originalEnv = { ...process.env }

    afterEach(() => {
      // Restore original environment
      delete process.env[ENV_VARS.ENABLED]
      delete process.env[ENV_VARS.THRESHOLD]
      delete process.env[ENV_VARS.TOOLS]
      delete process.env[ENV_VARS.TIMEOUT]
    })

    test("returns undefined when CODECODER_AUTO_APPROVE is not set", () => {
      delete process.env[ENV_VARS.ENABLED]
      const result = getAutoApproveFromEnv()
      expect(result).toBeUndefined()
    })

    test("returns undefined when CODECODER_AUTO_APPROVE is false", () => {
      process.env[ENV_VARS.ENABLED] = "false"
      const result = getAutoApproveFromEnv()
      expect(result).toBeUndefined()
    })

    test("returns config when CODECODER_AUTO_APPROVE is true", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      const result = getAutoApproveFromEnv()
      expect(result).toBeDefined()
      expect(result?.enabled).toBe(true)
      expect(result?.riskThreshold).toBe("low") // default
    })

    test("returns config when CODECODER_AUTO_APPROVE is 1", () => {
      process.env[ENV_VARS.ENABLED] = "1"
      const result = getAutoApproveFromEnv()
      expect(result).toBeDefined()
      expect(result?.enabled).toBe(true)
    })

    test("parses CODECODER_AUTO_APPROVE_THRESHOLD correctly", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.THRESHOLD] = "medium"
      const result = getAutoApproveFromEnv()
      expect(result?.riskThreshold).toBe("medium")
    })

    test("downgrades critical threshold to high for safety", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.THRESHOLD] = "critical"
      const result = getAutoApproveFromEnv()
      expect(result?.riskThreshold).toBe("high")
    })

    test("parses CODECODER_AUTO_APPROVE_TOOLS correctly", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.TOOLS] = "Read,Glob,Grep"
      const result = getAutoApproveFromEnv()
      expect(result?.allowedTools).toEqual(["Read", "Glob", "Grep"])
    })

    test("parses CODECODER_AUTO_APPROVE_TOOLS with spaces", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.TOOLS] = "Read, Glob , Grep"
      const result = getAutoApproveFromEnv()
      expect(result?.allowedTools).toEqual(["Read", "Glob", "Grep"])
    })

    test("parses CODECODER_AUTO_APPROVE_TIMEOUT correctly", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.TIMEOUT] = "5000"
      const result = getAutoApproveFromEnv()
      expect(result?.timeoutMs).toBe(5000)
    })

    test("ignores invalid timeout value", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.TIMEOUT] = "not-a-number"
      const result = getAutoApproveFromEnv()
      expect(result?.timeoutMs).toBeUndefined()
    })

    test("handles full config from environment", () => {
      process.env[ENV_VARS.ENABLED] = "true"
      process.env[ENV_VARS.THRESHOLD] = "low"
      process.env[ENV_VARS.TOOLS] = "Read,Glob"
      process.env[ENV_VARS.TIMEOUT] = "1000"

      const result = getAutoApproveFromEnv()
      expect(result).toEqual({
        enabled: true,
        riskThreshold: "low",
        allowedTools: ["Read", "Glob"],
        timeoutMs: 1000,
      })
    })
  })

  describe("adaptive risk assessment", () => {
    const createContext = (overrides: Partial<ExecutionContext> = {}): ExecutionContext => ({
      sessionId: "test-session",
      iteration: 1,
      errors: 0,
      successes: 10,
      ...overrides,
    })

    describe("evaluateAdaptiveRisk", () => {
      test("returns base risk for normal context", () => {
        const ctx = createContext()
        const result = evaluateAdaptiveRisk("Read", { file_path: "/some/file.ts" }, ctx)

        expect(result.baseRisk).toBe("safe")
        expect(result.contextFactors).toBeDefined()
        expect(result.adjustmentReason).toBeDefined()
      })

      test("decreases risk for high success rate with no errors", () => {
        const ctx = createContext({ successes: 100, errors: 0 })
        const result = evaluateAdaptiveRisk("Write", { file_path: "/some/file.ts" }, ctx)

        // Write is medium risk, should decrease to low
        expect(result.baseRisk).toBe("medium")
        expect(result.adjustment).toBeLessThan(0)
        expect(result.adjustedRisk).toBe("low")
      })

      test("increases risk when errors in session", () => {
        const ctx = createContext({ successes: 5, errors: 1 })
        const result = evaluateAdaptiveRisk("Read", { file_path: "/some/file.ts" }, ctx)

        // Should increase risk due to errors
        expect(result.baseRisk).toBe("safe")
        expect(result.adjustment).toBeGreaterThan(0)
        expect(result.adjustedRisk).toBe("low")
      })

      test("increases risk significantly for multiple errors", () => {
        const ctx = createContext({ successes: 5, errors: 5 })
        const result = evaluateAdaptiveRisk("Read", { file_path: "/some/file.ts" }, ctx)

        // Multiple errors should increase risk more
        expect(result.adjustment).toBeGreaterThan(1)
      })

      test("increases risk for production environment", () => {
        const ctx = createContext({ isProduction: true })
        const result = evaluateAdaptiveRisk("Read", { file_path: "/some/file.ts" }, ctx)

        expect(result.contextFactors.projectSensitivity).toBe("high")
        expect(result.adjustment).toBeGreaterThan(0)
      })

      test("detects high sensitivity from project path", () => {
        const ctx = createContext({ projectPath: "/app/production/api" })
        const result = evaluateAdaptiveRisk("Read", { file_path: "/some/file.ts" }, ctx)

        expect(result.contextFactors.projectSensitivity).toBe("high")
      })

      test("detects medium sensitivity from staging path", () => {
        const ctx = createContext({ projectPath: "/app/staging/api" })
        const result = evaluateAdaptiveRisk("Read", { file_path: "/some/file.ts" }, ctx)

        expect(result.contextFactors.projectSensitivity).toBe("medium")
      })

      test("includes success rate in context factors", () => {
        const ctx = createContext({ successes: 8, errors: 2 })
        const result = evaluateAdaptiveRisk("Read", {}, ctx)

        expect(result.contextFactors.successRate).toBe(0.8)
      })

      test("handles zero operations (no history)", () => {
        const ctx = createContext({ successes: 0, errors: 0 })
        const result = evaluateAdaptiveRisk("Read", {}, ctx)

        // Should default to 1.0 (assume good)
        expect(result.contextFactors.successRate).toBe(1.0)
      })
    })

    describe("adaptiveRiskAllowsApproval", () => {
      test("allows approval when adjusted risk below threshold", () => {
        const config = {
          baseRisk: "safe" as RiskLevel,
          contextFactors: {
            projectSensitivity: "low" as const,
            timeOfDay: "business" as const,
            successRate: 1.0,
            sessionErrorCount: 0,
            sessionIterations: 5,
            unattended: false,
          },
          adjustedRisk: "safe" as RiskLevel,
          adjustment: 0,
          adjustmentReason: "No adjustment",
        }

        expect(adaptiveRiskAllowsApproval(config, "medium")).toBe(true)
      })

      test("rejects when adjusted risk above threshold", () => {
        const config = {
          baseRisk: "medium" as RiskLevel,
          contextFactors: {
            projectSensitivity: "high" as const,
            timeOfDay: "after_hours" as const,
            successRate: 0.5,
            sessionErrorCount: 3,
            sessionIterations: 10,
            unattended: true,
          },
          adjustedRisk: "high" as RiskLevel,
          adjustment: 2,
          adjustmentReason: "Multiple errors",
        }

        expect(adaptiveRiskAllowsApproval(config, "low")).toBe(false)
      })

      test("always rejects critical risk", () => {
        const config = {
          baseRisk: "high" as RiskLevel,
          contextFactors: {
            projectSensitivity: "high" as const,
            timeOfDay: "after_hours" as const,
            successRate: 0.5,
            sessionErrorCount: 5,
            sessionIterations: 10,
            unattended: true,
          },
          adjustedRisk: "critical" as RiskLevel,
          adjustment: 3,
          adjustmentReason: "Multiple factors",
        }

        expect(adaptiveRiskAllowsApproval(config, "high")).toBe(false)
      })
    })

    describe("createAdaptiveAutoApproveHandler", () => {
      test("creates handler that respects adaptive risk", () => {
        const config: AutoApproveConfig = {
          enabled: true,
          allowedTools: ["Read", "Glob"],
          riskThreshold: "low",
          timeoutMs: 0,
          unattended: true,
        }
        const ctx = createContext({ successes: 100, errors: 0 })

        const handler = createAdaptiveAutoApproveHandler(config, ctx)

        // Read should be approved (safe risk)
        const result = handler("Read", { file_path: "/some/file.ts" })
        expect(result).toBe("once")
      })

      test("handler rejects when disabled", () => {
        const config: AutoApproveConfig = {
          enabled: false,
          allowedTools: ["Read"],
          riskThreshold: "medium",
          timeoutMs: 0,
          unattended: true,
        }
        const ctx = createContext()

        const handler = createAdaptiveAutoApproveHandler(config, ctx)
        const result = handler("Read", {})

        expect(result).toBeUndefined()
      })

      test("handler respects tool whitelist", () => {
        const config: AutoApproveConfig = {
          enabled: true,
          allowedTools: ["Read"],
          riskThreshold: "medium",
          timeoutMs: 0,
          unattended: true,
        }
        const ctx = createContext()

        const handler = createAdaptiveAutoApproveHandler(config, ctx)

        // Glob not in whitelist
        const result = handler("Glob", {})
        expect(result).toBeUndefined()
      })

      test("handler increases scrutiny with errors", () => {
        const config: AutoApproveConfig = {
          enabled: true,
          allowedTools: ["Write"],
          riskThreshold: "medium",
          timeoutMs: 0,
          unattended: true,
        }
        const ctxWithErrors = createContext({ errors: 5 })

        const handler = createAdaptiveAutoApproveHandler(config, ctxWithErrors)

        // Write is medium risk, but errors should push it higher
        const result = handler("Write", { file_path: "/some/file.ts" })
        expect(result).toBeUndefined() // Rejected due to increased risk
      })
    })
  })
})
