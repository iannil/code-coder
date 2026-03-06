/**
 * Grep search options
 */
export interface GrepOptions {
  /** The regex pattern to search for */
  pattern: string
  /** The path to search in (file or directory) */
  path?: string
  /** Glob pattern to filter files (e.g., "*.rs", "*.{ts,tsx}") */
  glob?: string
  /** File type to search (e.g., "rust", "typescript") */
  fileType?: string
  /** Whether to perform case-insensitive search */
  caseInsensitive?: boolean
  /** Output mode: "content", "files_with_matches", or "count" */
  outputMode?: 'content' | 'files_with_matches' | 'count'
  /** Number of context lines before match */
  contextBefore?: number
  /** Number of context lines after match */
  contextAfter?: number
  /** Limit the number of results */
  limit?: number
  /** Skip first N results */
  offset?: number
  /** Enable multiline matching */
  multiline?: boolean
  /** Show line numbers */
  lineNumbers?: boolean
}

/**
 * A single grep match
 */
export interface GrepMatch {
  /** Path to the file containing the match */
  path: string
  /** Line number (1-indexed) */
  lineNumber: number
  /** Column offset of the match (0-indexed) */
  column: number
  /** The matched line content */
  lineContent: string
}

/**
 * Result of a grep search
 */
export interface GrepResult {
  /** List of matches found */
  matches: GrepMatch[]
  /** List of files with matches (for files_with_matches mode) */
  files: string[]
  /** Total number of matches */
  totalMatches: number
  /** Total number of files searched */
  filesSearched: number
  /** Whether the search was truncated due to limits */
  truncated: boolean
}

/**
 * Glob search options
 */
export interface GlobOptions {
  /** The glob pattern to match (e.g., "**\/*.rs", "src/**\/*.ts") */
  pattern: string
  /** The path to search in (defaults to current directory) */
  path?: string
  /** Whether to include hidden files (starting with .) */
  includeHidden?: boolean
  /** Whether to respect .gitignore files */
  respectGitignore?: boolean
  /** Maximum depth to traverse (undefined for unlimited) */
  maxDepth?: number
  /** Limit the number of results */
  limit?: number
  /** Sort results by modification time (newest first) */
  sortByMtime?: boolean
  /** Only include files (no directories) */
  filesOnly?: boolean
  /** Follow symbolic links */
  followSymlinks?: boolean
}

/**
 * File information
 */
export interface FileInfo {
  /** File path */
  path: string
  /** File size in bytes */
  size: number
  /** Whether this is a directory */
  isDir: boolean
  /** Whether this is a symlink */
  isSymlink: boolean
  /** Last modification time (Unix timestamp) */
  modified?: number
  /** File extension (without the dot) */
  extension?: string
}

/**
 * Result of a glob search
 */
export interface GlobResult {
  /** List of matching files/directories */
  files: FileInfo[]
  /** Total number of matches (before limit) */
  totalMatches: number
  /** Whether the search was truncated due to limits */
  truncated: boolean
  /** Search duration in milliseconds */
  durationMs: number
}

/**
 * Read file options
 */
export interface ReadOptions {
  /** Starting line number (1-indexed, default: 1) */
  offset?: number
  /** Number of lines to read */
  limit?: number
  /** Maximum line length before truncation */
  maxLineLength?: number
  /** Whether to include line numbers in output */
  lineNumbers?: boolean
}

/**
 * Result of reading a file
 */
export interface ReadResult {
  /** The file content (with optional line numbers) */
  content: string
  /** Lines read as an array */
  lines: string[]
  /** Total number of lines in the file */
  totalLines: number
  /** Lines actually returned */
  linesReturned: number
  /** Whether content was truncated */
  truncated: boolean
  /** File size in bytes */
  size: number
  /** Whether the file appears to be binary */
  isBinary: boolean
}

/**
 * Edit operation
 */
export interface EditOperation {
  /** The text to find and replace */
  oldString: string
  /** The replacement text */
  newString: string
  /** Whether to replace all occurrences */
  replaceAll?: boolean
}

/**
 * Result of an edit operation
 */
export interface EditResult {
  /** Whether the edit was successful */
  success: boolean
  /** Number of replacements made */
  replacements: number
  /** The unified diff showing changes */
  diff: string
  /** Error message if failed */
  error?: string
}

/**
 * Result of a fuzzy replace operation
 */
export interface FuzzyReplaceResult {
  /** The new content after replacement */
  content: string
  /** Whether a match was found */
  found: boolean
  /** The actual string that was matched (may differ from old_string) */
  matchedString?: string
  /** Which replacer strategy succeeded */
  strategy?: string
  /** Error message if no match found */
  error?: string
}

// ============================================================================
// Session types
// ============================================================================

/**
 * Message role in a conversation
 */
export type MessageRole = 'system' | 'user' | 'assistant' | 'tool'

/**
 * A message in a conversation
 */
export interface Message {
  /** Unique message ID */
  id: string
  /** Message role */
  role: MessageRole
  /** Message content */
  content: string
  /** Timestamp in ISO 8601 format */
  timestamp: string
  /** Token count (estimated) */
  tokens?: number
  /** Associated tool call ID (for tool messages) */
  toolCallId?: string
  /** Tool name (for tool messages) */
  toolName?: string
  /** Whether this message has been compacted */
  compacted: boolean
}

/**
 * Session data
 */
export interface SessionData {
  /** Session ID */
  id: string
  /** Session name */
  name?: string
  /** Working directory */
  cwd: string
  /** Creation timestamp in ISO 8601 format */
  createdAt: string
  /** Last updated timestamp in ISO 8601 format */
  updatedAt: string
  /** Messages in this session */
  messages: Message[]
}

/**
 * Interface for MessageStore
 */
export interface IMessageStore {
  /** Add a message to the store */
  push(message: Message): void
  /** Get all messages */
  messages(): Message[]
  /** Get the last N messages */
  lastN(n: number): Message[]
  /** Get total token count */
  totalTokens(): number
  /** Clear all messages */
  clear(): void
  /** Get message count */
  len(): number
  /** Check if store is empty */
  isEmpty(): boolean
}

/**
 * Interface for SessionStore
 */
export interface ISessionStore {
  /** Save a session */
  save(session: SessionData): void
  /** Load a session by ID */
  load(id: string): SessionData | null
  /** List all sessions */
  list(): SessionData[]
  /** Delete a session */
  delete(id: string): boolean
}

// ============================================================================
// Security types
// ============================================================================

/**
 * A permission for access control
 */
export interface Permission {
  /** Tool name */
  tool: string
  /** Action pattern (can include wildcards) */
  action: string
  /** Resource pattern (optional, can include wildcards) */
  resource?: string
}

/**
 * A permission rule (allow or deny)
 */
export interface PermissionRule {
  /** The permission being granted or denied */
  permission: Permission
  /** Whether this is an allow (true) or deny (false) rule */
  allow: boolean
  /** Optional reason for this rule */
  reason?: string
}

/**
 * A secret entry in the vault
 */
export interface SecretEntry {
  /** Secret name/key */
  name: string
  /** Secret value */
  value: string
  /** Optional description */
  description?: string
}

/**
 * Interface for PermissionManager
 */
export interface IPermissionManager {
  /** Add a permission rule */
  addRule(rule: PermissionRule): void
  /** Grant a specific permission */
  grant(permission: Permission): void
  /** Check if a permission is allowed */
  check(permission: Permission): boolean
  /** Clear all rules and grants */
  clear(): void
}

/**
 * Interface for Vault
 */
export interface IVault {
  /** Store a secret */
  set(entry: SecretEntry): void
  /** Get a secret by name */
  get(name: string): SecretEntry | null
  /** Get just the secret value */
  getValue(name: string): string | null
  /** Delete a secret */
  delete(name: string): boolean
  /** List all secret names */
  list(): string[]
  /** Save the vault to disk (if backed by file) */
  save(): void
}

// ============================================================================
// Risk Assessment types
// ============================================================================

// RiskLevel is already defined in audit.ts - import from there
// export type RiskLevel = 'safe' | 'low' | 'medium' | 'high' | 'critical'

/**
 * Result of a risk assessment
 */
export interface RiskResult {
  /** Risk level as string */
  risk: string
  /** Human-readable reason for the risk level */
  reason: string
  /** Whether this operation can be auto-approved (not critical) */
  autoApprovable: boolean
}

// ============================================================================
// Shell Parser types (Phase 5)
// ============================================================================

/**
 * A parsed shell command with its arguments
 */
export interface NapiParsedCommand {
  /** The command name (e.g., "cd", "rm", "git") */
  name: string
  /** Command arguments */
  args: string[]
  /** Raw text of the entire command */
  raw: string
  /** Start byte position in source */
  startByte: number
  /** End byte position in source */
  endByte: number
}

/**
 * Result of parsing a shell command string
 */
export interface NapiShellParseResult {
  /** Successfully parsed commands */
  commands: NapiParsedCommand[]
  /** Whether parsing was successful */
  success: boolean
  /** Error message if parsing failed */
  error?: string
  /** Parse duration in milliseconds */
  durationMs: number
}

/**
 * Risk level for a command
 */
export type NapiCommandRiskLevel = 'Safe' | 'Low' | 'Medium' | 'High' | 'Critical'

/**
 * Risk assessment result for shell commands
 */
export interface NapiShellRiskAssessment {
  /** Overall risk level as string */
  level: string
  /** Reason for the risk level */
  reason: string
  /** Whether auto-approval is possible */
  autoApprovable: boolean
  /** Commands that contributed to the risk level */
  riskyCommands: string[]
}

/**
 * Permission patterns result
 */
export interface NapiPermissionPatterns {
  /** Exact command patterns */
  patterns: string[]
  /** Wildcard patterns for "always" permissions */
  alwaysPatterns: string[]
}
