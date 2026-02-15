/**
 * useAPI Hook
 *
 * Provides access to the API client with error handling wrapper.
 * Integrates with toast notifications for user feedback.
 */

import * as React from "react"
import { getClient, ApiClient, type ApiClientConfig, ApiError, NetworkError, TimeoutError } from "../lib/api"
import { useToast } from "./use-toast"

// ============================================================================
// Type Definitions
// ============================================================================

export interface UseAPIOptions extends ApiClientConfig {
  /**
   * Show toast notifications for errors
   * @default true
   */
  showErrorToasts?: boolean

  /**
   * Show toast notifications for success
   * @default false
   */
  showSuccessToasts?: boolean

  /**
   * Custom error handler
   */
  onError?: (error: Error) => void

  /**
   * Custom success handler
   */
  onSuccess?: (message?: string) => void
}

export interface UseAPIReturn {
  /** The API client instance */
  api: ApiClient

  /** Whether an API request is in progress */
  isLoading: boolean

  /** The last error that occurred */
  error: Error | null

  /** Clear the current error */
  clearError: () => void

  /** Wrapper for API calls with automatic error handling */
  execute: <T>(fn: () => Promise<T>, options?: ExecuteOptions) => Promise<T | null>

  /** Check if currently loading */
  isPending: boolean
}

export interface ExecuteOptions {
  /** Show toast for this call */
  showSuccessToast?: boolean

  /** Custom success message */
  successMessage?: string

  /** Custom error handler for this call */
  onError?: (error: Error) => void

  /** Custom success handler for this call */
  onSuccess?: () => void
}

// ============================================================================
// Hook Implementation
// ============================================================================

const defaultApiClient = getClient()

export function useAPI(options: UseAPIOptions = {}): UseAPIReturn {
  const {
    showErrorToasts = true,
    showSuccessToasts = false,
    onError: onErrorOption,
    onSuccess: onSuccessOption,
    baseUrl,
    apiKey,
    timeout,
    headers,
  } = options

  const [isLoading, setIsLoading] = React.useState(false)
  const [error, setError] = React.useState<Error | null>(null)

  // Memoize individual config values to create stable dependency
  const apiClient = React.useMemo(() => {
    // Create custom client if any config is provided, otherwise use default
    if (baseUrl !== undefined || apiKey !== undefined || timeout !== undefined || headers !== undefined) {
      return new ApiClient({ baseUrl, apiKey, timeout, headers })
    }
    return defaultApiClient
  }, [baseUrl, apiKey, timeout, headers])

  const { toast } = useToast()

  // Clear error method
  const clearError = React.useCallback(() => {
    setError(null)
  }, [])

  // Execute wrapper with error handling
  const execute = React.useCallback(
    async <T>(fn: () => Promise<T>, executeOptions?: ExecuteOptions): Promise<T | null> => {
      setIsLoading(true)
      setError(null)

      try {
        const result = await fn()

        // Handle success
        if (executeOptions?.successMessage || showSuccessToasts || executeOptions?.showSuccessToast) {
          const message = executeOptions?.successMessage ?? "Operation completed successfully"
          toast({ title: "Success", description: message, variant: "default" })
        }

        executeOptions?.onSuccess?.()
        onSuccessOption?.(executeOptions?.successMessage)

        return result
      } catch (err) {
        const error = err instanceof Error ? err : new Error("Unknown error occurred")
        setError(error)

        // Handle error
        if (showErrorToasts) {
          const title = getErrorTitle(error)
          const description = getErrorMessage(error)
          toast({ title, description, variant: "destructive" })
        }

        executeOptions?.onError?.(error)
        onErrorOption?.(error)

        return null
      } finally {
        setIsLoading(false)
      }
    },
    [showErrorToasts, showSuccessToasts, toast, onErrorOption, onSuccessOption],
  )

  return {
    api: apiClient,
    isLoading,
    error,
    clearError,
    execute,
    isPending: isLoading,
  }
}

// ============================================================================
// Error Handling Utilities
// ============================================================================

function getErrorTitle(error: Error): string {
  if (error instanceof ApiError) {
    switch (error.statusCode) {
      case 400:
        return "Bad Request"
      case 401:
        return "Unauthorized"
      case 403:
        return "Forbidden"
      case 404:
        return "Not Found"
      case 429:
        return "Too Many Requests"
      case 500:
        return "Server Error"
      case 503:
        return "Service Unavailable"
      default:
        return "API Error"
    }
  }

  if (error instanceof NetworkError) {
    return "Network Error"
  }

  if (error instanceof TimeoutError) {
    return "Request Timeout"
  }

  return "Error"
}

function getErrorMessage(error: Error): string {
  if (error instanceof ApiError) {
    return error.message
  }

  if (error instanceof NetworkError) {
    return "Please check your internet connection and try again."
  }

  if (error instanceof TimeoutError) {
    return "The request took too long to complete. Please try again."
  }

  return error.message || "An unexpected error occurred."
}

// ============================================================================
// Convenience Hooks
// ============================================================================

/**
 * Hook for API with automatic loading state reset
 */
export function useAPILazy(): UseAPIReturn & { executeLazy: <T>(fn: () => Promise<T>) => Promise<T | null> } {
  const apiHook = useAPI()

  const executeLazy = React.useCallback(
    async <T>(fn: () => Promise<T>): Promise<T | null> => {
      return apiHook.execute(fn)
    },
    [apiHook],
  )

  return {
    ...apiHook,
    executeLazy,
  }
}

/**
 * Hook that only provides the API client without loading state
 */
export function useAPIClient(): ApiClient {
  return React.useMemo(() => getClient(), [])
}

/**
 * Hook for API with a custom client configuration
 */
export function useAPIWithConfig(config: ApiClientConfig): UseAPIReturn {
  return useAPI({ ...config, showErrorToasts: true })
}

// ============================================================================
// Type Guards
// ============================================================================

/**
 * Check if an error is an ApiError
 */
export function isApiError(error: Error): error is ApiError {
  return error instanceof ApiError
}

/**
 * Check if an error is a NetworkError
 */
export function isNetworkError(error: Error): error is NetworkError {
  return error instanceof NetworkError
}

/**
 * Check if an error is a TimeoutError
 */
export function isTimeoutError(error: Error): error is TimeoutError {
  return error instanceof TimeoutError
}
