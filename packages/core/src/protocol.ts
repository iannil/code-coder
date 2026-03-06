/**
 * Protocol types for MCP Client and LSP Server
 */

// ============================================================================
// MCP Client types
// ============================================================================

/**
 * MCP transport type
 */
export type McpTransportType = 'stdio' | 'http' | 'sse'

/**
 * MCP client configuration
 */
export interface McpClientConfig {
  /** Client name */
  name: string
  /** Transport type */
  transport: McpTransportType
  /** Command to spawn (for stdio transport) */
  command?: string[]
  /** URL to connect to (for http/sse transport) */
  url?: string
  /** Environment variables (for stdio transport) */
  environment?: Record<string, string>
  /** Connection timeout in milliseconds */
  timeoutMs?: number
  /** HTTP headers (for http/sse transport) */
  headers?: Record<string, string>
  /** Working directory (for stdio transport) */
  cwd?: string
  /** OAuth configuration */
  oauth?: OAuthConfig
  /** Whether OAuth is disabled */
  oauthDisabled?: boolean
}

/**
 * OAuth configuration for an MCP server
 */
export interface OAuthConfig {
  /** Pre-registered client ID (optional) */
  clientId?: string
  /** Pre-registered client secret (optional) */
  clientSecret?: string
  /** OAuth scopes to request */
  scope?: string
}

/**
 * OAuth authentication status
 */
export type AuthStatus = 'not_authenticated' | 'authenticated' | 'expired'

/**
 * MCP connection status
 */
export interface McpConnectionStatus {
  /** Status type */
  status: 'connected' | 'disabled' | 'failed' | 'needs_auth' | 'needs_client_registration'
  /** Error message (for failed/needs_client_registration) */
  error?: string
}

/**
 * MCP tool definition
 */
export interface McpTool {
  /** Tool name */
  name: string
  /** Tool description */
  description: string
  /** Input schema (JSON Schema) */
  inputSchema: unknown
}

/**
 * MCP tool call result content item
 */
export interface McpContentItem {
  /** Content type (text, image, resource) */
  type: 'text' | 'image' | 'resource'
  /** Text content */
  text?: string
  /** Image data (base64) */
  data?: string
  /** MIME type */
  mimeType?: string
  /** Resource URI */
  uri?: string
}

/**
 * MCP tool call result
 */
export interface McpToolResult {
  /** Content items */
  content: McpContentItem[]
  /** Whether this is an error */
  isError: boolean
}

/**
 * MCP resource definition
 */
export interface McpResource {
  /** Resource URI */
  uri: string
  /** Resource name */
  name: string
  /** Resource description */
  description?: string
  /** MIME type */
  mimeType?: string
}

/**
 * MCP prompt definition
 */
export interface McpPrompt {
  /** Prompt name */
  name: string
  /** Prompt description */
  description?: string
  /** Prompt arguments */
  arguments: McpPromptArgument[]
}

/**
 * MCP prompt argument
 */
export interface McpPromptArgument {
  /** Argument name */
  name: string
  /** Argument description */
  description?: string
  /** Whether the argument is required */
  required: boolean
}

/**
 * MCP prompt result
 */
export interface McpPromptResult {
  /** Description of the prompt result */
  description?: string
  /** Messages from the prompt */
  messages: McpPromptMessage[]
}

/**
 * MCP prompt message
 */
export interface McpPromptMessage {
  /** Role of the message (user, assistant) */
  role: string
  /** Content of the message */
  content: unknown
}

/**
 * Interface for MCP Client Manager
 */
export interface IMcpClientManager {
  /** Add a client with the given configuration */
  add(name: string, config: McpClientConfig): Promise<McpConnectionStatus>
  /** Get connection status for all clients */
  status(): Promise<Record<string, McpConnectionStatus>>
  /** List all tools from all connected clients */
  listTools(): Promise<Record<string, McpTool>>
  /** Call a tool on a specific client */
  callTool(clientName: string, toolName: string, args: unknown): Promise<McpToolResult>
  /** Remove a client */
  remove(name: string): Promise<void>
  /** Close all connections */
  closeAll(): Promise<void>

  // Resource methods
  /** List resources from a specific client */
  listResources(clientName: string): Promise<McpResource[]>
  /** Read a resource from a specific client */
  readResource(clientName: string, uri: string): Promise<unknown>

  // Prompt methods
  /** List prompts from a specific client */
  listPrompts(clientName: string): Promise<McpPrompt[]>
  /** Get a prompt from a specific client */
  getPrompt(clientName: string, promptName: string, args?: Record<string, unknown>): Promise<McpPromptResult>

  // OAuth methods
  /** Load OAuth credentials from storage */
  loadOAuth(): Promise<void>
  /** Start OAuth authentication flow, returns authorization URL */
  startOAuth(serverName: string, serverUrl: string, redirectUri: string, config?: OAuthConfig): Promise<string>
  /** Complete OAuth authentication with authorization code */
  finishOAuth(serverName: string, authorizationCode: string, state: string): Promise<void>
  /** Remove OAuth credentials for a server */
  removeOAuth(serverName: string): Promise<void>
  /** Get OAuth authentication status for a server */
  getOAuthStatus(serverName: string): Promise<AuthStatus>
  /** Check if we have OAuth credentials for a server */
  hasOAuthCredentials(serverName: string): Promise<boolean>
  /** Cancel any pending OAuth flow for a server */
  cancelOAuth(serverName: string): Promise<void>
}

// ============================================================================
// LSP Server types
// ============================================================================

/**
 * LSP server information
 */
export interface LspServerInfo {
  /** Server identifier */
  id: string
  /** Supported file extensions */
  extensions: string[]
  /** Whether this is a global server */
  global?: boolean
}

/**
 * LSP server status
 */
export interface LspServerStatus {
  /** Status type */
  status: 'running' | 'starting' | 'stopped' | 'failed' | 'not_found'
  /** Error message (for failed status) */
  error?: string
}

/**
 * LSP initialization options
 */
export interface LspInitOptions {
  /** TypeScript SDK path */
  tsdk?: string
  /** Python path */
  pythonPath?: string
  /** Additional settings */
  settings?: Record<string, unknown>
}

/**
 * LSP position (0-indexed)
 */
export interface LspPosition {
  /** Line number (0-indexed) */
  line: number
  /** Character offset (0-indexed) */
  character: number
}

/**
 * LSP range
 */
export interface LspRange {
  /** Start position */
  start: LspPosition
  /** End position */
  end: LspPosition
}

/**
 * LSP location
 */
export interface LspLocation {
  /** Document URI */
  uri: string
  /** Range in the document */
  range: LspRange
}

/**
 * LSP diagnostic severity
 */
export type LspDiagnosticSeverity = 1 | 2 | 3 | 4 // Error, Warning, Information, Hint

/**
 * LSP diagnostic
 */
export interface LspDiagnostic {
  /** Range of the diagnostic */
  range: LspRange
  /** Severity */
  severity?: LspDiagnosticSeverity
  /** Diagnostic code */
  code?: string | number
  /** Source of the diagnostic */
  source?: string
  /** Diagnostic message */
  message: string
}

/**
 * LSP completion item
 */
export interface LspCompletionItem {
  /** Label to display */
  label: string
  /** Kind of completion */
  kind?: number
  /** Detail text */
  detail?: string
  /** Documentation */
  documentation?: string | { kind: 'markdown' | 'plaintext'; value: string }
  /** Text to insert */
  insertText?: string
}

/**
 * LSP hover result
 */
export interface LspHover {
  /** Hover contents */
  contents: string | { kind: 'markdown' | 'plaintext'; value: string } | Array<string | { kind: 'markdown' | 'plaintext'; value: string }>
  /** Range of the hovered text */
  range?: LspRange
}

/**
 * LSP document symbol
 */
export interface LspDocumentSymbol {
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

/**
 * LSP workspace symbol (includes container name and file URI)
 */
export interface LspWorkspaceSymbol {
  /** Symbol name */
  name: string
  /** Symbol kind (Function, Class, Method, etc.) */
  kind: string
  /** Container name (e.g., class name for a method) */
  containerName?: string
  /** File URI */
  uri: string
  /** Start line */
  startLine: number
  /** Start character */
  startCharacter: number
  /** End line */
  endLine: number
  /** End character */
  endCharacter: number
}

/**
 * LSP call hierarchy item
 */
export interface LspCallHierarchyItem {
  /** Symbol name */
  name: string
  /** Symbol kind (Function, Method, etc.) */
  kind: string
  /** Detail (e.g., signature) */
  detail?: string
  /** File URI */
  uri: string
  /** Range start line */
  startLine: number
  /** Range start character */
  startCharacter: number
  /** Range end line */
  endLine: number
  /** Range end character */
  endCharacter: number
  /** Selection range start line */
  selectionStartLine: number
  /** Selection range start character */
  selectionStartCharacter: number
  /** Selection range end line */
  selectionEndLine: number
  /** Selection range end character */
  selectionEndCharacter: number
}

/**
 * LSP call hierarchy range
 */
export interface LspCallRange {
  /** Start line */
  startLine: number
  /** Start character */
  startCharacter: number
  /** End line */
  endLine: number
  /** End character */
  endCharacter: number
}

/**
 * LSP call hierarchy incoming call
 */
export interface LspCallHierarchyIncomingCall {
  /** The item that makes the call */
  from: LspCallHierarchyItem
  /** Ranges where this call happens */
  fromRanges: LspCallRange[]
}

/**
 * LSP call hierarchy outgoing call
 */
export interface LspCallHierarchyOutgoingCall {
  /** The item being called */
  to: LspCallHierarchyItem
  /** Ranges where this call happens */
  fromRanges: LspCallRange[]
}

/**
 * Interface for LSP Server Manager
 */
export interface ILspServerManager {
  /** Start a language server for a file (auto-detects based on extension) */
  startForFile(filePath: string): Promise<string>
  /** Start a specific language server */
  start(serverId: string, root: string): Promise<string>
  /** Send a request to a language server */
  request<T = unknown>(key: string, method: string, params: unknown): Promise<T>
  /** Stop a language server */
  stop(key: string): Promise<void>
  /** Get status of a language server */
  status(key: string): Promise<LspServerStatus>
  /** Get all server statuses */
  allStatuses(): Promise<Record<string, LspServerStatus>>
  /** Stop all language servers */
  stopAll(): Promise<void>

  // Document operations
  /** Get hover information at a position */
  hover(key: string, uri: string, line: number, character: number): Promise<string | null>
  /** Go to definition */
  gotoDefinition(key: string, uri: string, line: number, character: number): Promise<LspLocation[]>
  /** Go to type definition */
  gotoTypeDefinition(key: string, uri: string, line: number, character: number): Promise<LspLocation[]>
  /** Find references */
  findReferences(key: string, uri: string, line: number, character: number, includeDeclaration?: boolean): Promise<LspLocation[]>
  /** Get document symbols */
  documentSymbols(key: string, uri: string): Promise<LspDocumentSymbol[]>

  // Workspace operations
  /** Search for symbols in the workspace */
  workspaceSymbol(key: string, query: string): Promise<LspWorkspaceSymbol[]>

  // Call hierarchy operations
  /** Prepare call hierarchy items at a position */
  prepareCallHierarchy(key: string, uri: string, line: number, character: number): Promise<LspCallHierarchyItem[]>
  /** Get incoming calls for a call hierarchy item */
  incomingCalls(key: string, item: LspCallHierarchyItem): Promise<LspCallHierarchyIncomingCall[]>
  /** Get outgoing calls from a call hierarchy item */
  outgoingCalls(key: string, item: LspCallHierarchyItem): Promise<LspCallHierarchyOutgoingCall[]>
}

// ============================================================================
// Supported language servers
// ============================================================================

/**
 * Supported language server types
 */
export type SupportedLspServer =
  | 'typescript'
  | 'deno'
  | 'rust-analyzer'
  | 'gopls'
  | 'pyright'
  | 'clangd'
  | 'vue'
  | 'svelte'
  | 'eslint'
  | 'biome'
  | 'java'
  | 'kotlin'
  | 'csharp'
  | 'fsharp'
  | 'swift'
  | 'elixir'
  | 'zig'
  | 'ruby'
  | 'lua'
  | 'php'
  | 'yaml'
  | 'terraform'
  | 'docker'
  | 'bash'
  | 'latex'
  | 'gleam'
  | 'clojure'
  | 'nix'
  | 'typst'
  | 'haskell'
  | 'ocaml'
  | 'astro'
  | 'prisma'
  | 'dart'
