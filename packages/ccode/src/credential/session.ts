/**
 * Session Manager - Playwright StorageState management
 *
 * Manages browser session files (cookies, localStorage) for login state persistence.
 * Integrates with the Credential Vault for automatic session loading/saving.
 *
 * Storage location: ~/.codecoder/sessions/{service}.json
 */

import path from "path"
import fs from "fs/promises"
import { Global } from "@/global"
import { CredentialVault, type CredentialEntry } from "./vault"
import { Log } from "@/util/log"

const log = Log.create({ service: "session-manager" })

// ============================================================================
// Types
// ============================================================================

/**
 * Playwright StorageState format
 */
export interface StorageState {
  cookies: Cookie[]
  origins: Origin[]
}

export interface Cookie {
  name: string
  value: string
  domain: string
  path: string
  expires: number
  httpOnly: boolean
  secure: boolean
  sameSite: "Strict" | "Lax" | "None"
}

export interface Origin {
  origin: string
  localStorage: Array<{ name: string; value: string }>
}

export interface SessionInfo {
  credentialId: string
  service: string
  sessionPath: string
  exists: boolean
  updatedAt?: number
  expiresAt?: number
  cookieCount?: number
}

// ============================================================================
// Constants
// ============================================================================

const SESSIONS_DIR = "sessions"
const SESSION_EXPIRY_DAYS = 30

// ============================================================================
// Session Manager
// ============================================================================

export class SessionManager {
  private sessionsDir: string
  private vault: CredentialVault

  private constructor(vault: CredentialVault, codecoderDir: string) {
    this.vault = vault
    this.sessionsDir = path.join(codecoderDir, SESSIONS_DIR)
  }

  /**
   * Create a SessionManager instance
   */
  static async create(codecoderDir: string = Global.Path.config): Promise<SessionManager> {
    const vault = await CredentialVault.load(codecoderDir)
    const manager = new SessionManager(vault, codecoderDir)
    await manager.ensureSessionsDir()
    return manager
  }

  /**
   * Create with existing vault
   */
  static withVault(vault: CredentialVault, codecoderDir: string = Global.Path.config): SessionManager {
    return new SessionManager(vault, codecoderDir)
  }

  private async ensureSessionsDir(): Promise<void> {
    await fs.mkdir(this.sessionsDir, { recursive: true, mode: 0o700 })
  }

  /**
   * Get the session file path for a service
   */
  getSessionPath(service: string): string {
    const safeName = service.replace(/[^a-zA-Z0-9_-]/g, "_")
    return path.join(this.sessionsDir, `${safeName}.json`)
  }

  /**
   * Check if a valid session exists for a credential
   */
  async hasValidSession(credentialId: string): Promise<boolean> {
    const credential = this.vault.get(credentialId)
    if (!credential?.login?.sessionPath) {
      return false
    }

    try {
      const stat = await fs.stat(credential.login.sessionPath)
      const age = Date.now() - stat.mtimeMs
      const maxAge = SESSION_EXPIRY_DAYS * 24 * 60 * 60 * 1000

      if (age > maxAge) {
        log.debug("Session expired", { service: credential.service, ageDays: age / 86400000 })
        return false
      }

      // Verify file is valid JSON
      const content = await fs.readFile(credential.login.sessionPath, "utf-8")
      const state = JSON.parse(content) as StorageState
      return Array.isArray(state.cookies) && state.cookies.length > 0
    } catch {
      return false
    }
  }

  /**
   * Load session state for a credential
   */
  async loadSession(credentialId: string): Promise<StorageState | null> {
    const credential = this.vault.get(credentialId)
    if (!credential?.login?.sessionPath) {
      return null
    }

    try {
      const content = await fs.readFile(credential.login.sessionPath, "utf-8")
      const state = JSON.parse(content) as StorageState
      log.info("Session loaded", { service: credential.service, cookies: state.cookies.length })
      return state
    } catch (error) {
      log.warn("Failed to load session", { service: credential.service, error })
      return null
    }
  }

  /**
   * Save session state for a credential
   */
  async saveSession(credentialId: string, state: StorageState): Promise<string> {
    const credential = this.vault.get(credentialId)
    if (!credential) {
      throw new Error(`Credential not found: ${credentialId}`)
    }

    await this.ensureSessionsDir()
    const sessionPath = this.getSessionPath(credential.service)

    // Save session file
    await fs.writeFile(sessionPath, JSON.stringify(state, null, 2), { mode: 0o600 })

    // Update credential with session path
    if (credential.login) {
      await this.vault.update(credentialId, {
        login: {
          ...credential.login,
          sessionPath,
          sessionUpdatedAt: Date.now(),
        },
      })
    }

    log.info("Session saved", {
      service: credential.service,
      cookies: state.cookies.length,
      path: sessionPath,
    })

    return sessionPath
  }

  /**
   * Clear session for a credential
   */
  async clearSession(credentialId: string): Promise<boolean> {
    const credential = this.vault.get(credentialId)
    if (!credential?.login?.sessionPath) {
      return false
    }

    try {
      await fs.unlink(credential.login.sessionPath)

      // Update credential to remove session reference
      await this.vault.update(credentialId, {
        login: {
          ...credential.login,
          sessionPath: undefined,
          sessionUpdatedAt: undefined,
        },
      })

      log.info("Session cleared", { service: credential.service })
      return true
    } catch {
      return false
    }
  }

  /**
   * Get session info for all login credentials
   */
  async listSessions(): Promise<SessionInfo[]> {
    const credentials = this.vault.list()
    const sessions: SessionInfo[] = []

    for (const cred of credentials) {
      if (cred.type !== "login") continue

      const fullCred = this.vault.get(cred.id)
      if (!fullCred?.login) continue

      const sessionPath = fullCred.login.sessionPath || this.getSessionPath(cred.service)
      let exists = false
      let cookieCount = 0
      let expiresAt: number | undefined

      try {
        const content = await fs.readFile(sessionPath, "utf-8")
        const state = JSON.parse(content) as StorageState
        exists = true
        cookieCount = state.cookies.length

        // Find earliest cookie expiry
        const now = Date.now() / 1000
        const validExpiries = state.cookies
          .map((c) => c.expires)
          .filter((e) => e > now)

        if (validExpiries.length > 0) {
          expiresAt = Math.min(...validExpiries) * 1000
        }
      } catch {
        // Session doesn't exist or is invalid
      }

      sessions.push({
        credentialId: cred.id,
        service: cred.service,
        sessionPath,
        exists,
        updatedAt: fullCred.login.sessionUpdatedAt,
        expiresAt,
        cookieCount,
      })
    }

    return sessions
  }

  /**
   * Clean up expired sessions
   */
  async cleanupExpiredSessions(): Promise<number> {
    const sessions = await this.listSessions()
    let cleaned = 0

    for (const session of sessions) {
      if (!session.exists) continue

      const expired =
        session.expiresAt && session.expiresAt < Date.now() ||
        session.updatedAt && Date.now() - session.updatedAt > SESSION_EXPIRY_DAYS * 86400000

      if (expired) {
        await this.clearSession(session.credentialId)
        cleaned++
      }
    }

    if (cleaned > 0) {
      log.info("Cleaned up expired sessions", { count: cleaned })
    }

    return cleaned
  }

  /**
   * Get the underlying vault
   */
  getVault(): CredentialVault {
    return this.vault
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Extract domain from URL for cookie matching
 */
export function extractDomain(url: string): string {
  try {
    const u = new URL(url)
    return u.hostname
  } catch {
    return url
  }
}

/**
 * Check if cookies are valid for a URL
 */
export function hasValidCookiesForUrl(state: StorageState, url: string): boolean {
  const domain = extractDomain(url)
  const now = Date.now() / 1000

  return state.cookies.some((cookie) => {
    const cookieDomain = cookie.domain.startsWith(".") ? cookie.domain.slice(1) : cookie.domain
    const domainMatch = domain === cookieDomain || domain.endsWith(`.${cookieDomain}`)
    const notExpired = cookie.expires === -1 || cookie.expires > now
    return domainMatch && notExpired
  })
}
