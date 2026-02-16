/**
 * MCP API Handler
 * Handles /api/mcp endpoints for MCP (Model Context Protocol) server management
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/mcp/status
 * Get status of all configured MCP servers
 */
export async function getMcpStatus(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { MCP } = await import("../../../mcp")
    const status = await MCP.status()

    return jsonResponse({
      success: true,
      data: status,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/mcp/tools
 * Get all available MCP tools from connected servers
 */
export async function getMcpTools(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { MCP } = await import("../../../mcp")
    const tools = await MCP.tools()

    // Convert tools to a serializable format
    const toolList = Object.entries(tools).map(([name, tool]) => ({
      name,
      description: (tool as any).description || "",
    }))

    return jsonResponse({
      success: true,
      data: toolList,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/mcp/resources
 * Get all available MCP resources from connected servers
 */
export async function getMcpResources(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { MCP } = await import("../../../mcp")
    const resources = await MCP.resources()

    return jsonResponse({
      success: true,
      data: resources,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/mcp/:name/connect
 * Connect (enable) an MCP server
 */
export async function connectMcp(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("MCP server name is required", 400)
    }

    const { MCP } = await import("../../../mcp")
    await MCP.connect(name)
    const status = await MCP.status()

    return jsonResponse({
      success: true,
      data: {
        name,
        status: status[name],
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/mcp/:name/disconnect
 * Disconnect (disable) an MCP server
 */
export async function disconnectMcp(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("MCP server name is required", 400)
    }

    const { MCP } = await import("../../../mcp")
    await MCP.disconnect(name)
    const status = await MCP.status()

    return jsonResponse({
      success: true,
      data: {
        name,
        status: status[name],
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/mcp/:name/toggle
 * Toggle an MCP server's enabled state
 */
export async function toggleMcp(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("MCP server name is required", 400)
    }

    const { MCP } = await import("../../../mcp")
    const currentStatus = await MCP.status()
    const serverStatus = currentStatus[name]

    if (!serverStatus) {
      return errorResponse(`MCP server "${name}" not found`, 404)
    }

    // Toggle based on current status
    if (serverStatus.status === "disabled") {
      await MCP.connect(name)
    } else {
      await MCP.disconnect(name)
    }

    const newStatus = await MCP.status()

    return jsonResponse({
      success: true,
      data: {
        name,
        status: newStatus[name],
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/mcp/:name/auth-status
 * Get authentication status for an MCP server
 */
export async function getMcpAuthStatus(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("MCP server name is required", 400)
    }

    const { MCP } = await import("../../../mcp")

    const supportsOAuth = await MCP.supportsOAuth(name)
    const authStatus = await MCP.getAuthStatus(name)

    return jsonResponse({
      success: true,
      data: {
        name,
        supportsOAuth,
        authStatus,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/mcp/:name/auth/start
 * Start OAuth authentication flow for an MCP server
 */
export async function startMcpAuth(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("MCP server name is required", 400)
    }

    const { MCP } = await import("../../../mcp")
    const result = await MCP.startAuth(name)

    return jsonResponse({
      success: true,
      data: result,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/mcp/:name/auth/finish
 * Complete OAuth authentication with authorization code
 */
export async function finishMcpAuth(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("MCP server name is required", 400)
    }

    const body = await readRequestBody(req.body)
    const { code } = JSON.parse(body) as { code: string }

    if (!code) {
      return errorResponse("Authorization code is required", 400)
    }

    const { MCP } = await import("../../../mcp")
    const status = await MCP.finishAuth(name, code)

    return jsonResponse({
      success: true,
      data: {
        name,
        status,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
