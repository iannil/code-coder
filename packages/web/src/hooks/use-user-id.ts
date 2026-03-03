/**
 * User ID Hook
 *
 * Provides a persistent anonymous user ID for the web frontend.
 * The ID is generated once and stored in localStorage for consistency
 * across sessions.
 *
 * Note: This is a placeholder until proper authentication is implemented.
 * When auth is added, this hook should be updated to return the
 * authenticated user's ID from the auth context.
 */

import { useMemo } from "react"

const USER_ID_STORAGE_KEY = "codecoder-web-user-id"

/**
 * Generate a unique user ID prefixed with "web-" to indicate channel source.
 */
function generateUserId(): string {
  return `web-${crypto.randomUUID()}`
}

/**
 * Get or create a persistent user ID from localStorage.
 */
function getOrCreateUserId(): string {
  // Check localStorage for existing ID
  const stored = localStorage.getItem(USER_ID_STORAGE_KEY)
  if (stored) {
    return stored
  }

  // Generate new ID and persist
  const newId = generateUserId()
  localStorage.setItem(USER_ID_STORAGE_KEY, newId)
  return newId
}

/**
 * Hook to get the current user's ID.
 * Returns a persistent anonymous ID until auth is implemented.
 *
 * @example
 * ```tsx
 * const userId = useUserId()
 * // userId: "web-a1b2c3d4-e5f6-..."
 * ```
 */
export function useUserId(): string {
  // Use useMemo to ensure ID is stable across re-renders
  return useMemo(() => getOrCreateUserId(), [])
}

/**
 * Get the user ID synchronously (for use outside of React components).
 */
export function getUserId(): string {
  return getOrCreateUserId()
}
