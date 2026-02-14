import { children, type JSX } from "solid-js"

/**
 * Safe string conversion for TUI rendering
 * Ensures all values passed to text elements are strings
 */
export function safeText(value: unknown): string {
  if (value === null || value === undefined) return ""
  if (typeof value === "string") return value
  if (typeof value === "number" || typeof value === "boolean") return String(value)
  if (value instanceof Error) return value.message
  // Handle arrays and objects safely
  if (Array.isArray(value)) {
    try {
      return JSON.stringify(value)
    } catch {
      return "[Array]"
    }
  }
  if (typeof value === "object") {
    try {
      return JSON.stringify(value)
    } catch {
      return "[Object]"
    }
  }
  // Fallback for any other type
  try {
    return String(value)
  } catch {
    return "[Unknown]"
  }
}

/**
 * Wraps a potentially undefined value with fallback
 */
export function safeStr(value: unknown, fallback = ""): string {
  if (value === null || value === undefined) return fallback
  if (typeof value === "string") return value
  return String(value)
}

/**
 * Recursively convert all non-string values in JSX children to strings.
 * This is used to sanitize children before passing to text elements.
 */
export function sanitizeChildren(childrenInput: JSX.Element): () => JSX.Element {
  const resolved = children(() => childrenInput)

  return () => {
    const result = resolved()
    if (Array.isArray(result)) {
      return result.map((child) => sanitizeValue(child)) as unknown as JSX.Element
    }
    return sanitizeValue(result) as JSX.Element
  }
}

function sanitizeValue(value: unknown): unknown {
  // Pass through valid types
  if (value === null || value === undefined) return ""
  if (typeof value === "string") return value
  if (typeof value === "number") return String(value)
  if (typeof value === "boolean") return value ? "true" : ""
  if (typeof value === "function") return value // Solid.js accessor
  if (Array.isArray(value)) return value.map(sanitizeValue)

  // Check if it's a Solid component/element (has special properties)
  if (typeof value === "object" && value !== null) {
    // Solid elements have certain internal properties
    if ("t" in value || "children" in value || "$$typeof" in value) {
      return value // Pass through Solid elements
    }
    // Convert plain objects to string representation
    return safeText(value)
  }

  return safeText(value)
}
