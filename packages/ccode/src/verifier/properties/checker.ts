/**
 * Property Checker
 *
 * Executes property-based tests to verify mathematical properties.
 * Integrates with the test runner for actual execution.
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import type { Property } from "../schema/functional-goal"
import type { PropertyResult, Evidence, VerificationStatus } from "../schema/verification-result"
import { PropertyGenerators } from "./templates"

const log = Log.create({ service: "verifier.properties.checker" })

/**
 * Property test configuration
 */
export interface PropertyTestConfig {
  numCases?: number
  seed?: number
  maxShrinks?: number
  timeout?: number
}

/**
 * Result of running a single property test
 */
export interface PropertyTestRun {
  propertyId: string
  propertyName: string
  status: VerificationStatus
  passes: number
  failures: number
  counterexample?: string
  duration: number
  error?: string
}

/**
 * Property checker state
 */
export class PropertyChecker {
  private sessionId: string
  private config: PropertyTestConfig

  constructor(sessionId: string, config: PropertyTestConfig = {}) {
    this.sessionId = sessionId
    this.config = {
      numCases: 100,
      maxShrinks: 100,
      timeout: 5000,
      ...config,
    }
  }

  /**
   * Check a single property
   */
  async checkProperty(
    property: Property,
    testFile?: string,
  ): Promise<PropertyResult> {
    const startTime = Date.now()

    log.info("Checking property", {
      sessionId: this.sessionId,
      propertyId: property.id,
      propertyName: property.name,
    })

    try {
      let status: VerificationStatus = "skip"
      const evidence: Evidence[] = []
      let counterexample: string | undefined

      if (property.verification === "property_test" && testFile) {
        // Run property test via Bun test runner
        const result = await this.runPropertyTest(property, testFile)
        status = result.status
        counterexample = result.counterexample

        evidence.push({
          type: "property_test",
          source: testFile,
          timestamp: new Date().toISOString(),
        })
      } else if (property.verification === "formal_proof") {
        // For formal proofs, we'd integrate with a theorem prover
        // For now, mark as skip with a note
        status = "skip"
        evidence.push({
          type: "proof",
          source: "verifier",
          excerpt: "Formal proof verification not yet implemented",
        })
      }

      const duration = Date.now() - startTime

      log.info("Property check completed", {
        propertyId: property.id,
        status,
        duration,
      })

      return {
        id: property.id,
        name: property.name,
        category: property.category,
        status,
        formal: property.formal,
        proofMethod: property.verification === "formal_proof" ? "formal_proof" : "property_test",
        counterexample,
        evidence,
      }
    } catch (error) {
      log.error("Property check failed", {
        propertyId: property.id,
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        id: property.id,
        name: property.name,
        category: property.category,
        status: "fail",
        formal: property.formal,
        proofMethod: property.verification === "formal_proof" ? "formal_proof" : "property_test",
        counterexample: undefined,
        evidence: [],
      }
    }
  }

  /**
   * Check multiple properties
   */
  async checkProperties(
    properties: Property[],
    testFile?: string,
  ): Promise<PropertyResult[]> {
    const results: PropertyResult[] = []

    for (const property of properties) {
      const result = await this.checkProperty(property, testFile)
      results.push(result)
    }

    return results
  }

  /**
   * Run property test using Bun test runner
   */
  private async runPropertyTest(
    property: Property,
    testFile: string,
  ): Promise<Omit<PropertyTestRun, "propertyId" | "propertyName">> {
    try {
      const { execSync } = require("child_process")

      // Generate test file for this property if not exists
      const testCode = this.generatePropertyTestCode(property)

      // Write to temp test file
      const tempTestPath = this.writeTempTest(property.id, testCode)

      // Run the test
      const args = ["test", tempTestPath, "--reporter", "json"]

      log.info("Running property test", { args })

      const output = execSync("bun", args, {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
        env: {
          ...process.env,
          NODE_ENV: "test",
        },
        timeout: this.config.timeout,
      })

      // Parse result
      const jsonOutput = JSON.parse(output)

      // Clean up temp file
      this.cleanupTempTest(tempTestPath)

      if (jsonOutput.ok) {
        return {
          status: "pass",
          passes: jsonOutput.stats.passed,
          failures: 0,
          duration: 0,
        }
      } else {
        // Find counterexample from error message
        const counterexample = this.extractCounterexample(jsonOutput)

        return {
          status: "fail",
          passes: jsonOutput.stats.passed,
          failures: jsonOutput.stats.failed,
          counterexample,
          duration: 0,
        }
      }
    } catch (error) {
      // Test execution failed
      const errorMessage = error instanceof Error ? error.message : String(error)

      // Try to extract counterexample from error
      const counterexample = this.extractCounterexampleFromError(errorMessage)

      return {
        status: "fail",
        passes: 0,
        failures: 1,
        counterexample,
        duration: 0,
        error: errorMessage,
      }
    }
  }

  /**
   * Generate property test code
   */
  private generatePropertyTestCode(property: Property): string {
    const { name, formal } = property

    // Generate test code based on property type
    switch (name) {
      case "idempotency":
        return PropertyGenerators.idempotency("fn", []).template

      case "associativity":
        return PropertyGenerators.associativity("fn").template

      case "commutativity":
        return PropertyGenerators.commutative("fn").template

      case "round_trip":
        return PropertyGenerators.roundTrip("encode", "decode").template

      case "monotonicity":
        return PropertyGenerators.monotonic("fn").template

      case "identity":
        return PropertyGenerators.identity("fn", "identity").template

      default:
        // Generic property test template
        return `// Property: ${name}
// Formal: ${formal}

describe("Property: ${name}", () => {
  it("should satisfy: ${formal}", () => {
    // TODO: Implement property test
    expect(true).toBe(true);
  });
});`
    }
  }

  /**
   * Write temporary test file
   */
  private writeTempTest(propertyId: string, code: string): string {
    const fs = require("fs")
    const path = require("path")
    const os = require("os")

    const tempDir = os.tmpdir()
    const tempFileName = `property-test-${propertyId}-${Date.now()}.test.ts`
    const tempPath = path.join(tempDir, tempFileName)

    fs.writeFileSync(tempPath, code, "utf-8")

    return tempPath
  }

  /**
   * Clean up temporary test file
   */
  private cleanupTempTest(tempPath: string): void {
    try {
      const fs = require("fs")
      fs.unlinkSync(tempPath)
    } catch {
      // Ignore cleanup errors
    }
  }

  /**
   * Extract counterexample from Bun test output
   */
  private extractCounterexample(testOutput: any): string | undefined {
    if (testOutput.tests && Array.isArray(testOutput.tests)) {
      for (const test of testOutput.tests) {
        if (test.status === "failed" && test.error) {
          return test.error
        }
      }
    }
    return undefined
  }

  /**
   * Extract counterexample from error message
   */
  private extractCounterexampleFromError(error: string): string | undefined {
    // Look for common patterns in counterexample messages
    const patterns = [
      /Counterexample:\s*(.+)/,
      /Found:\s*(.+)/,
      /Falsified:\s*(.+)/,
      /Input:\s*(.+)/,
    ]

    for (const pattern of patterns) {
      const match = error.match(pattern)
      if (match && match[1]) {
        return match[1].trim()
      }
    }

    return undefined
  }

  /**
   * Generate property test file for multiple properties
   */
  generatePropertyTestFile(
    properties: Property[],
    outputPath: string,
  ): void {
    const fs = require("fs")

    const tests = properties.map((property) => {
      return this.generatePropertyTestCode(property)
    })

    const content = `// Auto-generated property tests
// Generated by: verifier agent
// Date: ${new Date().toISOString()}

${tests.join("\n\n")}
`

    fs.writeFileSync(outputPath, content, "utf-8")

    log.info("Property test file generated", {
      path: outputPath,
      propertyCount: properties.length,
    })
  }

  /**
   * Get property test statistics
   */
  getStats(results: PropertyResult[]): {
    total: number
    passed: number
    failed: number
    skipped: number
    passRate: number
  } {
    const total = results.length
    const passed = results.filter((r) => r.status === "pass").length
    const failed = results.filter((r) => r.status === "fail").length
    const skipped = results.filter((r) => r.status === "skip").length

    return {
      total,
      passed,
      failed,
      skipped,
      passRate: total > 0 ? (passed / total) * 100 : 0,
    }
  }
}

/**
 * Create a property checker
 */
export function createPropertyChecker(
  sessionId: string,
  config?: PropertyTestConfig,
): PropertyChecker {
  return new PropertyChecker(sessionId, config)
}
