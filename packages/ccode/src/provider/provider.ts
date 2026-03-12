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
    constructor(modelId: string) {
      super(`Model not found: ${modelId}`)
      this.name = "ModelNotFoundError"
    }
    static isInstance(error: unknown): error is ModelNotFoundError {
      return error instanceof ModelNotFoundError
    }
  }

  export class InitError extends Error {
    constructor(message: string) {
      super(message)
      this.name = "ProviderInitError"
    }
    static isInstance(error: unknown): error is InitError {
      return error instanceof InitError
    }
  }

  export async function listAll(): Promise<{ all: Info[]; default?: string; connected: string[] }> {
    return { all: [], connected: [] }
  }

  export async function list(): Promise<Info[]> {
    return []
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
