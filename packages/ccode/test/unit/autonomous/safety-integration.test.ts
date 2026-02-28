// @ts-nocheck
import { describe, test, expect, beforeEach } from "bun:test"
import {
  isDestructiveOperation,
  getDestructiveRiskLevel,
  SafetyIntegration,
  type DestructiveCategory,
  type OperationHistory,
} from "@/autonomous/safety/integration"

describe("Safety Integration - Destructive Operation Detection", () => {
  describe("isDestructiveOperation", () => {
    test("should identify Bash operations as destructive", () => {
      const op = { tool: "Bash", input: { command: "rm -rf /" } }
      const result = isDestructiveOperation(op)

      expect(result).not.toBeNull()
      expect(result?.category).toBe("file_deletion")
      expect(result?.reversible).toBe(false)
      expect(result?.riskLevel).toBe("high")
    })

    test("should identify Write operations as destructive", () => {
      const op = { tool: "Write", input: { file_path: "/important/config" } }
      const result = isDestructiveOperation(op)

      expect(result).not.toBeNull()
      expect(result?.category).toBe("file_overwrite")
      expect(result?.riskLevel).toBe("medium")
      expect(result?.files).toEqual(["/important/config"])
    })

    test("should identify Edit operations as destructive", () => {
      const op = { tool: "Edit", input: { file_path: "/some/file.ts" } }
      const result = isDestructiveOperation(op)

      expect(result).not.toBeNull()
      expect(result?.category).toBe("file_overwrite")
      expect(result?.riskLevel).toBe("medium")
      expect(result?.files).toEqual(["/some/file.ts"])
    })

    test("should return null for Read operations", () => {
      const op = { tool: "Read", input: { file_path: "/test/file" } }
      const result = isDestructiveOperation(op)

      expect(result).toBeNull()
    })

    test("should return null for unknown tools", () => {
      const op = { tool: "UnknownTool", input: { data: "test" } }
      const result = isDestructiveOperation(op)

      expect(result).toBeNull()
    })

    test("should extract file_path from operation", () => {
      const op = { tool: "Write", input: { file_path: "/path/to/file.txt" } }
      const result = isDestructiveOperation(op)

      expect(result?.files).toEqual(["/path/to/file.txt"])
    })

    test("should include description with file info", () => {
      const op = { tool: "Write", input: { file_path: "/test.ts" } }
      const result = isDestructiveOperation(op)

      expect(result?.description).toBe("Write operation on /test.ts")
    })

    test("should handle operation without input", () => {
      const op = { tool: "Bash" }
      const result = isDestructiveOperation(op)

      expect(result).not.toBeNull()
      expect(result?.category).toBe("file_deletion")
      expect(result?.files).toBeUndefined()
    })
  })

  describe("getDestructiveRiskLevel", () => {
    test("should return high for Bash operations", () => {
      const op = { tool: "Bash", input: { command: "rm -rf /important" } }
      expect(getDestructiveRiskLevel(op)).toBe("high")
    })

    test("should return medium for Write operations", () => {
      const op = { tool: "Write", input: { file_path: "/some/file" } }
      expect(getDestructiveRiskLevel(op)).toBe("medium")
    })

    test("should return medium for Edit operations", () => {
      const op = { tool: "Edit", input: { file_path: "/some/file" } }
      expect(getDestructiveRiskLevel(op)).toBe("medium")
    })

    test("should return low for Read operations", () => {
      const op = { tool: "Read", input: { file_path: "/safe/file" } }
      expect(getDestructiveRiskLevel(op)).toBe("low")
    })

    test("should return low for non-destructive tools", () => {
      const op = { tool: "Glob", input: { pattern: "**/*.ts" } }
      expect(getDestructiveRiskLevel(op)).toBe("low")
    })
  })

  describe("Destructive Categories", () => {
    const categories: DestructiveCategory[] = [
      "file_deletion",
      "file_overwrite",
      "git_operations",
      "dependency_change",
      "database_write",
      "network_request",
    ]

    test("should have all expected categories", () => {
      expect(categories.length).toBeGreaterThan(0)
      expect(categories).toContain("file_deletion")
      expect(categories).toContain("file_overwrite")
      expect(categories).toContain("git_operations")
    })
  })
})

describe("Safety Integration - Enhanced DOOM_LOOP Detection", () => {
  let integration: SafetyIntegration

  const createHistory = (
    ops: Partial<OperationHistory>[]
  ): OperationHistory[] => {
    const now = Date.now()
    return ops.map((op, index) => ({
      id: `op-${index}`,
      type: op.type ?? "tool_call",
      timestamp: op.timestamp ?? now - (ops.length - index) * 1000,
      tool: op.tool ?? "TestTool",
      input: op.input ?? {},
      result: op.result ?? "success",
      error: op.error,
      metadata: op.metadata ?? {},
    }))
  }

  beforeEach(() => {
    integration = new SafetyIntegration("test-session", {
      loopDetection: {
        repeatThreshold: 3,
        errorRepeatThreshold: 3,
        windowSize: 10,
        similarityThreshold: 0.8,
        timeWindowMs: 60000,
      },
    })
  })

  describe("detectDoomLoop", () => {
    test("returns not detected for insufficient history", () => {
      const history = createHistory([
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(false)
      expect(result.reason).toBe("Insufficient history")
    })

    test("detects exact repeat pattern", () => {
      const history = createHistory([
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(true)
      expect(result.loopType).toBe("exact_repeat")
      expect(result.details.matchingOperations).toBe(3)
    })

    test("does not detect different operations", () => {
      const history = createHistory([
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "b.ts" } },
        { tool: "Read", input: { file: "c.ts" } },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(false)
    })

    test("detects similar error pattern", () => {
      const history = createHistory([
        {
          tool: "Bash",
          input: { command: "npm install package-a" },
          result: "error",
          error: "npm ERR! Could not resolve dependency @types/node@^16.0.0",
        },
        {
          tool: "Bash",
          input: { command: "npm install package-b" },
          result: "error",
          error: "npm ERR! Could not resolve dependency @types/react@^18.0.0",
        },
        {
          tool: "Bash",
          input: { command: "npm install package-c" },
          result: "error",
          error: "npm ERR! Could not resolve dependency @types/jest@^29.0.0",
        },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(true)
      expect(result.loopType).toBe("similar_error")
    })

    test("detects state oscillation", () => {
      const history = createHistory([
        { type: "state_transition", input: "PLANNING" },
        { type: "state_transition", input: "EXECUTING" },
        { type: "state_transition", input: "PLANNING" },
        { type: "state_transition", input: "EXECUTING" },
        { type: "state_transition", input: "PLANNING" },
        { type: "state_transition", input: "EXECUTING" },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(true)
      expect(result.loopType).toBe("state_oscillation")
    })

    test("filters operations by time window", () => {
      const now = Date.now()
      const history = createHistory([
        // Old operations (outside window)
        { tool: "Read", input: { file: "a.ts" }, timestamp: now - 120000 },
        { tool: "Read", input: { file: "a.ts" }, timestamp: now - 110000 },
        { tool: "Read", input: { file: "a.ts" }, timestamp: now - 100000 },
        // Recent operations (inside window)
        { tool: "Read", input: { file: "b.ts" }, timestamp: now - 10000 },
        { tool: "Read", input: { file: "c.ts" }, timestamp: now - 5000 },
        { tool: "Read", input: { file: "d.ts" }, timestamp: now - 1000 },
      ])

      const result = integration.detectDoomLoop(history)
      // Should not detect loop because the repeated "a.ts" operations are outside time window
      expect(result.detected).toBe(false)
    })

    test("calculates confidence for exact repeat", () => {
      const history = createHistory([
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
        { tool: "Read", input: { file: "a.ts" } },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(true)
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("returns no loop for mixed successful operations", () => {
      const history = createHistory([
        { tool: "Read", input: { file: "a.ts" }, result: "success" },
        { tool: "Write", input: { file: "b.ts" }, result: "success" },
        { tool: "Bash", input: { command: "test" }, result: "success" },
        { tool: "Glob", input: { pattern: "**/*" }, result: "success" },
        { tool: "Grep", input: { query: "foo" }, result: "success" },
      ])

      const result = integration.detectDoomLoop(history)
      expect(result.detected).toBe(false)
    })
  })
})
