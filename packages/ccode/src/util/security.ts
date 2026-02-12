import { resolve, normalize } from "path"
import { cwd } from "process"

/**
 * Validate and sanitize a file path to prevent path traversal attacks.
 * Resolves the path relative to a base directory and ensures it doesn't escape.
 *
 * @param userPath - User-provided file path
 * @param basePath - Base directory to resolve from (defaults to cwd)
 * @returns Validated, absolute path
 * @throws Error if path traversal is detected
 */
export function validatePath(userPath: string, basePath: string = cwd()): string {
  if (!userPath) {
    throw new Error("File path cannot be empty")
  }

  const resolved = resolve(basePath, userPath)
  const normalized = normalize(resolved)
  const baseNormalized = normalize(basePath)

  // Ensure the resolved path is within the base directory
  if (!normalized.startsWith(baseNormalized)) {
    throw new Error("Path traversal detected")
  }

  return normalized
}

/**
 * Escape a string for safe use in shell commands.
 * Prevents command injection by properly quoting and escaping special characters.
 *
 * @param arg - String to escape
 * @returns Safely escaped string wrapped in quotes
 */
export function escapeShellArg(arg: string): string {
  if (!arg) {
    return '""'
  }

  // Replace characters that could break out of quotes
  const escaped = arg
    .replace(/"/g, '\\"')  // Escape quotes
    .replace(/\n/g, '\\n')  // Escape newlines
    .replace(/\r/g, '\\r')  // Escape carriage returns
    .replace(/\t/g, '\\t')  // Escape tabs
    .replace(/\$/g, '\\$')  // Escape dollar signs
    .replace(/\\/g, '\\\\') // Escape backslashes

  return `"${escaped}"`
}

/**
 * Validate JSON file content against a Zod schema before parsing.
 * Returns parsed data or throws if validation fails.
 *
 * @param content - Raw JSON string content
 * @param schema - Zod schema to validate against
 * @param filePath - Optional file path for error messages
 * @returns Parsed and validated data
 */
export async function safeJsonParse<T>(
  content: string,
  schema: { parse: (data: unknown) => T },
  filePath?: string,
): Promise<T> {
  try {
    const parsed = JSON.parse(content)
    return schema.parse(parsed)
  } catch (error) {
    const location = filePath ? ` in ${filePath}` : ""
    throw new Error(`Invalid JSON${location}: ${error instanceof Error ? error.message : "Unknown error"}`)
  }
}
