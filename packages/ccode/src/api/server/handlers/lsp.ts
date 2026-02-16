/**
 * LSP API Handlers
 *
 * Handles LSP (Language Server Protocol) operations:
 * - LSP server status
 * - Diagnostics
 * - Configuration
 */

import type { RouteHandler } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { LSP } from "@/lsp"
import { Config } from "@/config/config"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    return "{}"
  }
  return await new Response(body).text()
}

// ============================================================================
// LSP Status Handlers
// ============================================================================

/**
 * Get LSP server status
 * GET /api/lsp/status
 */
export const getLspStatus: RouteHandler = async () => {
  try {
    const status = await LSP.status()
    return jsonResponse({ success: true, data: status })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get LSP diagnostics
 * GET /api/lsp/diagnostics
 */
export const getLspDiagnostics: RouteHandler = async () => {
  try {
    const diagnostics = await LSP.diagnostics()

    // Transform to a more usable format
    const formatted = Object.entries(diagnostics).map(([filePath, diags]) => ({
      filePath,
      diagnostics: diags.map((d) => ({
        ...d,
        pretty: LSP.Diagnostic.pretty(d),
      })),
    }))

    return jsonResponse({ success: true, data: formatted })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get LSP configuration
 * GET /api/lsp/config
 */
export const getLspConfig: RouteHandler = async () => {
  try {
    const cfg = await Config.get()

    // LSP config can be false (disabled) or an object
    const lspConfig = cfg.lsp === false ? { enabled: false } : { enabled: true, servers: cfg.lsp ?? {} }

    return jsonResponse({ success: true, data: lspConfig })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Check if LSP is available for a file
 * GET /api/lsp/available
 */
export const checkLspAvailable: RouteHandler = async (req) => {
  try {
    const filePath = req.url.searchParams.get("file")

    if (!filePath) {
      return errorResponse("File path required", 400)
    }

    const available = await LSP.hasClients(filePath)
    return jsonResponse({ success: true, data: { available, filePath } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Initialize LSP for the current project
 * POST /api/lsp/init
 */
export const initLsp: RouteHandler = async () => {
  try {
    await LSP.init()
    const status = await LSP.status()
    return jsonResponse({ success: true, data: { initialized: true, status } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Touch a file to trigger LSP analysis
 * POST /api/lsp/touch
 */
export const touchFile: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.filePath) {
      return errorResponse("File path required", 400)
    }

    await LSP.touchFile(body.filePath, body.waitForDiagnostics ?? false)
    return jsonResponse({ success: true })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// LSP Operations Handlers
// ============================================================================

/**
 * Get hover information
 * POST /api/lsp/hover
 */
export const getHover: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.file || body.line === undefined || body.character === undefined) {
      return errorResponse("File, line, and character required", 400)
    }

    const result = await LSP.hover({
      file: body.file,
      line: body.line,
      character: body.character,
    })

    return jsonResponse({ success: true, data: result })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Go to definition
 * POST /api/lsp/definition
 */
export const getDefinition: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.file || body.line === undefined || body.character === undefined) {
      return errorResponse("File, line, and character required", 400)
    }

    const result = await LSP.definition({
      file: body.file,
      line: body.line,
      character: body.character,
    })

    return jsonResponse({ success: true, data: result })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Find references
 * POST /api/lsp/references
 */
export const getReferences: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.file || body.line === undefined || body.character === undefined) {
      return errorResponse("File, line, and character required", 400)
    }

    const result = await LSP.references({
      file: body.file,
      line: body.line,
      character: body.character,
    })

    return jsonResponse({ success: true, data: result })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get workspace symbols
 * POST /api/lsp/workspace-symbols
 */
export const getWorkspaceSymbols: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    const query = body.query ?? ""
    const result = await LSP.workspaceSymbol(query)

    return jsonResponse({ success: true, data: result })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get document symbols
 * POST /api/lsp/document-symbols
 */
export const getDocumentSymbols: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.uri) {
      return errorResponse("URI required", 400)
    }

    const result = await LSP.documentSymbol(body.uri)
    return jsonResponse({ success: true, data: result })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
