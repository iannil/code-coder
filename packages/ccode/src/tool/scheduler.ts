/**
 * Scheduler Tool - Provides MCP tools for creating and managing scheduled tasks
 *
 * This allows agents to schedule recurring tasks that call other agents,
 * APIs, or shell commands on a cron schedule.
 *
 * Usage Examples:
 * - Schedule daily financial news: scheduler_create_task with agentName="macro"
 * - List all tasks: scheduler_list_tasks
 * - Delete a task: scheduler_delete_task with id="task-id"
 */

import z from "zod"
import { Tool } from "./tool"
import { ConfigManager } from "@codecoder-ai/util/config"
import { Log } from "@/util/log"

const log = Log.create({ service: "tool.scheduler" })

const configManager = new ConfigManager()
const getSchedulerEndpoint = (): string => {
  return process.env.CODECODER_API_URL || configManager.getCodeCoderEndpoint()
}
const REQUEST_TIMEOUT = 10000

/**
 * Task command schema matching the scheduler handler
 */
const TaskCommandSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("agent"),
    agentName: z.string().describe("The agent to invoke (e.g., 'macro', 'trader', 'picker')"),
    prompt: z.string().describe("The prompt to send to the agent"),
  }),
  z.object({
    type: z.literal("api"),
    endpoint: z.string().describe("The API endpoint URL"),
    method: z.enum(["GET", "POST", "PUT", "DELETE"]).describe("HTTP method"),
    body: z.record(z.string(), z.unknown()).optional().describe("Request body for POST/PUT"),
  }),
  z.object({
    type: z.literal("shell"),
    command: z.string().describe("Shell command to execute"),
  }),
])

type TaskCommand = z.infer<typeof TaskCommandSchema>

interface SchedulerApiResponse<T> {
  success: boolean
  data?: T
  error?: string
  meta?: Record<string, unknown>
}

async function callSchedulerApi<T>(
  path: string,
  options: RequestInit = {},
): Promise<{ data?: T; error?: string; status: number }> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT)

  try {
    const response = await fetch(`${getSchedulerEndpoint()}${path}`, {
      ...options,
      signal: controller.signal,
      headers: {
        "Content-Type": "application/json",
        ...options.headers,
      },
    })

    clearTimeout(timeout)

    const body = (await response.json()) as SchedulerApiResponse<T>

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
      error: `Scheduler API unavailable: ${error instanceof Error ? error.message : String(error)}`,
      status: 503,
    }
  }
}

const SCHEDULER_CREATE_DESCRIPTION = `Create a scheduled task that runs on a cron schedule.

Use this tool to set up recurring automated tasks. The task can:
- Call an agent with a specific prompt (type: "agent")
- Make an HTTP API call (type: "api")
- Run a shell command (type: "shell")

Cron Expression Examples:
- "0 8 * * *" = Daily at 8:00 AM
- "0 9 * * 1-5" = Weekdays at 9:00 AM
- "*/30 * * * *" = Every 30 minutes
- "0 0 1 * *" = First day of each month at midnight

IMPORTANT: Always use this tool instead of creating cron scripts or shell files for scheduled tasks.`

/**
 * Tool: scheduler_create_task
 * Creates a new scheduled task
 */
export const SchedulerCreateTaskTool = Tool.define("scheduler_create_task", {
  description: SCHEDULER_CREATE_DESCRIPTION,
  parameters: z.object({
    id: z.string().min(1).max(64).describe("Unique task identifier (e.g., 'daily-finance-news')"),
    expression: z.string().min(1).describe("Cron expression for schedule (e.g., '0 8 * * *' for daily at 8 AM)"),
    agentName: z.string().optional().describe("Agent to invoke (for agent-type tasks)"),
    prompt: z.string().optional().describe("Prompt for the agent (required if agentName is provided)"),
    endpoint: z.string().optional().describe("API endpoint URL (for api-type tasks)"),
    method: z.enum(["GET", "POST", "PUT", "DELETE"]).optional().describe("HTTP method (for api-type tasks)"),
    shellCommand: z.string().optional().describe("Shell command (for shell-type tasks)"),
    description: z.string().max(512).optional().describe("Human-readable description of the task"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "scheduler",
      patterns: [params.id],
      always: ["*"],
      metadata: {
        action: "create_task",
        taskId: params.id,
        expression: params.expression,
      },
    })

    // Determine command type based on provided parameters
    let command: TaskCommand
    if (params.agentName && params.prompt) {
      command = { type: "agent", agentName: params.agentName, prompt: params.prompt }
    } else if (params.endpoint && params.method) {
      command = { type: "api", endpoint: params.endpoint, method: params.method }
    } else if (params.shellCommand) {
      command = { type: "shell", command: params.shellCommand }
    } else {
      throw new Error("Must provide either (agentName + prompt), (endpoint + method), or shellCommand")
    }

    const payload = {
      id: params.id,
      expression: params.expression,
      command,
      description: params.description,
      enabled: true,
    }

    log.info("creating scheduled task", { taskId: params.id, expression: params.expression, commandType: command.type })

    const result = await callSchedulerApi<{ id: string }>("/api/v1/scheduler/tasks", {
      method: "POST",
      body: JSON.stringify(payload),
    })

    if (result.data) {
      return {
        title: `Created scheduled task: ${params.id}`,
        metadata: {
          taskId: result.data.id,
          expression: params.expression,
          commandType: command.type,
        },
        output: `Successfully created scheduled task "${params.id}".\n\nSchedule: ${params.expression}\nType: ${command.type}\n${params.description ? `Description: ${params.description}` : ""}`,
      }
    }

    throw new Error(result.error || "Failed to create scheduled task")
  },
})

const SCHEDULER_LIST_DESCRIPTION = `List all scheduled tasks.

Returns information about all configured scheduled tasks including:
- Task ID and description
- Cron expression (schedule)
- Command type (agent, api, or shell)
- Next scheduled run time
- Last run status`

/**
 * Tool: scheduler_list_tasks
 * Lists all scheduled tasks
 */
export const SchedulerListTasksTool = Tool.define("scheduler_list_tasks", {
  description: SCHEDULER_LIST_DESCRIPTION,
  parameters: z.object({}),
  async execute(_params, ctx) {
    await ctx.ask({
      permission: "scheduler",
      patterns: ["list"],
      always: ["*"],
      metadata: { action: "list_tasks" },
    })

    log.info("listing scheduled tasks")

    const result = await callSchedulerApi<
      Array<{
        id: string
        name?: string
        description?: string
        expression: string
        command: TaskCommand
        enabled: boolean
        nextRun: string
        lastRun?: string
        lastStatus?: string
      }>
    >("/api/v1/scheduler/tasks")

    if (result.data) {
      if (result.data.length === 0) {
        return {
          title: "Scheduled tasks: 0 tasks",
          metadata: { count: 0 },
          output: "No scheduled tasks found.",
        }
      }

      const taskList = result.data
        .map((task) => {
          const commandType = typeof task.command === "object" ? task.command.type : "shell"
          return `- ${task.id}: ${task.description || task.name || "No description"}
    Schedule: ${task.expression}
    Type: ${commandType}
    Next run: ${task.nextRun}
    Last status: ${task.lastStatus || "Never run"}`
        })
        .join("\n\n")

      return {
        title: `Scheduled tasks: ${result.data.length} tasks`,
        metadata: { count: result.data.length },
        output: `Found ${result.data.length} scheduled task(s):\n\n${taskList}`,
      }
    }

    throw new Error(result.error || "Failed to list scheduled tasks")
  },
})

const SCHEDULER_DELETE_DESCRIPTION = `Delete a scheduled task by ID.

Permanently removes a scheduled task. The task will no longer run.`

/**
 * Tool: scheduler_delete_task
 * Deletes a scheduled task
 */
export const SchedulerDeleteTaskTool = Tool.define("scheduler_delete_task", {
  description: SCHEDULER_DELETE_DESCRIPTION,
  parameters: z.object({
    id: z.string().min(1).describe("The ID of the task to delete"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "scheduler",
      patterns: [params.id],
      always: ["*"],
      metadata: {
        action: "delete_task",
        taskId: params.id,
      },
    })

    log.info("deleting scheduled task", { taskId: params.id })

    const result = await callSchedulerApi<{ deleted: boolean }>(`/api/v1/scheduler/tasks/${params.id}`, {
      method: "DELETE",
    })

    if (result.data !== undefined) {
      return {
        title: `Deleted scheduled task: ${params.id}`,
        metadata: { taskId: params.id, deleted: result.data.deleted },
        output: result.data.deleted
          ? `Successfully deleted scheduled task "${params.id}".`
          : `Task "${params.id}" was not found or already deleted.`,
      }
    }

    throw new Error(result.error || "Failed to delete scheduled task")
  },
})

const SCHEDULER_RUN_DESCRIPTION = `Manually trigger a scheduled task to run immediately.

Executes the task right now regardless of its cron schedule.
Useful for testing or forcing an immediate run.`

/**
 * Tool: scheduler_run_task
 * Manually triggers a scheduled task
 */
export const SchedulerRunTaskTool = Tool.define("scheduler_run_task", {
  description: SCHEDULER_RUN_DESCRIPTION,
  parameters: z.object({
    id: z.string().min(1).describe("The ID of the task to run"),
  }),
  async execute(params, ctx) {
    await ctx.ask({
      permission: "scheduler",
      patterns: [params.id],
      always: ["*"],
      metadata: {
        action: "run_task",
        taskId: params.id,
      },
    })

    log.info("manually running scheduled task", { taskId: params.id })

    const result = await callSchedulerApi<{
      executionId: string
      taskId: string
      status: string
      output?: string
      error?: string
    }>(`/api/v1/scheduler/tasks/${params.id}/run`, {
      method: "POST",
    })

    if (result.data) {
      const statusEmoji = result.data.status === "ok" ? "✓" : "✗"
      return {
        title: `${statusEmoji} Ran task: ${params.id}`,
        metadata: {
          executionId: result.data.executionId,
          taskId: result.data.taskId,
          status: result.data.status,
        },
        output: `Executed task "${params.id}"\n\nStatus: ${result.data.status}\nExecution ID: ${result.data.executionId}${result.data.output ? `\n\nOutput:\n${result.data.output}` : ""}${result.data.error ? `\n\nError:\n${result.data.error}` : ""}`,
      }
    }

    throw new Error(result.error || "Failed to run scheduled task")
  },
})
