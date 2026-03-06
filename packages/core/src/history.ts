/**
 * History management - Edit and Decision history tracking
 *
 * Provides TypeScript interface to the Rust-native history storage.
 * Requires native bindings - no JavaScript fallback.
 *
 * @example
 * ```typescript
 * import { HistoryStore } from '@codecoder-ai/core'
 *
 * // Open persistent history store
 * const history = new HistoryStore('/path/to/history.db')
 *
 * // Record an edit
 * const edit = await history.createEditRecord('project-id', {
 *   edits: [{ path: 'src/main.ts', type: 'update', additions: 10, deletions: 5 }],
 *   description: 'Fixed bug',
 *   agent: 'code-reviewer'
 * })
 *
 * // Record a decision
 * const decision = await history.createDecision('project-id', {
 *   type: 'architecture',
 *   title: 'Use Rust for core',
 *   description: 'Decided to use Rust for performance-critical code'
 * })
 * ```
 */

// ============================================================================
// Types
// ============================================================================

/** Type of file edit operation */
export type FileEditType = 'create' | 'update' | 'delete' | 'move'

/** A single file edit within an edit record */
export interface FileEdit {
  path: string
  type: FileEditType
  additions: number
  deletions: number
  preHash?: string
  postHash?: string
}

/** A record of one or more file edits */
export interface EditRecord {
  id: string
  sessionId?: string
  timestamp: number
  description?: string
  edits: FileEdit[]
  agent?: string
  model?: string
  tokensUsed?: number
  duration?: number
}

/** Input for creating an edit record */
export interface CreateEditRecordInput {
  edits: FileEdit[]
  sessionId?: string
  description?: string
  agent?: string
  model?: string
  tokensUsed?: number
  duration?: number
}

/** A group of related edits with aggregate stats */
export interface EditSession {
  id: string
  projectId: string
  startTime: number
  endTime?: number
  edits: string[]
  totalTokens: number
  totalDuration: number
  description?: string
}

/** Type of decision */
export type DecisionType = 'architecture' | 'implementation' | 'refactor' | 'bugfix' | 'feature' | 'other'

/** A decision record */
export interface DecisionRecord {
  id: string
  type: DecisionType
  title: string
  description: string
  rationale?: string
  alternatives?: string[]
  outcome?: string
  sessionId?: string
  files?: string[]
  tags?: string[]
  timestamp: number
}

/** Input for creating a decision */
export interface CreateDecisionInput {
  type: DecisionType
  title: string
  description: string
  rationale?: string
  alternatives?: string[]
  outcome?: string
  sessionId?: string
  files?: string[]
  tags?: string[]
}

/** ADR status */
export type AdrStatus = 'proposed' | 'accepted' | 'deprecated' | 'superseded' | 'rejected'

/** An alternative considered for an ADR */
export interface Alternative {
  description: string
  rejected: boolean
  reason?: string
}

/** An Architecture Decision Record */
export interface Adr {
  id: string
  title: string
  status: AdrStatus
  context: string
  decision: string
  consequences: string[]
  alternatives?: Alternative[]
  supersededBy?: string
  created: number
  updated: number
  tags?: string[]
}

/** Input for creating an ADR */
export interface CreateAdrInput {
  title: string
  context: string
  decision: string
  consequences: string[]
  status?: AdrStatus
  alternatives?: Alternative[]
  tags?: string[]
}

/** Agent statistics */
export interface AgentStats {
  name: string
  editCount: number
  tokenCount: number
}

/** File edit count */
export interface FileEditCount {
  path: string
  count: number
}

/** Edit statistics */
export interface EditStats {
  totalEdits: number
  totalAdditions: number
  totalDeletions: number
  totalFiles: number
  topFiles: FileEditCount[]
  agentStats: AgentStats[]
}

// ============================================================================
// Native Binding Types
// ============================================================================

interface NativeHistoryStoreHandle {
  createEditRecord(projectId: string, input: CreateEditRecordInput): EditRecord
  getEditRecord(projectId: string, id: string): EditRecord | null
  getRecentEdits(projectId: string, limit?: number): EditRecord[]
  getEditsBySession(projectId: string, sessionId: string): EditRecord[]
  getEditsByFile(projectId: string, filePath: string): EditRecord[]

  startEditSession(projectId: string, description?: string): EditSession
  getEditSession(projectId: string, id: string): EditSession | null
  endEditSession(projectId: string, id: string): EditSession | null
  getAllSessions(projectId: string): EditSession[]
  getActiveSessions(projectId: string): EditSession[]

  createDecision(projectId: string, input: CreateDecisionInput): DecisionRecord
  getDecision(projectId: string, id: string): DecisionRecord | null
  getRecentDecisions(projectId: string, limit?: number): DecisionRecord[]
  getDecisionsByType(projectId: string, decisionType: string): DecisionRecord[]
  searchDecisions(projectId: string, query: string): DecisionRecord[]
  deleteDecision(projectId: string, id: string): boolean

  createAdr(projectId: string, input: CreateAdrInput): Adr
  getAdr(projectId: string, id: string): Adr | null
  getAllAdrs(projectId: string): Adr[]
  formatAdrMarkdown(projectId: string, id: string): string | null

  getEditStats(projectId: string): EditStats
  cleanup(projectId: string, beforeTimestamp: number): number
  invalidate(projectId: string): void
}

interface NativeBindings {
  openHistoryStore(path: string): NativeHistoryStoreHandle
  createMemoryHistoryStore(): NativeHistoryStoreHandle
}

// Load native bindings - fail fast if not available
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const bindings = await import('./binding.js') as any
if (typeof bindings.openHistoryStore !== 'function') {
  throw new Error('Native history bindings not available. Ensure zero-core is compiled with napi-bindings feature.')
}
const nativeBindings = bindings as unknown as NativeBindings

// ============================================================================
// History Store Class
// ============================================================================

/**
 * History store for tracking edit and decision history
 *
 * Stores history persistently using SQLite via native Rust implementation.
 */
export class HistoryStore {
  private handle: NativeHistoryStoreHandle
  private dbPath: string

  /**
   * Open or create a history store
   * @param path - Path to the SQLite database file
   */
  constructor(path: string) {
    this.dbPath = path
    this.handle = nativeBindings.openHistoryStore(path)
  }

  /** Get the database path */
  get path(): string {
    return this.dbPath
  }

  // ========================================================================
  // Edit Records
  // ========================================================================

  /** Create and save an edit record */
  createEditRecord(projectId: string, input: CreateEditRecordInput): EditRecord {
    return this.handle.createEditRecord(projectId, input)
  }

  /** Get an edit record by ID */
  getEditRecord(projectId: string, id: string): EditRecord | null {
    return this.handle.getEditRecord(projectId, id)
  }

  /** Get recent edit records */
  getRecentEdits(projectId: string, limit = 20): EditRecord[] {
    return this.handle.getRecentEdits(projectId, limit)
  }

  /** Get edit records by session */
  getEditsBySession(projectId: string, sessionId: string): EditRecord[] {
    return this.handle.getEditsBySession(projectId, sessionId)
  }

  /** Get edit records by file */
  getEditsByFile(projectId: string, filePath: string): EditRecord[] {
    return this.handle.getEditsByFile(projectId, filePath)
  }

  // ========================================================================
  // Edit Sessions
  // ========================================================================

  /** Start a new edit session */
  startEditSession(projectId: string, description?: string): EditSession {
    return this.handle.startEditSession(projectId, description)
  }

  /** Get an edit session by ID */
  getEditSession(projectId: string, id: string): EditSession | null {
    return this.handle.getEditSession(projectId, id)
  }

  /** End an edit session */
  endEditSession(projectId: string, id: string): EditSession | null {
    return this.handle.endEditSession(projectId, id)
  }

  /** Get all edit sessions */
  getAllSessions(projectId: string): EditSession[] {
    return this.handle.getAllSessions(projectId)
  }

  /** Get active (not ended) sessions */
  getActiveSessions(projectId: string): EditSession[] {
    return this.handle.getActiveSessions(projectId)
  }

  // ========================================================================
  // Decision Records
  // ========================================================================

  /** Create and save a decision record */
  createDecision(projectId: string, input: CreateDecisionInput): DecisionRecord {
    return this.handle.createDecision(projectId, input)
  }

  /** Get a decision record by ID */
  getDecision(projectId: string, id: string): DecisionRecord | null {
    return this.handle.getDecision(projectId, id)
  }

  /** Get recent decisions */
  getRecentDecisions(projectId: string, limit = 10): DecisionRecord[] {
    return this.handle.getRecentDecisions(projectId, limit)
  }

  /** Get decisions by type */
  getDecisionsByType(projectId: string, decisionType: DecisionType): DecisionRecord[] {
    return this.handle.getDecisionsByType(projectId, decisionType)
  }

  /** Search decisions */
  searchDecisions(projectId: string, query: string): DecisionRecord[] {
    return this.handle.searchDecisions(projectId, query)
  }

  /** Delete a decision */
  deleteDecision(projectId: string, id: string): boolean {
    return this.handle.deleteDecision(projectId, id)
  }

  // ========================================================================
  // ADRs
  // ========================================================================

  /** Create and save an ADR */
  createAdr(projectId: string, input: CreateAdrInput): Adr {
    return this.handle.createAdr(projectId, input)
  }

  /** Get an ADR by ID */
  getAdr(projectId: string, id: string): Adr | null {
    return this.handle.getAdr(projectId, id)
  }

  /** Get all ADRs */
  getAllAdrs(projectId: string): Adr[] {
    return this.handle.getAllAdrs(projectId)
  }

  /** Format an ADR as markdown */
  formatAdrMarkdown(projectId: string, id: string): string | null {
    return this.handle.formatAdrMarkdown(projectId, id)
  }

  // ========================================================================
  // Statistics & Maintenance
  // ========================================================================

  /** Get edit statistics */
  getEditStats(projectId: string): EditStats {
    return this.handle.getEditStats(projectId)
  }

  /** Clean up old records */
  cleanup(projectId: string, beforeTimestamp: number): number {
    return this.handle.cleanup(projectId, beforeTimestamp)
  }

  /** Invalidate all history for a project */
  invalidate(projectId: string): void {
    this.handle.invalidate(projectId)
  }
}

/**
 * Create an in-memory history store (for testing)
 */
export function createMemoryHistoryStore(): HistoryStore {
  // Use a special path that the native code recognizes as in-memory
  const store = Object.create(HistoryStore.prototype) as HistoryStore
  Object.defineProperty(store, 'handle', { value: nativeBindings.createMemoryHistoryStore() })
  Object.defineProperty(store, 'dbPath', { value: ':memory:' })
  return store
}

// Export utility for checking native binding availability (always true now)
export const isHistoryNative = true
