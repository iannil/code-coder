/**
 * Tools API Client - Execute tools via the zero-api service
 */

import z from "zod"
import { getClient, type ZeroClient } from "./index"

// ============================================================================
// Tool Types (matching zero-core)
// ============================================================================

/** Grep options */
export const GrepOptions = z.object({
  pattern: z.string(),
  path: z.string().optional(),
  glob: z.string().optional(),
  case_insensitive: z.boolean().optional(),
  output_mode: z.enum(["content", "files_with_matches", "count"]).optional(),
  context_lines: z.number().optional(),
  max_results: z.number().optional(),
})

export type GrepOptions = z.infer<typeof GrepOptions>

/** Grep result */
export const GrepResult = z.object({
  matches: z.array(
    z.object({
      path: z.string(),
      line_number: z.number(),
      content: z.string(),
      context_before: z.array(z.string()).optional(),
      context_after: z.array(z.string()).optional(),
    })
  ),
  total_matches: z.number(),
  truncated: z.boolean(),
})

export type GrepResult = z.infer<typeof GrepResult>

/** Glob options */
export const GlobOptions = z.object({
  pattern: z.string(),
  path: z.string().optional(),
  max_results: z.number().optional(),
  include_hidden: z.boolean().optional(),
})

export type GlobOptions = z.infer<typeof GlobOptions>

/** Glob result */
export const GlobResult = z.object({
  files: z.array(z.string()),
  total_found: z.number(),
  truncated: z.boolean(),
})

export type GlobResult = z.infer<typeof GlobResult>

/** Read options */
export const ReadOptions = z.object({
  file_path: z.string(),
  offset: z.number().optional(),
  limit: z.number().optional(),
})

export type ReadOptions = z.infer<typeof ReadOptions>

/** Read result */
export const ReadResult = z.object({
  content: z.string(),
  line_count: z.number(),
  byte_size: z.number(),
  truncated: z.boolean(),
})

export type ReadResult = z.infer<typeof ReadResult>

/** Write options */
export const WriteOptions = z.object({
  file_path: z.string(),
  content: z.string(),
  create_dirs: z.boolean().optional(),
})

export type WriteOptions = z.infer<typeof WriteOptions>

/** Write result */
export const WriteResult = z.object({
  path: z.string(),
  bytes_written: z.number(),
  created: z.boolean(),
})

export type WriteResult = z.infer<typeof WriteResult>

/** Ls options */
export const LsOptions = z.object({
  path: z.string(),
  all: z.boolean().optional(),
  tree: z.boolean().optional(),
  depth: z.number().optional(),
})

export type LsOptions = z.infer<typeof LsOptions>

/** Ls result */
export const LsResult = z.object({
  output: z.string(),
  entry_count: z.number(),
})

export type LsResult = z.infer<typeof LsResult>

/** CodeSearch options */
export const CodeSearchOptions = z.object({
  query: z.string(),
  path: z.string().optional(),
  language: z.string().optional(),
  scope: z.enum(["function", "class", "method", "any"]).optional(),
  max_results: z.number().optional(),
})

export type CodeSearchOptions = z.infer<typeof CodeSearchOptions>

/** CodeSearch result */
export const CodeSearchResult = z.object({
  matches: z.array(
    z.object({
      path: z.string(),
      name: z.string(),
      kind: z.string(),
      start_line: z.number(),
      end_line: z.number(),
      content: z.string(),
      relevance: z.number(),
    })
  ),
  total_matches: z.number(),
})

export type CodeSearchResult = z.infer<typeof CodeSearchResult>

/** WebFetch options */
export const WebFetchOptions = z.object({
  url: z.string(),
  method: z.enum(["GET", "POST", "PUT", "DELETE", "HEAD"]).optional(),
  headers: z.record(z.string(), z.string()).optional(),
  body: z.string().optional(),
  timeout_ms: z.number().optional(),
})

export type WebFetchOptions = z.infer<typeof WebFetchOptions>

/** WebFetch result */
export const WebFetchResult = z.object({
  status_code: z.number(),
  headers: z.record(z.string(), z.string()),
  content: z.string(),
  content_type: z.string().optional(),
  elapsed_ms: z.number(),
})

export type WebFetchResult = z.infer<typeof WebFetchResult>

/** Truncate options */
export const TruncateOptions = z.object({
  content: z.string(),
  max_lines: z.number().optional(),
  max_chars: z.number().optional(),
  direction: z.enum(["head", "tail"]).optional(),
})

export type TruncateOptions = z.infer<typeof TruncateOptions>

/** Truncate result */
export const TruncateResult = z.object({
  content: z.string(),
  original_lines: z.number(),
  original_chars: z.number(),
  truncated: z.boolean(),
  saved_path: z.string().optional(),
})

export type TruncateResult = z.infer<typeof TruncateResult>

// ============================================================================
// Tool Info
// ============================================================================

export interface ToolInfo {
  name: string
  description: string
  available: boolean
}

// ============================================================================
// Tools Client
// ============================================================================

/**
 * Tools API client
 */
export class ToolsClient {
  constructor(private client: ZeroClient = getClient()) {}

  /**
   * List available tools
   */
  async list(): Promise<ToolInfo[]> {
    return this.client.get<ToolInfo[]>("/api/v1/tools")
  }

  /**
   * Execute grep search
   */
  async grep(options: GrepOptions): Promise<GrepResult> {
    return this.client.post<GrepResult>("/api/v1/tools/grep", { params: options })
  }

  /**
   * Execute glob pattern matching
   */
  async glob(options: GlobOptions): Promise<GlobResult> {
    return this.client.post<GlobResult>("/api/v1/tools/glob", { params: options })
  }

  /**
   * Read a file
   */
  async read(options: ReadOptions): Promise<ReadResult> {
    return this.client.post<ReadResult>("/api/v1/tools/read", { params: options })
  }

  /**
   * Write to a file
   */
  async write(options: WriteOptions): Promise<WriteResult> {
    return this.client.post<WriteResult>("/api/v1/tools/write", { params: options })
  }

  /**
   * List directory contents
   */
  async ls(options: LsOptions): Promise<LsResult> {
    return this.client.post<LsResult>("/api/v1/tools/ls", { params: options })
  }

  /**
   * Semantic code search
   */
  async codesearch(options: CodeSearchOptions): Promise<CodeSearchResult> {
    return this.client.post<CodeSearchResult>("/api/v1/tools/codesearch", { params: options })
  }

  /**
   * Fetch web content
   */
  async webfetch(options: WebFetchOptions): Promise<WebFetchResult> {
    return this.client.post<WebFetchResult>("/api/v1/tools/webfetch", { params: options })
  }

  /**
   * Truncate output
   */
  async truncate(options: TruncateOptions): Promise<TruncateResult> {
    return this.client.post<TruncateResult>("/api/v1/tools/truncate", { params: options })
  }

  /**
   * Execute any tool by name
   */
  async execute<T = unknown>(tool: string, params: Record<string, unknown>): Promise<T> {
    return this.client.post<T>(`/api/v1/tools/${tool}`, { params })
  }
}

/**
 * Singleton tools client
 */
let toolsClient: ToolsClient | null = null

/**
 * Get the tools client
 */
export function getToolsClient(): ToolsClient {
  if (!toolsClient) {
    toolsClient = new ToolsClient()
  }
  return toolsClient
}

/**
 * Namespace for tools API
 */
export namespace ZeroTools {
  export const list = () => getToolsClient().list()
  export const grep = (options: GrepOptions) => getToolsClient().grep(options)
  export const glob = (options: GlobOptions) => getToolsClient().glob(options)
  export const read = (options: ReadOptions) => getToolsClient().read(options)
  export const write = (options: WriteOptions) => getToolsClient().write(options)
  export const ls = (options: LsOptions) => getToolsClient().ls(options)
  export const codesearch = (options: CodeSearchOptions) => getToolsClient().codesearch(options)
  export const webfetch = (options: WebFetchOptions) => getToolsClient().webfetch(options)
  export const truncate = (options: TruncateOptions) => getToolsClient().truncate(options)
  export const execute = <T = unknown>(tool: string, params: Record<string, unknown>) =>
    getToolsClient().execute<T>(tool, params)
}
