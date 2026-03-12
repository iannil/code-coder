/**
 * LS Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const LsTool = {
  name: "LS",
  description: "List directory contents",
  input: z.object({
    path: z.string().optional(),
  }),
  output: z.object({
    entries: z.array(z.string()).optional(),
  }),
  metadata: z.object({
    entries: z.array(z.string()).optional(),
    count: z.number().optional(),
  }),
} satisfies Tool

// Alias for backwards compatibility
export const ListTool = LsTool
