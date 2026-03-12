/**
 * Glob Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const GlobTool = {
  name: "Glob",
  description: "Find files matching a pattern",
  input: z.object({
    pattern: z.string(),
    path: z.string().optional(),
  }),
  output: z.object({
    files: z.array(z.string()).optional(),
  }),
  metadata: z.object({
    files: z.array(z.string()).optional(),
    count: z.number().optional(),
  }),
} satisfies Tool
