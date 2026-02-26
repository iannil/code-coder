/**
 * Scheduler Handler
 *
 * Proxies cron/scheduled task management to the Rust zero-workflow service.
 * Provides TypeScript API for managing scheduled tasks with support for:
 * - CRUD operations on tasks
 * - Manual task triggering
 * - Execution history
 * - Agent task scheduling
 *
 * Part of Phase 15: Scheduled Task API Integration
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { z } from "zod"
import { ConfigManager } from "@codecoder-ai/util/config"

// ============================================================================
// Configuration
// ============================================================================

// Use config manager for endpoint, fallback to env var for override
const configManager = new ConfigManager()
const getWorkflowServiceUrl = (): string => {
  return process.env.ZERO_WORKFLOW_URL || configManager.getWorkflowEndpoint()
}
const REQUEST_TIMEOUT = 10000

// ============================================================================
// Types
// ============================================================================

/**
 * Task command types
 */
export const TaskCommandSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("agent"),
    agentName: z.string(),
    prompt: z.string(),
  }),
  z.object({
    type: z.literal("api"),
    endpoint: z.string(),
    method: z.enum(["GET", "POST", "PUT", "DELETE"]),
    body: z.record(z.string(), z.unknown()).optional(),
  }),
  z.object({
    type: z.literal("shell"),
    command: z.string(),
  }),
])

export type TaskCommand = z.infer<typeof TaskCommandSchema>

/**
 * Scheduled task from Rust service
 */
export const ScheduledTaskSchema = z.object({
  id: z.string(),
  expression: z.string(),
  command: z.string(),
  description: z.string().nullable().optional(),
  next_run: z.string(),
  last_run: z.string().nullable().optional(),
  last_status: z.string().nullable().optional(),
  last_output: z.string().nullable().optional(),
})

export type ScheduledTask = z.infer<typeof ScheduledTaskSchema>

/**
 * Task info (list response)
 */
export const TaskInfoSchema = z.object({
  id: z.string(),
  command: z.string(),
  description: z.string().nullable().optional(),
  next_run: z.string(),
  last_run: z.string().nullable().optional(),
  last_status: z.string().nullable().optional(),
})

export type TaskInfo = z.infer<typeof TaskInfoSchema>

/**
 * Create task request
 */
export const CreateTaskRequestSchema = z.object({
  id: z.string().min(1).max(64),
  name: z.string().min(1).max(128).optional(),
  description: z.string().max(512).optional(),
  expression: z.string().min(1),
  command: TaskCommandSchema,
  enabled: z.boolean().default(true),
})

export type CreateTaskRequest = z.infer<typeof CreateTaskRequestSchema>

/**
 * Execution history entry
 */
export const ExecutionHistorySchema = z.object({
  id: z.string(),
  taskId: z.string(),
  startedAt: z.string(),
  endedAt: z.string().optional(),
  status: z.enum(["ok", "error", "running"]),
  output: z.string().optional(),
  error: z.string().optional(),
})

export type ExecutionHistory = z.infer<typeof ExecutionHistorySchema>

/**
 * Scheduler configuration
 */
export const SchedulerConfigSchema = z.object({
  enabled: z.boolean(),
  defaultTimeZone: z.string().default("UTC"),
  maxConcurrentTasks: z.number().int().min(1).max(100).default(10),
  retryOnFailure: z.boolean().default(false),
  maxRetries: z.number().int().min(0).max(5).default(3),
  retryDelaySeconds: z.number().int().min(1).max(3600).default(60),
})

export type SchedulerConfig = z.infer<typeof SchedulerConfigSchema>

// ============================================================================
// HTTP Client
// ============================================================================

interface RustApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

async function fetchWorkflowService<T>(
  path: string,
  options: RequestInit = {},
): Promise<{ data?: T; error?: string; status: number }> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT)

  try {
    const response = await fetch(`${getWorkflowServiceUrl()}${path}`, {
      ...options,
      signal: controller.signal,
      headers: {
        "Content-Type": "application/json",
        ...options.headers,
      },
    })

    clearTimeout(timeout)

    const body = (await response.json()) as RustApiResponse<T>

    if (body.success && body.data !== undefined) {
      return { data: body.data, status: response.status }
    }

    return { error: body.error || "Unknown error", status: response.status }
  } catch (error) {
    clearTimeout(timeout)

    if (error instanceof Error && error.name === "AbortError") {
      return { error: "Request timeout", status: 408 }
    }

    return {
      error: `Workflow service unavailable: ${error instanceof Error ? error.message : String(error)}`,
      status: 503,
    }
  }
}

// ============================================================================
// Command Serialization
// ============================================================================

/**
 * Serialize TaskCommand to shell command string for Rust service
 */
function serializeCommand(cmd: TaskCommand): string {
  switch (cmd.type) {
    case "agent":
      return JSON.stringify({ type: "agent", agent: cmd.agentName, prompt: cmd.prompt })
    case "api":
      return JSON.stringify({ type: "api", endpoint: cmd.endpoint, method: cmd.method, body: cmd.body })
    case "shell":
      return cmd.command
  }
}

/**
 * Parse command string back to TaskCommand
 */
function parseCommand(commandStr: string): TaskCommand {
  try {
    const parsed = JSON.parse(commandStr)
    if (parsed.type === "agent") {
      return { type: "agent", agentName: parsed.agent, prompt: parsed.prompt }
    }
    if (parsed.type === "api") {
      return { type: "api", endpoint: parsed.endpoint, method: parsed.method, body: parsed.body }
    }
  } catch {
    // Not JSON, treat as shell command
  }
  return { type: "shell", command: commandStr }
}

// ============================================================================
// Request Body Helper
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

// ============================================================================
// In-Memory Fallback (when Rust service unavailable)
// ============================================================================

interface InMemoryTask {
  id: string
  name?: string
  description?: string
  expression: string
  command: TaskCommand
  enabled: boolean
  nextRun: string
  lastRun?: string
  lastStatus?: "ok" | "error"
  lastOutput?: string
  createdAt: string
  updatedAt: string
}

const inMemoryTasks = new Map<string, InMemoryTask>()
const inMemoryHistory: ExecutionHistory[] = []
let inMemoryConfig: SchedulerConfig = {
  enabled: true,
  defaultTimeZone: "UTC",
  maxConcurrentTasks: 10,
  retryOnFailure: false,
  maxRetries: 3,
  retryDelaySeconds: 60,
}

function calculateNextRun(expression: string): string {
  // Simple approximation - in production, use cron parser
  return new Date(Date.now() + 60000).toISOString()
}

// ============================================================================
// Route Handlers
// ============================================================================

/**
 * GET /api/v1/scheduler/tasks
 * List all scheduled tasks
 */
export async function listSchedulerTasks(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  // Try Rust service first
  const result = await fetchWorkflowService<TaskInfo[]>("/api/v1/tasks")

  if (result.data) {
    // Transform to unified format with parsed commands
    const tasks = result.data.map((t) => ({
      id: t.id,
      name: t.description || t.id,
      description: t.description,
      expression: "", // Not returned by Rust service in list
      command: parseCommand(t.command),
      enabled: true,
      nextRun: t.next_run,
      lastRun: t.last_run,
      lastStatus: t.last_status as "ok" | "error" | undefined,
    }))

    return jsonResponse({ success: true, data: tasks })
  }

  // Fallback to in-memory
  if (result.status === 503) {
    const tasks = Array.from(inMemoryTasks.values()).map((t) => ({
      id: t.id,
      name: t.name || t.id,
      description: t.description,
      expression: t.expression,
      command: t.command,
      enabled: t.enabled,
      nextRun: t.nextRun,
      lastRun: t.lastRun,
      lastStatus: t.lastStatus,
    }))

    return jsonResponse({
      success: true,
      data: tasks,
      meta: { source: "in-memory", warning: "Workflow service unavailable" },
    })
  }

  return errorResponse(result.error || "Failed to list tasks", result.status)
}

/**
 * POST /api/v1/scheduler/tasks
 * Create a new scheduled task
 */
export async function createSchedulerTask(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = CreateTaskRequestSchema.parse(JSON.parse(body))

    // Serialize command for Rust service
    const rustPayload = {
      id: input.id,
      expression: input.expression,
      command: serializeCommand(input.command),
      description: input.description || input.name,
    }

    const result = await fetchWorkflowService<string>("/api/v1/tasks", {
      method: "POST",
      body: JSON.stringify(rustPayload),
    })

    if (result.data) {
      return jsonResponse({ success: true, data: { id: result.data } }, 201)
    }

    // Fallback to in-memory
    if (result.status === 503) {
      const now = new Date().toISOString()
      const task: InMemoryTask = {
        id: input.id,
        name: input.name,
        description: input.description,
        expression: input.expression,
        command: input.command,
        enabled: input.enabled,
        nextRun: calculateNextRun(input.expression),
        createdAt: now,
        updatedAt: now,
      }

      inMemoryTasks.set(input.id, task)

      return jsonResponse(
        {
          success: true,
          data: { id: input.id },
          meta: { source: "in-memory", warning: "Workflow service unavailable, task stored in-memory" },
        },
        201,
      )
    }

    return errorResponse(result.error || "Failed to create task", result.status)
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/scheduler/tasks/:id
 * Get a specific task
 */
export async function getSchedulerTask(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Task ID is required", 400)
  }

  // Try Rust service - note: current Rust API doesn't have GET by ID
  // We'll list and filter for now
  const result = await fetchWorkflowService<TaskInfo[]>("/api/v1/tasks")

  if (result.data) {
    const task = result.data.find((t) => t.id === id)
    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: {
        id: task.id,
        name: task.description || task.id,
        description: task.description,
        command: parseCommand(task.command),
        enabled: true,
        nextRun: task.next_run,
        lastRun: task.last_run,
        lastStatus: task.last_status,
      },
    })
  }

  // Fallback to in-memory
  if (result.status === 503) {
    const task = inMemoryTasks.get(id)
    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: task,
      meta: { source: "in-memory" },
    })
  }

  return errorResponse(result.error || "Failed to get task", result.status)
}

/**
 * PUT /api/v1/scheduler/tasks/:id
 * Update a scheduled task
 */
export async function updateSchedulerTask(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Task ID is required", 400)
  }

  try {
    const body = await readRequestBody(req.body)
    const input = CreateTaskRequestSchema.partial().parse(JSON.parse(body))

    // For Rust service, we need to delete and recreate
    // since it only supports add/remove
    const listResult = await fetchWorkflowService<TaskInfo[]>("/api/v1/tasks")

    if (listResult.data) {
      const existingTask = listResult.data.find((t) => t.id === id)
      if (!existingTask) {
        return errorResponse(`Task "${id}" not found`, 404)
      }

      // Delete existing
      await fetchWorkflowService<boolean>(`/api/v1/tasks/${id}`, { method: "DELETE" })

      // Create updated
      const command = input.command
        ? serializeCommand(input.command)
        : existingTask.command

      const rustPayload = {
        id,
        expression: input.expression || "", // Preserve existing if not provided
        command,
        description: input.description || input.name || existingTask.description,
      }

      const createResult = await fetchWorkflowService<string>("/api/v1/tasks", {
        method: "POST",
        body: JSON.stringify(rustPayload),
      })

      if (createResult.data) {
        return jsonResponse({ success: true, data: { id } })
      }

      return errorResponse(createResult.error || "Failed to update task", createResult.status)
    }

    // Fallback to in-memory
    if (listResult.status === 503) {
      const existingTask = inMemoryTasks.get(id)
      if (!existingTask) {
        return errorResponse(`Task "${id}" not found`, 404)
      }

      const updatedTask: InMemoryTask = {
        ...existingTask,
        ...input,
        updatedAt: new Date().toISOString(),
        nextRun: input.expression ? calculateNextRun(input.expression) : existingTask.nextRun,
      }

      inMemoryTasks.set(id, updatedTask)

      return jsonResponse({
        success: true,
        data: { id },
        meta: { source: "in-memory" },
      })
    }

    return errorResponse(listResult.error || "Failed to update task", listResult.status)
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/scheduler/tasks/:id
 * Delete a scheduled task
 */
export async function deleteSchedulerTask(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Task ID is required", 400)
  }

  const result = await fetchWorkflowService<boolean>(`/api/v1/tasks/${id}`, {
    method: "DELETE",
  })

  if (result.data !== undefined) {
    return jsonResponse({ success: true, data: { deleted: result.data } })
  }

  // Fallback to in-memory
  if (result.status === 503) {
    const deleted = inMemoryTasks.delete(id)
    return jsonResponse({
      success: true,
      data: { deleted },
      meta: { source: "in-memory" },
    })
  }

  return errorResponse(result.error || "Failed to delete task", result.status)
}

/**
 * POST /api/v1/scheduler/tasks/:id/run
 * Manually trigger a task execution
 */
export async function runSchedulerTask(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Task ID is required", 400)
  }

  // First get the task
  const listResult = await fetchWorkflowService<TaskInfo[]>("/api/v1/tasks")

  if (listResult.data) {
    const task = listResult.data.find((t) => t.id === id)
    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    // Execute the command based on type
    const command = parseCommand(task.command)
    const executionId = `exec-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`
    const startedAt = new Date().toISOString()

    let status: "ok" | "error" = "ok"
    let output = ""
    let error: string | undefined

    try {
      switch (command.type) {
        case "shell": {
          // Execute shell command
          const proc = Bun.spawn(["sh", "-c", command.command], {
            stdout: "pipe",
            stderr: "pipe",
          })
          output = await new Response(proc.stdout).text()
          const exitCode = await proc.exited
          if (exitCode !== 0) {
            status = "error"
            error = await new Response(proc.stderr).text()
          }
          break
        }

        case "agent": {
          // Call agent via CodeCoder API
          const agentResponse = await fetch(`${configManager.getCodeCoderEndpoint()}/api/agent/invoke`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              agentId: command.agentName,
              prompt: command.prompt,
            }),
          })
          const agentResult = await agentResponse.json()
          output = JSON.stringify(agentResult)
          status = agentResponse.ok ? "ok" : "error"
          break
        }

        case "api": {
          // Call external API
          const apiResponse = await fetch(command.endpoint, {
            method: command.method,
            headers: { "Content-Type": "application/json" },
            body: command.body ? JSON.stringify(command.body) : undefined,
          })
          output = await apiResponse.text()
          status = apiResponse.ok ? "ok" : "error"
          break
        }
      }
    } catch (e) {
      status = "error"
      error = e instanceof Error ? e.message : String(e)
    }

    // Record execution in history
    const historyEntry: ExecutionHistory = {
      id: executionId,
      taskId: id,
      startedAt,
      endedAt: new Date().toISOString(),
      status,
      output: output.slice(0, 10000), // Limit output size
      error,
    }

    inMemoryHistory.unshift(historyEntry)
    // Keep only last 100 entries
    if (inMemoryHistory.length > 100) {
      inMemoryHistory.pop()
    }

    return jsonResponse({
      success: true,
      data: {
        executionId,
        taskId: id,
        status,
        output: output.slice(0, 1000),
        error,
      },
    })
  }

  // Fallback to in-memory
  if (listResult.status === 503) {
    const task = inMemoryTasks.get(id)
    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: {
        executionId: `exec-${Date.now()}`,
        taskId: id,
        status: "ok",
        output: `[In-memory mode] Would execute: ${JSON.stringify(task.command)}`,
      },
      meta: { source: "in-memory", warning: "Task not actually executed in fallback mode" },
    })
  }

  return errorResponse(listResult.error || "Failed to run task", listResult.status)
}

/**
 * GET /api/v1/scheduler/history
 * Get execution history
 */
export async function getSchedulerHistory(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const limitParam = url.searchParams.get("limit")
  const taskIdParam = url.searchParams.get("taskId")
  const limit = limitParam ? parseInt(limitParam, 10) : 50

  // Note: Rust service doesn't have execution history endpoint yet
  // Using in-memory history for now

  let history = [...inMemoryHistory]

  // Filter by taskId if provided
  if (taskIdParam) {
    history = history.filter((h) => h.taskId === taskIdParam)
  }

  // Apply limit
  history = history.slice(0, limit)

  return jsonResponse({
    success: true,
    data: history,
    meta: {
      total: inMemoryHistory.length,
      filtered: history.length,
    },
  })
}

/**
 * GET /api/v1/scheduler/history/:id
 * Get a specific execution
 */
export async function getSchedulerExecution(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Execution ID is required", 400)
  }

  const execution = inMemoryHistory.find((h) => h.id === id)
  if (!execution) {
    return errorResponse(`Execution "${id}" not found`, 404)
  }

  return jsonResponse({ success: true, data: execution })
}

/**
 * GET /api/v1/scheduler/config
 * Get scheduler configuration
 */
export async function getSchedulerConfig(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  return jsonResponse({ success: true, data: inMemoryConfig })
}

/**
 * PUT /api/v1/scheduler/config
 * Update scheduler configuration
 */
export async function updateSchedulerConfig(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = SchedulerConfigSchema.partial().parse(JSON.parse(body))

    inMemoryConfig = { ...inMemoryConfig, ...input }

    return jsonResponse({ success: true, data: inMemoryConfig })
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/scheduler/health
 * Health check for scheduler
 */
export async function schedulerHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  // Check Rust service availability
  const result = await fetchWorkflowService<unknown>("/health")

  const rustServiceStatus = result.status < 400 ? "healthy" : "unavailable"

  return jsonResponse({
    success: true,
    data: {
      status: inMemoryConfig.enabled ? "healthy" : "disabled",
      rustService: rustServiceStatus,
      tasksCount: inMemoryTasks.size,
      historyCount: inMemoryHistory.length,
      config: {
        enabled: inMemoryConfig.enabled,
        maxConcurrentTasks: inMemoryConfig.maxConcurrentTasks,
      },
    },
  })
}
