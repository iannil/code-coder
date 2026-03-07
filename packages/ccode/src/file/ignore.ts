/**
 * File Ignore Module
 *
 * High-performance file path filtering using native Rust implementation.
 * Supports gitignore-compatible patterns including negation.
 *
 * Part of Phase 10: Rust Migration
 */

import {
  shouldIgnorePath as nativeShouldIgnore,
  createIgnoreEngine,
  createIgnoreEngineWithConfig,
  getIgnoreDefaultPatterns,
  getIgnoreDefaultFolders,
  filterIgnoredPaths,
  filterPathsWithPatterns,
} from "@codecoder-ai/core"

import type { NapiIgnoreConfig, IgnoreEngineHandleType } from "@codecoder-ai/core"

// Verify native bindings are available at import time
if (typeof nativeShouldIgnore !== "function" || typeof createIgnoreEngine !== "function") {
  throw new Error(
    "@codecoder-ai/core native bindings required: Ignore engine not available. " +
    "Run: cd services/zero-core && cargo build --features napi-bindings"
  )
}

// After runtime validation, assert these functions exist (TypeScript can't narrow from runtime check)
const shouldIgnore = nativeShouldIgnore!
const createEngine_ = createIgnoreEngine!
const createEngineWithConfig = createIgnoreEngineWithConfig!
const getDefaultPatterns_ = getIgnoreDefaultPatterns!
const getDefaultFolders_ = getIgnoreDefaultFolders!
const filterPaths = filterIgnoredPaths!
const filterWithPatterns = filterPathsWithPatterns!

export namespace FileIgnore {
  /**
   * Configuration for creating a custom ignore engine
   */
  export interface Config {
    /** Use default folder patterns (default: true) */
    useDefaultFolders?: boolean
    /** Use default file patterns (default: true) */
    useDefaultFiles?: boolean
    /** Additional patterns to add */
    additionalPatterns?: string[]
    /** Patterns to whitelist (never ignore) */
    whitelistPatterns?: string[]
    /** Whether to respect .gitignore files (default: true) */
    respectGitignore?: boolean
    /** Whether to respect .ccignore files (default: true) */
    respectCcignore?: boolean
  }

  /**
   * Get all default patterns (for backward compatibility)
   */
  export const PATTERNS: string[] = getDefaultPatterns_()

  /**
   * Check if a file path should be ignored using default patterns.
   * This is the main function for simple path checking.
   *
   * @param filepath - Path to check
   * @param opts - Optional configuration
   * @returns true if the path should be ignored
   */
  export function match(
    filepath: string,
    opts?: {
      extra?: Bun.Glob[]
      whitelist?: Bun.Glob[]
    },
  ): boolean {
    // If no custom options, use the fast native path
    if (!opts?.extra?.length && !opts?.whitelist?.length) {
      return shouldIgnore(filepath)
    }

    // For custom globs, create a custom engine
    const additionalPatterns: string[] = []

    // Convert Bun.Glob to pattern strings (approximate conversion)
    if (opts?.extra) {
      for (const glob of opts.extra) {
        // Bun.Glob.pattern is not exposed, so we use toString or assume pattern
        const pattern = glob.toString()
        if (pattern) additionalPatterns.push(pattern)
      }
    }

    const config: NapiIgnoreConfig = {
      additionalPatterns,
    }

    const engine = createEngineWithConfig(config)

    // Check whitelist first
    if (opts?.whitelist) {
      for (const glob of opts.whitelist) {
        if (glob.match(filepath)) return false
      }
    }

    return engine.isIgnored(filepath)
  }

  /**
   * Create a custom ignore engine for repeated checks.
   * More efficient when checking many paths.
   */
  export function createEngine(config?: Config): IgnoreEngineHandleType {
    if (!config) {
      return createEngine_()
    }

    const napiConfig: NapiIgnoreConfig = {
      useDefaultFolders: config.useDefaultFolders,
      useDefaultFiles: config.useDefaultFiles,
      additionalPatterns: config.additionalPatterns,
      whitelistPatterns: config.whitelistPatterns,
      respectGitignore: config.respectGitignore,
      respectCcignore: config.respectCcignore,
    }

    return createEngineWithConfig(napiConfig)
  }

  /**
   * Filter a list of paths, returning only non-ignored paths.
   * More efficient than calling match() repeatedly.
   */
  export function filter(paths: string[], additionalPatterns?: string[]): string[] {
    if (additionalPatterns?.length) {
      return filterWithPatterns(paths, additionalPatterns)
    }
    return filterPaths(paths)
  }

  /**
   * Get the default ignored folder names
   */
  export function getDefaultFolders(): string[] {
    return getDefaultFolders_()
  }

  /**
   * Get the default ignore patterns
   */
  export function getDefaultPatterns(): string[] {
    return getDefaultPatterns_()
  }

  /** Check if native implementation is being used (always true now) */
  export function isNative(): boolean {
    return true
  }
}
