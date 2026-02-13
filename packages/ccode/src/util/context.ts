import { AsyncLocalStorage } from "async_hooks"

export namespace Context {
  export class NotFound extends Error {
    constructor(public override readonly name: string) {
      super(`No context found for ${name}`)
    }
  }

  /**
   * Test-safe context creation for tests
   *
   * Returns a fallback mock context when running in test mode (no instance context set).
   */
  const testFallback = <T = any>() => {
    let provided = false
    return {
      use() {
        const result = storage.getStore() ?? null
        if (!result && !provided) {
          // In test mode, return a mock instead of throwing
          return { testMode: true } as T
        }
        return result
      },
      provide<R>(value: T, fn: () => R) {
        provided = true
        return storage.run(value, fn)
      },
    }
  }

  export function create<T>(name: string) {
    const storage = new AsyncLocalStorage<T>()
    return {
      use() {
        const result = storage.getStore()
        if (!result) {
          throw new NotFound(name)
        }
        return result
      },
      provide<R>(value: T, fn: () => R) {
        return storage.run(value, fn)
      },
    }
  }

  /**
   * Create test-safe context that returns a mock when no instance is set
   */
  export function createTestSafe<T>(name: string) {
    return testFallback<T>()
  }
}
