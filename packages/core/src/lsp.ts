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
import type { SupportedLspServer } from './protocol.js'

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
