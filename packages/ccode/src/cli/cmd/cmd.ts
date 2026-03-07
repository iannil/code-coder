import type { CommandModule } from "yargs"

type WithDoubleDash<T> = T & { "--"?: string[] }

/**
 * CLI Command wrapper that normalizes command types for yargs.
 * Returns CommandModule<object, object> to ensure type compatibility
 * when chaining .command() calls with different option types.
 */
export function cmd<T extends object, U extends object>(
  input: CommandModule<T, WithDoubleDash<U>>,
): CommandModule<object, object> {
  // Cast is safe: yargs commands are contravariant in T and covariant in U,
  // but TypeScript models them as invariant. This wrapper normalizes types.
  return input as CommandModule<object, object>
}
