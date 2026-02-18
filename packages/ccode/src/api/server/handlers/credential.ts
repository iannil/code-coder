/**
 * Credential API Handler
 * Handles /api/credentials/* endpoints for credential vault management
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { CredentialVault, type CredentialEntry } from "@/credential/vault"
import { SessionManager, type StorageState } from "@/credential/session"

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
 * GET /api/credentials
 * List all credentials (without sensitive data)
 */
export async function listCredentials(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const vault = await CredentialVault.load()
    const credentials = vault.list()

    return jsonResponse({
      success: true,
      data: credentials,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/credentials/:id
 * Get a specific credential by ID (includes sensitive data for authorized access)
 */
export async function getCredential(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Credential ID is required", 400)
    }

    const vault = await CredentialVault.load()
    const credential = vault.get(id)

    if (!credential) {
      return errorResponse("Credential not found", 404)
    }

    return jsonResponse({
      success: true,
      data: credential,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/credentials
 * Add a new credential
 */
export async function addCredential(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Omit<CredentialEntry, "id" | "createdAt" | "updatedAt">

    // Validate required fields
    if (!input.type) {
      return errorResponse("Credential type is required", 400)
    }
    if (!input.name) {
      return errorResponse("Credential name is required", 400)
    }
    if (!input.service) {
      return errorResponse("Service name is required", 400)
    }

    // Validate type-specific fields
    if (input.type === "api_key" && !input.apiKey) {
      return errorResponse("API key is required for api_key type", 400)
    }
    if (input.type === "bearer_token" && !input.apiKey) {
      return errorResponse("Token is required for bearer_token type", 400)
    }
    if (input.type === "oauth" && !input.oauth) {
      return errorResponse("OAuth credentials are required for oauth type", 400)
    }
    if (input.type === "login" && !input.login) {
      return errorResponse("Login credentials are required for login type", 400)
    }

    const vault = await CredentialVault.load()
    const id = await vault.add({
      type: input.type,
      name: input.name,
      service: input.service,
      apiKey: input.apiKey,
      oauth: input.oauth,
      login: input.login,
      patterns: input.patterns || [],
    })

    return jsonResponse(
      {
        success: true,
        data: { id },
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/credentials/:id
 * Update an existing credential
 */
export async function updateCredential(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Credential ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<Omit<CredentialEntry, "id" | "createdAt">>

    const vault = await CredentialVault.load()
    const updated = await vault.update(id, input)

    if (!updated) {
      return errorResponse("Credential not found", 404)
    }

    return jsonResponse({
      success: true,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/credentials/:id
 * Delete a credential
 */
export async function deleteCredential(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Credential ID is required", 400)
    }

    const vault = await CredentialVault.load()
    const removed = await vault.remove(id)

    if (!removed) {
      return errorResponse("Credential not found", 404)
    }

    return jsonResponse({
      success: true,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/credentials/resolve
 * Resolve a credential for a given URL or service
 */
export async function resolveCredential(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url.searchParams.get("url")
    const service = req.url.searchParams.get("service")

    if (!url && !service) {
      return errorResponse("Either URL or service parameter is required", 400)
    }

    const vault = await CredentialVault.load()
    let credential: CredentialEntry | undefined

    if (url) {
      credential = vault.resolveForUrl(url)
    } else if (service) {
      credential = vault.getByService(service)
    }

    if (!credential) {
      return jsonResponse({
        success: true,
        data: null,
      })
    }

    return jsonResponse({
      success: true,
      data: credential,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Session Handlers
// ============================================================================

/**
 * GET /api/credentials/sessions
 * List all sessions for login credentials
 */
export async function listSessions(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const manager = await SessionManager.create()
    const sessions = await manager.listSessions()

    return jsonResponse({
      success: true,
      data: sessions,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/credentials/:id/session
 * Get session state for a credential
 */
export async function getSession(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Credential ID is required", 400)
    }

    const manager = await SessionManager.create()
    const session = await manager.loadSession(id)

    if (!session) {
      return jsonResponse({
        success: true,
        data: null,
      })
    }

    return jsonResponse({
      success: true,
      data: session,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/credentials/:id/session
 * Save session state for a credential
 */
export async function saveSession(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Credential ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const state = JSON.parse(body) as StorageState

    if (!state.cookies || !Array.isArray(state.cookies)) {
      return errorResponse("Invalid session state: cookies array required", 400)
    }

    const manager = await SessionManager.create()
    const sessionPath = await manager.saveSession(id, state)

    return jsonResponse({
      success: true,
      data: { sessionPath },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/credentials/:id/session
 * Clear session for a credential
 */
export async function clearSession(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Credential ID is required", 400)
    }

    const manager = await SessionManager.create()
    const cleared = await manager.clearSession(id)

    return jsonResponse({
      success: true,
      data: { cleared },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/credentials/sessions/cleanup
 * Clean up expired sessions
 */
export async function cleanupSessions(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const manager = await SessionManager.create()
    const cleaned = await manager.cleanupExpiredSessions()

    return jsonResponse({
      success: true,
      data: { cleaned },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
