/**
 * useLocalStorage Hook
 *
 * Synchronizes state with localStorage. Provides type-safe serialization
 * and handles JSON parsing errors gracefully.
 */

import * as React from "react"

// ============================================================================
// Type Definitions
// ============================================================================

export interface UseLocalStorageOptions<T> {
  /**
   * Custom serializer function
   * @default JSON.stringify
   */
  serializer?: (value: T) => string

  /**
   * Custom deserializer function
   * @default JSON.parse
   */
  deserializer?: (value: string) => T

  /**
   * Callback when value changes
   */
  onUpdate?: (value: T) => void

  /**
   * Synchronize across tabs/windows
   * @default true
   */
  syncAcrossTabs?: boolean

  /**
   * Storage key prefix
   */
  prefix?: string
}

export type LocalStorageReturn<T> = [
  T,
  (value: T | ((prev: T) => T)) => void,
  () => void,
]

// ============================================================================
// Hook Implementation
// ============================================================================

const DEFAULT_PREFIX = "ccode"

function getStorageKey(key: string, prefix?: string): string {
  return prefix ? `${prefix}:${key}` : `${DEFAULT_PREFIX}:${key}`
}

/**
 * Hook to synchronize state with localStorage
 *
 * @param key - Storage key
 * @param initialValue - Default value if key doesn't exist
 * @param options - Configuration options
 * @returns [storedValue, setValue, removeValue] tuple
 *
 * @example
 * ```tsx
 * const [theme, setTheme, removeTheme] = useLocalStorage("theme", "light")
 * ```
 */
export function useLocalStorage<T>(
  key: string,
  initialValue: T,
  options: UseLocalStorageOptions<T> = {},
): LocalStorageReturn<T> {
  const {
    serializer = JSON.stringify,
    deserializer = JSON.parse,
    onUpdate,
    syncAcrossTabs = true,
    prefix,
  } = options

  const storageKey = getStorageKey(key, prefix)

  // Get initial value from localStorage or use initialValue
  const readValue = React.useCallback((): T => {
    if (typeof window === "undefined") {
      return initialValue
    }

    try {
      const item = window.localStorage.getItem(storageKey)
      return item ? deserializer(item) : initialValue
    } catch (error) {
      console.warn(`Error reading localStorage key "${storageKey}":`, error)
      return initialValue
    }
  }, [initialValue, storageKey, deserializer])

  const [storedValue, setStoredValue] = React.useState<T>(readValue)

  // Return a wrapped version of useState's setter function that
  // persists the new value to localStorage
  const setValue = React.useCallback(
    (value: T | ((prev: T) => T)) => {
      if (typeof window === "undefined") {
        console.warn("Cannot set localStorage in server-side context")
        return
      }

      try {
        const valueToStore = value instanceof Function ? value(storedValue) : value
        setStoredValue(valueToStore)
        window.localStorage.setItem(storageKey, serializer(valueToStore))

        // Trigger custom callbacks
        if (syncAcrossTabs) {
          window.dispatchEvent(new StorageEvent("local-storage", {
            key: storageKey,
            newValue: serializer(valueToStore),
          }))
        }

        onUpdate?.(valueToStore)
      } catch (error) {
        console.warn(`Error setting localStorage key "${storageKey}":`, error)
      }
    },
    [storedValue, storageKey, serializer, syncAcrossTabs, onUpdate],
  )

  // Remove value from localStorage
  const removeValue = React.useCallback(() => {
    if (typeof window === "undefined") {
      return
    }

    try {
      window.localStorage.removeItem(storageKey)
      setStoredValue(initialValue)

      if (syncAcrossTabs) {
        window.dispatchEvent(new StorageEvent("local-storage", {
          key: storageKey,
          newValue: null,
        }))
      }
    } catch (error) {
      console.warn(`Error removing localStorage key "${storageKey}":`, error)
    }
  }, [storageKey, initialValue, syncAcrossTabs])

  // Listen for changes in other tabs
  React.useEffect(() => {
    if (!syncAcrossTabs || typeof window === "undefined") {
      return
    }

    const handleStorageChange = (e: StorageEvent | CustomEvent) => {
      if ("key" in e && e.key !== storageKey) {
        return
      }

      try {
        const newValue = "newValue" in e && e.newValue !== null
          ? deserializer(e.newValue)
          : initialValue
        setStoredValue(newValue)
      } catch (error) {
        console.warn(`Error parsing storage event for "${storageKey}":`, error)
      }
    }

    // Standard storage event (only works for other tabs)
    window.addEventListener("storage", handleStorageChange as EventListener)

    // Custom event for same-tab updates
    window.addEventListener("local-storage", handleStorageChange as EventListener)

    return () => {
      window.removeEventListener("storage", handleStorageChange as EventListener)
      window.removeEventListener("local-storage", handleStorageChange as EventListener)
    }
  }, [storageKey, initialValue, deserializer, syncAcrossTabs])

  return [storedValue, setValue, removeValue]
}

// ============================================================================
// Variants for Specific Types
// ============================================================================

/**
 * Hook for boolean values in localStorage
 */
export function useLocalStorageBoolean(
  key: string,
  initialValue: boolean = false,
  options?: Omit<UseLocalStorageOptions<boolean>, "serializer" | "deserializer">,
): LocalStorageReturn<boolean> {
  return useLocalStorage(key, initialValue, {
    ...options,
    serializer: (v) => String(v),
    deserializer: (v) => v === "true",
  })
}

/**
 * Hook for number values in localStorage
 */
export function useLocalStorageNumber(
  key: string,
  initialValue: number = 0,
  options?: Omit<UseLocalStorageOptions<number>, "serializer" | "deserializer">,
): LocalStorageReturn<number> {
  return useLocalStorage(key, initialValue, {
    ...options,
    serializer: (v) => String(v),
    deserializer: (v) => Number(v),
  })
}

/**
 * Hook for string values in localStorage (no JSON serialization)
 */
export function useLocalStorageString(
  key: string,
  initialValue: string = "",
  options?: Omit<UseLocalStorageOptions<string>, "serializer" | "deserializer">,
): LocalStorageReturn<string> {
  return useLocalStorage(key, initialValue, {
    ...options,
    serializer: (v) => v,
    deserializer: (v) => v,
  })
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Check if localStorage is available
 */
export function isLocalStorageAvailable(): boolean {
  if (typeof window === "undefined") {
    return false
  }

  try {
    const test = "__storage_test__"
    window.localStorage.setItem(test, test)
    window.localStorage.removeItem(test)
    return true
  } catch {
    return false
  }
}

/**
 * Get a value from localStorage without using the hook
 */
export function getLocalStorageItem<T>(key: string, prefix?: string): T | null {
  if (typeof window === "undefined" || !isLocalStorageAvailable()) {
    return null
  }

  try {
    const storageKey = getStorageKey(key, prefix)
    const item = window.localStorage.getItem(storageKey)
    return item ? JSON.parse(item) : null
  } catch {
    return null
  }
}

/**
 * Set a value in localStorage without using the hook
 */
export function setLocalStorageItem<T>(key: string, value: T, prefix?: string): void {
  if (typeof window === "undefined" || !isLocalStorageAvailable()) {
    return
  }

  try {
    const storageKey = getStorageKey(key, prefix)
    window.localStorage.setItem(storageKey, JSON.stringify(value))
  } catch (error) {
    console.warn(`Error setting localStorage key "${key}":`, error)
  }
}

/**
 * Remove a value from localStorage without using the hook
 */
export function removeLocalStorageItem(key: string, prefix?: string): void {
  if (typeof window === "undefined" || !isLocalStorageAvailable()) {
    return
  }

  try {
    const storageKey = getStorageKey(key, prefix)
    window.localStorage.removeItem(storageKey)
  } catch (error) {
    console.warn(`Error removing localStorage key "${key}":`, error)
  }
}
