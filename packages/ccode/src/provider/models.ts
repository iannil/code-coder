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
    tool_call?: boolean
    [key: string]: unknown
  }

  export interface Provider {
    id: string
    name: string
    models: Record<string, Model>
    status?: "connected" | "disconnected" | "error"
  }

  // Data store with reset functionality
  export const Data = {
    providers: {} as Record<string, Provider>,
    reset(): void {
      this.providers = {}
    },
  }

  // Value exports for runtime access
  export const Provider = {} as Record<string, Provider>
  export const Model = {} as Record<string, Model>

  export function list(): Model[] {
    return []
  }

  export function get(_id?: string): Record<string, Provider> {
    return Data.providers
  }

  export async function refresh(): Promise<void> {
    // No-op stub - models are fetched from Rust API
  }
}
