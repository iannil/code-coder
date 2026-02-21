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
 * # Run with coverage
 * bun test --coverage ./test/evaluation/
 * ```
 *
 * ## Test Organization
 *
 * - **confidence.eval.test.ts** - Confidence evolution tests (E1-E5)
 * - **awareness.eval.test.ts** - Self-awareness tests (A1-A4)
 * - **verification.eval.test.ts** - Verification loop tests (V1-V4)
 * - **acquisition.eval.test.ts** - Resource acquisition tests (R1-R3)
 * - **crystallization.eval.test.ts** - Skill crystallization tests (C1-C3)
 * - **memory.eval.test.ts** - Memory persistence tests (M1-M4)
 * - **e2e-evolution.eval.test.ts** - End-to-end evolution tests
 * - **stress.eval.test.ts** - Stress and performance tests
 *
 * ## Fixtures
 *
 * - **mock-candidates.ts** - Pre-defined skill candidates for testing
 * - **mock-sessions.ts** - Simulated session data
 * - **expected-results.ts** - Expected outcomes and thresholds
 *
 * ## Utilities
 *
 * - **metrics.ts** - Metric calculation functions
 * - **reporters.ts** - Report generation and formatting
 *
 * ## Notes
 *
 * - All 207 tests pass without requiring LLM API access
 * - Tests use mock data and default fallback behaviors
 * - The test suite validates both unit functionality and integration flows
 * - When LLM providers are unavailable, scenario generation and name generation
 *   gracefully fall back to default implementations
 */

// Re-export fixtures
export * from "./fixtures"

// Re-export utilities
export * from "./utils"
