/**
 * WebFetch Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export const WebFetchTool = {
  name: "WebFetch",
  description: "Fetch content from a URL",
  input: z.object({
    url: z.string(),
    prompt: z.string().optional(),
  }),
  output: z.object({
    content: z.string().optional(),
  }),
  metadata: z.object({
    content: z.string().optional(),
    statusCode: z.number().optional(),
  }),
  // Stub init method for compatibility
  init: () => Promise.resolve(),
} satisfies Tool & { init: () => Promise<void> }
