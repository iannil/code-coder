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
// Note: Native bindings are REQUIRED - no JavaScript fallbacks
export * from './types.js'
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
export * from './mcp.js'
export * from './compaction.js'

// Memory module (merged from @codecoder-ai/memory)
// Re-exported as namespace to avoid conflicts with util/config.ts DEFAULT_CONFIG
export * as memory from './memory/index.js'

// Util module (merged from @codecoder-ai/util)
export * from './util/index.js'

// Export permission module (selective to avoid conflicts)
export {
  AutoApproveEngine,
  evaluateToolApproval,
  evaluateAdaptiveToolApproval,
  canToolBeSafeAutoApproved,
  isPermissionNative,
  type AutoApproveConfig,
  type ToolInput,
  type ApprovalDecision,
  type ExecutionContext,
  type AdaptiveRiskResult,
  type PermissionRiskLevel,
  // Note: RiskResult and AuditEntry are renamed to avoid conflicts
  type RiskResult as PermissionRiskResult,
  type AuditEntry as PermissionAuditEntry,
} from './permission.js'

// Try to load native bindings - REQUIRED for operation
let nativeBindings: typeof import('./binding.js') | null = null

try {
  nativeBindings = await import('./binding.js')
} catch {
  // Native bindings not available - will throw on first use
  console.error('@codecoder-ai/core: Native bindings not found. Run `cargo build` in services/zero-core.')
}

export const isNative = nativeBindings !== null

// Helper to require native bindings
function requireNative<T>(name: string, fn: T | undefined): T {
  if (!fn) {
    throw new Error(`Native binding required: ${name}. Build native modules with \`cargo build\` in services/zero-core.`)
  }
  return fn
}

// Export native implementations (required - no fallbacks)
export const grep = nativeBindings?.grep ?? (() => { throw new Error('Native binding required: grep') })
export const glob = nativeBindings?.glob ?? (() => { throw new Error('Native binding required: glob') })
export const readFile = nativeBindings?.readFile ?? (() => { throw new Error('Native binding required: readFile') })
export const editFile = nativeBindings?.editFile ?? (() => { throw new Error('Native binding required: editFile') })
export const version = nativeBindings?.version ?? (() => { throw new Error('Native binding required: version') })
export const init = nativeBindings?.init ?? (() => { throw new Error('Native binding required: init') })

// Export all native bindings (handles, functions, enums) for advanced use
// Phase 32: Patch/Diff
export const PatchApplicatorHandle = nativeBindings?.PatchApplicatorHandle
export const EditorHandle = nativeBindings?.EditorHandle
export const similarityRatio = nativeBindings?.similarityRatio
export const findBestMatch = nativeBindings?.findBestMatch
export const computeDiff = nativeBindings?.computeDiff
export const diffLines = nativeBindings?.diffLines
export const createTwoFilesPatch = nativeBindings?.createTwoFilesPatch

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

// Phase 10: File Ignore Engine (native ignore pattern matching)
export const IgnoreEngineHandle = nativeBindings?.IgnoreEngineHandle
export const shouldIgnorePath = nativeBindings?.shouldIgnorePath
export const createIgnoreEngine = nativeBindings?.createIgnoreEngine
export const createIgnoreEngineWithConfig = nativeBindings?.createIgnoreEngineWithConfig
export const getIgnoreDefaultPatterns = nativeBindings?.getIgnoreDefaultPatterns
export const getIgnoreDefaultFolders = nativeBindings?.getIgnoreDefaultFolders
export const filterIgnoredPaths = nativeBindings?.filterIgnoredPaths
export const filterPathsWithPatterns = nativeBindings?.filterPathsWithPatterns

// Phase 12: Hash Embedding (SIMD-accelerated hash-based embeddings)
export const generateHashEmbedding = nativeBindings?.generateHashEmbedding
export const generateHashEmbeddingWithInfo = nativeBindings?.generateHashEmbeddingWithInfo
export const generateHashEmbeddingsBatch = nativeBindings?.generateHashEmbeddingsBatch
export const generateCombinedHashEmbedding = nativeBindings?.generateCombinedHashEmbedding
export const generatePositionalHashEmbedding = nativeBindings?.generatePositionalHashEmbedding
export const hashEmbeddingSimilarity = nativeBindings?.hashEmbeddingSimilarity

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

// Phase: Auto-Approve Engine
export const AutoApproveEngineHandle = nativeBindings?.AutoApproveEngineHandle
export const createAutoApproveEngine = nativeBindings?.createAutoApproveEngine
export const createSafeOnlyEngine = nativeBindings?.createSafeOnlyEngine
export const createPermissiveEngine = nativeBindings?.createPermissiveEngine
export const evaluateAutoApprove = nativeBindings?.evaluateAutoApprove
export const evaluateAdaptiveAutoApprove = nativeBindings?.evaluateAdaptiveAutoApprove
export const canSafeAutoApprove = nativeBindings?.canSafeAutoApprove

// Note: Remote Policy is exported from './security.js' via `export * from './security.js'`
// The RemotePolicy class, getRemoteRiskLevel, isRemoteDangerous, isRemoteSafe are all available there

// Phase: Compaction (context window management)
export const CompactorHandle = nativeBindings?.CompactorHandle
export const createCompactor = nativeBindings?.createCompactor
export const createCompactorWithLimits = nativeBindings?.createCompactorWithLimits
// Note: estimateTokens is also available from session.js

// Phase 5: Prune (context window management via tool output compaction)
export const isOverflow = nativeBindings?.isOverflow
export const computePrunePlan = nativeBindings?.computePrunePlan
export const computePrunePlanWithTurns = nativeBindings?.computePrunePlanWithTurns
export const createDefaultPruneConfig = nativeBindings?.createDefaultPruneConfig

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

// Config Loader (JSONC parsing, config merging, schema validation)
export const ConfigLoaderHandle = nativeBindings?.ConfigLoaderHandle
export const createConfigLoader = nativeBindings?.createConfigLoader

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
// Note: PtySessionHandle and PtyManagerHandle not available in current binding
export const PtySessionHandle = undefined
export const PtyManagerHandle = undefined
export const spawnPty = nativeBindings?.spawnPty
export const spawnPtyCommand = nativeBindings?.spawnPtyCommand

// Phase: File Watcher (native notify crate - replaces @parcel/watcher)
export const FileWatcherHandle = nativeBindings?.FileWatcherHandle
export const createFileWatcher = nativeBindings?.createFileWatcher
export const createFileWatcherWithConfig = nativeBindings?.createFileWatcherWithConfig
export const watchPath = nativeBindings?.watchPath

// Phase: Hook Pattern Matching (native regex with SIMD)
export const PatternSetHandle = nativeBindings?.PatternSetHandle
export const createPatternSet = nativeBindings?.createPatternSet
export const scanPatterns = nativeBindings?.scanPatterns
export const scanContentPatterns = nativeBindings?.scanContentPatterns
export const matchesPattern = nativeBindings?.matchesPattern
export const containsPattern = nativeBindings?.containsPattern

// Phase 8: Tool Registry (unified tool discovery and execution)
export const ToolRegistryHandle = nativeBindings?.ToolRegistryHandle
export const createToolRegistry = nativeBindings?.createToolRegistry
export const getBuiltinToolSpecs = nativeBindings?.getBuiltinToolSpecs
export const getNativeToolNames = nativeBindings?.getNativeToolNames

// Context Loader (high-performance project scanning)
export const ContextLoaderHandle = nativeBindings?.ContextLoaderHandle
export const createContextLoader = nativeBindings?.createContextLoader
export const scanDirectory = nativeBindings?.scanDirectory
export const extractDirectoryDependencies = nativeBindings?.extractDirectoryDependencies

// Embedding Index (SIMD-accelerated KNN search)
export const EmbeddingIndexHandle = nativeBindings?.EmbeddingIndexHandle
export const createEmbeddingIndex = nativeBindings?.createEmbeddingIndex

// Memory System (unified memory management)
export const MemorySystemHandle = nativeBindings?.MemorySystemHandle
export const createMemorySystem = nativeBindings?.createMemorySystem

// Markdown Memory (dual-layer transparent memory)
export const MarkdownMemoryHandle = nativeBindings?.MarkdownMemoryHandle
export const createMarkdownMemory = nativeBindings?.createMarkdownMemory

// Observability (emit API for tracing, metrics, cost tracking)
// TODO: NAPI bindings not yet implemented in Rust.
// These are exported as undefined to allow TypeScript compilation while preserving
// runtime checks in tracer.ts. When implementing, add type definitions to binding.d.ts.
export const ObservabilityStoreHandle: undefined = undefined
export const openObservabilityStore: undefined = undefined
export const createMemoryObservabilityStore: undefined = undefined

// Phase 11: Skill Parser (native YAML frontmatter parsing)
export const parseSkillContent = nativeBindings?.parseSkillContent
export const parseSkillFromFile = nativeBindings?.parseSkillFromFile
export const parseSkillMetadataOnly = nativeBindings?.parseSkillMetadataOnly
export const validateSkillContent = nativeBindings?.validateSkillContent
export const parseSkillsBatch = nativeBindings?.parseSkillsBatch
export const extractSkillFrontmatter = nativeBindings?.extractSkillFrontmatter
export const stripSkillFrontmatter = nativeBindings?.stripSkillFrontmatter

// Phase: Truncation (output truncation for large results)
export const TruncatorHandle = nativeBindings?.TruncatorHandle
export const truncateOutput = nativeBindings?.truncateOutput
export const truncatePreview = nativeBindings?.truncatePreview

// Phase: Read (native file reading with mmap support)
export const ReaderHandle = nativeBindings?.ReaderHandle
export const readFileWithLines = nativeBindings?.readFileWithLines
export const readFileRange = nativeBindings?.readFileRange
export const isBinaryFile = nativeBindings?.isBinaryFile
export const countFileLines = nativeBindings?.countFileLines

// Phase 2: Safety Guardrails (loop detection, state tracking)
export const SafetyGuardrailsHandle = nativeBindings?.SafetyGuardrailsHandle
export const createSafetyGuardrails = nativeBindings?.createSafetyGuardrails

// Phase 2: Safety Constraints (resource budget management)
export const SafetyGuardHandle = nativeBindings?.SafetyGuardHandle
export const createSafetyGuard = nativeBindings?.createSafetyGuard

// Phase 2: CLOSE Decision Framework
export const evaluateClose = nativeBindings?.evaluateClose

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
  // Note: PtySessionHandle and PtyManagerHandle not exported from current binding
  // Config Loader types
  NapiProviderConfig,
  NapiAgentConfig,
  NapiCommandConfig,
  NapiSecretsConfig,
  NapiConfig,
  NapiValidationIssue,
  ConfigLoaderHandle as ConfigLoaderHandleType,
  // Compaction types
  NapiCompactionStrategy,
  NapiCompactionResult,
  CompactorHandle as CompactorHandleType,
  // Prune types (Phase 5)
  NapiPruneConfig,
  NapiPartReference,
  NapiPrunePlan,
  NapiModelLimit,
  NapiTokenUsage,
  NapiToolPartInfo,
  NapiMessageInfo,
  // File Watcher types
  WatchEvent,
  WatchEventKind,
  FileWatcherConfig,
  FileWatcherHandle as FileWatcherHandleType,
  // Hook Pattern Matching types
  PatternMatchResult,
  ContentMatchResult,
  PatternSetHandle as PatternSetHandleType,
  // Tool Registry types
  NapiToolSpec,
  NapiToolExecuteResult,
  NapiValidationResult,
  NapiToolCall,
  NapiBatchResult,
  ToolRegistryHandle as ToolRegistryHandleType,
  // Graph types
  NapiNodeData,
  NapiPathResult,
  NapiCycleResult,
  NapiDecisionNode,
  NapiActionNode,
  NapiOutcomeNode,
  NapiCausalChain,
  NapiCausalQuery,
  NapiCausalStats,
  NapiCausalPattern,
  NapiSimilarDecision,
  NapiTrendAnalysis,
  NapiAgentInsights,
  NapiCallNode,
  NapiRecursionInfo,
  NapiSemanticNode,
  NapiSemanticStats,
  GraphEngineHandle as GraphEngineHandleType,
  CausalGraphHandle as CausalGraphHandleType,
  CallGraphHandle as CallGraphHandleType,
  SemanticGraphHandle as SemanticGraphHandleType,
  // Context Loader types
  NapiFileEntry,
  NapiDirectoryStructure,
  NapiFileIndex,
  NapiDependencyGraph,
  NapiScanOptions,
  NapiScanResult,
  ContextLoaderHandle as ContextLoaderHandleType,
  // Embedding Index types
  NapiEmbeddingSearchResult,
  NapiEmbeddingItem,
  NapiEmbeddingIndexStats,
  EmbeddingIndexHandle as EmbeddingIndexHandleType,
  // Memory System types
  NapiHistoryStats,
  NapiVectorStats,
  NapiTokenizerStats,
  NapiMemoryStats,
  NapiHistorySnapshot,
  NapiVectorSnapshotData,
  NapiMemorySnapshot,
  NapiImportOptions,
  NapiImportResult,
  NapiCleanupResult,
  MemorySystemHandle as MemorySystemHandleType,
  // Markdown Memory types
  NapiDailyEntry,
  NapiDailyEntryType,
  NapiMemoryCategory,
  NapiMemorySection,
  NapiMemoryContext,
  NapiMarkdownMemoryConfig,
  MarkdownMemoryHandle as MarkdownMemoryHandleType,
  // Skill Parser types
  NapiSkillMetadata,
  NapiParsedSkill,
  NapiSkillParseError,
  NapiParsedSkillResult,
  // File Ignore Engine types (Phase 10)
  NapiIgnoreConfig,
  NapiIgnoreCheckResult,
  IgnoreEngineHandle as IgnoreEngineHandleType,
  // Hash Embedding types (Phase 12)
  NapiHashEmbeddingResult,
  // Truncation types
  NapiTruncateOptions,
  NapiTruncateResult,
  TruncatorHandle as TruncatorHandleType,
  // Read types
  NapiReadOptions,
  NapiReadResult,
  ReaderHandle as ReaderHandleType,
  // Context and Fingerprint types (Phase 16)
  NapiProjectLanguage,
  NapiProjectLanguage as ProjectLanguage,  // Alias for backward compatibility
  NapiFrameworkType,
  NapiPackageManager,
  NapiFrameworkInfo,
  NapiBuildToolInfo,
  NapiTestFrameworkInfo,
  NapiPackageInfo,
  NapiConfigFile,
  NapiDirectoryInfo,
  NapiFingerprintInfo,
  NapiFingerprintInput,
  // Chunking types
  NapiChunk,
  NapiChunkerConfig,
  // Safety Guardrails types (Phase 2)
  NapiToolResult,
  NapiGuardrailConfig,
  NapiLoopDetection,
  NapiSafetyCheckResult,
  NapiGuardrailStats,
  SafetyGuardrailsHandle as SafetyGuardrailsHandleType,
  // Safety Constraints types (Phase 2)
  NapiResourceBudget,
  NapiResourceUsage,
  NapiConstraintCheckResult,
  NapiResourceWarning,
  NapiCheckWithWarnings,
  SafetyGuardHandle as SafetyGuardHandleType,
  // CLOSE Decision Framework types (Phase 2)
  NapiCloseDimension as NapiCLOSEDimension,
  NapiCloseEvaluation as NapiCLOSEEvaluation,
  NapiCloseWeights as NapiCLOSEWeights,
  NapiCloseInput as NapiCLOSEInput,
} from './binding.d.ts'
