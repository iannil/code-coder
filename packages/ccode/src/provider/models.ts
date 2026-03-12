/**
 * Models Type Stubs
 * @deprecated Models are now implemented in Rust.
 */

export interface ModelInfo {
  id: string
  name: string
  provider: string
  contextWindow?: number
  pricing?: { input: number; output: number }
}

export const Models: ModelInfo[] = []

// Development models namespace
export namespace ModelsDev {
  export interface Model {
    id: string
    name: string
    provider: string
    status?: string
    [key: string]: unknown
  }

  export interface Provider {
    id: string
    name: string
    models: Model[]
    status?: "connected" | "disconnected" | "error"
  }

  export const Data: Record<string, Provider> = {}

  // Value exports for runtime access
  export const Provider = {} as Record<string, Provider>
  export const Model = {} as Record<string, Model>

  export function list(): Model[] {
    return []
  }

  export function get(_id: string): Model | undefined {
    return undefined
  }

  export async function refresh(): Promise<void> {
    // No-op stub
  }
}
