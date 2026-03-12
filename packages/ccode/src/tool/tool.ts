/**
 * Tool Type Stubs
 * @deprecated Tools are now implemented in Rust.
 */

import type { z } from "zod"

export interface ToolInfo {
  name: string
  description: string
  input?: z.ZodType
  output?: z.ZodType
  metadata?: z.ZodType
}

export namespace Tool {
  export interface Info extends ToolInfo {}

  export type InferParameters<T> = T extends { input: z.ZodType<infer P> } ? P : Record<string, unknown>

  export type InferMetadata<T> = T extends { metadata: z.ZodType<infer M> } ? M : Record<string, unknown>

  export type InferOutput<T> = T extends { output: z.ZodType<infer O> } ? O : unknown

  export type Permission = {
    allowed: boolean
    reason?: string
  }

  export type InputType<T> = InferParameters<T>
  export type OutputType<T> = InferOutput<T>

  export interface Context {
    sessionID: string
    messageID?: string
    partID?: string
    directory?: string
    [key: string]: unknown
  }
}

export type Tool = ToolInfo
