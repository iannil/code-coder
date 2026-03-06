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
import { SCHEDULER_REQUEST_TIMEOUT_MS } from "@/config/timeouts"

const log = Log.create({ service: "tool.scheduler" })

const configManager = new ConfigManager()
const getSchedulerEndpoint = (): string => {
  return process.env.CODECODER_API_URL || configManager.getCodeCoderEndpoint()
}

/**
 * Task command schema matching the scheduler handler
 */
const TaskCommandSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("agent"),
    agentName: z.string().describe("The agent to invoke (e.g., 'macro', 'trader', 'picker')"),
    prompt: z.string().describe("The prompt to send to the agent"),
    callbackChannelType: z.string().optional().describe("Callback channel type for sending results back to IM"),
    callbackChannelId: z.string().optional().describe("Callback channel ID for sending results back to IM"),
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
  z.object({
    type: z.literal("channel_message"),
    channelType: z.string().describe("Channel type: telegram, feishu, wecom, dingtalk, discord, slack"),
    channelId: z.string().describe("Channel/chat ID to send the message to"),
    message: z.string().describe("Message text to send"),
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
  const timeout = setTimeout(() => controller.abort(), SCHEDULER_REQUEST_TIMEOUT_MS)

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

const SCHEDULER_CREATE_DESCRIPTION = `Create a RECURRING scheduled task that runs on a cron schedule.

IMPORTANT: This tool is for RECURRING/PERIODIC tasks only!
- For "do something in X minutes" → use scheduler_delay_task instead
- For "remind me in 1 hour" → use scheduler_delay_task instead

Use this tool to set up automated tasks that repeat on a schedule:
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

    // Auto-detect channel info from context for agent task callbacks
    const autoChannelType = ctx.extra?.channelType as string | undefined
    const autoChannelId = ctx.extra?.channelId as string | undefined

    // Determine command type based on provided parameters
    let command: TaskCommand
    if (params.agentName && params.prompt) {
      // Agent task - automatically include callback channel info from context
      command = {
        type: "agent",
        agentName: params.agentName,
        prompt: params.prompt,
        callbackChannelType: autoChannelType,
        callbackChannelId: autoChannelId,
      }
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

const SCHEDULER_DELAY_DESCRIPTION = `Schedule a one-time task to run after a delay.

Use this tool when user asks to "do something in X minutes/hours" or "remind me in X minutes".
This is for ONE-TIME delayed execution, NOT for recurring tasks.

Examples:
- "Send me a message in 5 minutes" → delayMinutes: 5, channelType: "telegram", channelId: "xxx", channelMessage: "..."
- "Remind me in 1 hour" → delayMinutes: 60
- "Check this in 30 seconds" → delaySeconds: 30

IMPORTANT FOR IM MESSAGES:
When user asks to send a message via IM (Telegram, etc.), you MUST use channelType, channelId, and channelMessage parameters.
Using agentName/prompt will only create a local session and NOT send the message back to the IM channel.

IMPORTANT: Use this tool for one-time delayed tasks. Use scheduler_create_task for recurring/periodic tasks.`

/**
 * Tool: scheduler_delay_task
 * Creates a one-time delayed task
 */
export const SchedulerDelayTaskTool = Tool.define("scheduler_delay_task", {
  description: SCHEDULER_DELAY_DESCRIPTION,
  parameters: z.object({
    delayMinutes: z.number().min(1).max(1440).optional().describe("Delay in minutes before execution (1-1440, i.e., up to 24 hours)"),
    delaySeconds: z.number().min(30).max(3600).optional().describe("Delay in seconds before execution (30-3600). Use for delays less than 1 minute."),
    agentName: z.string().optional().describe("Agent to invoke (e.g., 'general', 'macro'). Note: This creates a local session, NOT an IM message."),
    prompt: z.string().optional().describe("Prompt for the agent"),
    message: z.string().optional().describe("Simple message to send (uses 'general' agent). Note: This creates a local session, NOT an IM message."),
    channelType: z.string().optional().describe("Channel type for IM message: telegram, feishu, wecom, dingtalk, discord, slack. Use this for sending messages back to IM."),
    channelId: z.string().optional().describe("Channel/chat ID to send the message to. Required when channelType is specified."),
    channelMessage: z.string().optional().describe("Message text to send to the channel. Required when channelType is specified."),
    endpoint: z.string().optional().describe("API endpoint URL (for api-type tasks)"),
    method: z.enum(["GET", "POST", "PUT", "DELETE"]).optional().describe("HTTP method (for api-type tasks)"),
    shellCommand: z.string().optional().describe("Shell command (for shell-type tasks)"),
    description: z.string().max(512).optional().describe("Human-readable description of the task"),
  }),
  async execute(params, ctx) {
    // Calculate delay
    let delayMs: number
    if (params.delayMinutes) {
      delayMs = params.delayMinutes * 60 * 1000
    } else if (params.delaySeconds) {
      delayMs = params.delaySeconds * 1000
    } else {
      throw new Error("Must provide either delayMinutes or delaySeconds")
    }

    // Calculate target time (round to next minute for cron)
    const now = new Date()
    const targetTime = new Date(now.getTime() + delayMs)
    // Round up to next minute
    targetTime.setSeconds(0, 0)
    if (targetTime.getTime() <= now.getTime()) {
      targetTime.setMinutes(targetTime.getMinutes() + 1)
    }

    // Generate unique task ID
    const taskId = `delayed-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`

    // Build cron expression for exact time (minute hour day month weekday year)
    // Using 7-field cron: second minute hour day month weekday year
    const minute = targetTime.getUTCMinutes()
    const hour = targetTime.getUTCHours()
    const day = targetTime.getUTCDate()
    const month = targetTime.getUTCMonth() + 1
    const year = targetTime.getUTCFullYear()
    // 6-field cron (second minute hour day month weekday)
    const expression = `0 ${minute} ${hour} ${day} ${month} *`

    await ctx.ask({
      permission: "scheduler",
      patterns: [taskId],
      always: ["*"],
      metadata: {
        action: "delay_task",
        taskId,
        targetTime: targetTime.toISOString(),
        delayMinutes: params.delayMinutes,
        delaySeconds: params.delaySeconds,
      },
    })

    // Determine command type - channel_message takes priority for IM delivery
    // Auto-detect channel info from context if not explicitly provided
    const autoChannelType = ctx.extra?.channelType as string | undefined
    const autoChannelId = ctx.extra?.channelId as string | undefined

    let command: TaskCommand
    if (params.channelType && params.channelId && params.channelMessage) {
      // Explicit IM channel message - sends directly to the channel
      command = { type: "channel_message", channelType: params.channelType, channelId: params.channelId, message: params.channelMessage }
    } else if (params.channelMessage && autoChannelType && autoChannelId) {
      // Auto-detected channel from context + explicit message
      command = { type: "channel_message", channelType: autoChannelType, channelId: autoChannelId, message: params.channelMessage }
    } else if (params.message && autoChannelType && autoChannelId) {
      // Simple message shortcut with auto-detected channel - sends to IM
      command = { type: "channel_message", channelType: autoChannelType, channelId: autoChannelId, message: params.message }
    } else if (params.message) {
      // Simple message shortcut without channel context - creates local session only
      command = { type: "agent", agentName: "general", prompt: params.message }
    } else if (params.agentName && params.prompt) {
      // Agent task - automatically include callback channel info from context so results are sent back to IM
      command = {
        type: "agent",
        agentName: params.agentName,
        prompt: params.prompt,
        callbackChannelType: autoChannelType,
        callbackChannelId: autoChannelId,
      }
    } else if (params.endpoint && params.method) {
      command = { type: "api", endpoint: params.endpoint, method: params.method }
    } else if (params.shellCommand) {
      command = { type: "shell", command: params.shellCommand }
    } else {
      throw new Error("Must provide (channelType + channelId + channelMessage), message, (agentName + prompt), (endpoint + method), or shellCommand")
    }

    const delayDesc = params.delayMinutes
      ? `${params.delayMinutes} minute(s)`
      : `${params.delaySeconds} second(s)`

    const payload = {
      id: taskId,
      expression,
      command,
      description: params.description || `One-time task scheduled for ${targetTime.toISOString()}`,
      enabled: true,
    }

    log.info("creating delayed task", {
      taskId,
      targetTime: targetTime.toISOString(),
      expression,
      commandType: command.type,
    })

    const result = await callSchedulerApi<{ id: string }>("/api/v1/scheduler/tasks", {
      method: "POST",
      body: JSON.stringify(payload),
    })

    if (result.data) {
      return {
        title: `Scheduled task for ${delayDesc} from now`,
        metadata: {
          taskId: result.data.id,
          targetTime: targetTime.toISOString(),
          expression,
          commandType: command.type,
        },
        output: `Task scheduled to run at ${targetTime.toISOString()} (in ${delayDesc}).\n\nTask ID: ${taskId}\nType: ${command.type}\n${params.description ? `Description: ${params.description}` : ""}`,
      }
    }

    throw new Error(result.error || "Failed to create delayed task")
  },
})
