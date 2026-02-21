import { Log } from "@/util/log"
import { Provider } from "@/provider/provider"
import { generateText } from "ai"
import { BootstrapTypes } from "./types"
import { ConfidenceSystem } from "./confidence"
import { CandidateStore } from "./candidate-store"

const log = Log.create({ service: "bootstrap.verification" })

/**
 * ExecutionLoop handles verification and self-correction of skill candidates.
 */
export namespace ExecutionLoop {
  const MAX_VERIFICATION_ATTEMPTS = 3
  const MAX_CORRECTION_ATTEMPTS = 2

  /**
   * Generate test scenarios for a candidate
   */
  export async function generateTestScenarios(
    candidate: BootstrapTypes.SkillCandidate,
  ): Promise<BootstrapTypes.TestScenario[]> {
    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const prompt = `Generate test scenarios for this skill candidate.

Skill Name: ${candidate.name}
Description: ${candidate.description}
Type: ${candidate.type}

Original Problem:
${candidate.source.problem.slice(0, 500)}

Original Solution:
${candidate.source.solution.slice(0, 500)}

Generate 3-5 test scenarios that would verify this skill works correctly.
Each scenario should be a different use case or edge case.

Format your response as a JSON array:
[
  {
    "name": "scenario name",
    "description": "what this tests",
    "input": "example input or trigger",
    "expectedBehavior": "what should happen"
  }
]

Return ONLY the JSON array, no other text.`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: 1000,
        temperature: 0.4,
      })

      // Parse JSON response
      const jsonMatch = result.text.match(/\[[\s\S]*\]/)
      if (!jsonMatch) {
        log.warn("failed to parse test scenarios", { response: result.text.slice(0, 200) })
        return generateDefaultScenarios(candidate)
      }

      const scenarios = JSON.parse(jsonMatch[0]) as Array<{
        name: string
        description: string
        input: string
        expectedBehavior: string
      }>

      return scenarios.map((s, i) => ({
        id: `scenario_${candidate.id}_${i}`,
        name: s.name,
        description: s.description,
        input: s.input,
        expectedBehavior: s.expectedBehavior,
      }))
    } catch (error) {
      log.warn("failed to generate test scenarios", { error })
      return generateDefaultScenarios(candidate)
    }
  }

  /**
   * Generate default test scenarios when LLM fails
   */
  function generateDefaultScenarios(
    candidate: BootstrapTypes.SkillCandidate,
  ): BootstrapTypes.TestScenario[] {
    return [
      {
        id: `scenario_${candidate.id}_0`,
        name: "Basic Usage",
        description: "Test basic functionality with similar input",
        input: candidate.source.problem.slice(0, 200),
        expectedBehavior: "Should produce similar solution",
      },
      {
        id: `scenario_${candidate.id}_1`,
        name: "Edge Case",
        description: "Test with minimal input",
        input: "minimal test",
        expectedBehavior: "Should handle gracefully",
      },
    ]
  }

  /**
   * Verify a candidate against its test scenarios
   */
  export async function verify(
    candidate: BootstrapTypes.SkillCandidate,
  ): Promise<BootstrapTypes.VerificationResult> {
    log.info("verifying candidate", { id: candidate.id, name: candidate.name })

    // Generate or use existing test scenarios
    const scenarios =
      candidate.verification.testScenarios && candidate.verification.testScenarios.length > 0
        ? await hydrateScenarios(candidate.verification.testScenarios, candidate)
        : await generateTestScenarios(candidate)

    const results: BootstrapTypes.TestScenario[] = []
    let passedCount = 0

    for (const scenario of scenarios) {
      const result = await verifyScenario(candidate, scenario)
      results.push(result)
      if (result.result?.passed) {
        passedCount++
      }
    }

    const passRate = results.length > 0 ? passedCount / results.length : 0
    const passed = passRate >= 0.6 // 60% pass rate required

    // Calculate new confidence
    const confidence = ConfidenceSystem.calculate({
      verificationPassed: passed,
      usageCount: candidate.metadata.usageCount,
      successRate: passRate,
      scenarioCoverage: results.length / 5, // Assume 5 is ideal
    })

    const verificationResult: BootstrapTypes.VerificationResult = {
      passed,
      confidence,
      scenarios: results,
    }

    // Update candidate
    await CandidateStore.update(candidate.id, (c) => {
      c.verification.status = passed ? "passed" : "failed"
      c.verification.attempts++
      c.verification.confidence = confidence
      c.verification.lastResult = JSON.stringify({
        passed,
        passRate,
        timestamp: Date.now(),
      })
      c.verification.testScenarios = results.map((s) => s.id)
    })

    log.info("verification complete", {
      id: candidate.id,
      passed,
      passRate,
      confidence,
    })

    return verificationResult
  }

  /**
   * Verify a single scenario
   */
  async function verifyScenario(
    candidate: BootstrapTypes.SkillCandidate,
    scenario: BootstrapTypes.TestScenario,
  ): Promise<BootstrapTypes.TestScenario> {
    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const prompt = `Evaluate if this skill would handle the given scenario correctly.

Skill: ${candidate.name}
Description: ${candidate.description}
Type: ${candidate.type}

Skill Content:
${JSON.stringify(candidate.content, null, 2).slice(0, 1000)}

Test Scenario:
Name: ${scenario.name}
Description: ${scenario.description}
Input: ${scenario.input}
Expected: ${scenario.expectedBehavior}

Would this skill correctly handle this scenario?
Respond with JSON:
{
  "passed": true/false,
  "actual": "what would actually happen",
  "reasoning": "brief explanation"
}

Return ONLY the JSON, no other text.`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: 500,
        temperature: 0.2,
      })

      const jsonMatch = result.text.match(/\{[\s\S]*\}/)
      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[0])
        return {
          ...scenario,
          result: {
            passed: parsed.passed === true,
            actual: parsed.actual,
          },
        }
      }

      return {
        ...scenario,
        result: {
          passed: false,
          error: "Failed to parse verification response",
        },
      }
    } catch (error) {
      return {
        ...scenario,
        result: {
          passed: false,
          error: error instanceof Error ? error.message : String(error),
        },
      }
    }
  }

  /**
   * Hydrate scenario IDs back to full scenarios
   */
  async function hydrateScenarios(
    scenarioIds: string[],
    candidate: BootstrapTypes.SkillCandidate,
  ): Promise<BootstrapTypes.TestScenario[]> {
    // For now, regenerate scenarios - in future could cache them
    return generateTestScenarios(candidate)
  }

  /**
   * Attempt to self-correct a failed candidate
   */
  export async function selfCorrect(
    candidate: BootstrapTypes.SkillCandidate,
    failedScenarios: BootstrapTypes.TestScenario[],
  ): Promise<BootstrapTypes.SkillCandidate | null> {
    if (candidate.verification.attempts >= MAX_CORRECTION_ATTEMPTS) {
      log.info("max correction attempts reached", { id: candidate.id })
      return null
    }

    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const failureSummary = failedScenarios
        .filter((s) => s.result && !s.result.passed)
        .map(
          (s) => `- ${s.name}: Expected "${s.expectedBehavior}", got "${s.result?.actual || "error"}"`,
        )
        .join("\n")

      const prompt = `A skill candidate failed verification. Suggest corrections.

Skill: ${candidate.name}
Description: ${candidate.description}
Type: ${candidate.type}

Current Content:
${JSON.stringify(candidate.content, null, 2).slice(0, 1500)}

Failures:
${failureSummary}

Suggest a corrected version of the skill content that would pass these scenarios.
Return JSON with the corrected content:
{
  "correctedContent": { ... },
  "explanation": "what was changed and why"
}

Return ONLY the JSON, no other text.`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: 1500,
        temperature: 0.3,
      })

      const jsonMatch = result.text.match(/\{[\s\S]*\}/)
      if (!jsonMatch) {
        log.warn("failed to parse correction response", { response: result.text.slice(0, 200) })
        return null
      }

      const correction = JSON.parse(jsonMatch[0])
      if (!correction.correctedContent) {
        return null
      }

      // Update candidate with corrections
      const updated = await CandidateStore.update(candidate.id, (c) => {
        c.content = {
          ...c.content,
          ...correction.correctedContent,
        }
        c.verification.status = "pending"
        c.verification.lastResult = JSON.stringify({
          type: "correction",
          explanation: correction.explanation,
          timestamp: Date.now(),
        })
      })

      log.info("applied self-correction", {
        id: candidate.id,
        explanation: correction.explanation?.slice(0, 100),
      })

      return updated ?? null
    } catch (error) {
      log.warn("self-correction failed", { error })
      return null
    }
  }

  /**
   * Run full verification loop with self-correction
   */
  export async function runVerificationLoop(
    candidate: BootstrapTypes.SkillCandidate,
  ): Promise<BootstrapTypes.VerificationResult> {
    let currentCandidate = candidate
    let attempts = 0

    while (attempts < MAX_VERIFICATION_ATTEMPTS) {
      attempts++

      const result = await verify(currentCandidate)

      if (result.passed) {
        return result
      }

      // Try self-correction
      const failedScenarios = result.scenarios.filter((s) => s.result && !s.result.passed)
      const corrected = await selfCorrect(currentCandidate, failedScenarios)

      if (!corrected) {
        return result
      }

      currentCandidate = corrected
    }

    // Return last result
    return verify(currentCandidate)
  }
}
