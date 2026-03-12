/**
 * TodoWrite Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const TodoWriteTool = {
  name: "TodoWrite",
  description: "Write todo items",
  input: z.object({
    todos: z.array(
      z.object({
        id: z.string(),
        content: z.string(),
        status: z.enum(["pending", "in_progress", "completed"]).optional(),
      }),
    ),
  }),
  output: z.object({
    success: z.boolean().optional(),
  }),
  metadata: z.object({
    count: z.number().optional(),
    todos: z
      .array(
        z.object({
          id: z.string(),
          content: z.string(),
          status: z.enum(["pending", "in_progress", "completed"]).optional(),
        }),
      )
      .optional(),
  }),
} satisfies Tool
