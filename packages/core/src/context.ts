/**
 * Context module - Project fingerprinting and relevance scoring
 *
 * This module provides TypeScript types and wrappers for:
 * - Project fingerprinting (detect frameworks, build tools, test frameworks)
 * - Content relevance scoring for context prioritization
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
// Fallback Implementations (used when native bindings unavailable)
// ============================================================================

/** Generate project fingerprint (fallback implementation) */
export function generateFingerprintFallback(rootPath: string): FingerprintInfo {
  // This is a minimal fallback - native implementation is preferred
  const path = require('path')
  const fs = require('fs')

  const projectId = hashString(rootPath)
  const language = detectLanguageFallback(rootPath)
  const packageInfo = readPackageJsonFallback(rootPath)

  return {
    projectId,
    frameworks: [],
    buildTools: [],
    testFrameworks: [],
    package: packageInfo,
    configs: [],
    language,
    hasTypeScript: fs.existsSync(path.join(rootPath, 'tsconfig.json')),
    languageVersion: undefined,
    directories: {
      tests: [],
    },
    hash: projectId,
  }
}

function detectLanguageFallback(rootPath: string): ProjectLanguage {
  const path = require('path')
  const fs = require('fs')

  if (fs.existsSync(path.join(rootPath, 'tsconfig.json'))) return 'TypeScript'
  if (fs.existsSync(path.join(rootPath, 'Cargo.toml'))) return 'Rust'
  if (fs.existsSync(path.join(rootPath, 'go.mod'))) return 'Go'
  if (fs.existsSync(path.join(rootPath, 'pyproject.toml'))) return 'Python'
  if (fs.existsSync(path.join(rootPath, 'package.json'))) return 'JavaScript'
  return 'Other'
}

function readPackageJsonFallback(rootPath: string): PackageInfo {
  const path = require('path')
  const fs = require('fs')

  const packageJsonPath = path.join(rootPath, 'package.json')
  if (!fs.existsSync(packageJsonPath)) {
    return { manager: 'Unknown' }
  }

  try {
    const content = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'))
    return {
      name: content.name,
      version: content.version,
      manager: detectPackageManagerFallback(rootPath),
    }
  } catch {
    return { manager: 'Unknown' }
  }
}

function detectPackageManagerFallback(rootPath: string): PackageManager {
  const path = require('path')
  const fs = require('fs')

  if (fs.existsSync(path.join(rootPath, 'bun.lock')) || fs.existsSync(path.join(rootPath, 'bun.lockb'))) return 'Bun'
  if (fs.existsSync(path.join(rootPath, 'pnpm-lock.yaml'))) return 'Pnpm'
  if (fs.existsSync(path.join(rootPath, 'yarn.lock'))) return 'Yarn'
  if (fs.existsSync(path.join(rootPath, 'package-lock.json'))) return 'Npm'
  if (fs.existsSync(path.join(rootPath, 'Cargo.lock'))) return 'Cargo'
  if (fs.existsSync(path.join(rootPath, 'go.sum'))) return 'Go'
  return 'Unknown'
}

/** Score content relevance (fallback implementation) */
export function scoreRelevanceFallback(query: string, content: string): RelevanceScore {
  const queryWords = extractKeywords(query.toLowerCase())
  const contentWords = extractKeywords(content.toLowerCase())

  const matched = [...queryWords].filter((w) => contentWords.has(w))
  const keywordScore = queryWords.size > 0 ? matched.length / queryWords.size : 0

  return {
    score: keywordScore * 0.5,
    keywordScore,
    structuralScore: 0.5,
    recencyScore: 0.5,
    matchedKeywords: matched,
  }
}

function extractKeywords(text: string): Set<string> {
  const stopWords = new Set([
    'the',
    'a',
    'an',
    'is',
    'are',
    'was',
    'were',
    'be',
    'been',
    'being',
    'have',
    'has',
    'had',
    'do',
    'does',
    'did',
    'will',
    'would',
    'could',
    'should',
    'may',
    'might',
    'must',
    'shall',
    'can',
    'need',
    'of',
    'in',
    'to',
    'for',
    'with',
    'on',
    'at',
    'by',
    'from',
    'as',
    'and',
    'or',
    'but',
    'if',
    'then',
    'else',
    'when',
    'where',
    'why',
    'how',
    'all',
    'each',
    'every',
    'both',
    'few',
    'more',
    'most',
    'this',
    'that',
    'these',
    'those',
    'it',
    'its',
  ])

  const words = text.match(/\b[a-z_][a-z0-9_]*\b/gi) || []
  return new Set(words.filter((w) => w.length >= 2 && !stopWords.has(w.toLowerCase())).map((w) => w.toLowerCase()))
}

/** Compute fingerprint similarity (fallback implementation) */
export function fingerprintSimilarityFallback(a: FingerprintInfo, b: FingerprintInfo): number {
  let score = 0

  // Language match (40%)
  if (a.language === b.language) score += 0.4

  // Framework overlap (30%)
  const aFrameworks = new Set(a.frameworks.map((f) => f.name))
  const bFrameworks = new Set(b.frameworks.map((f) => f.name))
  const frameworkIntersection = [...aFrameworks].filter((f) => bFrameworks.has(f)).length
  const frameworkUnion = new Set([...aFrameworks, ...bFrameworks]).size
  if (frameworkUnion > 0) score += 0.3 * (frameworkIntersection / frameworkUnion)

  // Build tool overlap (15%)
  const aTools = new Set(a.buildTools.map((t) => t.name))
  const bTools = new Set(b.buildTools.map((t) => t.name))
  const toolIntersection = [...aTools].filter((t) => bTools.has(t)).length
  const toolUnion = new Set([...aTools, ...bTools]).size
  if (toolUnion > 0) score += 0.15 * (toolIntersection / toolUnion)

  // Test framework overlap (15%)
  const aTests = new Set(a.testFrameworks.map((t) => t.name))
  const bTests = new Set(b.testFrameworks.map((t) => t.name))
  const testIntersection = [...aTests].filter((t) => bTests.has(t)).length
  const testUnion = new Set([...aTests, ...bTests]).size
  if (testUnion > 0) score += 0.15 * (testIntersection / testUnion)

  return score
}

/** Compute content hash (fallback implementation) */
export function contentHashFallback(content: string): string {
  // Simple hash implementation
  let hash = 0n
  const prime = 0x100000001b3n
  const offset = 0xcbf29ce484222325n

  hash = offset
  for (let i = 0; i < content.length; i++) {
    hash ^= BigInt(content.charCodeAt(i))
    hash *= prime
  }

  return hash.toString(16).padStart(16, '0').slice(0, 16)
}

function hashString(str: string): string {
  return contentHashFallback(str)
}

// ============================================================================
// Exports (Native with Fallback)
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
  // Native bindings not available
}

/** Generate project fingerprint */
export const generateFingerprint = nativeBindings?.generateFingerprint ?? generateFingerprintFallback

/** Compute similarity between two fingerprints */
export const fingerprintSimilarity = nativeBindings?.fingerprintSimilarity ?? fingerprintSimilarityFallback

/** Generate human-readable description of a fingerprint */
export const describeFingerprint =
  nativeBindings?.describeFingerprint ??
  ((fp: FingerprintInfo): string => {
    const parts: string[] = []
    if (fp.language !== 'Other') parts.push(fp.language)
    if (fp.frameworks.length > 0) parts.push(fp.frameworks.map((f) => f.name).join(', '))
    if (fp.buildTools.length > 0) parts.push(fp.buildTools.map((t) => t.name).join(', '))
    return parts.length > 0 ? parts.join(' • ') : 'Unknown project type'
  })

/** Score content relevance */
export const scoreRelevance = nativeBindings?.scoreRelevance ?? scoreRelevanceFallback

/** Score content relevance with custom config */
export const scoreRelevanceWithConfig =
  nativeBindings?.scoreRelevanceWithConfig ??
  ((query: string, content: string, _config: RelevanceScorerConfig): RelevanceScore =>
    scoreRelevanceFallback(query, content))

/** Score multiple files and return sorted by relevance */
export const scoreFiles =
  nativeBindings?.scoreFiles ??
  ((query: string, files: FileMetadata[]): ScoredFile[] => {
    return files
      .map((f) => ({
        path: f.path,
        score: scoreRelevanceFallback(query, f.content),
      }))
      .filter((f) => f.score.score >= DEFAULT_RELEVANCE_CONFIG.minScore)
      .sort((a, b) => b.score.score - a.score.score)
  })

/** Compute content hash for deduplication */
export const contentHash = nativeBindings?.contentHash ?? contentHashFallback
