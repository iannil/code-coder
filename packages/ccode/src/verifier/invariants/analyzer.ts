/**
 * Invariant Analyzer
 *
 * Analyzes code to identify and verify invariants.
 * Extracts potential invariants from code patterns and test coverage.
 */

import { Log } from "@/util/log"
import type { Invariant } from "../schema/functional-goal"
import type { InvariantResult, Evidence, VerificationStatus } from "../schema/verification-result"
import { InvariantPatterns, findApplicablePatterns, generateInvariants } from "./patterns"

const log = Log.create({ service: "verifier.invariants.analyzer" })

/**
 * Invariant analysis configuration
 */
export interface InvariantAnalysisConfig {
  includeDirs?: string[]
  excludeDirs?: string[]
  filePatterns?: string[]
}

/**
 * Detected invariant in code
 */
export interface DetectedInvariant {
  invariant: Invariant
  location: { file: string; line: number }
  confidence: "high" | "medium" | "low"
  source: "pattern" | "explicit_check" | "inferred"
}

/**
 * Invariant analyzer state
 */
export class InvariantAnalyzer {
  private sessionId: string
  private config: InvariantAnalysisConfig

  constructor(sessionId: string, config: InvariantAnalysisConfig = {}) {
    this.sessionId = sessionId
    this.config = {
      filePatterns: ["**/*.ts", "**/*.js", "**/*.tsx", "**/*.jsx"],
      excludeDirs: ["node_modules", "dist", "build", ".git"],
      ...config,
    }
  }

  /**
   * Analyze a module for invariants
   */
  async analyzeModule(modulePath: string): Promise<DetectedInvariant[]> {
    log.info("Analyzing module for invariants", {
      sessionId: this.sessionId,
      modulePath,
    })

    const invariants: DetectedInvariant[] = []

    try {
      // 1. Find explicit invariant checks (assertions, console.assert, etc.)
      const explicitInvariants = await this.findExplicitInvariants(modulePath)
      invariants.push(...explicitInvariants)

      // 2. Detect patterns that suggest invariants
      const patternInvariants = await this.detectPatternInvariants(modulePath)
      invariants.push(...patternInvariants)

      // 3. Infer from type definitions
      const inferredInvariants = await this.inferInvariantsFromTypes(modulePath)
      invariants.push(...inferredInvariants)

      log.info("Invariant analysis completed", {
        modulePath,
        count: invariants.length,
      })

      return invariants
    } catch (error) {
      log.error("Invariant analysis failed", {
        modulePath,
        error: error instanceof Error ? error.message : String(error),
      })

      return []
    }
  }

  /**
   * Find explicit invariant checks in code
   */
  private async findExplicitInvariants(
    modulePath: string,
  ): Promise<DetectedInvariant[]> {
    const invariants: DetectedInvariant[] = []

    // Patterns for explicit checks
    const patterns = [
      // Assertion patterns
      { regex: /assert\((.+)\)/g, type: "assertion" },
      { regex: /console\.assert\((.+)\)/g, type: "console_assert" },
      { regex: /invariant\((.+)\)/g, type: "explicit_invariant" },

      // Conditional throws that might be invariants
      { regex: /if\s*\(!?([^)]+)\)\s*{[^}]*throw/g, type: "guard_clause" },

      // Type guards
      { regex: /if\s*\(!?([^)]+\s*instanceof\s+[^)]+)\)/g, type: "type_guard" },

      // Validation functions
      { regex: /validate\(?([^)]+)\)?/g, type: "validation" },
      { regex: /check\(?([^)]+)\)?/g, type: "check" },
    ]

    for (const pattern of patterns) {
      const matches = await this.findPatternMatches(modulePath, pattern.regex)

      for (const match of matches) {
        invariants.push({
          invariant: {
            id: `INV-EXPLICIT-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
            name: `explicit_${pattern.type}`,
            statement: `Explicit check: ${match.content}`,
            formal: match.content,
            scope: "function",
            violation: "throws Error or fails assertion",
          },
          location: { file: match.file, line: match.line },
          confidence: "high",
          source: "explicit_check",
        })
      }
    }

    return invariants
  }

  /**
   * Detect invariants from code patterns
   */
  private async detectPatternInvariants(
    modulePath: string,
  ): Promise<DetectedInvariant[]> {
    const invariants: DetectedInvariant[] = []

    // Find function names
    const functionNames = await this.extractFunctionNames(modulePath)

    // For each function, check if invariant patterns apply
    for (const funcName of functionNames) {
      const applicablePatterns = findApplicablePatterns(funcName)

      for (const pattern of applicablePatterns) {
        // Check if the pattern is likely applicable by looking for related code
        const hasRelatedCode = await this.hasRelatedCode(modulePath, funcName, pattern)

        if (hasRelatedCode) {
          invariants.push({
            invariant: pattern.generate({ functionName: funcName }),
            location: { file: modulePath, line: 0 },
            confidence: "medium",
            source: "pattern",
          })
        }
      }
    }

    return invariants
  }

  /**
   * Infer invariants from type definitions
   */
  private async inferInvariantsFromTypes(
    modulePath: string,
  ): Promise<DetectedInvariant[]> {
    const invariants: DetectedInvariant[] = []

    // Look for type annotations that suggest invariants
    const typePatterns = [
      {
        regex: /:\s*ReadonlyArray<(.+)>/g,
        invariant: {
          name: "immutable_array",
          statement: "Array is never modified",
          formal: "forall arr: is_readonly(arr)",
        },
      },
      {
        regex: /:\s*NonNullable<(.+)>/g,
        invariant: {
          name: "non_nullable",
          statement: "Value is never null or undefined",
          formal: "x !== null && x !== undefined",
        },
      },
      {
        regex: /:\s*Positive\s*=/g,
        invariant: {
          name: "positive_number",
          statement: "Number is always positive",
          formal: "x > 0",
        },
      },
      {
        regex: /:\s*NonNegative\s*=/g,
        invariant: {
          name: "non_negative",
          statement: "Number is never negative",
          formal: "x >= 0",
        },
      },
    ]

    for (const pattern of typePatterns) {
      const matches = await this.findPatternMatches(modulePath, pattern.regex)

      for (const match of matches) {
        invariants.push({
          invariant: {
            id: `INF-TYPE-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
            name: pattern.invariant.name,
            statement: pattern.invariant.statement,
            formal: pattern.invariant.formal,
            scope: "function",
            violation: "Type system violation",
          },
          location: { file: match.file, line: match.line },
          confidence: "medium",
          source: "inferred",
        })
      }
    }

    return invariants
  }

  /**
   * Find pattern matches in files
   */
  private async findPatternMatches(
    modulePath: string,
    regex: RegExp,
  ): Promise<Array<{ file: string; line: number; content: string }>> {
    const matches: Array<{ file: string; line: number; content: string }> = []

    try {
      const fs = require("fs")
      const path = require("path")

      if (fs.statSync(modulePath).isDirectory()) {
        // Search all TypeScript/JavaScript files in directory
        const files = this.getAllFiles(modulePath, this.config.filePatterns ?? [])

        for (const file of files) {
          const content = fs.readFileSync(file, "utf-8")
          const lines = content.split("\n")

          let match
          const re = new RegExp(regex.source, regex.flags)
          while ((match = re.exec(content)) !== null) {
            const lineNum = content.substring(0, match.index).split("\n").length
            matches.push({
              file,
              line: lineNum,
              content: match[1] ?? match[0],
            })
          }
        }
      } else {
        // Single file
        const content = fs.readFileSync(modulePath, "utf-8")

        let match
        const re = new RegExp(regex.source, regex.flags)
        while ((match = re.exec(content)) !== null) {
          const lineNum = content.substring(0, match.index).split("\n").length
          matches.push({
            file: modulePath,
            line: lineNum,
            content: match[1] ?? match[0],
          })
        }
      }
    } catch (error) {
      log.error("Failed to find pattern matches", {
        modulePath,
        error: error instanceof Error ? error.message : String(error),
      })
    }

    return matches
  }

  /**
   * Extract function names from a module
   */
  private async extractFunctionNames(modulePath: string): Promise<string[]> {
    const names: Set<string> = new Set()

    try {
      const fs = require("fs")
      const content = fs.readFileSync(modulePath, "utf-8")

      // Match function declarations: function name(), const name =, export const name =, etc.
      const patterns = [
        /function\s+(\w+)/g,
        /const\s+(\w+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>/g,
        /export\s+(?:const|function)\s+(\w+)/g,
        /(\w+)\s*\([^)]*\)\s*{/g, // Method declarations
      ]

      for (const pattern of patterns) {
        let match
        while ((match = pattern.exec(content)) !== null) {
          // Skip common non-function names
          if (!["if", "while", "for", "switch", "catch"].includes(match[1])) {
            names.add(match[1])
          }
        }
      }
    } catch (error) {
      log.error("Failed to extract function names", {
        modulePath,
        error: error instanceof Error ? error.message : String(error),
      })
    }

    return Array.from(names)
  }

  /**
   * Check if module has related code for a pattern
   */
  private async hasRelatedCode(
    modulePath: string,
    funcName: string,
    pattern: { applicableTo: string[] },
  ): Promise<boolean> {
    try {
      const fs = require("fs")
      const content = fs.readFileSync(modulePath, "utf-8").toLowerCase()

      // Check if any applicable keywords appear
      return pattern.applicableTo.some((keyword) => content.includes(keyword))
    } catch {
      return false
    }
  }

  /**
   * Get all files matching patterns in a directory
   */
  private getAllFiles(dir: string, patterns: string[]): string[] {
    const fs = require("fs")
    const path = require("path")
    const files: string[] = []
    const excludeDirs = this.config.excludeDirs ?? []

    function scanDir(currentDir: string, patterns: string[]) {
      try {
        const entries = fs.readdirSync(currentDir, { withFileTypes: true })

        for (const entry of entries) {
          const fullPath = path.join(currentDir, entry.name)

          if (entry.isDirectory()) {
            // Skip excluded directories
            if (excludeDirs.includes(entry.name)) continue
            scanDir(fullPath, patterns)
          } else if (entry.isFile()) {
            // Check if file matches any pattern
            if (patterns.some((p) => fullPath.endsWith(p.slice(2)))) {
              files.push(fullPath)
            }
          }
        }
      } catch {
        // Skip directories we can't read
      }
    }

    scanDir(dir, this.config.filePatterns ?? [])
    return files
  }

  /**
   * Verify an invariant against test results
   */
  verifyInvariant(
    invariant: Invariant,
    testResults: Array<{ passed: boolean; output?: string }>,
  ): InvariantResult {
    const evidence: Evidence[] = []
    const violations: string[] = []

    // Check if any tests violated the invariant
    for (const result of testResults) {
      if (!result.passed && result.output) {
        violations.push(result.output)
        evidence.push({
          type: "test",
          source: "test",
          excerpt: result.output,
        })
      }
    }

    const status: VerificationStatus = violations.length === 0 ? "pass" : "fail"

    return {
      id: invariant.id,
      name: invariant.name,
      status,
      scope: invariant.scope,
      violations,
      evidence,
    }
  }

  /**
   * Get statistics on detected invariants
   */
  getStats(detected: DetectedInvariant[]): {
    total: number
    byConfidence: Record<string, number>
    bySource: Record<string, number>
    byScope: Record<string, number>
  } {
    const stats = {
      total: detected.length,
      byConfidence: {} as Record<string, number>,
      bySource: {} as Record<string, number>,
      byScope: {} as Record<string, number>,
    }

    for (const d of detected) {
      stats.byConfidence[d.confidence] = (stats.byConfidence[d.confidence] ?? 0) + 1
      stats.bySource[d.source] = (stats.bySource[d.source] ?? 0) + 1
      stats.byScope[d.invariant.scope] = (stats.byScope[d.invariant.scope] ?? 0) + 1
    }

    return stats
  }
}

/**
 * Create an invariant analyzer
 */
export function createInvariantAnalyzer(
  sessionId: string,
  config?: InvariantAnalysisConfig,
): InvariantAnalyzer {
  return new InvariantAnalyzer(sessionId, config)
}
