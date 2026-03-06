/**
 * JavaScript fallback implementations when native bindings are not available.
 *
 * These implementations use Node.js/Bun built-in modules and provide
 * compatible (but slower) versions of the native tools.
 */

import * as fs from 'node:fs'
import * as path from 'node:path'
import type {
  GrepOptions,
  GrepResult,
  GrepMatch,
  GlobOptions,
  GlobResult,
  FileInfo,
  ReadOptions,
  ReadResult,
  EditOperation,
  EditResult,
  Permission,
  PermissionRule,
  SecretEntry,
  IPermissionManager,
  IVault,
} from './types.js'

/**
 * Fallback grep implementation using JavaScript
 */
export async function grep(options: GrepOptions): Promise<GrepResult> {
  const searchPath = options.path ?? '.'
  const pattern = new RegExp(options.pattern, options.caseInsensitive ? 'gi' : 'g')
  const outputMode = options.outputMode ?? 'files_with_matches'
  const limit = options.limit ?? 10000

  const matches: GrepMatch[] = []
  const files: string[] = []
  let totalMatches = 0
  let filesSearched = 0
  let truncated = false

  const walkDir = async (dir: string) => {
    if (truncated) return

    const entries = await fs.promises.readdir(dir, { withFileTypes: true })

    for (const entry of entries) {
      if (truncated) break

      const fullPath = path.join(dir, entry.name)

      // Skip hidden files and node_modules
      if (entry.name.startsWith('.') || entry.name === 'node_modules') continue

      if (entry.isDirectory()) {
        await walkDir(fullPath)
      } else if (entry.isFile()) {
        // Check glob filter
        if (options.glob && !minimatch(fullPath, options.glob)) continue

        try {
          const content = await fs.promises.readFile(fullPath, 'utf-8')
          const lines = content.split('\n')
          let fileHasMatch = false

          for (let i = 0; i < lines.length; i++) {
            const line = lines[i]!
            if (pattern.test(line)) {
              fileHasMatch = true
              totalMatches++

              if (outputMode === 'content' && matches.length < limit) {
                matches.push({
                  path: fullPath,
                  lineNumber: i + 1,
                  column: 0,
                  lineContent: line,
                })
              }
            }
          }

          if (fileHasMatch) {
            filesSearched++
            if (outputMode === 'files_with_matches' && files.length < limit) {
              files.push(fullPath)
            }
          }

          if ((outputMode === 'content' && matches.length >= limit) ||
              (outputMode === 'files_with_matches' && files.length >= limit)) {
            truncated = true
          }
        } catch {
          // Skip files that can't be read
        }
      }
    }
  }

  const stat = await fs.promises.stat(searchPath)
  if (stat.isFile()) {
    // Search single file
    const content = await fs.promises.readFile(searchPath, 'utf-8')
    const lines = content.split('\n')

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]!
      if (pattern.test(line)) {
        totalMatches++
        if (outputMode === 'content') {
          matches.push({
            path: searchPath,
            lineNumber: i + 1,
            column: 0,
            lineContent: line,
          })
        }
      }
    }

    if (totalMatches > 0) {
      filesSearched = 1
      if (outputMode === 'files_with_matches') {
        files.push(searchPath)
      }
    }
  } else {
    await walkDir(searchPath)
  }

  return {
    matches,
    files,
    totalMatches,
    filesSearched,
    truncated,
  }
}

/**
 * Simple minimatch implementation
 */
function minimatch(filePath: string, pattern: string): boolean {
  const regexPattern = pattern
    .replace(/\./g, '\\.')
    .replace(/\*\*/g, '.*')
    .replace(/\*/g, '[^/]*')
    .replace(/\?/g, '.')

  return new RegExp(`^${regexPattern}$`).test(filePath)
}

/**
 * Fallback glob implementation
 */
export async function glob(options: GlobOptions): Promise<GlobResult> {
  const start = Date.now()
  const searchPath = options.path ?? '.'
  const pattern = options.pattern
  const limit = options.limit ?? 10000
  const filesOnly = options.filesOnly ?? true

  const files: FileInfo[] = []
  let totalMatches = 0
  let truncated = false

  const walkDir = async (dir: string, depth: number = 0) => {
    if (truncated) return
    if (options.maxDepth !== undefined && depth > options.maxDepth) return

    const entries = await fs.promises.readdir(dir, { withFileTypes: true })

    for (const entry of entries) {
      if (truncated) break

      const fullPath = path.join(dir, entry.name)
      const relativePath = path.relative(searchPath, fullPath)

      // Skip hidden files unless includeHidden
      if (!options.includeHidden && entry.name.startsWith('.')) continue

      // Skip node_modules if respecting gitignore
      if (options.respectGitignore !== false && entry.name === 'node_modules') continue

      const isMatch = minimatch(relativePath, pattern) || minimatch(fullPath, pattern)

      if (entry.isDirectory()) {
        if (isMatch && !filesOnly) {
          totalMatches++
          if (files.length < limit) {
            const stat = await fs.promises.stat(fullPath)
            files.push({
              path: fullPath,
              size: 0,
              isDir: true,
              isSymlink: false,
              modified: Math.floor(stat.mtimeMs / 1000),
              extension: undefined,
            })
          } else {
            truncated = true
          }
        }
        await walkDir(fullPath, depth + 1)
      } else if (entry.isFile() && isMatch) {
        totalMatches++
        if (files.length < limit) {
          const stat = await fs.promises.stat(fullPath)
          files.push({
            path: fullPath,
            size: Number(stat.size),
            isDir: false,
            isSymlink: entry.isSymbolicLink(),
            modified: Math.floor(stat.mtimeMs / 1000),
            extension: path.extname(entry.name).slice(1) || undefined,
          })
        } else {
          truncated = true
        }
      }
    }
  }

  await walkDir(searchPath)

  // Sort by mtime if requested
  if (options.sortByMtime) {
    files.sort((a, b) => (b.modified ?? 0) - (a.modified ?? 0))
  }

  return {
    files,
    totalMatches,
    truncated,
    durationMs: Date.now() - start,
  }
}

/**
 * Fallback read implementation
 */
export function readFile(filePath: string, options?: ReadOptions): ReadResult {
  const content = fs.readFileSync(filePath, 'utf-8')
  const allLines = content.split('\n')
  const totalLines = allLines.length

  const offset = Math.max(0, (options?.offset ?? 1) - 1)
  const maxLines = options?.limit ?? totalLines
  const maxLineLength = options?.maxLineLength ?? 2000

  let lines = allLines.slice(offset, offset + maxLines)
  const truncated = offset + maxLines < totalLines

  // Truncate long lines
  lines = lines.map(line =>
    line.length > maxLineLength ? line.slice(0, maxLineLength) + '...' : line
  )

  // Format with line numbers if requested
  let formattedContent: string
  if (options?.lineNumbers !== false) {
    const width = String(offset + lines.length).length
    formattedContent = lines
      .map((line, i) => `${String(offset + i + 1).padStart(width)}\t${line}`)
      .join('\n')
  } else {
    formattedContent = lines.join('\n')
  }

  // Check for binary content
  const isBinary = content.includes('\0')

  return {
    content: isBinary ? '[Binary file content not shown]' : formattedContent,
    lines: isBinary ? [] : lines,
    totalLines,
    linesReturned: lines.length,
    truncated,
    size: Buffer.byteLength(content, 'utf-8'),
    isBinary,
  }
}

/**
 * Fallback edit implementation
 */
export function editFile(filePath: string, operation: EditOperation): EditResult {
  try {
    const content = fs.readFileSync(filePath, 'utf-8')

    // Check if old_string exists
    if (!content.includes(operation.oldString)) {
      return {
        success: false,
        replacements: 0,
        diff: '',
        error: `old_string not found in file: ${filePath}`,
      }
    }

    // Check uniqueness unless replaceAll
    if (!operation.replaceAll) {
      const count = content.split(operation.oldString).length - 1
      if (count > 1) {
        return {
          success: false,
          replacements: 0,
          diff: '',
          error: `old_string is not unique in file (found ${count} occurrences). Use replaceAll=true or provide more context.`,
        }
      }
    }

    // Perform replacement
    const newContent = operation.replaceAll
      ? content.replaceAll(operation.oldString, operation.newString)
      : content.replace(operation.oldString, operation.newString)

    const replacements = operation.replaceAll
      ? content.split(operation.oldString).length - 1
      : 1

    // Generate simple diff
    const diff = `--- a/${filePath}\n+++ b/${filePath}\n@@ -1 +1 @@\n-${operation.oldString}\n+${operation.newString}`

    // Write atomically
    const tempPath = `${filePath}.tmp.${Date.now()}`
    fs.writeFileSync(tempPath, newContent)
    fs.renameSync(tempPath, filePath)

    return {
      success: true,
      replacements,
      diff,
    }
  } catch (err) {
    return {
      success: false,
      replacements: 0,
      diff: '',
      error: String(err),
    }
  }
}

// ============================================================================
// Session fallback implementations
// ============================================================================

import type { Message, SessionData, IMessageStore, ISessionStore } from './types.js'

/**
 * Generate a UUID v4
 */
function generateUuid(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0
    const v = c === 'x' ? r : (r & 0x3) | 0x8
    return v.toString(16)
  })
}

/**
 * Fallback MessageStore implementation using pure JavaScript
 */
export class FallbackMessageStore implements IMessageStore {
  private messages_: Message[] = []

  push(message: Message): void {
    this.messages_.push(message)
  }

  messages(): Message[] {
    return [...this.messages_]
  }

  lastN(n: number): Message[] {
    const start = Math.max(0, this.messages_.length - n)
    return this.messages_.slice(start)
  }

  totalTokens(): number {
    return this.messages_.reduce((sum, msg) => sum + (msg.tokens ?? 0), 0)
  }

  clear(): void {
    this.messages_ = []
  }

  len(): number {
    return this.messages_.length
  }

  isEmpty(): boolean {
    return this.messages_.length === 0
  }
}

/**
 * Fallback SessionStore implementation using in-memory Map
 *
 * Note: This is a non-persistent fallback. For production use,
 * the native SQLite-based implementation is recommended.
 */
export class FallbackSessionStore implements ISessionStore {
  private sessions: Map<string, SessionData> = new Map()
  readonly path: string

  constructor(path: string) {
    this.path = path
    // Note: In fallback mode, we don't actually use the path
    // since we're storing in memory
  }

  save(session: SessionData): void {
    this.sessions.set(session.id, {
      ...session,
      messages: [...session.messages],
    })
  }

  load(id: string): SessionData | null {
    const session = this.sessions.get(id)
    if (!session) return null
    return {
      ...session,
      messages: [...session.messages],
    }
  }

  list(): SessionData[] {
    return Array.from(this.sessions.values())
      .map((s) => ({
        ...s,
        messages: [], // Don't include messages in list
      }))
      .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
  }

  delete(id: string): boolean {
    return this.sessions.delete(id)
  }
}

/**
 * Create a new user message
 */
export function createUserMessage(content: string): Message {
  return {
    id: generateUuid(),
    role: 'user',
    content,
    timestamp: new Date().toISOString(),
    compacted: false,
  }
}

/**
 * Create a new assistant message
 */
export function createAssistantMessage(content: string): Message {
  return {
    id: generateUuid(),
    role: 'assistant',
    content,
    timestamp: new Date().toISOString(),
    compacted: false,
  }
}

/**
 * Create a new system message
 */
export function createSystemMessage(content: string): Message {
  return {
    id: generateUuid(),
    role: 'system',
    content,
    timestamp: new Date().toISOString(),
    compacted: false,
  }
}

/**
 * Create a new tool message
 */
export function createToolMessage(
  toolCallId: string,
  toolName: string,
  content: string
): Message {
  return {
    id: generateUuid(),
    role: 'tool',
    content,
    timestamp: new Date().toISOString(),
    toolCallId,
    toolName,
    compacted: false,
  }
}

/**
 * Create a new session
 */
export function createSession(cwd: string, name?: string): SessionData {
  const now = new Date().toISOString()
  return {
    id: generateUuid(),
    name,
    cwd,
    createdAt: now,
    updatedAt: now,
    messages: [],
  }
}

/**
 * Estimate token count for a message (rough approximation)
 */
export function estimateTokens(content: string): number {
  // Simple estimation: ~4 characters per token
  return Math.ceil(content.length / 4)
}

// ============================================================================
// Security fallback implementations
// ============================================================================

/**
 * Fallback PermissionManager implementation
 *
 * Provides in-memory permission management when native bindings are unavailable.
 */
export class FallbackPermissionManager implements IPermissionManager {
  private rules: PermissionRule[] = []
  private granted: Set<string> = new Set()

  /** Add a permission rule */
  addRule(rule: PermissionRule): void {
    this.rules.push(rule)
  }

  /** Grant a specific permission */
  grant(permission: Permission): void {
    const key = `${permission.tool}:${permission.action}:${permission.resource ?? '*'}`
    this.granted.add(key)
  }

  /** Check if a permission is allowed */
  check(permission: Permission): boolean {
    const key = `${permission.tool}:${permission.action}:${permission.resource ?? '*'}`
    if (this.granted.has(key)) {
      return true
    }

    // Check rules in reverse order (last rule wins)
    for (let i = this.rules.length - 1; i >= 0; i--) {
      const rule = this.rules[i]!
      if (this.matchesPermission(rule.permission, permission)) {
        return rule.allow
      }
    }

    // Default deny
    return false
  }

  /** Clear all rules and grants */
  clear(): void {
    this.rules = []
    this.granted.clear()
  }

  private matchesPermission(pattern: Permission, target: Permission): boolean {
    return (
      this.matchesPattern(pattern.tool, target.tool) &&
      this.matchesPattern(pattern.action, target.action) &&
      this.matchesResource(pattern.resource, target.resource)
    )
  }

  private matchesPattern(pattern: string, value: string): boolean {
    if (pattern === '*') return true
    if (pattern.endsWith('*')) {
      return value.startsWith(pattern.slice(0, -1))
    }
    if (pattern.startsWith('*')) {
      return value.endsWith(pattern.slice(1))
    }
    return pattern === value
  }

  private matchesResource(pattern: string | undefined, value: string | undefined): boolean {
    if (pattern === undefined) return true
    if (value === undefined) return false
    return this.matchesPattern(pattern, value)
  }
}

/**
 * Fallback Vault implementation
 *
 * Provides in-memory secret storage when native bindings are unavailable.
 * WARNING: This does NOT provide encryption - use only for development/testing.
 */
export class FallbackVault implements IVault {
  private secrets: Map<string, SecretEntry> = new Map()
  readonly path: string

  constructor(path: string, _password: string) {
    this.path = path
    // Note: In fallback mode, password is ignored (no encryption)
  }

  /** Store a secret */
  set(entry: SecretEntry): void {
    this.secrets.set(entry.name, { ...entry })
  }

  /** Get a secret by name */
  get(name: string): SecretEntry | null {
    const entry = this.secrets.get(name)
    return entry ? { ...entry } : null
  }

  /** Get just the secret value */
  getValue(name: string): string | null {
    return this.secrets.get(name)?.value ?? null
  }

  /** Delete a secret */
  delete(name: string): boolean {
    return this.secrets.delete(name)
  }

  /** List all secret names */
  list(): string[] {
    return Array.from(this.secrets.keys())
  }

  /** Save the vault (no-op in fallback mode) */
  save(): void {
    // In fallback mode, vault is in-memory only
    // No persistence available
  }
}

/**
 * Create an in-memory vault (for testing) - internal use only
 * Use the exported version from security.ts instead
 */
function createFallbackMemoryVault(password: string): FallbackVault {
  return new FallbackVault(':memory:', password)
}
