/**
 * Task Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const TaskTool = {
  name: "Task",
  description: "Launch a task agent",
  input: z.object({
    description: z.string(),
    prompt: z.string(),
    subagent_type: z.string(),
  }),
  output: z.object({
    result: z.string().optional(),
  }),
  metadata: z.object({
    agentId: z.string().optional(),
    status: z.string().optional(),
    sessionId: z.string().optional(),
    summary: z
      .array(
        z.object({
          type: z.string(),
          tool: z.string().optional(),
          content: z.string().optional(),
          state: z.object({
            status: z.enum(["pending", "running", "completed", "error"]).optional(),
            error: z.string().optional(),
          }).optional(),
        }),
      )
      .optional(),
  }),
} satisfies Tool
