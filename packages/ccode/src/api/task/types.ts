/**
 * Task API Type Definitions
 * Types for the async task flow model supporting ZeroBot integration
 */

import z from "zod"
import { Identifier } from "@/id/id"

// ============================================================================
// Task Status
// ============================================================================

export const TaskStatus = z.enum(["pending", "running", "awaiting_approval", "completed", "failed"])
export type TaskStatus = z.infer<typeof TaskStatus>

// ============================================================================
// Task Context
// ============================================================================

export const TaskContext = z
  .object({
    /** ZeroBot user identifier */
    userID: z.string(),
    /** Platform source: telegram, discord, slack, etc. */
    platform: z.string(),
    /** Conversation identifier for session continuity (e.g., "telegram:765318302") */
    conversationId: z.string().optional(),
    /** Recent chat history for context */
    chatHistory: z.array(z.any()).optional(),
    /** Marker for remote calls - always "remote" for ZeroBot */
    source: z.literal("remote"),
  })
  .meta({
    ref: "TaskContext",
  })
export type TaskContext = z.infer<typeof TaskContext>

// ============================================================================
// Task Definition
// ============================================================================

export const Task = z
  .object({
    /** Unique task identifier */
    id: Identifier.schema("task"),
    /** Associated session ID */
    sessionID: Identifier.schema("session"),
    /** Current task status */
    status: TaskStatus,
    /** Agent name to invoke */
    agent: z.string(),
    /** User prompt/request */
    prompt: z.string(),
    /** Remote context information */
    context: TaskContext,
    /** Final output (set on completion) */
    output: z.string().optional(),
    /** Error message (set on failure) */
    error: z.string().optional(),
    /** Creation timestamp */
    createdAt: z.string().datetime(),
    /** Last update timestamp */
    updatedAt: z.string().datetime(),
  })
  .meta({
    ref: "Task",
  })
export type Task = z.infer<typeof Task>

// ============================================================================
// Task Events
// ============================================================================

export const ThoughtEvent = z.object({
  type: z.literal("thought"),
  data: z.string(),
})
export type ThoughtEvent = z.infer<typeof ThoughtEvent>

export const ToolUseEvent = z.object({
  type: z.literal("tool_use"),
  data: z.object({
    tool: z.string(),
    args: z.any(),
    result: z.any().optional(),
  }),
})
export type ToolUseEvent = z.infer<typeof ToolUseEvent>

export const OutputEvent = z.object({
  type: z.literal("output"),
  data: z.string(),
})
export type OutputEvent = z.infer<typeof OutputEvent>

export const ConfirmationRequest = z.object({
  /** Unique confirmation request ID */
  requestID: z.string(),
  /** Tool that requires approval */
  tool: z.string(),
  /** Human-readable description of the operation */
  description: z.string(),
  /** Arguments being passed to the tool */
  args: z.any(),
  /** Available actions: once, always, reject */
  actions: z.array(z.string()),
})
export type ConfirmationRequest = z.infer<typeof ConfirmationRequest>

export const ConfirmationEvent = z.object({
  type: z.literal("confirmation"),
  data: ConfirmationRequest,
})
export type ConfirmationEvent = z.infer<typeof ConfirmationEvent>

export const FinishEvent = z.object({
  type: z.literal("finish"),
  data: z.object({
    success: z.boolean(),
    output: z.string().optional(),
    error: z.string().optional(),
  }),
})
export type FinishEvent = z.infer<typeof FinishEvent>

export const ProgressEvent = z.object({
  type: z.literal("progress"),
  data: z.object({
    stage: z.string(),
    message: z.string(),
    percentage: z.number().min(0).max(100).optional(),
  }),
})
export type ProgressEvent = z.infer<typeof ProgressEvent>

export const DebugInfoEvent = z.object({
  type: z.literal("debug_info"),
  data: z.object({
    model: z.string().optional(),
    provider: z.string().optional(),
    input_tokens: z.number().optional(),
    output_tokens: z.number().optional(),
    total_tokens: z.number().optional(),
    duration_ms: z.number().optional(),
    request_bytes: z.number().optional(),
    response_bytes: z.number().optional(),
  }),
})
export type DebugInfoEvent = z.infer<typeof DebugInfoEvent>

export const AgentInfoEvent = z.object({
  type: z.literal("agent_info"),
  data: z.object({
    agent: z.string(),
    display_name: z.string().optional(),
    is_primary: z.boolean().optional(),
    duration_ms: z.number().optional(),
  }),
})
export type AgentInfoEvent = z.infer<typeof AgentInfoEvent>

export const TaskEvent = z.discriminatedUnion("type", [
  ThoughtEvent,
  ToolUseEvent,
  OutputEvent,
  ConfirmationEvent,
  FinishEvent,
  ProgressEvent,
  DebugInfoEvent,
  AgentInfoEvent,
])
export type TaskEvent = z.infer<typeof TaskEvent>

// ============================================================================
// API Request/Response Types
// ============================================================================

export const CreateTaskRequest = z
  .object({
    /** Agent name to invoke */
    agent: z.string(),
    /** User prompt/request */
    prompt: z.string(),
    /** Remote context information */
    context: TaskContext,
    /** Optional existing session ID for continuity */
    sessionID: z.string().optional(),
    /** Optional model override */
    model: z.string().optional(),
  })
  .meta({
    ref: "CreateTaskRequest",
  })
export type CreateTaskRequest = z.infer<typeof CreateTaskRequest>

export const InteractTaskRequest = z
  .object({
    /** Action to take: approve or reject */
    action: z.enum(["approve", "reject"]),
    /** Optional rejection reason */
    reason: z.string().optional(),
    /** Reply type: once, always, or reject */
    reply: z.enum(["once", "always", "reject"]).optional(),
  })
  .meta({
    ref: "InteractTaskRequest",
  })
export type InteractTaskRequest = z.infer<typeof InteractTaskRequest>

export const TaskResponse = z
  .object({
    success: z.boolean(),
    data: Task.optional(),
    error: z.string().optional(),
  })
  .meta({
    ref: "TaskResponse",
  })
export type TaskResponse = z.infer<typeof TaskResponse>

export const TaskListResponse = z
  .object({
    success: z.boolean(),
    data: z.array(Task).optional(),
    error: z.string().optional(),
  })
  .meta({
    ref: "TaskListResponse",
  })
export type TaskListResponse = z.infer<typeof TaskListResponse>
