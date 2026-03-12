/**
 * Bash Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const BashTool = {
  name: "Bash",
  description: "Execute bash commands",
  input: z.object({
    command: z.string(),
    workdir: z.string().optional(),
    description: z.string().optional(),
    timeout: z.number().optional(),
  }),
  output: z.object({
    output: z.string().optional(),
    exitCode: z.number().optional(),
  }),
  metadata: z.object({
    output: z.string().optional(),
    exitCode: z.number().optional(),
  }),
} satisfies Tool
