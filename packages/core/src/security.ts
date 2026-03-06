/**
 * Security module - PermissionManager and Vault wrappers
 *
 * These classes provide a TypeScript interface to the Rust-native security
 * features. Native bindings are REQUIRED - no JavaScript fallback.
 *
 * @example
 * ```typescript
 * import { PermissionManager, Vault } from '@codecoder-ai/core'
 *
 * // Permission management
 * const permissions = new PermissionManager()
 * permissions.addRule({ permission: { tool: 'file', action: 'read' }, allow: true })
 * permissions.check({ tool: 'file', action: 'read' }) // true
 *
 * // Encrypted vault storage
 * const vault = new Vault('/path/to/vault.enc', 'password')
 * vault.set({ name: 'api_key', value: 'sk-xxx' })
 * vault.save()
 * ```
 */

import type { Permission, PermissionRule, SecretEntry, IPermissionManager, IVault } from './types.js'

// Import native bindings directly - fail-fast if not available
import {
  createPermissionManager,
  openVault,
  createMemoryVault as nativeCreateMemoryVault,
  createInjectionScanner,
  createInjectionScannerWithConfig,
  scanInjection as nativeScanInjection,
  quickCheckInjection as nativeQuickCheckInjection,
  sanitizeInjectionInput as nativeSanitizeInjectionInput,
  type PermissionManagerHandle,
  type VaultHandle,
  type InjectionScannerHandle,
  type NapiPermission,
  type NapiPermissionRule,
  type NapiSecretEntry,
  type InjectionScanResult as NativeInjectionScanResult,
} from './binding.js'

// Conversion utilities
function toNativePermission(p: Permission): NapiPermission {
  return {
    tool: p.tool,
    action: p.action,
    resource: p.resource,
  }
}

function toNativePermissionRule(r: PermissionRule): NapiPermissionRule {
  return {
    permission: toNativePermission(r.permission),
    allow: r.allow,
    reason: r.reason,
  }
}

function toNativeSecretEntry(e: SecretEntry): NapiSecretEntry {
  return {
    name: e.name,
    value: e.value,
    description: e.description,
  }
}

function fromNativeSecretEntry(e: NapiSecretEntry): SecretEntry {
  return {
    name: e.name,
    value: e.value,
    description: e.description,
  }
}

/**
 * Permission manager for access control
 *
 * Manages permission rules and grants for tools and resources.
 * Uses native Rust implementation.
 */
export class PermissionManager implements IPermissionManager {
  private handle: PermissionManagerHandle

  constructor() {
    this.handle = createPermissionManager()
  }

  /** Add a permission rule */
  addRule(rule: PermissionRule): void {
    this.handle.addRule(toNativePermissionRule(rule))
  }

  /** Grant a specific permission */
  grant(permission: Permission): void {
    this.handle.grant(toNativePermission(permission))
  }

  /** Check if a permission is allowed */
  check(permission: Permission): boolean {
    return this.handle.check(toNativePermission(permission))
  }

  /** Clear all rules and grants */
  clear(): void {
    this.handle.clear()
  }
}

/**
 * Encrypted vault for storing secrets
 *
 * Stores secrets with ChaCha20-Poly1305 encryption.
 * Uses native Rust implementation.
 */
export class Vault implements IVault {
  private _handle: VaultHandle
  private vaultPath: string

  /**
   * Open or create a vault
   * @param path - Path to the vault file
   * @param password - Master password for encryption
   */
  constructor(path: string, password: string) {
    this.vaultPath = path
    this._handle = openVault(path, password)
  }

  /** Store a secret */
  set(entry: SecretEntry): void {
    this._handle.set(toNativeSecretEntry(entry))
  }

  /** Get a secret by name */
  get(name: string): SecretEntry | null {
    const result = this._handle.get(name)
    return result ? fromNativeSecretEntry(result) : null
  }

  /** Get just the secret value */
  getValue(name: string): string | null {
    return this._handle.getValue(name)
  }

  /** Delete a secret */
  delete(name: string): boolean {
    return this._handle.delete(name)
  }

  /** List all secret names */
  list(): string[] {
    return this._handle.list()
  }

  /** Save the vault to disk */
  save(): void {
    this._handle.save()
  }

  /** Get the vault path */
  get path(): string {
    return this.vaultPath
  }
}

/**
 * Create an in-memory vault (for testing)
 */
export function createMemoryVault(password: string): Vault {
  const vault = Object.create(Vault.prototype) as Vault
  ;(vault as unknown as { _handle: VaultHandle })._handle = nativeCreateMemoryVault(password)
  ;(vault as unknown as { vaultPath: string }).vaultPath = ':memory:'
  return vault
}

// Native bindings are always available
export const isSecurityNative = true

// ============================================================================
// Prompt Injection Scanner
// ============================================================================

/**
 * Injection types
 */
export type InjectionType =
  | 'jailbreak'
  | 'role_override'
  | 'instruction_leak'
  | 'delimiter_attack'
  | 'encoding_bypass'
  | 'context_manipulation'

/**
 * Injection severity levels
 */
export type InjectionSeverity = 'low' | 'medium' | 'high' | 'critical'

/**
 * Detected injection pattern
 */
export interface InjectionPattern {
  /** Type of injection */
  injectionType: string
  /** Matched text */
  matched: string
  /** Position in input string */
  position: number
  /** Severity level */
  severity: string
  /** Description of the pattern */
  description: string
}

/**
 * Injection scan result
 */
export interface InjectionScanResult {
  /** Whether injection was detected */
  detected: boolean
  /** Confidence level (0.0-1.0) */
  confidence: number
  /** Detected patterns */
  patterns: InjectionPattern[]
  /** Sanitized input (if injection detected) */
  sanitized?: string
  /** Scan duration in milliseconds */
  durationMs: number
}

/**
 * Scanner configuration
 */
export interface InjectionScannerConfig {
  /** Enable strict mode (lower detection threshold) */
  strict?: boolean
  /** Maximum input length to scan */
  maxInputLength?: number
  /** Check for encoding bypass attempts */
  checkEncodingBypass?: boolean
}

// JavaScript regex patterns for defense-in-depth (backup layer)
// These serve as a secondary check in case native bindings miss something

// Helper to ensure 'g' flag is present without duplication
function toGlobalRegex(pattern: RegExp): RegExp {
  if (pattern.global) return pattern
  return new RegExp(pattern.source, pattern.flags + 'g')
}

const JAILBREAK_PATTERNS = [
  { pattern: /\bDAN\s*(mode|prompt|jailbreak)?\b/i, severity: 'high' as const, description: 'DAN jailbreak attempt' },
  { pattern: /\bdeveloper\s+mode\s*(enabled|on|activated)?\b/i, severity: 'high' as const, description: 'Developer mode activation' },
  { pattern: /\bjailbreak(ed|ing)?\s*(mode|prompt|enabled)?\b/i, severity: 'high' as const, description: 'Jailbreak attempt' },
  { pattern: /\bbypass\s+(safety|security|restrictions|filters)\b/i, severity: 'high' as const, description: 'Bypass safety attempt' },
  { pattern: /\bpretend\s+you\s+have\s+no\s+rules\b/i, severity: 'high' as const, description: 'No rules pretend' },
  { pattern: /\bno\s+rules?\s+(mode|enabled)\b/i, severity: 'high' as const, description: 'No rules mode' },
  { pattern: /\bact\s+as\s+if\s+you\s+have\s+no\s+(limits?|restrictions?|rules?|boundaries)\b/i, severity: 'medium' as const, description: 'Act as if no limits' },
]

const ROLE_OVERRIDE_PATTERNS = [
  { pattern: /\bignore\s+(all\s+)?(previous|prior|above)\s+(instructions?|prompts?|rules?)\b/i, severity: 'critical' as const, description: 'Ignore previous instructions' },
  { pattern: /\bdisregard\s+(all\s+)?(previous|prior|system)\s+(instructions?|prompts?)\b/i, severity: 'critical' as const, description: 'Disregard prior prompts' },
  { pattern: /\bforget\s+(everything|all|what)\s+(you\s+)?(know|were\s+told|learned)\b/i, severity: 'high' as const, description: 'Forget knowledge' },
  { pattern: /\bnew\s+instructions?:\s/i, severity: 'high' as const, description: 'New instructions injection' },
]

const DELIMITER_PATTERNS = [
  { pattern: /<\/?system>/gi, severity: 'high' as const, description: 'System tag injection' },
  { pattern: /<\/?human>/gi, severity: 'high' as const, description: 'Human tag injection' },
  { pattern: /<\/?assistant>/gi, severity: 'high' as const, description: 'Assistant tag injection' },
  { pattern: /\[INST\]|\[\/INST\]/gi, severity: 'high' as const, description: 'INST delimiter injection' },
  { pattern: /\[\[\s*(SYSTEM|ADMIN|OVERRIDE|ROOT)\s*[^\]]*\]\]/gi, severity: 'high' as const, description: 'Double bracket injection' },
]

const INSTRUCTION_LEAK_PATTERNS = [
  { pattern: /\b(print|show|reveal|display|output)\s+(your\s+)?(system\s+)?(prompt|instructions?)\b/i, severity: 'high' as const, description: 'Print system prompt' },
  { pattern: /\brepeat\s+(your\s+)?(system\s+)?(prompt|instructions?)\b/i, severity: 'high' as const, description: 'Repeat instructions' },
  { pattern: /\bdump\s+(your\s+)?(context|memory|conversation)\b/i, severity: 'high' as const, description: 'Dump context' },
]

const CONTEXT_MANIPULATION_PATTERNS = [
  { pattern: /\b(user|human)\s*:\s*\n/i, severity: 'high' as const, description: 'Fake user turn' },
  { pattern: /\b(assistant|claude|ai)\s*:\s*\n/i, severity: 'high' as const, description: 'Fake assistant turn' },
]

/**
 * Prompt Injection Scanner
 *
 * Scans text input for potential prompt injection attacks.
 * Uses native Rust implementation with JavaScript regex backup layer.
 */
export class InjectionScanner {
  private handle: InjectionScannerHandle
  private config: InjectionScannerConfig

  constructor(config: InjectionScannerConfig = {}) {
    this.config = { strict: false, maxInputLength: 100000, checkEncodingBypass: true, ...config }
    this.handle = config
      ? createInjectionScannerWithConfig(this.config)
      : createInjectionScanner()
  }

  /** Scan input for injection patterns */
  scan(input: string): InjectionScanResult {
    return this.handle.scan(input)
  }

  /** Quick check for injection (faster than full scan) */
  quickCheck(input: string): boolean {
    return this.handle.quickCheck(input)
  }

  /** Sanitize input by removing injection patterns */
  sanitize(input: string): string {
    return this.handle.sanitize(input)
  }

  /**
   * Backup JS-only scan for defense-in-depth
   * Used internally for additional validation
   */
  jsScan(input: string): InjectionScanResult {
    const start = performance.now()
    const patterns: InjectionPattern[] = []
    const text = input.slice(0, this.config.maxInputLength ?? 100000)

    // Scan jailbreak patterns
    for (const { pattern, severity, description } of JAILBREAK_PATTERNS) {
      for (const match of text.matchAll(toGlobalRegex(pattern))) {
        patterns.push({
          injectionType: 'jailbreak',
          matched: match[0],
          position: match.index!,
          severity,
          description,
        })
      }
    }

    // Scan role override patterns
    for (const { pattern, severity, description } of ROLE_OVERRIDE_PATTERNS) {
      for (const match of text.matchAll(toGlobalRegex(pattern))) {
        patterns.push({
          injectionType: 'role_override',
          matched: match[0],
          position: match.index!,
          severity,
          description,
        })
      }
    }

    // Scan delimiter patterns
    for (const { pattern, severity, description } of DELIMITER_PATTERNS) {
      for (const match of text.matchAll(toGlobalRegex(pattern))) {
        patterns.push({
          injectionType: 'delimiter_attack',
          matched: match[0],
          position: match.index!,
          severity,
          description,
        })
      }
    }

    // Scan instruction leak patterns
    for (const { pattern, severity, description } of INSTRUCTION_LEAK_PATTERNS) {
      for (const match of text.matchAll(toGlobalRegex(pattern))) {
        patterns.push({
          injectionType: 'instruction_leak',
          matched: match[0],
          position: match.index!,
          severity,
          description,
        })
      }
    }

    // Scan context manipulation patterns
    for (const { pattern, severity, description } of CONTEXT_MANIPULATION_PATTERNS) {
      for (const match of text.matchAll(toGlobalRegex(pattern))) {
        patterns.push({
          injectionType: 'context_manipulation',
          matched: match[0],
          position: match.index!,
          severity,
          description,
        })
      }
    }

    // Calculate confidence
    const weights: Record<string, number> = { low: 0.1, medium: 0.3, high: 0.6, critical: 1.0 }
    const totalWeight = patterns.reduce((sum, p) => sum + (weights[p.severity] ?? 0), 0)
    const confidence = Math.min(1, totalWeight / 2)

    const detected = this.config.strict ? patterns.length > 0 : confidence >= 0.3

    return {
      detected,
      confidence,
      patterns,
      sanitized: detected ? this.jsSanitize(text) : undefined,
      durationMs: performance.now() - start,
    }
  }

  /** Backup JS-only sanitize */
  private jsSanitize(input: string): string {
    let text = input
    text = text.replace(/<\/?(?:system|human|assistant)>/gi, '')
    text = text.replace(/\[INST\]|\[\/INST\]/gi, '')
    text = text.replace(/\[\[.*?(?:SYSTEM|ADMIN|OVERRIDE).*?\]\]/gi, '')
    text = text.replace(/\b(ignore|disregard|forget)\s+(all\s+)?(previous|prior)/gi, '[FILTERED]')
    text = text.replace(/\b(user|human|assistant|claude)\s*:\s*\n/gi, '')
    return text
  }
}

/** Convenience function: Scan for injection */
export function scanForInjection(input: string): InjectionScanResult {
  return nativeScanInjection(input)
}

/** Convenience function: Quick check for injection */
export function quickCheckInjection(input: string): boolean {
  return nativeQuickCheckInjection(input)
}

/** Convenience function: Sanitize input */
export function sanitizeInjectionInput(input: string): string {
  return nativeSanitizeInjectionInput(input)
}
