/**
 * Bootstrap Flywheel Evaluation Test Suite
 *
 * This module provides comprehensive evaluation tests for CodeCoder's
 * autonomous research and evolution capabilities. The tests are organized
 * into dimensions matching the evaluation plan.
 *
 * ## Running Tests
 *
 * ```bash
 * cd packages/ccode
 *
 * # Run all evaluation tests
 * bun test ./test/evaluation/
 *
 * # Run specific dimension
 * bun test ./test/evaluation/confidence.eval.test.ts
 * bun test ./test/evaluation/awareness.eval.test.ts
 * bun test ./test/evaluation/verification.eval.test.ts
 *
 * # Run reliability tests
 * bun test ./test/evaluation/agent-collaboration.eval.test.ts
 * bun test ./test/evaluation/autonomous-extreme.eval.test.ts
 * bun test ./test/evaluation/memory-integrity.eval.test.ts
 * bun test ./test/evaluation/tool-robustness.eval.test.ts
 * bun test ./test/evaluation/complex-scenarios.eval.test.ts
 * bun test ./test/evaluation/chaos-engineering.eval.test.ts
 *
 * # Run with coverage
 * bun test --coverage ./test/evaluation/
 * ```
 *
 * ## Test Organization
 *
 * ### Bootstrap Flywheel Tests
 * - **confidence.eval.test.ts** - Confidence evolution tests (E1-E5)
 * - **awareness.eval.test.ts** - Self-awareness tests (A1-A4)
 * - **verification.eval.test.ts** - Verification loop tests (V1-V4)
 * - **acquisition.eval.test.ts** - Resource acquisition tests (R1-R3)
 * - **crystallization.eval.test.ts** - Skill crystallization tests (C1-C3)
 * - **memory.eval.test.ts** - Memory persistence tests (M1-M4)
 * - **e2e-evolution.eval.test.ts** - End-to-end evolution tests
 * - **stress.eval.test.ts** - Stress and performance tests
 *
 * ### Reliability Tests (7 Dimensions)
 * - **agent-collaboration.eval.test.ts** - Multi-Agent collaboration (Dimension 1)
 * - **autonomous-extreme.eval.test.ts** - Autonomous execution limits (Dimension 2)
 * - **memory-integrity.eval.test.ts** - Memory system integrity (Dimension 3)
 * - **tool-robustness.eval.test.ts** - Tool robustness (Dimension 4)
 * - **complex-scenarios.eval.test.ts** - Real-world scenarios (Dimension 5)
 * - **chaos-engineering.eval.test.ts** - Chaos engineering (Dimension 6)
 *
 * ## Fixtures
 *
 * - **mock-candidates.ts** - Pre-defined skill candidates for testing
 * - **mock-sessions.ts** - Simulated session data
 * - **expected-results.ts** - Expected outcomes and thresholds
 * - **complex-scenarios.ts** - Complex multi-agent scenario data
 *
 * ## Utilities
 *
 * - **metrics.ts** - Metric calculation functions
 * - **reporters.ts** - Report generation and formatting
 * - **chaos.ts** - Chaos engineering utilities
 * - **metrics-complex.ts** - Advanced complexity metrics
 *
 * ## Configuration
 *
 * - **config.ts** - Evaluation test configuration and thresholds
 *
 * ## Notes
 *
 * - All tests pass without requiring LLM API access
 * - Tests use mock data and default fallback behaviors
 * - The test suite validates both unit functionality and integration flows
 * - When LLM providers are unavailable, scenario generation and name generation
 *   gracefully fall back to default implementations
 */

// Re-export configuration
export * from "./config"

// Re-export fixtures
export * from "./fixtures"

// Re-export utilities
export * from "./utils"
