/**
 * Credential Module
 *
 * Provides unified credential management for ZeroBot and CodeCoder.
 * Supports API keys, OAuth tokens, and login credentials with encrypted storage.
 */

export {
  CredentialVault,
  createApiKeyCredential,
  createOAuthCredential,
  createLoginCredential,
  createBearerTokenCredential,
  type CredentialEntry,
  type CredentialType,
  type CredentialSummary,
  type OAuthCredential,
  type LoginCredential,
} from "./vault"

export {
  CredentialResolver,
  createOAuth2RefreshHandler,
  OAUTH2_TOKEN_URLS,
  type ResolvedCredential,
  type OAuthRefreshResult,
  type OAuthRefreshHandler,
} from "./resolver"

export {
  SessionManager,
  extractDomain,
  hasValidCookiesForUrl,
  type StorageState,
  type Cookie,
  type Origin,
  type SessionInfo,
} from "./session"
