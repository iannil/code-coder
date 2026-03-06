/**
 * @codecoder-ai/core - High-performance Rust-native tools
 *
 * This package provides native Rust implementations of file operations
 * for improved performance and reliability.
 *
 * @example
 * ```typescript
 * import { grep, glob, readFile, editFile } from '@codecoder-ai/core'
 *
 * // Search for patterns in files
 * const result = await grep({ pattern: 'fn main', path: 'src/' })
 *
 * // Find files matching a glob pattern
 * const files = await glob({ pattern: '**\/*.rs' })
 *
 * // Read a file
 * const content = readFile('/path/to/file.txt')
 *
 * // Edit a file
 * const edit = editFile('/path/to/file.txt', {
 *   oldString: 'old',
 *   newString: 'new'
 * })
 * ```
 *
 * @example Session management
 * ```typescript
 * import { MessageStore, SessionStore } from '@codecoder-ai/core'
 *
 * // In-memory message storage
 * const store = new MessageStore()
 * store.push({ role: 'user', content: 'Hello!' })
 *
 * // Persistent session storage
 * const sessions = new SessionStore('/path/to/sessions.db')
 * sessions.save({ id: 'abc', cwd: '/project', messages: store.messages() })
 * ```
 *
 * @example Security management
 * ```typescript
 * import { PermissionManager, Vault } from '@codecoder-ai/core'
 *
 * // Permission management
 * const permissions = new PermissionManager()
 * permissions.addRule({ permission: { tool: 'file', action: 'read' }, allow: true })
 *
 * // Encrypted vault storage
 * const vault = new Vault('/path/to/vault.enc', 'password')
 * vault.set({ name: 'api_key', value: 'sk-xxx' })
 * vault.save()
 * ```
 */

// Re-export native bindings
// Note: In development, these will be stubs until the native module is built
export * from './types.js'
export * from './fallback.js'
export * from './session.js'
export * from './security.js'
export * from './protocol.js'
export * from './context.js'
export * from './memory.js'
export * from './audit.js'
export * from './history.js'
export * from './context-cache.js'
export * from './pty.js'
export * from './lsp.js'

// Try to load native bindings, fall back to JS implementation
let nativeBindings: typeof import('./binding.js') | null = null

try {
  nativeBindings = await import('./binding.js')
} catch {
  // Native bindings not available, will use fallback
  console.warn('@codecoder-ai/core: Native bindings not found, using JavaScript fallback')
}

export const isNative = nativeBindings !== null

// Export either native or fallback implementations
export const grep = nativeBindings?.grep ?? (await import('./fallback.js')).grep
export const glob = nativeBindings?.glob ?? (await import('./fallback.js')).glob
export const readFile = nativeBindings?.readFile ?? (await import('./fallback.js')).readFile
export const editFile = nativeBindings?.editFile ?? (await import('./fallback.js')).editFile
export const version = nativeBindings?.version ?? (() => '0.1.0')
export const init = nativeBindings?.init ?? (() => {})

// Export all native bindings (handles, functions, enums) for advanced use
// Phase 32: Patch/Diff
export const PatchApplicatorHandle = nativeBindings?.PatchApplicatorHandle
export const EditorHandle = nativeBindings?.EditorHandle
export const similarityRatio = nativeBindings?.similarityRatio
export const findBestMatch = nativeBindings?.findBestMatch
export const computeDiff = nativeBindings?.computeDiff

// Phase 2.1: Fuzzy Replace (Edit Tool Replacers)
export const replaceWithFuzzyMatch = nativeBindings?.replaceWithFuzzyMatch
export const levenshteinDistance = nativeBindings?.levenshteinDistance

// Phase 31: Knowledge Graph
export const GraphEngineHandle = nativeBindings?.GraphEngineHandle
export const CausalGraphHandle = nativeBindings?.CausalGraphHandle
export const CallGraphHandle = nativeBindings?.CallGraphHandle
export const SemanticGraphHandle = nativeBindings?.SemanticGraphHandle

// Phase 33: Context/Relevance
export const scoreRelevance = nativeBindings?.scoreRelevance
export const scoreRelevanceWithConfig = nativeBindings?.scoreRelevanceWithConfig
export const scoreFiles = nativeBindings?.scoreFiles
export const contentHash = nativeBindings?.contentHash
export const generateFingerprint = nativeBindings?.generateFingerprint
export const fingerprintSimilarity = nativeBindings?.fingerprintSimilarity
export const describeFingerprint = nativeBindings?.describeFingerprint

// Phase 34: Trace
export const TraceStoreHandle = nativeBindings?.TraceStoreHandle
export const openTraceStore = nativeBindings?.openTraceStore
export const createMemoryTraceStore = nativeBindings?.createMemoryTraceStore

// Phase 37: Web Fingerprints
export const WebFingerprintEngineHandle = nativeBindings?.WebFingerprintEngineHandle
export const detectWebTechnologies = nativeBindings?.detectWebTechnologies
export const getWebFingerprints = nativeBindings?.getWebFingerprints
export const getWebFingerprintsByCategory = nativeBindings?.getWebFingerprintsByCategory
export const getWebCategories = nativeBindings?.getWebCategories

// Phase 38: JAR Analyzer
export const JarAnalyzerHandle = nativeBindings?.JarAnalyzerHandle
export const FingerprintEngineHandle = nativeBindings?.FingerprintEngineHandle
export const analyzeJar = nativeBindings?.analyzeJar
export const jarAnalysisSummary = nativeBindings?.jarAnalysisSummary
export const parseClassFileSync = nativeBindings?.parseClassFileSync
export const detectJavaTechnologies = nativeBindings?.detectJavaTechnologies

// Memory/Vector operations
export const cosineSimilarity = nativeBindings?.cosineSimilarity
export const normalizeVector = nativeBindings?.normalizeVector
export const vectorDistance = nativeBindings?.vectorDistance
export const vectorToBytes = nativeBindings?.vectorToBytes
export const bytesToVector = nativeBindings?.bytesToVector
export const hybridMergeResults = nativeBindings?.hybridMergeResults
export const chunkText = nativeBindings?.chunkText
export const chunkTextWithConfig = nativeBindings?.chunkTextWithConfig
export const estimateTokens = nativeBindings?.estimateTokens

// Storage
export const KvStoreHandle = nativeBindings?.KvStoreHandle
export const openKvStore = nativeBindings?.openKvStore
export const createMemoryKvStore = nativeBindings?.createMemoryKvStore

// State machine and task queue
export const StateMachineHandle = nativeBindings?.StateMachineHandle
export const createStateMachine = nativeBindings?.createStateMachine
export const TaskQueueHandle = nativeBindings?.TaskQueueHandle
export const createTaskQueue = nativeBindings?.createTaskQueue

// Injection scanner
export const InjectionScannerHandle = nativeBindings?.InjectionScannerHandle
export const createInjectionScanner = nativeBindings?.createInjectionScanner
export const createInjectionScannerWithConfig = nativeBindings?.createInjectionScannerWithConfig
export const scanInjection = nativeBindings?.scanInjection
export const scanInjectionWithConfig = nativeBindings?.scanInjectionWithConfig
export const quickCheckInjection = nativeBindings?.quickCheckInjection
export const sanitizeInjectionInput = nativeBindings?.sanitizeInjectionInput

// Phase 2.7: Risk Assessment
export const assessBashRisk = nativeBindings?.assessBashRisk
export const assessFileRisk = nativeBindings?.assessFileRisk
export const getToolBaseRisk = nativeBindings?.getToolBaseRisk
export const checkRiskThreshold = nativeBindings?.checkRiskThreshold
export const parseRiskLevel = nativeBindings?.parseRiskLevel

// Enums
export const AutonomousState = nativeBindings?.AutonomousState
export const StateCategory = nativeBindings?.StateCategory
export const TaskPriority = nativeBindings?.TaskPriority
export const TaskStatus = nativeBindings?.TaskStatus

// Phase 2.4: Provider Transform
export const normalizeMessages = nativeBindings?.normalizeMessages
export const applyCaching = nativeBindings?.applyCaching
export const remapProviderOptions = nativeBindings?.remapProviderOptions
export const getSdkKey = nativeBindings?.getSdkKey
export const getTemperature = nativeBindings?.getTemperature
export const getTopP = nativeBindings?.getTopP
export const getTopK = nativeBindings?.getTopK
export const transformMessages = nativeBindings?.transformMessages

// Phase 5: Shell Parser (native tree-sitter)
export const ShellParserHandle = nativeBindings?.ShellParserHandle
export const parseShellCommand = nativeBindings?.parseShellCommand
export const assessShellCommandsRisk = nativeBindings?.assessShellCommandsRisk
export const extractShellDirectories = nativeBindings?.extractShellDirectories
export const extractShellPermissionPatterns = nativeBindings?.extractShellPermissionPatterns
export const isFileCommand = nativeBindings?.isFileCommand
export const isDangerousCommand = nativeBindings?.isDangerousCommand

// Phase 8.1: Git Operations (native libgit2)
export const GitOpsHandle = nativeBindings?.GitOpsHandle
export const openGitRepo = nativeBindings?.openGitRepo
export const initGitRepo = nativeBindings?.initGitRepo
export const cloneGitRepo = nativeBindings?.cloneGitRepo
export const isGitRepo = nativeBindings?.isGitRepo

// Phase 11: Markdown Parser (native pulldown-cmark)
export const parseMarkdown = nativeBindings?.parseMarkdown
export const extractMarkdownHeadings = nativeBindings?.extractMarkdownHeadings
export const extractMarkdownCodeBlocks = nativeBindings?.extractMarkdownCodeBlocks
export const extractMarkdownLinks = nativeBindings?.extractMarkdownLinks
export const extractMarkdownImages = nativeBindings?.extractMarkdownImages
export const renderMarkdownToHtml = nativeBindings?.renderMarkdownToHtml
export const extractMarkdownFrontmatter = nativeBindings?.extractMarkdownFrontmatter
export const stripMarkdownFrontmatter = nativeBindings?.stripMarkdownFrontmatter

// Phase 12: PTY (native portable-pty)
export const PtySessionHandle = nativeBindings?.PtySessionHandle
export const PtyManagerHandle = nativeBindings?.PtyManagerHandle
export const spawnPty = nativeBindings?.spawnPty
export const spawnPtyCommand = nativeBindings?.spawnPtyCommand

// Re-export types from binding.d.ts
export type {
  NapiGitStatus,
  NapiFileStatus,
  NapiCommitResult,
  NapiCommitInfo,
  NapiDiffFile,
  NapiDiffResult,
  NapiOperationResult,
  NapiInitOptions,
  NapiCloneOptions,
  NapiWorktreeInfo,
  GitOpsHandle as GitOpsHandleType,
  // Phase 11: Markdown types
  NapiMarkdownHeading,
  NapiMarkdownCodeBlock,
  NapiMarkdownLink,
  NapiMarkdownImage,
  NapiMarkdownNode,
  // Phase 12: PTY types
  NapiPtyConfig,
  NapiPtyState,
  NapiPtyInfo,
  PtySessionHandle as PtySessionHandleType,
  PtyManagerHandle as PtyManagerHandleType,
} from './binding.d.ts'
