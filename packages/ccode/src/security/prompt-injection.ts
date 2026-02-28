/**
 * Prompt Injection Scanner
 *
 * Provides multi-layer detection of prompt injection attacks including:
 * - Jailbreak attempts
 * - Role override attacks
 * - Instruction leakage attempts
 * - Delimiter attacks
 *
 * @package security
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "security.prompt-injection" })

// ============================================================================
// Types
// ============================================================================

/**
 * Injection pattern types
 */
export type InjectionType =
  | "jailbreak"
  | "role_override"
  | "instruction_leak"
  | "delimiter_attack"
  | "encoding_bypass"
  | "context_manipulation"

/**
 * Severity levels for detected patterns
 */
export type InjectionSeverity = "low" | "medium" | "high" | "critical"

/**
 * Detected injection pattern
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

  /** Patterns to ignore (for whitelisting) */
  ignorePatterns: RegExp[]
}

// ============================================================================
// Pattern Definitions
// ============================================================================

interface PatternDef {
  pattern: RegExp
  type: InjectionType
  severity: InjectionSeverity
  description: string
}

/**
 * Common jailbreak patterns
 * Based on known attack vectors (DAN, STAN, developer mode, etc.)
 */
const JAILBREAK_PATTERNS: PatternDef[] = [
  {
    pattern: /\bDAN\s*(mode|prompt|jailbreak)?\b/i,
    type: "jailbreak",
    severity: "high",
    description: "DAN (Do Anything Now) jailbreak attempt",
  },
  {
    pattern: /\bSTAN\s*(mode|prompt)?\b/i,
    type: "jailbreak",
    severity: "high",
    description: "STAN jailbreak variant",
  },
  {
    pattern: /\bdeveloper\s+mode\s*(enabled|on|activated)?\b/i,
    type: "jailbreak",
    severity: "high",
    description: "Developer mode activation attempt",
  },
  {
    pattern: /\bjailbreak(ed|ing)?\s*(mode|prompt|enabled)?\b/i,
    type: "jailbreak",
    severity: "high",
    description: "Explicit jailbreak request",
  },
  {
    pattern: /\bunlock(ed)?\s+(mode|capabilities|restrictions)\b/i,
    type: "jailbreak",
    severity: "medium",
    description: "Unlock capabilities attempt",
  },
  {
    pattern: /\bbypass\s+(safety|security|restrictions|filters)\b/i,
    type: "jailbreak",
    severity: "high",
    description: "Safety bypass attempt",
  },
  {
    pattern: /\bno\s+(rules|restrictions|limits|boundaries)\b/i,
    type: "jailbreak",
    severity: "medium",
    description: "Rules removal attempt",
  },
  {
    pattern: /\bpretend\s+(you\s+)?(?:are|have|can)\s+(?:no|full|unlimited)/i,
    type: "jailbreak",
    severity: "high",
    description: "Pretend no limits prompt",
  },
  {
    pattern: /\bhypothetical(ly)?\s+.*\bno\s+(ethical|moral|safety)\b/i,
    type: "jailbreak",
    severity: "high",
    description: "Hypothetical bypass attempt",
  },
]

/**
 * Role override patterns
 * Attempts to override system instructions or change persona
 */
const ROLE_OVERRIDE_PATTERNS: PatternDef[] = [
  {
    pattern: /\bignore\s+(all\s+)?(previous|prior|above)\s+(instructions?|prompts?|rules?)\b/i,
    type: "role_override",
    severity: "critical",
    description: "Ignore previous instructions",
  },
  {
    pattern: /\bdisregard\s+(all\s+)?(previous|prior|system)\s+(instructions?|prompts?)\b/i,
    type: "role_override",
    severity: "critical",
    description: "Disregard instructions",
  },
  {
    pattern: /\bforget\s+(everything|all|what)\s+(you\s+)?(know|were\s+told|learned)\b/i,
    type: "role_override",
    severity: "high",
    description: "Memory reset attempt",
  },
  {
    pattern: /\byou\s+are\s+now\s+(?:a|an|the)\s+\w+/i,
    type: "role_override",
    severity: "medium",
    description: "Role reassignment attempt",
  },
  {
    pattern: /\bfrom\s+now\s+on\s+(you\s+)?(are|will|must|should)\b/i,
    type: "role_override",
    severity: "medium",
    description: "Behavioral override attempt",
  },
  {
    pattern: /\bnew\s+(instructions?|persona|identity|role)\s*:/i,
    type: "role_override",
    severity: "high",
    description: "New instructions injection",
  },
  {
    pattern: /\bact\s+as\s+(if\s+)?(you\s+)?(are|were|have)\s+(no|unlimited|full)/i,
    type: "role_override",
    severity: "high",
    description: "Act without limits prompt",
  },
  {
    pattern: /\b(system|assistant)\s*(prompt|message|instruction)\s*:/i,
    type: "role_override",
    severity: "high",
    description: "System prompt injection",
  },
]

/**
 * Instruction leakage patterns
 * Attempts to extract system prompts or hidden instructions
 */
const INSTRUCTION_LEAK_PATTERNS: PatternDef[] = [
  {
    pattern: /\b(print|show|reveal|display|output)\s+(your\s+)?(system\s+)?(prompt|instructions?)\b/i,
    type: "instruction_leak",
    severity: "high",
    description: "System prompt extraction",
  },
  {
    pattern: /\bwhat\s+(are\s+)?(your|the)\s+(system\s+)?(instructions?|prompts?|rules?)\b/i,
    type: "instruction_leak",
    severity: "medium",
    description: "Instructions query",
  },
  {
    pattern: /\brepeat\s+(your\s+)?(system\s+)?(prompt|instructions?)\b/i,
    type: "instruction_leak",
    severity: "high",
    description: "Prompt repetition request",
  },
  {
    pattern: /\b(tell|explain)\s+me\s+(your|the)\s+(initial|original|hidden)\s+(prompt|instructions?)\b/i,
    type: "instruction_leak",
    severity: "high",
    description: "Hidden prompt extraction",
  },
  {
    pattern: /\bdump\s+(your\s+)?(context|memory|state|prompt)\b/i,
    type: "instruction_leak",
    severity: "high",
    description: "Context dump request",
  },
  {
    pattern: /\bcopy\s+(the\s+)?(text|content)\s+(above|before|preceding)\b/i,
    type: "instruction_leak",
    severity: "medium",
    description: "Copy preceding text",
  },
]

/**
 * Delimiter attack patterns
 * Attempts to break out of context using special markers
 */
const DELIMITER_PATTERNS: PatternDef[] = [
  {
    pattern: /<\/?system>/gi,
    type: "delimiter_attack",
    severity: "high",
    description: "System tag injection",
  },
  {
    pattern: /<\/?human>/gi,
    type: "delimiter_attack",
    severity: "high",
    description: "Human tag injection",
  },
  {
    pattern: /<\/?assistant>/gi,
    type: "delimiter_attack",
    severity: "high",
    description: "Assistant tag injection",
  },
  {
    pattern: /\[INST\]|\[\/INST\]/gi,
    type: "delimiter_attack",
    severity: "high",
    description: "Instruction delimiter injection",
  },
  {
    pattern: /```\s*(system|instruction|prompt)\s*\n/gi,
    type: "delimiter_attack",
    severity: "medium",
    description: "Code block delimiter attack",
  },
  {
    pattern: /#{3,}\s*(END|STOP|IGNORE|SYSTEM)/gi,
    type: "delimiter_attack",
    severity: "medium",
    description: "Markdown delimiter attack",
  },
  {
    pattern: /---+\s*(END|NEW|SYSTEM)\s*(PROMPT|CONTEXT|INSTRUCTIONS?)?/gi,
    type: "delimiter_attack",
    severity: "medium",
    description: "Horizontal rule delimiter attack",
  },
  {
    pattern: /\[\[.*?(SYSTEM|ADMIN|OVERRIDE).*?\]\]/gi,
    type: "delimiter_attack",
    severity: "high",
    description: "Double bracket injection",
  },
]

/**
 * Encoding bypass patterns
 * Base64, hex, ROT13, and other encoding attempts
 */
const ENCODING_BYPASS_PATTERNS: PatternDef[] = [
  {
    pattern: /\bdecode\s+(this|the\s+following)\s*(base64|hex|rot13)/i,
    type: "encoding_bypass",
    severity: "medium",
    description: "Decode encoded payload",
  },
  {
    pattern: /aWdub3JlIHByZXZpb3Vz/i, // "ignore previous" in base64
    type: "encoding_bypass",
    severity: "high",
    description: "Base64 encoded instruction override",
  },
  {
    pattern: /\\x[0-9a-f]{2}(\\x[0-9a-f]{2}){3,}/gi,
    type: "encoding_bypass",
    severity: "medium",
    description: "Hex escape sequence",
  },
  {
    pattern: /\\u[0-9a-f]{4}(\\u[0-9a-f]{4}){3,}/gi,
    type: "encoding_bypass",
    severity: "medium",
    description: "Unicode escape sequence",
  },
]

/**
 * Context manipulation patterns
 * Attempts to manipulate conversation context
 */
const CONTEXT_MANIPULATION_PATTERNS: PatternDef[] = [
  {
    pattern: /\b(user|human)\s*:\s*\n/gi,
    type: "context_manipulation",
    severity: "high",
    description: "Fake user turn injection",
  },
  {
    pattern: /\b(assistant|claude|ai)\s*:\s*\n/gi,
    type: "context_manipulation",
    severity: "high",
    description: "Fake assistant turn injection",
  },
  {
    pattern: /\[(conversation|chat)\s+(history|log|context)\]/gi,
    type: "context_manipulation",
    severity: "medium",
    description: "Fake conversation history",
  },
  {
    pattern: /\bprevious\s+response\s*:\s*\n/gi,
    type: "context_manipulation",
    severity: "medium",
    description: "Fake previous response",
  },
]

/**
 * All patterns combined
 */
const ALL_PATTERNS: PatternDef[] = [
  ...JAILBREAK_PATTERNS,
  ...ROLE_OVERRIDE_PATTERNS,
  ...INSTRUCTION_LEAK_PATTERNS,
  ...DELIMITER_PATTERNS,
  ...ENCODING_BYPASS_PATTERNS,
  ...CONTEXT_MANIPULATION_PATTERNS,
]

// ============================================================================
// Scanner Implementation
// ============================================================================

/**
 * Default scanner configuration
 */
const DEFAULT_CONFIG: ScannerConfig = {
  strict: false,
  maxInputLength: 100000,
  checkEncodingBypass: true,
  ignorePatterns: [],
}

/**
 * Prompt Injection Scanner
 *
 * Scans text input for potential injection attacks.
 */
export class PromptInjectionScanner {
  private config: ScannerConfig

  constructor(config: Partial<ScannerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Scan input for injection patterns
   *
   * @param input Text to scan
   * @returns Scan result with detected patterns
   */
  scan(input: string): InjectionScanResult {
    const startTime = performance.now()
    const patterns: InjectionPattern[] = []

    // Truncate if too long
    const text = input.slice(0, this.config.maxInputLength)

    // Collect patterns to check
    const patternsToCheck = this.config.checkEncodingBypass
      ? ALL_PATTERNS
      : ALL_PATTERNS.filter((p) => p.type !== "encoding_bypass")

    // Scan for each pattern
    for (const def of patternsToCheck) {
      // Skip if matches ignore pattern
      if (this.config.ignorePatterns.some((ignore) => ignore.test(text))) {
        continue
      }

      // Find all matches
      const regex = new RegExp(def.pattern.source, def.pattern.flags + (def.pattern.global ? "" : "g"))
      let match: RegExpExecArray | null

      while ((match = regex.exec(text)) !== null) {
        patterns.push({
          type: def.type,
          match: match[0],
          position: match.index,
          severity: def.severity,
          description: def.description,
        })

        // Prevent infinite loop for zero-length matches
        if (match[0].length === 0) {
          regex.lastIndex++
        }
      }
    }

    // Calculate confidence based on severity and count
    const confidence = this.calculateConfidence(patterns)
    const detected = this.config.strict ? patterns.length > 0 : confidence >= 0.3

    const result: InjectionScanResult = {
      detected,
      confidence,
      patterns,
      durationMs: performance.now() - startTime,
    }

    // Generate sanitized version if injection detected
    if (detected) {
      result.sanitized = this.sanitize(text)
    }

    if (detected) {
      log.warn("Prompt injection detected", {
        confidence,
        patternCount: patterns.length,
        types: [...new Set(patterns.map((p) => p.type))],
      })
    }

    return result
  }

  /**
   * Calculate confidence score based on detected patterns
   */
  private calculateConfidence(patterns: InjectionPattern[]): number {
    if (patterns.length === 0) return 0

    // Weight by severity
    const severityWeights: Record<InjectionSeverity, number> = {
      low: 0.1,
      medium: 0.3,
      high: 0.6,
      critical: 1.0,
    }

    const totalWeight = patterns.reduce((sum, p) => sum + severityWeights[p.severity], 0)

    // Normalize to 0-1 range, with diminishing returns for many patterns
    return Math.min(1, totalWeight / 2)
  }

  /**
   * Sanitize input by removing or escaping injection patterns
   *
   * @param input Text to sanitize
   * @returns Sanitized text
   */
  sanitize(input: string): string {
    let text = input

    // Remove delimiter attacks
    text = text.replace(/<\/?(?:system|human|assistant)>/gi, "")
    text = text.replace(/\[INST\]|\[\/INST\]/gi, "")
    text = text.replace(/\[\[.*?(?:SYSTEM|ADMIN|OVERRIDE).*?\]\]/gi, "")

    // Escape potential role overrides
    text = text.replace(/\b(ignore|disregard|forget)\s+(all\s+)?(previous|prior)/gi, "[FILTERED]")

    // Remove fake turn markers
    text = text.replace(/\b(user|human|assistant|claude)\s*:\s*\n/gi, "")

    return text
  }

  /**
   * Quick check if input might contain injection
   * (Faster than full scan, for pre-filtering)
   */
  quickCheck(input: string): boolean {
    // Check for obvious markers
    const quickPatterns = [
      /ignore.*previous.*instruction/i,
      /disregard.*prior.*prompt/i,
      /<\/?system>/i,
      /\bDAN\b/,
      /jailbreak/i,
      /bypass.*safety/i,
    ]

    return quickPatterns.some((p) => p.test(input))
  }

  /**
   * Update scanner configuration
   */
  updateConfig(config: Partial<ScannerConfig>): void {
    this.config = { ...this.config, ...config }
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
  return getScanner().scan(input)
}

/**
 * Quick check for injection (convenience function)
 */
export function quickCheckInjection(input: string): boolean {
  return getScanner().quickCheck(input)
}

/**
 * Sanitize input (convenience function)
 */
export function sanitizeInput(input: string): string {
  return getScanner().sanitize(input)
}
