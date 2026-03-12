/**
 * Edit Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const EditTool = {
  name: "Edit",
  description: "Edit file contents",
  input: z.object({
    file_path: z.string(),
    filePath: z.string().optional(), // Alias for compatibility
    old_string: z.string(),
    new_string: z.string(),
    replace_all: z.boolean().optional(),
    replaceAll: z.boolean().optional(), // Alias for compatibility
  }),
  output: z.object({
    success: z.boolean().optional(),
  }),
  metadata: z.object({
    replacements: z.number().optional(),
    diff: z.string().optional(),
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
