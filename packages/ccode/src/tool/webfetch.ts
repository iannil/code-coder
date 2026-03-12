/**
 * WebFetch Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import { z } from "zod"
import type { Tool } from "./tool"

export interface WebFetchExecutor {
  execute: (
    input: { url: string; format?: string },
    ctx: {
      sessionID: string
      messageID: string
      agent: string
      abort: AbortSignal
      ask: (req: unknown) => Promise<unknown>
      metadata: () => void
    }
  ) => Promise<{ output?: string }>
}

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
  init: async (_options: Record<string, unknown>): Promise<WebFetchExecutor> => ({
    execute: async () => ({ output: undefined }),
  }),
} satisfies Tool & { init: (options: Record<string, unknown>) => Promise<WebFetchExecutor> }
