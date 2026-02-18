/**
 * Credential Resolver - Automatic credential matching and injection
 *
 * Provides logic for:
 * - Matching credentials to URLs/services
 * - Automatic OAuth token refresh
 * - HTTP header injection for API calls
 */

import type { CredentialEntry, OAuthCredential } from "./vault"
import { CredentialVault } from "./vault"
import { Global } from "@/global"
import { Log } from "@/util/log"

const log = Log.create({ service: "credential-resolver" })

// ============================================================================
// Types
// ============================================================================

export interface ResolvedCredential {
  entry: CredentialEntry
  headers: Record<string, string>
  needsRefresh: boolean
}

export interface OAuthRefreshResult {
  accessToken: string
  refreshToken?: string
  expiresAt?: number
}

export type OAuthRefreshHandler = (
  entry: CredentialEntry,
  oauth: OAuthCredential,
) => Promise<OAuthRefreshResult | null>

// ============================================================================
// Credential Resolver
// ============================================================================

export class CredentialResolver {
  private vault: CredentialVault
  private refreshHandlers: Map<string, OAuthRefreshHandler> = new Map()

  private constructor(vault: CredentialVault) {
    this.vault = vault
  }

  /**
   * Create a new credential resolver
   */
  static async create(codecoderDir: string = Global.Path.config): Promise<CredentialResolver> {
    const vault = await CredentialVault.load(codecoderDir)
    return new CredentialResolver(vault)
  }

  /**
   * Get the underlying vault
   */
  getVault(): CredentialVault {
    return this.vault
  }

  /**
   * Register an OAuth refresh handler for a specific service
   */
  registerRefreshHandler(service: string, handler: OAuthRefreshHandler): void {
    this.refreshHandlers.set(service, handler)
  }

  /**
   * Resolve credentials for a URL and return HTTP headers
   */
  async resolveForUrl(url: string): Promise<ResolvedCredential | null> {
    const entry = this.vault.resolveForUrl(url)
    if (!entry) {
      return null
    }

    return this.buildResolvedCredential(entry)
  }

  /**
   * Resolve credentials for a service by name
   */
  async resolveForService(service: string): Promise<ResolvedCredential | null> {
    const entry = this.vault.getByService(service)
    if (!entry) {
      return null
    }

    return this.buildResolvedCredential(entry)
  }

  /**
   * Build resolved credential with appropriate headers
   */
  private async buildResolvedCredential(entry: CredentialEntry): Promise<ResolvedCredential> {
    const headers: Record<string, string> = {}
    let needsRefresh = false

    switch (entry.type) {
      case "api_key":
        if (entry.apiKey) {
          // Use X-API-Key header for API keys
          headers["X-API-Key"] = entry.apiKey
        }
        break

      case "bearer_token":
        if (entry.apiKey) {
          headers["Authorization"] = `Bearer ${entry.apiKey}`
        }
        break

      case "oauth":
        if (entry.oauth?.accessToken) {
          // Check if token needs refresh
          if (entry.oauth.expiresAt && Date.now() >= entry.oauth.expiresAt - 60000) {
            needsRefresh = true
            log.debug("OAuth token expired or expiring soon", { service: entry.service })
          }
          headers["Authorization"] = `Bearer ${entry.oauth.accessToken}`
        }
        break

      case "login":
        // Login credentials don't produce headers directly
        // They're used for browser automation
        break
    }

    return { entry, headers, needsRefresh }
  }

  /**
   * Refresh OAuth tokens if needed
   */
  async refreshOAuth(id: string): Promise<boolean> {
    const entry = this.vault.get(id)
    if (!entry?.oauth) {
      log.warn("Cannot refresh: credential not found or not OAuth", { id })
      return false
    }

    const handler = this.refreshHandlers.get(entry.service)
    if (!handler) {
      log.warn("No refresh handler registered for service", { service: entry.service })
      return false
    }

    try {
      const result = await handler(entry, entry.oauth)
      if (!result) {
        log.warn("OAuth refresh handler returned null", { service: entry.service })
        return false
      }

      await this.vault.updateOAuthTokens(id, result.accessToken, result.refreshToken, result.expiresAt)

      log.info("OAuth tokens refreshed", { service: entry.service })
      return true
    } catch (error) {
      log.error("OAuth refresh failed", { service: entry.service, error })
      return false
    }
  }

  /**
   * Get headers for a URL, automatically refreshing OAuth if needed
   */
  async getHeadersForUrl(url: string): Promise<Record<string, string>> {
    const resolved = await this.resolveForUrl(url)
    if (!resolved) {
      return {}
    }

    if (resolved.needsRefresh && resolved.entry.type === "oauth") {
      const refreshed = await this.refreshOAuth(resolved.entry.id)
      if (refreshed) {
        // Re-resolve to get updated token
        const updated = await this.resolveForUrl(url)
        return updated?.headers ?? {}
      }
    }

    return resolved.headers
  }

  /**
   * Inject credentials into fetch options
   */
  async injectCredentials(url: string, init: RequestInit = {}): Promise<RequestInit> {
    const headers = await this.getHeadersForUrl(url)
    if (Object.keys(headers).length === 0) {
      return init
    }

    const existingHeaders = new Headers(init.headers)
    for (const [key, value] of Object.entries(headers)) {
      // Don't override existing headers
      if (!existingHeaders.has(key)) {
        existingHeaders.set(key, value)
      }
    }

    return {
      ...init,
      headers: existingHeaders,
    }
  }
}

// ============================================================================
// Standard OAuth Refresh Handlers
// ============================================================================

/**
 * Create an OAuth2 token refresh handler for standard OAuth2 providers
 */
export function createOAuth2RefreshHandler(tokenUrl: string): OAuthRefreshHandler {
  return async (entry, oauth) => {
    if (!oauth.refreshToken || !oauth.clientId) {
      return null
    }

    const params = new URLSearchParams({
      grant_type: "refresh_token",
      refresh_token: oauth.refreshToken,
      client_id: oauth.clientId,
    })

    if (oauth.clientSecret) {
      params.set("client_secret", oauth.clientSecret)
    }

    const response = await fetch(tokenUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: params.toString(),
    })

    if (!response.ok) {
      log.error("OAuth2 token refresh failed", {
        status: response.status,
        service: entry.service,
      })
      return null
    }

    const data = (await response.json()) as {
      access_token: string
      refresh_token?: string
      expires_in?: number
    }

    return {
      accessToken: data.access_token,
      refreshToken: data.refresh_token ?? oauth.refreshToken,
      expiresAt: data.expires_in ? Date.now() + data.expires_in * 1000 : undefined,
    }
  }
}

// ============================================================================
// Common OAuth2 Token URLs
// ============================================================================

export const OAUTH2_TOKEN_URLS: Record<string, string> = {
  google: "https://oauth2.googleapis.com/token",
  github: "https://github.com/login/oauth/access_token",
  microsoft: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
  slack: "https://slack.com/api/oauth.v2.access",
  discord: "https://discord.com/api/oauth2/token",
}
