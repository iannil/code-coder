/**
 * Mock skill candidates for evaluation tests
 *
 * These fixtures provide controlled data for testing the Bootstrap Flywheel
 * without relying on LLM-generated content.
 */

import type { BootstrapTypes } from "@/bootstrap/types"

/**
 * Generate a unique candidate ID for testing
 */
export function generateTestId(prefix = "test"): string {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`
}

/**
 * Create a mock skill candidate with sensible defaults
 */
export function createMockCandidate(
  overrides: Partial<BootstrapTypes.SkillCandidate> = {},
): BootstrapTypes.SkillCandidate {
  const now = Date.now()
  const id = overrides.id ?? generateTestId("cand")

  return {
    id,
    type: "workflow",
    name: "mock-skill",
    description: "A mock skill for testing",
    trigger: {
      type: "auto",
      context: "Testing context",
    },
    content: {
      steps: ["Step 1: Analyze input", "Step 2: Process data", "Step 3: Return result"],
    },
    source: {
      sessionId: generateTestId("sess"),
      toolCalls: ["tc_1", "tc_2", "tc_3"],
      problem: "How to process data efficiently",
      solution: "Use a three-step workflow for optimal processing",
    },
    verification: {
      status: "pending",
      attempts: 0,
      confidence: 0.3,
    },
    metadata: {
      created: now,
      updated: now,
      usageCount: 0,
    },
    ...overrides,
  }
}

/**
 * Pre-defined candidate fixtures for specific test scenarios
 */
export const MOCK_CANDIDATES = {
  /**
   * A candidate that just passed verification
   */
  verified: createMockCandidate({
    id: "cand_verified_001",
    name: "format-json",
    description: "Format JSON files with proper indentation",
    type: "workflow",
    content: {
      steps: [
        "1. read: Read the JSON file",
        "2. bash: Run prettier --parser json",
        "3. edit: Write formatted content back",
      ],
    },
    verification: {
      status: "passed",
      attempts: 1,
      confidence: 0.65,
      lastResult: JSON.stringify({ passed: true, passRate: 0.8, timestamp: Date.now() }),
    },
    metadata: {
      created: Date.now() - 86400000, // 1 day ago
      updated: Date.now(),
      usageCount: 3,
      successCount: 2,
      failureCount: 1,
    },
  }),

  /**
   * A candidate that failed verification
   */
  failed: createMockCandidate({
    id: "cand_failed_001",
    name: "deploy-production",
    description: "Deploy application to production",
    type: "workflow",
    content: {
      steps: ["1. bash: npm run build", "2. bash: docker push", "3. bash: kubectl apply"],
    },
    verification: {
      status: "failed",
      attempts: 2,
      confidence: 0.25,
      lastResult: JSON.stringify({ passed: false, passRate: 0.4, timestamp: Date.now() }),
    },
    metadata: {
      created: Date.now() - 172800000, // 2 days ago
      updated: Date.now(),
      usageCount: 5,
      successCount: 2,
      failureCount: 3,
    },
  }),

  /**
   * A high-confidence candidate ready for promotion
   */
  readyForPromotion: createMockCandidate({
    id: "cand_promo_001",
    name: "run-tests-with-coverage",
    description: "Run tests with coverage reporting",
    type: "workflow",
    content: {
      steps: [
        "1. bash: bun test --coverage",
        "2. read: Read coverage/lcov.info",
        "3. task: Report coverage to user",
      ],
    },
    verification: {
      status: "passed",
      attempts: 1,
      confidence: 0.72,
      lastResult: JSON.stringify({ passed: true, passRate: 0.9, timestamp: Date.now() }),
    },
    metadata: {
      created: Date.now() - 604800000, // 7 days ago
      updated: Date.now(),
      usageCount: 8,
      successCount: 7,
      failureCount: 1,
    },
  }),

  /**
   * An experimental candidate with low confidence
   */
  experimental: createMockCandidate({
    id: "cand_exp_001",
    name: "generate-api-docs",
    description: "Generate API documentation from code",
    type: "pattern",
    content: {
      code: `// Extract JSDoc comments and generate markdown`,
    },
    verification: {
      status: "pending",
      attempts: 0,
      confidence: 0.15,
    },
    metadata: {
      created: Date.now() - 3600000, // 1 hour ago
      updated: Date.now(),
      usageCount: 1,
    },
  }),

  /**
   * A candidate that should be discarded (low confidence, many attempts)
   */
  shouldDiscard: createMockCandidate({
    id: "cand_discard_001",
    name: "broken-workflow",
    description: "A workflow that keeps failing",
    type: "workflow",
    content: {
      steps: ["1. bash: invalid_command"],
    },
    verification: {
      status: "failed",
      attempts: 5,
      confidence: 0.12,
      lastResult: JSON.stringify({ passed: false, passRate: 0.0, timestamp: Date.now() }),
    },
    metadata: {
      created: Date.now() - 259200000, // 3 days ago
      updated: Date.now(),
      usageCount: 5,
      successCount: 0,
      failureCount: 5,
    },
  }),

  /**
   * A code pattern candidate
   */
  codePattern: createMockCandidate({
    id: "cand_pattern_001",
    name: "error-handling-pattern",
    description: "Standard error handling with logging",
    type: "pattern",
    content: {
      code: `try {
  const result = await riskyOperation()
  return result
} catch (error) {
  log.error('Operation failed:', error)
  throw new Error('Detailed user-friendly message')
}`,
    },
    verification: {
      status: "passed",
      attempts: 1,
      confidence: 0.68,
    },
    metadata: {
      created: Date.now() - 432000000, // 5 days ago
      updated: Date.now(),
      usageCount: 4,
      successCount: 4,
      failureCount: 0,
    },
  }),

  /**
   * An agent-type candidate
   */
  agentType: createMockCandidate({
    id: "cand_agent_001",
    name: "code-explainer",
    description: "Explain code in simple terms",
    type: "agent",
    content: {
      agentPrompt: `You are a code explanation expert. When given code:
1. Identify the programming language
2. Explain the purpose of the code
3. Walk through the logic step by step
4. Highlight any potential issues`,
    },
    verification: {
      status: "passed",
      attempts: 1,
      confidence: 0.55,
    },
    metadata: {
      created: Date.now() - 518400000, // 6 days ago
      updated: Date.now(),
      usageCount: 6,
      successCount: 5,
      failureCount: 1,
    },
  }),
} as const

/**
 * Create multiple candidates for batch testing
 */
export function createMockCandidateBatch(count: number): BootstrapTypes.SkillCandidate[] {
  return Array.from({ length: count }, (_, i) =>
    createMockCandidate({
      id: generateTestId(`batch_${i}`),
      name: `batch-skill-${i}`,
      verification: {
        status: "pending",
        attempts: 0,
        confidence: Math.random() * 0.5 + 0.2, // 0.2 - 0.7 range
      },
    }),
  )
}

/**
 * Create a candidate with specific confidence for evolution tests
 */
export function createCandidateWithConfidence(confidence: number): BootstrapTypes.SkillCandidate {
  return createMockCandidate({
    id: generateTestId(`conf_${Math.floor(confidence * 100)}`),
    name: `confidence-${Math.floor(confidence * 100)}`,
    verification: {
      status: confidence >= 0.6 ? "passed" : "pending",
      attempts: Math.floor(confidence * 5),
      confidence,
    },
  })
}
