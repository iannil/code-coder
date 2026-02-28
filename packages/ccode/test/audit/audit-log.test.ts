import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { createAuditLog, type AuditLog, type AuditEntryInput } from "@/audit/audit-log"
import fs from "fs/promises"
import os from "os"
import path from "path"

describe("audit-log", () => {
  let auditLog: AuditLog
  let testDbPath: string

  beforeEach(async () => {
    // Create a temporary database for each test
    testDbPath = path.join(os.tmpdir(), `audit-test-${Date.now()}.db`)
    auditLog = createAuditLog(testDbPath)
    await auditLog.initialize()
  })

  afterEach(async () => {
    // Close the database and clean up
    auditLog.close()
    try {
      await fs.unlink(testDbPath)
      await fs.unlink(`${testDbPath}-shm`).catch(() => {})
      await fs.unlink(`${testDbPath}-wal`).catch(() => {})
    } catch {
      // Ignore cleanup errors
    }
  })

  describe("log", () => {
    test("logs an entry and returns ID", async () => {
      const entry: AuditEntryInput = {
        sessionId: "test-session",
        type: "tool_call",
        action: "Read",
        input: { file_path: "/test.ts" },
        result: "success",
        reason: "File read successfully",
        metadata: { tool: "Read" },
      }

      const id = await auditLog.log(entry)
      expect(id).toMatch(/^audit_\d+_[a-z0-9]+$/)
    })

    test("logs multiple entries", async () => {
      const entries: AuditEntryInput[] = [
        {
          sessionId: "test-session",
          type: "permission",
          action: "Read:approve",
          input: { tool: "Read" },
          result: "approved",
          risk: "safe",
          autoApproved: true,
          reason: "Auto-approved safe tool",
          metadata: {},
        },
        {
          sessionId: "test-session",
          type: "tool_call",
          action: "Read",
          input: { file_path: "/test.ts" },
          result: "success",
          reason: "File read",
          metadata: {},
        },
      ]

      for (const entry of entries) {
        await auditLog.log(entry)
      }

      const count = await auditLog.count()
      expect(count).toBe(2)
    })
  })

  describe("query", () => {
    beforeEach(async () => {
      // Add test data
      await auditLog.log({
        sessionId: "session-1",
        type: "permission",
        action: "Read:approve",
        input: {},
        result: "approved",
        risk: "safe",
        autoApproved: true,
        reason: "Auto-approved",
        metadata: {},
      })

      await auditLog.log({
        sessionId: "session-1",
        type: "tool_call",
        action: "Read",
        input: {},
        result: "success",
        reason: "Success",
        metadata: {},
      })

      await auditLog.log({
        sessionId: "session-2",
        type: "error",
        action: "Bash",
        input: { command: "fail" },
        result: "error",
        risk: "high",
        reason: "Command failed",
        metadata: {},
      })
    })

    test("queries all entries", async () => {
      const entries = await auditLog.query()
      expect(entries.length).toBe(3)
    })

    test("filters by sessionId", async () => {
      const entries = await auditLog.query({ sessionId: "session-1" })
      expect(entries.length).toBe(2)
      expect(entries.every((e) => e.sessionId === "session-1")).toBe(true)
    })

    test("filters by type", async () => {
      const entries = await auditLog.query({ type: "permission" })
      expect(entries.length).toBe(1)
      expect(entries[0].type).toBe("permission")
    })

    test("filters by result", async () => {
      const entries = await auditLog.query({ result: "error" })
      expect(entries.length).toBe(1)
      expect(entries[0].result).toBe("error")
    })

    test("filters by autoApproved", async () => {
      const entries = await auditLog.query({ autoApproved: true })
      expect(entries.length).toBe(1)
      expect(entries[0].autoApproved).toBe(true)
    })

    test("filters by risk", async () => {
      const entries = await auditLog.query({ risk: "high" })
      expect(entries.length).toBe(1)
      expect(entries[0].risk).toBe("high")
    })

    test("filters by action pattern", async () => {
      const entries = await auditLog.query({ action: "Read" })
      expect(entries.length).toBe(2) // Read:approve and Read
    })

    test("applies limit", async () => {
      const entries = await auditLog.query({ limit: 1 })
      expect(entries.length).toBe(1)
    })

    test("applies offset", async () => {
      const allEntries = await auditLog.query()
      const offsetEntries = await auditLog.query({ offset: 1 })
      expect(offsetEntries.length).toBe(allEntries.length - 1)
    })

    test("combines multiple filters", async () => {
      const entries = await auditLog.query({
        sessionId: "session-1",
        type: "permission",
      })
      expect(entries.length).toBe(1)
    })
  })

  describe("exportReport", () => {
    beforeEach(async () => {
      await auditLog.log({
        sessionId: "report-session",
        type: "session_start",
        action: "start",
        input: null,
        result: "success",
        reason: "Session started",
        metadata: {},
      })

      await auditLog.log({
        sessionId: "report-session",
        type: "permission",
        action: "Read:approve",
        input: {},
        result: "approved",
        risk: "safe",
        autoApproved: true,
        reason: "Auto-approved",
        metadata: {},
      })

      await auditLog.log({
        sessionId: "report-session",
        type: "tool_call",
        action: "Read",
        input: {},
        result: "success",
        reason: "Success",
        metadata: {},
      })

      await auditLog.log({
        sessionId: "report-session",
        type: "error",
        action: "Write",
        input: {},
        result: "error",
        risk: "medium",
        reason: "Write failed",
        metadata: {},
      })

      await auditLog.log({
        sessionId: "report-session",
        type: "session_end",
        action: "end",
        input: null,
        result: "success",
        reason: "Session ended",
        metadata: {},
      })
    })

    test("generates report with summary", async () => {
      const report = await auditLog.exportReport("report-session")

      expect(report.sessionId).toBe("report-session")
      expect(report.summary.totalEntries).toBe(5)
      expect(report.summary.byType.session_start).toBe(1)
      expect(report.summary.byType.session_end).toBe(1)
      expect(report.summary.byType.permission).toBe(1)
      expect(report.summary.byType.tool_call).toBe(1)
      expect(report.summary.byType.error).toBe(1)
      expect(report.summary.autoApprovedCount).toBe(1)
    })

    test("includes all entries in report", async () => {
      const report = await auditLog.exportReport("report-session")

      expect(report.entries.length).toBe(5)
      // Check all entry types are present
      const types = report.entries.map((e) => e.type)
      expect(types).toContain("session_start")
      expect(types).toContain("session_end")
      expect(types).toContain("permission")
      expect(types).toContain("tool_call")
      expect(types).toContain("error")
    })

    test("calculates time range", async () => {
      const report = await auditLog.exportReport("report-session")

      expect(report.summary.timeRange.start).toBeGreaterThan(0)
      expect(report.summary.timeRange.end).toBeGreaterThanOrEqual(report.summary.timeRange.start)
      expect(report.summary.timeRange.durationMs).toBeGreaterThanOrEqual(0)
    })
  })

  describe("count", () => {
    test("returns 0 for empty database", async () => {
      const count = await auditLog.count()
      expect(count).toBe(0)
    })

    test("returns total count", async () => {
      await auditLog.log({
        sessionId: "s1",
        type: "tool_call",
        action: "test",
        input: {},
        result: "success",
        reason: "test",
        metadata: {},
      })
      await auditLog.log({
        sessionId: "s2",
        type: "tool_call",
        action: "test",
        input: {},
        result: "success",
        reason: "test",
        metadata: {},
      })

      const count = await auditLog.count()
      expect(count).toBe(2)
    })

    test("returns filtered count", async () => {
      await auditLog.log({
        sessionId: "s1",
        type: "tool_call",
        action: "test",
        input: {},
        result: "success",
        reason: "test",
        metadata: {},
      })
      await auditLog.log({
        sessionId: "s2",
        type: "permission",
        action: "test",
        input: {},
        result: "approved",
        reason: "test",
        metadata: {},
      })

      const count = await auditLog.count({ type: "permission" })
      expect(count).toBe(1)
    })
  })
})
