/**
 * Read Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const ReadTool = {
  name: "Read",
  description: "Read file contents",
  input: z.object({
    file_path: z.string(),
    filePath: z.string().optional(), // Alias for compatibility
    offset: z.number().optional(),
    limit: z.number().optional(),
  }),
  output: z.object({
    content: z.string().optional(),
  }),
  metadata: z.object({
    content: z.string().optional(),
    lineCount: z.number().optional(),
  }),
} satisfies Tool
