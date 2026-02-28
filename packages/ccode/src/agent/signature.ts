/**
 * Agent Signature Verification
 *
 * Provides Ed25519 signature verification for Agent definitions to ensure integrity.
 * Uses Node.js/Bun native crypto module for Ed25519 support.
 *
 * @package agent
 */

import { Log } from "@/util/log"
import { Global } from "@/global"
import crypto from "crypto"
import fs from "fs/promises"
import path from "path"
import z from "zod"

const log = Log.create({ service: "agent.signature" })

// ============================================================================
// Types
// ============================================================================

/**
 * Trust level for agent verification
 */
export type TrustLevel = "verified" | "unverified" | "untrusted" | "self_signed"

/**
 * Agent manifest with signature information
 */
export interface AgentManifest {
  /** Agent name */
  name: string

  /** Agent version */
  version: string

  /** SHA-256 hash of agent definition */
  hash: string

  /** Ed25519 signature (hex encoded) */
  signature: string

  /** Public key (hex encoded) */
  publicKey: string

  /** Timestamp when signed */
  signedAt: number

  /** List of capabilities this agent has */
  capabilities: string[]
}

/**
 * Zod schema for agent manifest
 */
export const AgentManifestSchema = z.object({
  name: z.string(),
  version: z.string(),
  hash: z.string(),
  signature: z.string(),
  publicKey: z.string(),
  signedAt: z.number(),
  capabilities: z.array(z.string()),
})

/**
 * Verification result
 */
export interface VerificationResult {
  /** Trust level */
  trust: TrustLevel

  /** Whether signature is valid */
  valid: boolean

  /** Verification message */
  message: string

  /** Agent manifest (if found) */
  manifest?: AgentManifest

  /** Verification timestamp */
  verifiedAt: number
}

/**
 * Agent definition for signing
 */
export interface AgentDefinition {
  name: string
  prompt?: string
  description?: string
  mode: string
  options: Record<string, unknown>
  permission: unknown[]
}

/**
 * Trusted keys store
 */
export interface TrustedKeysStore {
  /** Trusted public keys (hex encoded) */
  keys: string[]

  /** Last updated timestamp */
  updatedAt: number
}

// ============================================================================
// Constants
// ============================================================================

const TRUSTED_KEYS_FILE = "trusted-keys.json"
const MANIFESTS_DIR = "agent-manifests"

// ============================================================================
// Signature Verification
// ============================================================================

/**
 * Agent Signature Verifier
 *
 * Verifies Ed25519 signatures for agent definitions.
 */
export class AgentSignatureVerifier {
  private trustedKeys: Set<string> = new Set()
  private manifests: Map<string, AgentManifest> = new Map()
  private initialized = false

  /**
   * Initialize the verifier by loading trusted keys
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    await this.loadTrustedKeys()
    await this.loadManifests()
    this.initialized = true

    log.info("Agent signature verifier initialized", {
      trustedKeys: this.trustedKeys.size,
      manifests: this.manifests.size,
    })
  }

  /**
   * Verify an agent's signature
   *
   * @param agentDef Agent definition to verify
   * @returns Verification result
   */
  async verify(agentDef: AgentDefinition): Promise<VerificationResult> {
    await this.ensureInitialized()

    const manifest = this.manifests.get(agentDef.name)

    // No manifest found - agent is unverified
    if (!manifest) {
      return {
        trust: "unverified",
        valid: false,
        message: `No manifest found for agent "${agentDef.name}"`,
        verifiedAt: Date.now(),
      }
    }

    // Compute hash of current definition
    const currentHash = this.hashAgentDefinition(agentDef)

    // Check if hash matches
    if (currentHash !== manifest.hash) {
      return {
        trust: "untrusted",
        valid: false,
        message: `Agent definition has been modified since signing`,
        manifest,
        verifiedAt: Date.now(),
      }
    }

    // Verify signature
    const signatureValid = this.verifySignature(manifest)

    if (!signatureValid) {
      return {
        trust: "untrusted",
        valid: false,
        message: `Invalid signature for agent "${agentDef.name}"`,
        manifest,
        verifiedAt: Date.now(),
      }
    }

    // Check if public key is trusted
    if (this.trustedKeys.has(manifest.publicKey)) {
      return {
        trust: "verified",
        valid: true,
        message: `Agent "${agentDef.name}" is verified`,
        manifest,
        verifiedAt: Date.now(),
      }
    }

    // Valid signature but untrusted key
    return {
      trust: "self_signed",
      valid: true,
      message: `Agent "${agentDef.name}" has valid signature but key is not trusted`,
      manifest,
      verifiedAt: Date.now(),
    }
  }

  /**
   * Sign an agent definition (developer tool)
   *
   * @param agentDef Agent definition to sign
   * @param privateKey Ed25519 private key (hex encoded)
   * @returns Agent manifest with signature
   */
  sign(agentDef: AgentDefinition, privateKey: string): AgentManifest {
    const hash = this.hashAgentDefinition(agentDef)

    // Generate key pair from private key
    const privateKeyBuffer = Buffer.from(privateKey, "hex")
    const keyPair = crypto.createPrivateKey({
      key: privateKeyBuffer,
      format: "der",
      type: "pkcs8",
    })

    // Sign the hash
    const signature = crypto.sign(null, Buffer.from(hash), keyPair)

    // Extract public key
    const publicKeyObj = crypto.createPublicKey(keyPair)
    const publicKey = publicKeyObj.export({ type: "spki", format: "der" }).toString("hex")

    return {
      name: agentDef.name,
      version: "1.0.0",
      hash,
      signature: signature.toString("hex"),
      publicKey,
      signedAt: Date.now(),
      capabilities: this.extractCapabilities(agentDef),
    }
  }

  /**
   * Generate a new Ed25519 key pair
   *
   * @returns Key pair (public and private keys in hex)
   */
  generateKeyPair(): { publicKey: string; privateKey: string } {
    const { publicKey, privateKey } = crypto.generateKeyPairSync("ed25519")

    return {
      publicKey: publicKey.export({ type: "spki", format: "der" }).toString("hex"),
      privateKey: privateKey.export({ type: "pkcs8", format: "der" }).toString("hex"),
    }
  }

  /**
   * Get trusted public keys
   */
  getTrustedKeys(): string[] {
    return Array.from(this.trustedKeys)
  }

  /**
   * Add a trusted public key
   */
  async addTrustedKey(publicKey: string): Promise<void> {
    await this.ensureInitialized()
    this.trustedKeys.add(publicKey)
    await this.saveTrustedKeys()
  }

  /**
   * Remove a trusted public key
   */
  async removeTrustedKey(publicKey: string): Promise<void> {
    await this.ensureInitialized()
    this.trustedKeys.delete(publicKey)
    await this.saveTrustedKeys()
  }

  /**
   * Save an agent manifest
   */
  async saveManifest(manifest: AgentManifest): Promise<void> {
    await this.ensureInitialized()

    const manifestDir = path.join(Global.Path.data, MANIFESTS_DIR)
    await fs.mkdir(manifestDir, { recursive: true })

    const filePath = path.join(manifestDir, `${manifest.name}.manifest.json`)
    await fs.writeFile(filePath, JSON.stringify(manifest, null, 2), "utf-8")

    this.manifests.set(manifest.name, manifest)

    log.info("Agent manifest saved", { agent: manifest.name })
  }

  /**
   * Verify all agents and return results
   */
  async verifyAll(agents: AgentDefinition[]): Promise<Map<string, VerificationResult>> {
    await this.ensureInitialized()

    const results = new Map<string, VerificationResult>()

    for (const agent of agents) {
      const result = await this.verify(agent)
      results.set(agent.name, result)
    }

    return results
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async ensureInitialized(): Promise<void> {
    if (!this.initialized) {
      await this.initialize()
    }
  }

  /**
   * Hash an agent definition
   */
  private hashAgentDefinition(agentDef: AgentDefinition): string {
    // Create deterministic JSON representation
    const normalized = {
      name: agentDef.name,
      prompt: agentDef.prompt,
      description: agentDef.description,
      mode: agentDef.mode,
      options: this.sortObject(agentDef.options),
      permission: agentDef.permission,
    }

    const json = JSON.stringify(normalized, null, 0)
    return crypto.createHash("sha256").update(json).digest("hex")
  }

  /**
   * Sort object keys for deterministic hashing
   */
  private sortObject(obj: Record<string, unknown>): Record<string, unknown> {
    const sorted: Record<string, unknown> = {}
    for (const key of Object.keys(obj).sort()) {
      const value = obj[key]
      sorted[key] = typeof value === "object" && value !== null && !Array.isArray(value)
        ? this.sortObject(value as Record<string, unknown>)
        : value
    }
    return sorted
  }

  /**
   * Verify a signature in a manifest
   */
  private verifySignature(manifest: AgentManifest): boolean {
    try {
      const publicKeyBuffer = Buffer.from(manifest.publicKey, "hex")
      const publicKey = crypto.createPublicKey({
        key: publicKeyBuffer,
        format: "der",
        type: "spki",
      })

      const signature = Buffer.from(manifest.signature, "hex")
      const hash = Buffer.from(manifest.hash)

      return crypto.verify(null, hash, publicKey, signature)
    } catch (error) {
      log.warn("Signature verification failed", {
        agent: manifest.name,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Extract capabilities from agent definition
   */
  private extractCapabilities(agentDef: AgentDefinition): string[] {
    const capabilities: string[] = []

    if (agentDef.mode === "primary") capabilities.push("primary_mode")
    if (agentDef.mode === "subagent") capabilities.push("subagent_mode")

    // Check permissions for capabilities
    const permissionStr = JSON.stringify(agentDef.permission)
    if (permissionStr.includes('"bash"')) capabilities.push("bash_execution")
    if (permissionStr.includes('"edit"')) capabilities.push("file_editing")
    if (permissionStr.includes('"websearch"')) capabilities.push("web_search")

    return capabilities
  }

  /**
   * Load trusted keys from file
   */
  private async loadTrustedKeys(): Promise<void> {
    const filePath = path.join(Global.Path.data, TRUSTED_KEYS_FILE)

    try {
      const content = await fs.readFile(filePath, "utf-8")
      const store = JSON.parse(content) as TrustedKeysStore
      this.trustedKeys = new Set(store.keys)
    } catch (error) {
      // File doesn't exist or is invalid - start with empty set
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
        log.warn("Failed to load trusted keys", {
          error: error instanceof Error ? error.message : String(error),
        })
      }
      this.trustedKeys = new Set()
    }
  }

  /**
   * Save trusted keys to file
   */
  private async saveTrustedKeys(): Promise<void> {
    const filePath = path.join(Global.Path.data, TRUSTED_KEYS_FILE)

    await fs.mkdir(path.dirname(filePath), { recursive: true })

    const store: TrustedKeysStore = {
      keys: Array.from(this.trustedKeys),
      updatedAt: Date.now(),
    }

    await fs.writeFile(filePath, JSON.stringify(store, null, 2), "utf-8")
  }

  /**
   * Load agent manifests from directory
   */
  private async loadManifests(): Promise<void> {
    const manifestDir = path.join(Global.Path.data, MANIFESTS_DIR)

    try {
      await fs.mkdir(manifestDir, { recursive: true })
      const files = await fs.readdir(manifestDir)

      for (const file of files) {
        if (!file.endsWith(".manifest.json")) continue

        try {
          const filePath = path.join(manifestDir, file)
          const content = await fs.readFile(filePath, "utf-8")
          const manifest = AgentManifestSchema.parse(JSON.parse(content))
          this.manifests.set(manifest.name, manifest)
        } catch (error) {
          log.warn("Failed to load manifest", {
            file,
            error: error instanceof Error ? error.message : String(error),
          })
        }
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
        log.warn("Failed to load manifests directory", {
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }
  }
}

// ============================================================================
// Singleton and Convenience Functions
// ============================================================================

let verifierInstance: AgentSignatureVerifier | null = null

/**
 * Get the global verifier instance
 */
export function getVerifier(): AgentSignatureVerifier {
  if (!verifierInstance) {
    verifierInstance = new AgentSignatureVerifier()
  }
  return verifierInstance
}

/**
 * Create a new verifier instance
 */
export function createVerifier(): AgentSignatureVerifier {
  return new AgentSignatureVerifier()
}

/**
 * Verify an agent (convenience function)
 */
export async function verifyAgent(agentDef: AgentDefinition): Promise<VerificationResult> {
  const verifier = getVerifier()
  await verifier.initialize()
  return verifier.verify(agentDef)
}

/**
 * Generate a new key pair (convenience function)
 */
export function generateKeyPair(): { publicKey: string; privateKey: string } {
  return getVerifier().generateKeyPair()
}
