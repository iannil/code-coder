// Registry of all lazy instances for test reset
const lazyRegistry = new Set<Lazy<unknown>>()

export function lazy<T>(fn: () => T): Lazy<T> {
  let value: T | undefined
  let loaded = false

  const result = (): T => {
    if (loaded) return value as T
    loaded = true
    value = fn()
    return value as T
  }

  result.reset = () => {
    loaded = false
    value = undefined
  }

  // Register for global reset
  lazyRegistry.add(result as Lazy<unknown>)

  return result
}

export type Lazy<T> = (() => T) & {
  reset: () => void
}

/**
 * Reset all lazy singletons - used for test isolation.
 * This clears all cached lazy values, causing them to be recomputed on next access.
 */
export function resetAllLazy() {
  for (const lazy of lazyRegistry) {
    lazy.reset()
  }
}
