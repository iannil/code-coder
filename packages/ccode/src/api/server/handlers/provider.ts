/**
 * Provider API Handler
 * Handles /api/providers endpoints for model provider management
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/providers
 * List all available providers with connection status
 */
export async function listProviders(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { Provider } = await import("../../../provider/provider")
    const result = await Provider.listAll()

    return jsonResponse({
      success: true,
      data: result,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/providers/connected
 * List only connected providers
 */
export async function listConnectedProviders(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { Provider } = await import("../../../provider/provider")
    const providers = await Provider.list()

    return jsonResponse({
      success: true,
      data: Object.values(providers),
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/providers/auth
 * Get authentication methods for all providers
 */
export async function getProviderAuthMethods(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { Provider } = await import("../../../provider/provider")
    const authMethods = await Provider.authMethods()

    return jsonResponse({
      success: true,
      data: authMethods,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/providers/:providerId
 * Get details for a specific provider
 */
export async function getProvider(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { providerId } = params

    if (!providerId) {
      return errorResponse("Provider ID is required", 400)
    }

    const { Provider } = await import("../../../provider/provider")
    const providers = await Provider.list()
    const provider = providers[providerId]

    if (!provider) {
      return errorResponse(`Provider "${providerId}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: provider,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/providers/:providerId/models
 * Get models for a specific provider
 */
export async function getProviderModels(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { providerId } = params

    if (!providerId) {
      return errorResponse("Provider ID is required", 400)
    }

    const { Provider } = await import("../../../provider/provider")
    const providers = await Provider.list()
    const provider = providers[providerId]

    if (!provider) {
      return errorResponse(`Provider "${providerId}" not found`, 404)
    }

    const models = Object.values(provider.models)

    return jsonResponse({
      success: true,
      data: models,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
