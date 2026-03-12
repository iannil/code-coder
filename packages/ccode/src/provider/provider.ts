/**
 * Provider Type Stubs
 * @deprecated Providers are now implemented in Rust.
 */

export namespace Provider {
  export interface Info {
    id: string
    name: string
  }

  export interface ModelInfo {
    id: string
    name: string
    provider: string
    contextWindow?: number
    pricing?: { input: number; output: number }
  }

  export class ModelNotFoundError extends Error {
    readonly data: { providerID: string; modelID: string; suggestions?: string[] }
    constructor(providerID: string, modelID: string, suggestions?: string[]) {
      super(`Model not found: ${providerID}/${modelID}`)
      this.name = "ModelNotFoundError"
      this.data = { providerID, modelID, suggestions }
    }
    static isInstance(error: unknown): error is ModelNotFoundError {
      return error instanceof ModelNotFoundError
    }
  }

  export class InitError extends Error {
    readonly data: { providerID: string }
    constructor(providerID: string, message?: string) {
      super(message ?? `Failed to initialize provider: ${providerID}`)
      this.name = "ProviderInitError"
      this.data = { providerID }
    }
    static isInstance(error: unknown): error is InitError {
      return error instanceof InitError
    }
  }

  export async function listAll(): Promise<{ all: Info[]; default?: string; connected: string[] }> {
    return { all: [], connected: [] }
  }

  export async function list(): Promise<Record<string, { models: Record<string, unknown> }>> {
    return {}
  }

  export function authMethods(): Record<string, { type: "oauth" | "api"; label: string }[]> {
    return {}
  }

  export function defaultModel(): string | undefined {
    return undefined
  }

  export async function getModel(_modelId: string): Promise<ModelInfo | undefined> {
    return undefined
  }
}
