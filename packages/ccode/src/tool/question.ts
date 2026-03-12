/**
 * Question Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const QuestionTool = {
  name: "AskUserQuestion",
  description: "Ask the user a question",
  input: z.object({
    questions: z.array(
      z.object({
        question: z.string(),
        header: z.string(),
        multiSelect: z.boolean().optional(),
        options: z.array(
          z.object({
            label: z.string(),
            description: z.string().optional(),
          }),
        ),
      }),
    ),
  }),
  output: z.object({
    answers: z.record(z.string(), z.string()).optional(),
  }),
  metadata: z.object({
    answers: z.record(z.string(), z.string()).optional(),
  }),
} satisfies Tool
