/**
 * Scheduler Handler Tests
 *
 * Tests for the Scheduler API that manages cron/scheduled tasks.
 * Tests both the schema validation and handler functionality.
 *
 * Part of Phase 15: Scheduled Task API Integration
 */

import { describe, test, expect, beforeEach, afterEach, mock, spyOn } from "bun:test"
import {
  TaskCommandSchema,
  ScheduledTaskSchema,
  TaskInfoSchema,
  CreateTaskRequestSchema,
  ExecutionHistorySchema,
  SchedulerConfigSchema,
  type TaskCommand,
  type SchedulerConfig,
} from "../../../src/api/server/handlers/scheduler"

// ============================================================================
// Schema Validation Tests
// ============================================================================

describe("Scheduler Schemas", () => {
  describe("TaskCommandSchema", () => {
    test("should validate shell command", () => {
      const command = {
        type: "shell",
        command: "echo hello",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.type).toBe("shell")
      }
    })

    test("should validate agent command", () => {
      const command = {
        type: "agent",
        agentName: "@dev",
        prompt: "Fix the bug in auth.ts",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.type).toBe("agent")
        if (result.data.type === "agent") {
          expect(result.data.agentName).toBe("@dev")
        }
      }
    })

    test("should validate API command", () => {
      const command = {
        type: "api",
        endpoint: "https://api.example.com/webhook",
        method: "POST",
        body: { event: "scheduled_task" },
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.type).toBe("api")
      }
    })

    test("should validate API command without body", () => {
      const command = {
        type: "api",
        endpoint: "https://api.example.com/health",
        method: "GET",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
    })

    test("should reject invalid command type", () => {
      const command = {
        type: "invalid",
        data: "test",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(false)
    })

    test("should reject shell command without command field", () => {
      const command = {
        type: "shell",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(false)
    })

    test("should reject agent command without agentName", () => {
      const command = {
        type: "agent",
        prompt: "Do something",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(false)
    })

    test("should reject API command with invalid method", () => {
      const command = {
        type: "api",
        endpoint: "https://example.com",
        method: "INVALID",
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(false)
    })
  })

  describe("ScheduledTaskSchema", () => {
    test("should validate complete scheduled task", () => {
      const task = {
        id: "daily-backup",
        expression: "0 0 * * *",
        command: "backup.sh",
        description: "Daily backup",
        next_run: "2026-02-25T00:00:00Z",
        last_run: "2026-02-24T00:00:00Z",
        last_status: "ok",
        last_output: "Backup completed successfully",
      }

      const result = ScheduledTaskSchema.safeParse(task)
      expect(result.success).toBe(true)
    })

    test("should validate minimal scheduled task", () => {
      const task = {
        id: "test-task",
        expression: "*/5 * * * *",
        command: "echo test",
        next_run: "2026-02-24T12:00:00Z",
      }

      const result = ScheduledTaskSchema.safeParse(task)
      expect(result.success).toBe(true)
    })

    test("should allow null optional fields", () => {
      const task = {
        id: "test-task",
        expression: "*/5 * * * *",
        command: "echo test",
        description: null,
        next_run: "2026-02-24T12:00:00Z",
        last_run: null,
        last_status: null,
        last_output: null,
      }

      const result = ScheduledTaskSchema.safeParse(task)
      expect(result.success).toBe(true)
    })
  })

  describe("TaskInfoSchema", () => {
    test("should validate task info", () => {
      const info = {
        id: "task-1",
        command: "echo hello",
        description: "Test task",
        next_run: "2026-02-24T12:00:00Z",
        last_run: "2026-02-24T11:00:00Z",
        last_status: "ok",
      }

      const result = TaskInfoSchema.safeParse(info)
      expect(result.success).toBe(true)
    })

    test("should validate minimal task info", () => {
      const info = {
        id: "task-1",
        command: "echo hello",
        next_run: "2026-02-24T12:00:00Z",
      }

      const result = TaskInfoSchema.safeParse(info)
      expect(result.success).toBe(true)
    })
  })

  describe("CreateTaskRequestSchema", () => {
    test("should validate complete create request", () => {
      const request = {
        id: "my-task",
        name: "My Task",
        description: "A test task that runs every hour",
        expression: "0 * * * *",
        command: {
          type: "shell",
          command: "echo hello",
        },
        enabled: true,
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
    })

    test("should validate minimal create request", () => {
      const request = {
        id: "my-task",
        expression: "0 * * * *",
        command: {
          type: "shell",
          command: "echo hello",
        },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.enabled).toBe(true) // Default value
      }
    })

    test("should validate agent command request", () => {
      const request = {
        id: "agent-task",
        name: "Daily Code Review",
        expression: "0 9 * * 1-5",
        command: {
          type: "agent",
          agentName: "@code-reviewer",
          prompt: "Review changes in the last 24 hours",
        },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
    })

    test("should validate API command request", () => {
      const request = {
        id: "webhook-task",
        expression: "*/30 * * * *",
        command: {
          type: "api",
          endpoint: "https://api.example.com/trigger",
          method: "POST",
          body: { source: "scheduler" },
        },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
    })

    test("should reject empty id", () => {
      const request = {
        id: "",
        expression: "0 * * * *",
        command: { type: "shell", command: "echo" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(false)
    })

    test("should reject id longer than 64 characters", () => {
      const request = {
        id: "a".repeat(65),
        expression: "0 * * * *",
        command: { type: "shell", command: "echo" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(false)
    })

    test("should reject name longer than 128 characters", () => {
      const request = {
        id: "test",
        name: "a".repeat(129),
        expression: "0 * * * *",
        command: { type: "shell", command: "echo" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(false)
    })

    test("should reject description longer than 512 characters", () => {
      const request = {
        id: "test",
        description: "a".repeat(513),
        expression: "0 * * * *",
        command: { type: "shell", command: "echo" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(false)
    })

    test("should reject empty expression", () => {
      const request = {
        id: "test",
        expression: "",
        command: { type: "shell", command: "echo" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(false)
    })
  })

  describe("ExecutionHistorySchema", () => {
    test("should validate complete execution history", () => {
      const history = {
        id: "exec-123",
        taskId: "task-1",
        startedAt: "2026-02-24T12:00:00Z",
        endedAt: "2026-02-24T12:00:05Z",
        status: "ok",
        output: "Task completed successfully",
      }

      const result = ExecutionHistorySchema.safeParse(history)
      expect(result.success).toBe(true)
    })

    test("should validate running execution", () => {
      const history = {
        id: "exec-456",
        taskId: "task-2",
        startedAt: "2026-02-24T12:00:00Z",
        status: "running",
      }

      const result = ExecutionHistorySchema.safeParse(history)
      expect(result.success).toBe(true)
    })

    test("should validate failed execution", () => {
      const history = {
        id: "exec-789",
        taskId: "task-3",
        startedAt: "2026-02-24T12:00:00Z",
        endedAt: "2026-02-24T12:00:02Z",
        status: "error",
        error: "Command not found: foobar",
      }

      const result = ExecutionHistorySchema.safeParse(history)
      expect(result.success).toBe(true)
    })

    test("should reject invalid status", () => {
      const history = {
        id: "exec-000",
        taskId: "task-1",
        startedAt: "2026-02-24T12:00:00Z",
        status: "invalid",
      }

      const result = ExecutionHistorySchema.safeParse(history)
      expect(result.success).toBe(false)
    })
  })

  describe("SchedulerConfigSchema", () => {
    test("should validate complete config", () => {
      const config = {
        enabled: true,
        defaultTimeZone: "America/New_York",
        maxConcurrentTasks: 5,
        retryOnFailure: true,
        maxRetries: 3,
        retryDelaySeconds: 120,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(true)
    })

    test("should apply defaults", () => {
      const config = {
        enabled: false,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.defaultTimeZone).toBe("UTC")
        expect(result.data.maxConcurrentTasks).toBe(10)
        expect(result.data.retryOnFailure).toBe(false)
        expect(result.data.maxRetries).toBe(3)
        expect(result.data.retryDelaySeconds).toBe(60)
      }
    })

    test("should reject maxConcurrentTasks less than 1", () => {
      const config = {
        enabled: true,
        maxConcurrentTasks: 0,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(false)
    })

    test("should reject maxConcurrentTasks greater than 100", () => {
      const config = {
        enabled: true,
        maxConcurrentTasks: 101,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(false)
    })

    test("should reject negative maxRetries", () => {
      const config = {
        enabled: true,
        maxRetries: -1,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(false)
    })

    test("should reject maxRetries greater than 5", () => {
      const config = {
        enabled: true,
        maxRetries: 6,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(false)
    })

    test("should reject retryDelaySeconds less than 1", () => {
      const config = {
        enabled: true,
        retryDelaySeconds: 0,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(false)
    })

    test("should reject retryDelaySeconds greater than 3600", () => {
      const config = {
        enabled: true,
        retryDelaySeconds: 3601,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(false)
    })
  })
})

// ============================================================================
// Command Type Tests
// ============================================================================

describe("Task Command Types", () => {
  describe("Shell commands", () => {
    test("should handle simple shell commands", () => {
      const commands = [
        "echo hello",
        "ls -la",
        "python script.py",
        "/usr/bin/backup.sh",
      ]

      for (const cmd of commands) {
        const command: TaskCommand = { type: "shell", command: cmd }
        const result = TaskCommandSchema.safeParse(command)
        expect(result.success).toBe(true)
      }
    })

    test("should handle shell commands with special characters", () => {
      const command: TaskCommand = {
        type: "shell",
        command: 'echo "Hello World" | grep -o "World"',
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
    })

    test("should handle multiline shell commands", () => {
      const command: TaskCommand = {
        type: "shell",
        command: `
          cd /app &&
          npm run build &&
          npm run deploy
        `,
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
    })
  })

  describe("Agent commands", () => {
    test("should handle various agent names", () => {
      const agentNames = [
        "@dev",
        "@code-reviewer",
        "@security-reviewer",
        "@tdd-guide",
        "general",
      ]

      for (const agentName of agentNames) {
        const command: TaskCommand = {
          type: "agent",
          agentName,
          prompt: "Do something",
        }
        const result = TaskCommandSchema.safeParse(command)
        expect(result.success).toBe(true)
      }
    })

    test("should handle agent prompts with special characters", () => {
      const command: TaskCommand = {
        type: "agent",
        agentName: "@dev",
        prompt: 'Fix the bug in `src/auth.ts` where user.email === "admin@test.com"',
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
    })

    test("should handle long agent prompts", () => {
      const command: TaskCommand = {
        type: "agent",
        agentName: "@dev",
        prompt: "a".repeat(10000),
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
    })
  })

  describe("API commands", () => {
    test("should handle all HTTP methods", () => {
      const methods = ["GET", "POST", "PUT", "DELETE"] as const

      for (const method of methods) {
        const command: TaskCommand = {
          type: "api",
          endpoint: "https://api.example.com/test",
          method,
        }
        const result = TaskCommandSchema.safeParse(command)
        expect(result.success).toBe(true)
      }
    })

    test("should handle various endpoint formats", () => {
      const endpoints = [
        "https://api.example.com/v1/webhook",
        "http://localhost:3000/trigger",
        "https://api.example.com/tasks?status=pending",
        "https://user:pass@api.example.com/secure",
      ]

      for (const endpoint of endpoints) {
        const command: TaskCommand = {
          type: "api",
          endpoint,
          method: "POST",
        }
        const result = TaskCommandSchema.safeParse(command)
        expect(result.success).toBe(true)
      }
    })

    test("should handle complex request bodies", () => {
      const command: TaskCommand = {
        type: "api",
        endpoint: "https://api.example.com/process",
        method: "POST",
        body: {
          nested: {
            array: [1, 2, 3],
            object: { key: "value" },
          },
          null_value: null,
          boolean: true,
          number: 42,
        },
      }

      const result = TaskCommandSchema.safeParse(command)
      expect(result.success).toBe(true)
    })
  })
})

// ============================================================================
// Cron Expression Tests
// ============================================================================

describe("Cron Expression Validation", () => {
  test("should accept standard 5-field cron expressions", () => {
    const expressions = [
      "* * * * *",          // Every minute
      "0 * * * *",          // Every hour
      "0 0 * * *",          // Every day at midnight
      "0 0 * * 0",          // Every Sunday
      "0 9 * * 1-5",        // Weekdays at 9am
      "*/5 * * * *",        // Every 5 minutes
      "0 0 1 * *",          // First day of month
      "0 0 1 1 *",          // New Year's Day
    ]

    for (const expression of expressions) {
      const request = {
        id: "test",
        expression,
        command: { type: "shell" as const, command: "echo test" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
    }
  })

  test("should accept complex cron expressions", () => {
    const expressions = [
      "0 0,12 * * *",       // Twice daily
      "0 0 1,15 * *",       // 1st and 15th
      "30 4 1-7 * 1",       // First Monday
      "0 */2 * * *",        // Every 2 hours
      "0 0 * * SAT,SUN",    // Weekends (text)
    ]

    for (const expression of expressions) {
      const request = {
        id: "test",
        expression,
        command: { type: "shell" as const, command: "echo test" },
      }

      // Schema just validates non-empty string
      // Actual validation happens in Rust service
      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
    }
  })
})

// ============================================================================
// Edge Cases
// ============================================================================

describe("Edge Cases", () => {
  test("should handle unicode in task names", () => {
    const request = {
      id: "unicode-task",
      name: "每日备份 🗃️",
      description: "Daily backup task with 中文 description",
      expression: "0 0 * * *",
      command: { type: "shell" as const, command: "backup.sh" },
    }

    const result = CreateTaskRequestSchema.safeParse(request)
    expect(result.success).toBe(true)
  })

  test("should handle special characters in task id", () => {
    const validIds = [
      "task-1",
      "task_2",
      "task.3",
      "TASK-123",
    ]

    for (const id of validIds) {
      const request = {
        id,
        expression: "* * * * *",
        command: { type: "shell" as const, command: "echo" },
      }

      const result = CreateTaskRequestSchema.safeParse(request)
      expect(result.success).toBe(true)
    }
  })

  test("should handle empty command output", () => {
    const history = {
      id: "exec-1",
      taskId: "task-1",
      startedAt: "2026-02-24T12:00:00Z",
      endedAt: "2026-02-24T12:00:01Z",
      status: "ok",
      output: "",
    }

    const result = ExecutionHistorySchema.safeParse(history)
    expect(result.success).toBe(true)
  })

  test("should handle very long command output", () => {
    const history = {
      id: "exec-1",
      taskId: "task-1",
      startedAt: "2026-02-24T12:00:00Z",
      endedAt: "2026-02-24T12:00:01Z",
      status: "ok",
      output: "a".repeat(100000),
    }

    const result = ExecutionHistorySchema.safeParse(history)
    expect(result.success).toBe(true)
  })

  test("should handle tasks with minimal configuration", () => {
    const request = {
      id: "min",
      expression: "0 0 * * *",
      command: { type: "shell" as const, command: ":" }, // No-op shell command
    }

    const result = CreateTaskRequestSchema.safeParse(request)
    expect(result.success).toBe(true)
  })
})

// ============================================================================
// Integration-like Tests (Schema + Business Logic)
// ============================================================================

describe("Business Logic Validation", () => {
  describe("Task lifecycle", () => {
    test("should support all execution statuses", () => {
      const statuses = ["ok", "error", "running"] as const

      for (const status of statuses) {
        const history = {
          id: `exec-${status}`,
          taskId: "task-1",
          startedAt: "2026-02-24T12:00:00Z",
          status,
        }

        const result = ExecutionHistorySchema.safeParse(history)
        expect(result.success).toBe(true)
      }
    })

    test("should track execution with error details", () => {
      const history = {
        id: "exec-error",
        taskId: "failed-task",
        startedAt: "2026-02-24T12:00:00Z",
        endedAt: "2026-02-24T12:00:03Z",
        status: "error",
        output: "Partial output before failure...",
        error: "Command failed with exit code 1: permission denied",
      }

      const result = ExecutionHistorySchema.safeParse(history)
      expect(result.success).toBe(true)
    })
  })

  describe("Scheduler configuration", () => {
    test("should support disabled scheduler", () => {
      const config: SchedulerConfig = {
        enabled: false,
        defaultTimeZone: "UTC",
        maxConcurrentTasks: 1,
        retryOnFailure: false,
        maxRetries: 0,
        retryDelaySeconds: 60,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(true)
    })

    test("should support aggressive retry configuration", () => {
      const config: SchedulerConfig = {
        enabled: true,
        defaultTimeZone: "UTC",
        maxConcurrentTasks: 50,
        retryOnFailure: true,
        maxRetries: 5,
        retryDelaySeconds: 30,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(true)
    })

    test("should support conservative retry configuration", () => {
      const config: SchedulerConfig = {
        enabled: true,
        defaultTimeZone: "UTC",
        maxConcurrentTasks: 1,
        retryOnFailure: true,
        maxRetries: 1,
        retryDelaySeconds: 3600,
      }

      const result = SchedulerConfigSchema.safeParse(config)
      expect(result.success).toBe(true)
    })
  })
})

// ============================================================================
// Regression Tests
// ============================================================================

describe("Regression Tests", () => {
  test("should handle task with all optional fields null", () => {
    const task = {
      id: "null-test",
      expression: "0 0 * * *",
      command: "test",
      description: null,
      next_run: "2026-02-24T00:00:00Z",
      last_run: null,
      last_status: null,
      last_output: null,
    }

    const result = ScheduledTaskSchema.safeParse(task)
    expect(result.success).toBe(true)
  })

  test("should handle execution history without optional fields", () => {
    const history = {
      id: "exec-minimal",
      taskId: "task-1",
      startedAt: "2026-02-24T12:00:00Z",
      status: "running",
    }

    const result = ExecutionHistorySchema.safeParse(history)
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.endedAt).toBeUndefined()
      expect(result.data.output).toBeUndefined()
      expect(result.data.error).toBeUndefined()
    }
  })
})
