/**
 * Write Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const WriteTool = {
  name: "Write",
  description: "Write file contents",
  input: z.object({
    file_path: z.string(),
    filePath: z.string().optional(), // Alias for compatibility
    content: z.string(),
  }),
  output: z.object({
    success: z.boolean().optional(),
  }),
  metadata: z.object({
    bytesWritten: z.number().optional(),
    diagnostics: z
      .array(
        z.object({
          line: z.number(),
          column: z.number(),
          message: z.string(),
          severity: z.enum(["error", "warning", "info"]).optional(),
        }),
      )
      .optional(),
  }),
} satisfies Tool
