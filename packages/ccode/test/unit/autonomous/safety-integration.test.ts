// @ts-nocheck
import { describe, test, expect } from "bun:test"
import {
  isDestructiveOperation,
  getDestructiveRiskLevel,
  type DestructiveCategory,
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
