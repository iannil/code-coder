/**
 * Prompt Injection Scanner
 *
 * Thin wrapper around @codecoder-ai/core native Rust implementation.
 * Provides multi-layer detection of prompt injection attacks including:
 * - Jailbreak attempts
 * - Role override attacks
 * - Instruction leakage attempts
 * - Delimiter attacks
 *
 * @package security
 */

import {
  InjectionScanner as CoreInjectionScanner,
  scanForInjection as coreScanForInjection,
  quickCheckInjection as coreQuickCheckInjection,
  sanitizeInjectionInput as coreSanitizeInjectionInput,
  type InjectionScanResult as CoreInjectionScanResult,
  type InjectionPattern as CoreInjectionPattern,
  type InjectionScannerConfig as CoreInjectionScannerConfig,
  type InjectionType as CoreInjectionType,
  type InjectionSeverity as CoreInjectionSeverity,
} from "@codecoder-ai/core"

// ============================================================================
// Types - Re-export with compatibility aliases
// ============================================================================

/**
 * Injection pattern types
 */
export type InjectionType = CoreInjectionType

/**
 * Severity levels for detected patterns
 */
export type InjectionSeverity = CoreInjectionSeverity

/**
 * Detected injection pattern
 * Note: Core uses `injectionType` and `matched`, we alias for compatibility
 */
export interface InjectionPattern {
  /** Type of injection */
  type: InjectionType
  /** Matched pattern text */
  match: string
  /** Position in input string */
  position: number
  /** Severity of the injection */
  severity: InjectionSeverity
  /** Description of what was detected */
  description: string
}

/**
 * Scan result
 */
export interface InjectionScanResult {
  /** Whether injection was detected */
  detected: boolean
  /** Confidence level (0-1) */
  confidence: number
  /** All detected patterns */
  patterns: InjectionPattern[]
  /** Sanitized version of input (if possible) */
  sanitized?: string
  /** Scan duration in ms */
  durationMs: number
}

/**
 * Scanner configuration
 */
export interface ScannerConfig {
  /** Enable strict mode (lower thresholds) */
  strict: boolean
  /** Maximum input length to scan (default: 100000) */
  maxInputLength: number
  /** Enable encoding bypass detection */
  checkEncodingBypass: boolean
  /** Patterns to ignore (for whitelisting) - Note: not supported in native impl */
  ignorePatterns: RegExp[]
}

// ============================================================================
// Conversion Utilities
// ============================================================================

function convertPattern(p: CoreInjectionPattern): InjectionPattern {
  return {
    type: p.injectionType as InjectionType,
    match: p.matched,
    position: p.position,
    severity: p.severity as InjectionSeverity,
    description: p.description,
  }
}

function convertResult(r: CoreInjectionScanResult): InjectionScanResult {
  return {
    detected: r.detected,
    confidence: r.confidence,
    patterns: r.patterns.map(convertPattern),
    sanitized: r.sanitized,
    durationMs: r.durationMs,
  }
}

function toCoreScannerConfig(config: Partial<ScannerConfig>): CoreInjectionScannerConfig {
  return {
    strict: config.strict,
    maxInputLength: config.maxInputLength,
    checkEncodingBypass: config.checkEncodingBypass,
  }
}

// ============================================================================
// Scanner Implementation - Wrapper around native
// ============================================================================

/**
 * Prompt Injection Scanner
 *
 * Scans text input for potential injection attacks.
 * Uses native Rust implementation from @codecoder-ai/core.
 */
export class PromptInjectionScanner {
  private scanner: CoreInjectionScanner
  private config: ScannerConfig

  constructor(config: Partial<ScannerConfig> = {}) {
    this.config = {
      strict: false,
      maxInputLength: 100000,
      checkEncodingBypass: true,
      ignorePatterns: [],
      ...config,
    }
    this.scanner = new CoreInjectionScanner(toCoreScannerConfig(this.config))
  }

  /**
   * Scan input for injection patterns
   */
  scan(input: string): InjectionScanResult {
    const result = convertResult(this.scanner.scan(input))

    // Apply ignorePatterns filter
    if (this.config.ignorePatterns.length > 0) {
      const filteredPatterns = result.patterns.filter((p) => !this.config.ignorePatterns.some((ignore) => ignore.test(p.match)))

      // Recalculate confidence based on filtered patterns
      const weights = { low: 0.1, medium: 0.3, high: 0.6, critical: 1.0 }
      const totalWeight = filteredPatterns.reduce((sum, p) => sum + weights[p.severity], 0)
      const confidence = Math.min(1, totalWeight / 2)
      const detected = this.config.strict ? filteredPatterns.length > 0 : confidence >= 0.3

      return {
        ...result,
        patterns: filteredPatterns,
        detected,
        confidence,
      }
    }

    return result
  }

  /**
   * Sanitize input by removing or escaping injection patterns
   */
  sanitize(input: string): string {
    return this.scanner.sanitize(input)
  }

  /**
   * Quick check if input might contain injection
   * (Faster than full scan, for pre-filtering)
   */
  quickCheck(input: string): boolean {
    return this.scanner.quickCheck(input)
  }

  /**
   * Update scanner configuration
   */
  updateConfig(config: Partial<ScannerConfig>): void {
    this.config = { ...this.config, ...config }
    this.scanner = new CoreInjectionScanner(toCoreScannerConfig(this.config))
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

let scannerInstance: PromptInjectionScanner | null = null

/**
 * Get the global scanner instance
 */
export function getScanner(): PromptInjectionScanner {
  if (!scannerInstance) {
    scannerInstance = new PromptInjectionScanner()
  }
  return scannerInstance
}

/**
 * Create a new scanner instance
 */
export function createScanner(config?: Partial<ScannerConfig>): PromptInjectionScanner {
  return new PromptInjectionScanner(config)
}

/**
 * Scan input for prompt injection (convenience function)
 */
export function scanForInjection(input: string): InjectionScanResult {
  return convertResult(coreScanForInjection(input))
}

/**
 * Quick check for injection (convenience function)
 */
export function quickCheckInjection(input: string): boolean {
  if (!coreQuickCheckInjection) throw new Error("Native bindings not available")
  return coreQuickCheckInjection(input)
}

/**
 * Sanitize input (convenience function)
 */
export function sanitizeInput(input: string): string {
  if (!coreSanitizeInjectionInput) throw new Error("Native bindings not available")
  return coreSanitizeInjectionInput(input)
}
