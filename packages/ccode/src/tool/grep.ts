/**
 * Grep Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const GrepTool = {
  name: "Grep",
  description: "Search for patterns in files",
  input: z.object({
    pattern: z.string(),
    path: z.string().optional(),
    glob: z.string().optional(),
  }),
  output: z.object({
    matches: z.array(z.string()).optional(),
  }),
  metadata: z.object({
    matches: z.array(z.string()).optional(),
    count: z.number().optional(),
  }),
} satisfies Tool
