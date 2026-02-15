/**
 * Custom Hooks Index
 *
 * Exports all custom hooks for the CodeCoder web application
 */

// SSE Hook
export {
  useSSE,
  useSSEStatus,
  useSSEMessages,
  type UseSSEOptions,
  type UseSSEReturn,
} from "./use-sse"

// Debounce Hooks
export {
  useDebounce,
  useDebouncedCallback,
  useDebounceValue,
  type UseDebounceOptions,
  type DebouncedState,
  type DebouncedValueControls,
} from "./use-debounce"

// Local Storage Hooks
export {
  useLocalStorage,
  useLocalStorageBoolean,
  useLocalStorageNumber,
  useLocalStorageString,
  getLocalStorageItem,
  setLocalStorageItem,
  removeLocalStorageItem,
  isLocalStorageAvailable,
  type UseLocalStorageOptions,
  type LocalStorageReturn,
} from "./use-local-storage"

// API Hooks
export {
  useAPI,
  useAPILazy,
  useAPIClient,
  useAPIWithConfig,
  isApiError,
  isNetworkError,
  isTimeoutError,
  type UseAPIOptions,
  type UseAPIReturn,
  type ExecuteOptions,
} from "./use-api"

// Toast Hook (re-export for convenience)
export { useToast, toast } from "./use-toast"
