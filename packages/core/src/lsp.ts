/**
 * LSP (Language Server Protocol) Client
 *
 * Provides high-level access to language servers for IDE-like features.
 */

// Re-export types from protocol for convenience
export type {
  LspServerInfo,
  LspServerStatus,
  LspLocation,
  LspPosition,
  LspRange,
  LspDiagnostic,
  LspDiagnosticSeverity,
  LspCompletionItem,
  LspHover,
  SupportedLspServer,
} from './protocol.js'

// ============================================================================
// Additional LSP types
// ============================================================================

export interface LspSymbol {
  /** Symbol name */
  name: string
  /** Symbol kind (Function, Class, Method, etc.) */
  kind: string
  /** Start line */
  startLine: number
  /** Start character */
  startCharacter: number
  /** End line */
  endLine: number
  /** End character */
  endCharacter: number
}

export interface LspTextEdit {
  /** Start line */
  startLine: number
  /** Start character */
  startCharacter: number
  /** End line */
  endLine: number
  /** End character */
  endCharacter: number
  /** New text to insert */
  newText: string
}

export interface LspFormatOptions {
  /** Tab size */
  tabSize?: number
  /** Use spaces instead of tabs */
  insertSpaces?: boolean
}

// ============================================================================
// Extension to Server mapping
// ============================================================================

export const LSP_EXTENSIONS: Record<string, SupportedLspServer | undefined> = {
  ts: 'typescript',
  tsx: 'typescript',
  js: 'typescript',
  jsx: 'typescript',
  mjs: 'typescript',
  cjs: 'typescript',
  rs: 'rust-analyzer',
  go: 'gopls',
  py: 'pyright',
  pyi: 'pyright',
  c: 'clangd',
  cpp: 'clangd',
  cc: 'clangd',
  cxx: 'clangd',
  h: 'clangd',
  hpp: 'clangd',
}

// Need to import the type, but avoid circular reference
import type { SupportedLspServer, LspServerStatus, LspCompletionItem } from './protocol.js'

// Try to load native LSP bindings
import type {
  LspServerManagerHandle,
  LspLocation as NativeLspLocation,
  LspSymbol as NativeLspSymbol,
  LspCompletionItem as NativeLspCompletionItem,
  LspTextEdit as NativeLspTextEdit,
} from './binding.d.ts'

let createNativeLspServerManager: (() => LspServerManagerHandle) | null = null
let nativeDetectLanguageId: ((extension: string) => string) | null = null

try {
  const bindings = await import('./binding.js')
  createNativeLspServerManager = bindings.createLspServerManager
  nativeDetectLanguageId = bindings.detectLanguageId
} catch {
  // Native bindings not available
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Convert a file path to a file URI
 */
export function pathToUri(filePath: string): string {
  if (filePath.startsWith('file://')) return filePath
  return `file://${filePath}`
}

/**
 * Convert a file URI to a file path
 */
export function uriToPath(uri: string): string {
  if (uri.startsWith('file://')) return uri.slice(7)
  return uri
}

/**
 * Get the language ID for a file extension
 */
export function getLanguageId(extension: string): string {
  const ext = extension.replace(/^\./, '').toLowerCase()
  const map: Record<string, string> = {
    ts: 'typescript',
    tsx: 'typescriptreact',
    js: 'javascript',
    jsx: 'javascriptreact',
    mjs: 'javascript',
    cjs: 'javascript',
    rs: 'rust',
    go: 'go',
    py: 'python',
    pyi: 'python',
    c: 'c',
    cpp: 'cpp',
    cc: 'cpp',
    cxx: 'cpp',
    h: 'c',
    hpp: 'cpp',
  }
  return map[ext] ?? ext
}

// ============================================================================
// Utility: Apply text edits to content
// ============================================================================

/**
 * Apply LSP text edits to content
 */
export function applyTextEdits(content: string, edits: LspTextEdit[]): string {
  // Sort edits in reverse order (bottom to top) to avoid offset issues
  const sortedEdits = [...edits].sort((a, b) => {
    if (a.startLine !== b.startLine) return b.startLine - a.startLine
    return b.startCharacter - a.startCharacter
  })

  const lines = content.split('\n')

  for (const edit of sortedEdits) {
    // Get the affected range
    const startLineText = lines[edit.startLine] ?? ''
    const endLineText = lines[edit.endLine] ?? ''

    // Build the new content
    const before = startLineText.slice(0, edit.startCharacter)
    const after = endLineText.slice(edit.endCharacter)
    const newContent = before + edit.newText + after

    // Replace the lines
    const newLines = newContent.split('\n')
    lines.splice(edit.startLine, edit.endLine - edit.startLine + 1, ...newLines)
  }

  return lines.join('\n')
}

// ============================================================================
// Native LSP Server Manager Wrapper
// ============================================================================

/**
 * Hover result with markdown content
 */
export interface HoverResult {
  /** Markdown content */
  contents: string
}

/**
 * LSP Server Manager - TypeScript wrapper for native Rust implementation
 *
 * Provides high-level access to language servers with automatic server management.
 *
 * @example
 * ```typescript
 * import { LspServerManager } from '@codecoder-ai/core'
 *
 * const manager = new LspServerManager()
 *
 * // Start server for a file
 * const key = await manager.startForFile('/path/to/file.ts')
 *
 * // Get hover info
 * const hover = await manager.hover(key, 'file:///path/to/file.ts', 10, 5)
 *
 * // Get completions
 * const completions = await manager.completion(key, 'file:///path/to/file.ts', 10, 5)
 *
 * // Go to definition
 * const definitions = await manager.gotoDefinition(key, 'file:///path/to/file.ts', 10, 5)
 * ```
 */
export class LspServerManager {
  private handle: LspServerManagerHandle | null = null

  constructor() {
    if (createNativeLspServerManager) {
      this.handle = createNativeLspServerManager()
    }
  }

  /**
   * Check if native implementation is available
   */
  get isNative(): boolean {
    return this.handle !== null
  }

  private ensureHandle(): LspServerManagerHandle {
    if (!this.handle) {
      throw new Error('LSP server manager native bindings not available')
    }
    return this.handle
  }

  // ===========================================================================
  // Basic Methods
  // ===========================================================================

  /**
   * Start an LSP server for a file (auto-detects server type)
   * @param filePath Path to the file
   * @returns Server key for subsequent operations
   */
  async startForFile(filePath: string): Promise<string> {
    const handle = this.ensureHandle()
    return handle.startForFile(filePath)
  }

  /**
   * Start a specific LSP server
   * @param serverId Server identifier (e.g., 'typescript', 'rust-analyzer')
   * @param root Project root path
   * @returns Server key for subsequent operations
   */
  async start(serverId: string, root: string): Promise<string> {
    const handle = this.ensureHandle()
    return handle.start(serverId, root)
  }

  /**
   * Send a raw LSP request
   * @param key Server key
   * @param method LSP method (e.g., 'textDocument/hover')
   * @param params Request parameters
   */
  async request<T = unknown>(key: string, method: string, params: unknown): Promise<T> {
    const handle = this.ensureHandle()
    return handle.request(key, method, params) as T
  }

  /**
   * Stop a running LSP server
   * @param key Server key
   */
  async stop(key: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.stop(key)
  }

  /**
   * Get the status of a server
   * @param key Server key
   */
  async status(key: string): Promise<LspServerStatus> {
    const handle = this.ensureHandle()
    const nativeStatus = await handle.status(key)
    return {
      ...nativeStatus,
      status: nativeStatus.status as LspServerStatus['status'],
    }
  }

  /**
   * Get status of all servers
   */
  async allStatuses(): Promise<Record<string, LspServerStatus>> {
    const handle = this.ensureHandle()
    const statuses = await handle.allStatuses()
    const result: Record<string, LspServerStatus> = {}
    for (const [name, nativeStatus] of Object.entries(statuses)) {
      result[name] = {
        ...nativeStatus,
        status: nativeStatus.status as LspServerStatus['status'],
      }
    }
    return result
  }

  /**
   * Stop all running servers
   */
  async stopAll(): Promise<void> {
    const handle = this.ensureHandle()
    await handle.stopAll()
  }

  // ===========================================================================
  // Document Synchronization
  // ===========================================================================

  /**
   * Notify the server that a document was opened
   * @param key Server key
   * @param uri Document URI
   * @param languageId Language identifier
   * @param version Document version
   * @param text Document content
   */
  async didOpen(key: string, uri: string, languageId: string, version: number, text: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.didOpen(key, uri, languageId, version, text)
  }

  /**
   * Notify the server that a document was closed
   * @param key Server key
   * @param uri Document URI
   */
  async didClose(key: string, uri: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.didClose(key, uri)
  }

  /**
   * Notify the server that a document was changed
   * @param key Server key
   * @param uri Document URI
   * @param version New document version
   * @param text New document content
   */
  async didChange(key: string, uri: string, version: number, text: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.didChange(key, uri, version, text)
  }

  // ===========================================================================
  // Convenience Methods
  // ===========================================================================

  /**
   * Get hover information at a position
   * @param key Server key
   * @param uri Document URI
   * @param line Line number (0-indexed)
   * @param character Character offset (0-indexed)
   * @returns Hover markdown content or null
   */
  async hover(key: string, uri: string, line: number, character: number): Promise<HoverResult | null> {
    const handle = this.ensureHandle()
    const content = await handle.hover(key, uri, line, character)
    return content ? { contents: content } : null
  }

  /**
   * Go to definition
   * @param key Server key
   * @param uri Document URI
   * @param line Line number (0-indexed)
   * @param character Character offset (0-indexed)
   * @returns Array of definition locations
   */
  async gotoDefinition(key: string, uri: string, line: number, character: number): Promise<LspSymbol[]> {
    const handle = this.ensureHandle()
    const locations = await handle.gotoDefinition(key, uri, line, character)
    return locations.map(this.fromNativeLocation)
  }

  /**
   * Go to type definition
   * @param key Server key
   * @param uri Document URI
   * @param line Line number (0-indexed)
   * @param character Character offset (0-indexed)
   * @returns Array of type definition locations
   */
  async gotoTypeDefinition(key: string, uri: string, line: number, character: number): Promise<LspSymbol[]> {
    const handle = this.ensureHandle()
    const locations = await handle.gotoTypeDefinition(key, uri, line, character)
    return locations.map(this.fromNativeLocation)
  }

  /**
   * Find all references to a symbol
   * @param key Server key
   * @param uri Document URI
   * @param line Line number (0-indexed)
   * @param character Character offset (0-indexed)
   * @param includeDeclaration Include the declaration in results
   * @returns Array of reference locations
   */
  async findReferences(
    key: string,
    uri: string,
    line: number,
    character: number,
    includeDeclaration = true
  ): Promise<LspSymbol[]> {
    const handle = this.ensureHandle()
    const locations = await handle.findReferences(key, uri, line, character, includeDeclaration)
    return locations.map(this.fromNativeLocation)
  }

  /**
   * Get document symbols (outline)
   * @param key Server key
   * @param uri Document URI
   * @returns Array of document symbols
   */
  async documentSymbols(key: string, uri: string): Promise<LspSymbol[]> {
    const handle = this.ensureHandle()
    const symbols = await handle.documentSymbols(key, uri)
    return symbols.map(this.fromNativeSymbol)
  }

  /**
   * Get completion items at a position
   * @param key Server key
   * @param uri Document URI
   * @param line Line number (0-indexed)
   * @param character Character offset (0-indexed)
   * @returns Array of completion items
   */
  async completion(key: string, uri: string, line: number, character: number): Promise<LspCompletionItem[]> {
    const handle = this.ensureHandle()
    const items = await handle.completion(key, uri, line, character)
    return items.map(this.fromNativeCompletionItem)
  }

  /**
   * Format a document
   * @param key Server key
   * @param uri Document URI
   * @param options Format options
   * @returns Array of text edits to apply
   */
  async formatDocument(key: string, uri: string, options?: LspFormatOptions): Promise<LspTextEdit[]> {
    const handle = this.ensureHandle()
    const edits = await handle.formatDocument(key, uri, options?.tabSize, options?.insertSpaces)
    return edits.map(this.fromNativeTextEdit)
  }

  // ===========================================================================
  // Private Converters
  // ===========================================================================

  private fromNativeLocation(loc: NativeLspLocation): LspSymbol {
    return {
      name: uriToPath(loc.uri),
      kind: 'Location',
      startLine: loc.startLine,
      startCharacter: loc.startCharacter,
      endLine: loc.endLine,
      endCharacter: loc.endCharacter,
    }
  }

  private fromNativeSymbol(sym: NativeLspSymbol): LspSymbol {
    return {
      name: sym.name,
      kind: sym.kind,
      startLine: sym.startLine,
      startCharacter: sym.startCharacter,
      endLine: sym.endLine,
      endCharacter: sym.endCharacter,
    }
  }

  private fromNativeCompletionItem(item: NativeLspCompletionItem): LspCompletionItem {
    return {
      label: item.label,
      kind: item.kind ? this.completionKindToNumber(item.kind) : undefined,
      detail: item.detail,
      documentation: item.documentation,
      insertText: item.insertText,
    }
  }

  /** Convert string completion kind to LSP number */
  private completionKindToNumber(kind: string): number {
    const map: Record<string, number> = {
      Text: 1,
      Method: 2,
      Function: 3,
      Constructor: 4,
      Field: 5,
      Variable: 6,
      Class: 7,
      Interface: 8,
      Module: 9,
      Property: 10,
      Unit: 11,
      Value: 12,
      Enum: 13,
      Keyword: 14,
      Snippet: 15,
      Color: 16,
      File: 17,
      Reference: 18,
      Folder: 19,
      EnumMember: 20,
      Constant: 21,
      Struct: 22,
      Event: 23,
      Operator: 24,
      TypeParameter: 25,
    }
    return map[kind] ?? 1 // Default to Text
  }

  private fromNativeTextEdit(edit: NativeLspTextEdit): LspTextEdit {
    return {
      startLine: edit.startLine,
      startCharacter: edit.startCharacter,
      endLine: edit.endLine,
      endCharacter: edit.endCharacter,
      newText: edit.newText,
    }
  }
}

/**
 * Check if LSP native bindings are available
 */
export const isLspNative = createNativeLspServerManager !== null

/**
 * Detect language ID for a file extension using native implementation
 * Falls back to getLanguageId if native not available
 */
export function detectLanguageIdNative(extension: string): string {
  return nativeDetectLanguageId ? nativeDetectLanguageId(extension) : getLanguageId(extension)
}
