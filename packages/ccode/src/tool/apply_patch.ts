/**
 * Apply Patch Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const ApplyPatchTool = {
  name: "ApplyPatch",
  description: "Apply a unified diff patch",
  input: z.object({
    patch: z.string(),
  }),
  output: z.object({
    success: z.boolean().optional(),
  }),
  metadata: z.object({
    filesChanged: z.number().optional(),
    files: z
      .array(
        z.object({
          type: z.string(),
          relativePath: z.string(),
          filePath: z.string(),
          deletions: z.number(),
          additions: z.number().optional(),
          diff: z.string().optional(),
        }),
      )
      .optional(),
  }),
} satisfies Tool
