/**
 * Context module - Project fingerprinting and relevance scoring
 *
 * This module provides TypeScript types and wrappers for:
 * - Project fingerprinting (detect frameworks, build tools, test frameworks)
 * - Content relevance scoring for context prioritization
 *
 * NOTE: Native bindings are REQUIRED. No JavaScript fallbacks.
 *
 * @example
 * ```typescript
 * import { generateFingerprint, scoreRelevance } from '@codecoder-ai/core/context'
 *
 * // Generate project fingerprint
 * const fingerprint = generateFingerprint('/path/to/project')
 * console.log(fingerprint.language) // 'typescript'
 * console.log(fingerprint.frameworks) // [{ name: 'React', version: '18.0.0', ... }]
 *
 * // Score content relevance
 * const score = scoreRelevance('authentication', 'User auth handler')
 * console.log(score.score) // 0.75
 * ```
 */

// ============================================================================
// Fingerprint Types
// ============================================================================

/** Project programming language */
export type ProjectLanguage =
  | 'TypeScript'
  | 'JavaScript'
  | 'Python'
  | 'Go'
  | 'Rust'
  | 'Java'
  | 'CSharp'
  | 'Other'

/** Package manager */
export type PackageManager =
  | 'Npm'
  | 'Bun'
  | 'Yarn'
  | 'Pnpm'
  | 'Pip'
  | 'Poetry'
  | 'Cargo'
  | 'Go'
  | 'Maven'
  | 'Gradle'
  | 'Nuget'
  | 'Unknown'

/** Framework type */
export type FrameworkType =
  | 'Frontend'
  | 'Backend'
  | 'Fullstack'
  | 'Mobile'
  | 'Desktop'
  | 'Cli'
  | 'Library'

/** Framework information */
export interface FrameworkInfo {
  /** Framework name (e.g., 'React', 'Next.js') */
  name: string
  /** Version string if detected */
  version?: string
  /** Type of framework */
  frameworkType: FrameworkType
}

/** Build tool information */
export interface BuildToolInfo {
  /** Tool name (e.g., 'Vite', 'Webpack') */
  name: string
  /** Config file path if found */
  config?: string
  /** Version string if detected */
  version?: string
}

/** Test framework information */
export interface TestFrameworkInfo {
  /** Framework name (e.g., 'Jest', 'Vitest') */
  name: string
  /** Config file path if found */
  config?: string
  /** Test runner command if detected */
  runner?: string
}

/** Package information */
export interface PackageInfo {
  /** Package name from manifest */
  name?: string
  /** Package version */
  version?: string
  /** Detected package manager */
  manager: PackageManager
}

/** Configuration file information */
export interface ConfigFile {
  /** Full path to config file */
  path: string
  /** File name */
  name: string
}

/** Directory structure information */
export interface DirectoryInfo {
  /** Source directory */
  src?: string
  /** Components directory */
  components?: string
  /** Pages directory */
  pages?: string
  /** Routes directory */
  routes?: string
  /** Test directories */
  tests: string[]
  /** Library directory */
  lib?: string
  /** Distribution directory */
  dist?: string
  /** Build directory */
  build?: string
  /** Public assets directory */
  public?: string
}

/** Full project fingerprint */
export interface FingerprintInfo {
  /** Unique project identifier (hash) */
  projectId: string
  /** Detected frameworks */
  frameworks: FrameworkInfo[]
  /** Detected build tools */
  buildTools: BuildToolInfo[]
  /** Detected test frameworks */
  testFrameworks: TestFrameworkInfo[]
  /** Package information */
  package: PackageInfo
  /** Configuration files found */
  configs: ConfigFile[]
  /** Primary programming language */
  language: ProjectLanguage
  /** Whether TypeScript is used */
  hasTypeScript: boolean
  /** Language version if detected */
  languageVersion?: string
  /** Directory structure */
  directories: DirectoryInfo
  /** Fingerprint content hash */
  hash: string
}

// ============================================================================
// Relevance Types
// ============================================================================

/** Relevance score with breakdown */
export interface RelevanceScore {
  /** Overall relevance score (0.0 - 1.0) */
  score: number
  /** Keyword match score component */
  keywordScore: number
  /** Structural relevance score component */
  structuralScore: number
  /** Recency score component */
  recencyScore: number
  /** List of matched keywords */
  matchedKeywords: string[]
}

/** Configuration for relevance scorer */
export interface RelevanceScorerConfig {
  /** Weight for keyword matching (0.0 - 1.0) */
  keywordWeight: number
  /** Weight for structural relevance (0.0 - 1.0) */
  structuralWeight: number
  /** Weight for recency (0.0 - 1.0) */
  recencyWeight: number
  /** Minimum score threshold */
  minScore: number
  /** Whether to use case-insensitive matching */
  caseInsensitive: boolean
}

/** File metadata for relevance scoring */
export interface FileMetadata {
  /** File path */
  path: string
  /** File content */
  content: string
  /** Last modified timestamp (unix seconds) */
  modified?: number
  /** File extension */
  extension?: string
}

/** Scored file result */
export interface ScoredFile {
  /** File path */
  path: string
  /** Relevance score */
  score: RelevanceScore
}

// ============================================================================
// Default Config
// ============================================================================

/** Default relevance scorer configuration */
export const DEFAULT_RELEVANCE_CONFIG: RelevanceScorerConfig = {
  keywordWeight: 0.5,
  structuralWeight: 0.3,
  recencyWeight: 0.2,
  minScore: 0.1,
  caseInsensitive: true,
}

// ============================================================================
// Native Bindings (Required)
// ============================================================================

let nativeBindings: {
  generateFingerprint?: (path: string) => FingerprintInfo
  fingerprintSimilarity?: (a: FingerprintInfo, b: FingerprintInfo) => number
  describeFingerprint?: (fp: FingerprintInfo) => string
  scoreRelevance?: (query: string, content: string) => RelevanceScore
  scoreRelevanceWithConfig?: (query: string, content: string, config: RelevanceScorerConfig) => RelevanceScore
  scoreFiles?: (query: string, files: FileMetadata[]) => ScoredFile[]
  contentHash?: (content: string) => string
} | null = null

try {
  nativeBindings = require('../binding.js')
} catch {
  // Native bindings not available - will throw on use
}

// Helper to create a function that throws if native binding is missing
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function requireNative<T>(name: string, fn: T | undefined): T {
  if (fn) return fn
  // Return a function that throws - type assertion needed for flexibility
  const throwFn = (): never => {
    throw new Error(`Native binding required: ${name}. Build native modules with \`cargo build\` in services/zero-core.`)
  }
  return throwFn as unknown as T
}

/** Generate project fingerprint */
export const generateFingerprint = requireNative('generateFingerprint', nativeBindings?.generateFingerprint)

/** Compute similarity between two fingerprints */
export const fingerprintSimilarity = requireNative('fingerprintSimilarity', nativeBindings?.fingerprintSimilarity)

/** Generate human-readable description of a fingerprint */
export const describeFingerprint = requireNative('describeFingerprint', nativeBindings?.describeFingerprint)

/** Score content relevance */
export const scoreRelevance = requireNative('scoreRelevance', nativeBindings?.scoreRelevance)

/** Score content relevance with custom config */
export const scoreRelevanceWithConfig = requireNative('scoreRelevanceWithConfig', nativeBindings?.scoreRelevanceWithConfig)

/** Score multiple files and return sorted by relevance */
export const scoreFiles = requireNative('scoreFiles', nativeBindings?.scoreFiles)

/** Compute content hash for deduplication */
export const contentHash = requireNative('contentHash', nativeBindings?.contentHash)
