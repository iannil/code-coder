/**
 * MessageV2 Type Stubs
 * @deprecated This module has been moved to Rust. Use @/types instead.
 */

export namespace MessageV2 {
  export interface Part {
    type: string
    sessionID?: string
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
    parts: Part[]
    time?: { created: number }
    model?: { providerID: string; modelID: string }
    agent?: string
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
