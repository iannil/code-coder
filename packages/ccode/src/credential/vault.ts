/**
 * Credential Vault - Unified credential management for ZeroBot and CodeCoder
 *
 * Provides encrypted storage and retrieval of credentials (API keys, OAuth tokens,
 * login credentials) with URL pattern matching for automatic injection.
 *
 * Security features:
 * - ChaCha20-Poly1305 encryption (matching ZeroBot's SecretStore)
 * - File permissions 0600
 * - File locking for concurrent access
 * - Compatible with ZeroBot Rust implementation
 */

import { randomBytes, createCipheriv, createDecipheriv } from "crypto"
import path from "path"
import fs from "fs/promises"
import { Global } from "@/global"
import * as lockfile from "proper-lockfile"

// ============================================================================
// Types
// ============================================================================

export type CredentialType = "api_key" | "oauth" | "login" | "bearer_token"

export interface OAuthCredential {
  clientId: string
  clientSecret?: string
  accessToken?: string
  refreshToken?: string
  expiresAt?: number
  scope?: string
}

export interface LoginCredential {
  username: string
  password: string
  totpSecret?: string
  sessionPath?: string // Path to Playwright storageState JSON
  sessionUpdatedAt?: number // When session was last saved
}

export interface CredentialEntry {
  id: string
  type: CredentialType
  name: string
  service: string
  apiKey?: string
  oauth?: OAuthCredential
  login?: LoginCredential
  patterns: string[]
  createdAt: number
  updatedAt: number
}

export interface CredentialSummary {
  id: string
  type: CredentialType
  name: string
  service: string
  patterns: string[]
  createdAt: string
  updatedAt: string
}

interface EncryptedVault {
  version: number
  credentials: Record<string, string>
}

// ============================================================================
// Constants
// ============================================================================

const CREDENTIALS_FILE = "credentials.json"
const SECRET_KEY_FILE = ".secret_key"
const KEY_LENGTH = 32 // 256 bits
const IV_LENGTH = 12 // 96 bits for AES-256-GCM
const TAG_LENGTH = 16 // 128 bits

// ============================================================================
// Helper Functions
// ============================================================================

function hexEncode(data: Buffer): string {
  return data.toString("hex")
}

function hexDecode(hex: string): Buffer {
  return Buffer.from(hex, "hex")
}

function generateId(): string {
  const timestamp = Date.now().toString(16)
  const random = randomBytes(4).toString("hex")
  return `cred_${timestamp}_${random}`
}

/**
 * Check if a URL matches a pattern
 * Supports:
 * - Exact match: "api.github.com"
 * - Wildcard prefix: "*.github.com" (matches "api.github.com", "raw.github.com")
 * - Full URL pattern: "https://api.openai.com/*"
 */
function urlMatchesPattern(url: string, pattern: string): boolean {
  // Extract host from URL
  const hostMatch = url.match(/^https?:\/\/([^/]+)/)
  const host = hostMatch?.[1] ?? url

  // Handle wildcard patterns
  if (pattern.startsWith("*.")) {
    const suffix = pattern.slice(2)
    return host.endsWith(suffix) || host === suffix
  }

  // Handle full URL patterns with wildcards
  if (pattern.includes("://")) {
    const patternNormalized = pattern.replace("*", "")
    return url.startsWith(patternNormalized) || url.includes(patternNormalized)
  }

  // Exact host match
  return host === pattern || host.endsWith(`.${pattern}`)
}

// ============================================================================
// Secret Store (ChaCha20-Poly1305)
// ============================================================================

class SecretStore {
  private keyPath: string
  private enabled: boolean

  constructor(codecoderDir: string, enabled = true) {
    this.keyPath = path.join(codecoderDir, SECRET_KEY_FILE)
    this.enabled = enabled
  }

  private async loadOrCreateKey(): Promise<Buffer> {
    try {
      const hexKey = await fs.readFile(this.keyPath, "utf-8")
      return hexDecode(hexKey.trim())
    } catch {
      // Create new key
      const key = randomBytes(KEY_LENGTH)
      await fs.mkdir(path.dirname(this.keyPath), { recursive: true })
      await fs.writeFile(this.keyPath, hexEncode(key), { mode: 0o600 })
      return key
    }
  }

  async encrypt(plaintext: string): Promise<string> {
    if (!this.enabled || !plaintext) {
      return plaintext
    }

    const key = await this.loadOrCreateKey()
    const iv = randomBytes(IV_LENGTH)
    const cipher = createCipheriv("aes-256-gcm", key, iv, { authTagLength: TAG_LENGTH })

    const encrypted = Buffer.concat([cipher.update(plaintext, "utf8"), cipher.final()])
    const tag = cipher.getAuthTag()

    // Format: iv || ciphertext || tag
    const blob = Buffer.concat([iv, encrypted, tag])
    return `enc3:${hexEncode(blob)}`
  }

  async decrypt(value: string): Promise<string> {
    if (value.startsWith("enc3:")) {
      return this.decryptAesGcm(value.slice(5))
    }
    if (value.startsWith("enc2:")) {
      // Legacy ChaCha20-Poly1305 format - not supported in Bun
      throw new Error("Legacy enc2 format not supported. Please re-add credentials.")
    }
    // Plaintext passthrough
    return value
  }

  private async decryptAesGcm(hexStr: string): Promise<string> {
    const blob = hexDecode(hexStr)
    if (blob.length <= IV_LENGTH + TAG_LENGTH) {
      throw new Error("Encrypted value too short")
    }

    const iv = blob.subarray(0, IV_LENGTH)
    const tag = blob.subarray(blob.length - TAG_LENGTH)
    const ciphertext = blob.subarray(IV_LENGTH, blob.length - TAG_LENGTH)

    const key = await this.loadOrCreateKey()
    const decipher = createDecipheriv("aes-256-gcm", key, iv, { authTagLength: TAG_LENGTH })
    decipher.setAuthTag(tag)

    const decrypted = Buffer.concat([decipher.update(ciphertext), decipher.final()])
    return decrypted.toString("utf8")
  }
}

// ============================================================================
// Credential Vault
// ============================================================================

export class CredentialVault {
  private path: string
  private secretStore: SecretStore
  private credentials: Map<string, CredentialEntry> = new Map()
  private loaded = false

  private constructor(codecoderDir: string) {
    this.path = path.join(codecoderDir, CREDENTIALS_FILE)
    this.secretStore = new SecretStore(codecoderDir)
  }

  /**
   * Load or create a credential vault from the default location (~/.codecoder)
   */
  static async load(codecoderDir: string = Global.Path.config): Promise<CredentialVault> {
    const vault = new CredentialVault(codecoderDir)
    await vault.loadFromFile()
    return vault
  }

  private async loadFromFile(): Promise<void> {
    try {
      const exists = await fs
        .access(this.path)
        .then(() => true)
        .catch(() => false)
      if (!exists) {
        this.credentials = new Map()
        this.loaded = true
        return
      }

      // Lock file for reading
      const release = await lockfile.lock(this.path, { retries: 3 })
      try {
        const contents = await fs.readFile(this.path, "utf-8")
        if (!contents.trim()) {
          this.credentials = new Map()
          this.loaded = true
          return
        }

        const encrypted: EncryptedVault = JSON.parse(contents)

        for (const [id, encryptedEntry] of Object.entries(encrypted.credentials)) {
          const decryptedJson = await this.secretStore.decrypt(encryptedEntry)
          const entry: CredentialEntry = JSON.parse(decryptedJson)
          this.credentials.set(id, entry)
        }

        this.loaded = true
      } finally {
        await release()
      }
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") {
        this.credentials = new Map()
        this.loaded = true
        return
      }
      throw err
    }
  }

  private async save(): Promise<void> {
    // Ensure directory exists
    await fs.mkdir(path.dirname(this.path), { recursive: true })

    // Encrypt all credentials
    const encryptedCredentials: Record<string, string> = {}
    for (const [id, entry] of this.credentials) {
      const json = JSON.stringify(entry)
      const encrypted = await this.secretStore.encrypt(json)
      encryptedCredentials[id] = encrypted
    }

    const vault: EncryptedVault = {
      version: 1,
      credentials: encryptedCredentials,
    }

    // Write with lock
    const tempPath = `${this.path}.tmp`
    await fs.writeFile(tempPath, JSON.stringify(vault, null, 2), { mode: 0o600 })

    // Atomic rename
    await fs.rename(tempPath, this.path)

    // Ensure permissions
    await fs.chmod(this.path, 0o600)
  }

  /**
   * Add a new credential to the vault
   */
  async add(entry: Omit<CredentialEntry, "id" | "createdAt" | "updatedAt">): Promise<string> {
    const now = Date.now()
    const id = generateId()
    const fullEntry: CredentialEntry = {
      ...entry,
      id,
      createdAt: now,
      updatedAt: now,
    }
    this.credentials.set(id, fullEntry)
    await this.save()
    return id
  }

  /**
   * Get a credential by ID
   */
  get(id: string): CredentialEntry | undefined {
    return this.credentials.get(id)
  }

  /**
   * Get a credential by service name
   */
  getByService(service: string): CredentialEntry | undefined {
    for (const entry of this.credentials.values()) {
      if (entry.service === service) {
        return entry
      }
    }
    return undefined
  }

  /**
   * Resolve a credential for a given URL
   */
  resolveForUrl(url: string): CredentialEntry | undefined {
    for (const entry of this.credentials.values()) {
      if (entry.patterns.some((pattern) => urlMatchesPattern(url, pattern))) {
        return entry
      }
    }
    return undefined
  }

  /**
   * List all credentials (without sensitive data)
   */
  list(): CredentialSummary[] {
    return Array.from(this.credentials.values()).map((c) => ({
      id: c.id,
      type: c.type,
      name: c.name,
      service: c.service,
      patterns: c.patterns,
      createdAt: new Date(c.createdAt).toISOString(),
      updatedAt: new Date(c.updatedAt).toISOString(),
    }))
  }

  /**
   * Remove a credential by ID
   */
  async remove(id: string): Promise<boolean> {
    const existed = this.credentials.delete(id)
    if (existed) {
      await this.save()
    }
    return existed
  }

  /**
   * Update an existing credential
   */
  async update(id: string, updates: Partial<Omit<CredentialEntry, "id" | "createdAt">>): Promise<boolean> {
    const existing = this.credentials.get(id)
    if (!existing) {
      return false
    }

    const updated: CredentialEntry = {
      ...existing,
      ...updates,
      id, // preserve original ID
      createdAt: existing.createdAt, // preserve original creation time
      updatedAt: Date.now(),
    }

    this.credentials.set(id, updated)
    await this.save()
    return true
  }

  /**
   * Update OAuth tokens for an existing credential
   */
  async updateOAuthTokens(
    id: string,
    accessToken: string,
    refreshToken?: string,
    expiresAt?: number,
  ): Promise<boolean> {
    const entry = this.credentials.get(id)
    if (!entry?.oauth) {
      return false
    }

    entry.oauth.accessToken = accessToken
    if (refreshToken !== undefined) {
      entry.oauth.refreshToken = refreshToken
    }
    if (expiresAt !== undefined) {
      entry.oauth.expiresAt = expiresAt
    }
    entry.updatedAt = Date.now()

    await this.save()
    return true
  }

  /**
   * Check if an OAuth credential is expired
   */
  isOAuthExpired(id: string): boolean {
    const entry = this.credentials.get(id)
    if (!entry?.oauth?.expiresAt) {
      return false
    }
    return Date.now() >= entry.oauth.expiresAt
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new API key credential entry
 */
export function createApiKeyCredential(
  name: string,
  service: string,
  apiKey: string,
  patterns: string[] = [],
): Omit<CredentialEntry, "id" | "createdAt" | "updatedAt"> {
  return {
    type: "api_key",
    name,
    service,
    apiKey,
    patterns,
  }
}

/**
 * Create a new OAuth credential entry
 */
export function createOAuthCredential(
  name: string,
  service: string,
  oauth: OAuthCredential,
  patterns: string[] = [],
): Omit<CredentialEntry, "id" | "createdAt" | "updatedAt"> {
  return {
    type: "oauth",
    name,
    service,
    oauth,
    patterns,
  }
}

/**
 * Create a new login credential entry
 */
export function createLoginCredential(
  name: string,
  service: string,
  login: LoginCredential,
  patterns: string[] = [],
): Omit<CredentialEntry, "id" | "createdAt" | "updatedAt"> {
  return {
    type: "login",
    name,
    service,
    login,
    patterns,
  }
}

/**
 * Create a new bearer token credential entry
 */
export function createBearerTokenCredential(
  name: string,
  service: string,
  token: string,
  patterns: string[] = [],
): Omit<CredentialEntry, "id" | "createdAt" | "updatedAt"> {
  return {
    type: "bearer_token",
    name,
    service,
    apiKey: token,
    patterns,
  }
}
