import { describe, test, expect, mock, beforeEach, afterEach } from "bun:test"
import {
  HandsBridge,
  createBridge,
  HandsApiError,
  type HandConfig,
  type HandExecution,
  type TriggerResponse,
  HandConfigSchema,
  HandExecutionSchema,
} from "@/autonomous/hands"

describe("hands-bridge", () => {
  describe("HandsBridge", () => {
    let bridge: HandsBridge

    beforeEach(() => {
      bridge = createBridge({ baseUrl: "http://localhost:4432" })
    })

    describe("constructor", () => {
      test("creates bridge with default URL", () => {
        const defaultBridge = createBridge()
        expect(defaultBridge).toBeDefined()
      })

      test("creates bridge with custom URL", () => {
        const customBridge = createBridge({ baseUrl: "http://custom:9999" })
        expect(customBridge).toBeDefined()
      })

      test("trims trailing slash from URL", () => {
        const bridge = createBridge({ baseUrl: "http://localhost:4432/" })
        // Internal URL should not have trailing slash
        expect(bridge).toBeDefined()
      })

      test("accepts custom timeout", () => {
        const bridge = createBridge({ timeoutMs: 60000 })
        expect(bridge).toBeDefined()
      })
    })

    describe("HandsApiError", () => {
      test("creates error with status and body", () => {
        const error = new HandsApiError(404, "Not found", "/api/v1/hands/test")

        expect(error.status).toBe(404)
        expect(error.body).toBe("Not found")
        expect(error.url).toBe("/api/v1/hands/test")
        expect(error.name).toBe("HandsApiError")
        expect(error.message).toContain("404")
        expect(error.message).toContain("Not found")
      })

      test("error is instanceof Error", () => {
        const error = new HandsApiError(500, "Internal error", "/test")
        expect(error instanceof Error).toBe(true)
        expect(error instanceof HandsApiError).toBe(true)
      })
    })
  })

  describe("schemas", () => {
    describe("HandConfigSchema", () => {
      test("parses valid hand config", () => {
        const input = {
          id: "test-hand",
          name: "Test Hand",
          agent: "macro",
          enabled: true,
        }

        const result = HandConfigSchema.parse(input)

        expect(result.id).toBe("test-hand")
        expect(result.name).toBe("Test Hand")
        expect(result.agent).toBe("macro")
        expect(result.enabled).toBe(true)
        expect(result.version).toBe("1.0.0") // default
      })

      test("applies defaults", () => {
        const input = {
          id: "minimal",
          name: "Minimal",
          agent: "explore",
        }

        const result = HandConfigSchema.parse(input)

        expect(result.version).toBe("1.0.0")
        expect(result.description).toBe("")
        expect(result.enabled).toBe(true)
      })

      test("accepts optional fields", () => {
        const input = {
          id: "full-config",
          name: "Full Config",
          version: "2.0.0",
          description: "A full configuration",
          schedule: "0 */30 * * * *",
          agent: "trader",
          enabled: false,
          memoryPath: "hands/test/{date}.md",
          params: { threshold: 0.7 },
        }

        const result = HandConfigSchema.parse(input)

        expect(result.schedule).toBe("0 */30 * * * *")
        expect(result.memoryPath).toBe("hands/test/{date}.md")
        expect(result.params).toEqual({ threshold: 0.7 })
      })
    })

    describe("HandExecutionSchema", () => {
      test("parses valid execution", () => {
        const input = {
          id: "exec-001",
          handId: "test-hand",
          status: "completed",
          startedAt: 1709100000000,
          completedAt: 1709100060000,
          durationMs: 60000,
          output: "Task completed",
          success: true,
          qualityScore: 85,
          tokensUsed: 1000,
          costUsd: 0.05,
        }

        const result = HandExecutionSchema.parse(input)

        expect(result.id).toBe("exec-001")
        expect(result.status).toBe("completed")
        expect(result.success).toBe(true)
        expect(result.qualityScore).toBe(85)
      })

      test("accepts minimal execution", () => {
        const input = {
          id: "exec-002",
          handId: "test-hand",
          status: "pending",
          startedAt: 1709100000000,
        }

        const result = HandExecutionSchema.parse(input)

        expect(result.id).toBe("exec-002")
        expect(result.status).toBe("pending")
        expect(result.completedAt).toBeUndefined()
        expect(result.success).toBeUndefined()
      })

      test("validates status enum", () => {
        const validStatuses = ["pending", "running", "completed", "failed", "paused"]

        for (const status of validStatuses) {
          const input = {
            id: "test",
            handId: "hand",
            status,
            startedAt: Date.now(),
          }

          const result = HandExecutionSchema.parse(input)
          expect(result.status).toBe(status)
        }
      })

      test("rejects invalid status", () => {
        const input = {
          id: "test",
          handId: "hand",
          status: "invalid",
          startedAt: Date.now(),
        }

        expect(() => HandExecutionSchema.parse(input)).toThrow()
      })
    })
  })

  describe("types", () => {
    test("HandConfig type is compatible with schema", () => {
      const config: HandConfig = {
        id: "type-test",
        name: "Type Test",
        version: "1.0.0",
        description: "Testing types",
        agent: "build",
        enabled: true,
        schedule: "0 * * * *",
        autonomy: {
          level: "bold",
          unattended: true,
          maxIterations: 5,
        },
        resources: {
          maxTokens: 10000,
          maxCostUsd: 1.0,
          maxDurationSec: 300,
        },
      }

      expect(config.id).toBe("type-test")
      expect(config.autonomy?.level).toBe("bold")
    })

    test("HandExecution type is compatible with schema", () => {
      const execution: HandExecution = {
        id: "exec-type-test",
        handId: "test-hand",
        status: "running",
        startedAt: Date.now(),
      }

      expect(execution.status).toBe("running")
    })

    test("TriggerResponse type structure", () => {
      const response: TriggerResponse = {
        success: true,
        executionId: "exec-123",
        message: "Hand triggered successfully",
      }

      expect(response.success).toBe(true)
      expect(response.executionId).toBe("exec-123")
    })
  })

  describe("autonomy levels", () => {
    test("all autonomy levels are valid", () => {
      const levels = ["lunatic", "insane", "crazy", "wild", "bold", "timid"] as const

      for (const level of levels) {
        const config: HandConfig = {
          id: `level-${level}`,
          name: `Level ${level}`,
          agent: "explore",
          enabled: true,
          version: "1.0.0",
          description: "",
          autonomy: {
            level,
            unattended: true,
            maxIterations: 3,
          },
        }

        expect(config.autonomy?.level).toBe(level)
      }
    })
  })

  describe("risk thresholds", () => {
    test("all risk thresholds are valid", () => {
      const thresholds = ["safe", "low", "medium", "high"] as const

      for (const threshold of thresholds) {
        const config: HandConfig = {
          id: `risk-${threshold}`,
          name: `Risk ${threshold}`,
          agent: "build",
          enabled: true,
          version: "1.0.0",
          description: "",
          autonomy: {
            level: "bold",
            unattended: true,
            maxIterations: 5,
            autoApprove: {
              enabled: true,
              allowedTools: ["Read", "Glob"],
              riskThreshold: threshold,
              timeoutMs: 30000,
            },
          },
        }

        expect(config.autonomy?.autoApprove?.riskThreshold).toBe(threshold)
      }
    })
  })
})
