/**
 * useDebounce Hook
 *
 * Debounces a value, delaying updates until after a specified delay has passed
 * since the last change. Useful for search inputs, API calls, and other
 * performance-sensitive scenarios.
 */

import * as React from "react"

// ============================================================================
// Hook Implementation
// ============================================================================

export interface UseDebounceOptions {
  /**
   * Delay in milliseconds before updating the debounced value
   * @default 500
   */
  delay?: number

  /**
   * Whether to debounce on the leading edge (immediate first call)
   * @default false
   */
  leading?: boolean

  /**
   * Maximum time to wait before invoking the function
   */
  maxWait?: number
}

export function useDebounce<T>(value: T, delay: number = 500): T {
  const [debouncedValue, setDebouncedValue] = React.useState<T>(value)

  React.useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value)
    }, delay)

    return () => {
      clearTimeout(handler)
    }
  }, [value, delay])

  return debouncedValue
}

// ============================================================================
// Debounced Callback Hook
// ============================================================================

export interface DebouncedState {
  /**
   * The debounced callback function
   */
  run: () => void

  /**
   * Cancel any pending debounced call
   */
  cancel: () => void

  /**
   * Immediately invoke the debounced function
   */
  flush: () => void

  /**
   * Whether there is a pending call
   */
  pending: boolean
}

/**
 * Creates a debounced version of a callback function
 *
 * @param callback - Function to debounce
 * @param delay - Delay in milliseconds (default: 500)
 * @returns Debounced state object with run, cancel, and flush methods
 */
export function useDebouncedCallback(
  callback: () => void,
  delay: number = 500,
): DebouncedState {
  const timeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)
  const pendingRef = React.useRef(false)
  const callbackRef = React.useRef(callback)

  // Keep callback ref updated
  React.useEffect(() => {
    callbackRef.current = callback
  }, [callback])

  const run = React.useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }

    pendingRef.current = true
    timeoutRef.current = setTimeout(() => {
      callbackRef.current()
      pendingRef.current = false
      timeoutRef.current = null
    }, delay)
  }, [delay])

  const cancel = React.useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    pendingRef.current = false
  }, [])

  const flush = React.useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    if (pendingRef.current) {
      callbackRef.current()
      pendingRef.current = false
    }
  }, [])

  // Cleanup on unmount
  React.useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  return {
    run,
    cancel,
    flush,
    pending: pendingRef.current,
  }
}

// ============================================================================
// Debounced Value Hook with Controls
// ============================================================================

export interface DebouncedValueControls<T> {
  /**
   * The current debounced value
   */
  value: T

  /**
   * Whether the value is updating (pending)
   */
  isPending: boolean

  /**
   * Immediately set the value without debounce
   */
  flush: (value: T) => void

  /**
   * Cancel pending update
   */
  cancel: () => void
}

/**
 * Extended debounce hook with manual controls
 *
 * @param initialValue - Initial value
 * @param delay - Delay in milliseconds (default: 500)
 * @returns Debounced value with controls
 */
export function useDebounceValue<T>(
  initialValue: T,
  delay: number = 500,
): DebouncedValueControls<T> {
  const [value, setValue] = React.useState<T>(initialValue)
  const [pendingValue, setPendingValue] = React.useState<T | null>(null)
  const timeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)

  React.useEffect(() => {
    if (pendingValue === null) return

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }

    timeoutRef.current = setTimeout(() => {
      setValue(pendingValue)
      setPendingValue(null)
    }, delay)

    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [pendingValue, delay])

  const flush = React.useCallback((newValue: T) => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }
    setValue(newValue)
    setPendingValue(null)
  }, [])

  const cancel = React.useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }
    setPendingValue(null)
  }, [])

  return {
    value,
    isPending: pendingValue !== null,
    flush,
    cancel,
  }
}
