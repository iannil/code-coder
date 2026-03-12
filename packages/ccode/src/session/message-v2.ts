/**
 * MessageV2 Type Stubs
 * @deprecated This module has been moved to Rust. Use @/types instead.
 */

export namespace MessageV2 {
  export interface ToolState {
    status?: string
    title?: string
    input?: Record<string, unknown>
    output?: string
  }

  export interface Part {
    type: string
    sessionID?: string
    tool?: string
    state?: ToolState
    text?: string
    time?: { start?: number; end?: number }
    [key: string]: unknown
  }

  export interface WithParts {
    info: { id: string }
    parts: Part[]
  }

  export interface Assistant {
    id: string
    role: "assistant"
    sessionID: string
    parentID?: string
    modelID?: string
    providerID?: string
    mode?: string
    agent?: string
    path?: { cwd: string; root: string }
    cost?: number
    tokens?: {
      input: number
      output: number
      reasoning: number
      cache: { read: number; write: number }
    }
    parts?: Part[]
    time?: { created: number }
    model?: { providerID: string; modelID: string }
  }

  export interface User {
    id: string
    role: "user"
    sessionID: string
    parts: Part[]
    time?: { created: number }
  }

  export const Event = {
    PartUpdated: { type: "message.part.updated" as const },
  }
}
