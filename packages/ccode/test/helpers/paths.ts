/**
 * Test Path Helper
 *
 * Provides dynamic path resolution for E2E tests.
 * Replaces hardcoded paths to ensure tests are portable across machines.
 */

import { join, resolve } from "path"

/**
 * Get the project root directory (packages/ccode)
 * Works regardless of where the test is run from.
 */
export function getProjectRoot(): string {
  // Use import.meta.dir for the current file's directory, then navigate up
  // This file is at: packages/ccode/test/helpers/paths.ts
  // Project root is:  packages/ccode/
  return resolve(import.meta.dir, "../..")
}

/**
 * Get the test directory root
 * @returns Path to packages/ccode/test
 */
export function getTestDir(): string {
  return join(getProjectRoot(), "test")
}

/**
 * Get the source directory
 * @returns Path to packages/ccode/src
 */
export function getSrcDir(): string {
  return join(getProjectRoot(), "src")
}

/**
 * Get the E2E test directory
 * @returns Path to packages/ccode/test/e2e
 */
export function getE2EDir(): string {
  return join(getTestDir(), "e2e")
}

/**
 * Get the monorepo root (parent of packages/)
 * @returns Path to the monorepo root
 */
export function getMonorepoRoot(): string {
  return resolve(getProjectRoot(), "../..")
}

/**
 * Create a temporary test project directory
 * @param suffix - Optional suffix for uniqueness
 * @returns Path to a temp directory for testing
 */
export function getTempTestDir(suffix = ""): string {
  const timestamp = Date.now()
  const dirName = suffix ? `test-project-${suffix}-${timestamp}` : `test-project-${timestamp}`
  return join("/tmp", dirName)
}

/**
 * Resolve a path relative to the project root
 * @param relativePath - Path relative to project root
 * @returns Absolute path
 */
export function resolveFromProjectRoot(...relativePaths: string[]): string {
  return join(getProjectRoot(), ...relativePaths)
}

/**
 * Standard paths for E2E tests
 */
export const TestPaths = {
  /** Path to use as working directory in E2E tests */
  get cwd(): string {
    return getProjectRoot()
  },

  /** Entry point for the TUI */
  get entryPoint(): string {
    return resolveFromProjectRoot("src", "index.ts")
  },

  /** Temporary test project directory */
  get tempProject(): string {
    return "/tmp/test-project"
  },

  /** Temporary test project with session */
  get tempProjectWithSession(): string {
    return "/tmp/test-project-with-session"
  },
} as const
