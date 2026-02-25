/**
 * Autonomous Mode - Autonomous Execution System
 *
 * A fully autonomous agent system that can:
 * - Plan complex tasks independently
 * - Make technical decisions using CLOSE framework
 * - Execute complete TDD cycles
 * - Self-correct when tests fail
 * - Manage checkpoints and rollbacks
 * - Score quality and "craziness"
 *
 * Based on the "祝融说" (ZhuRong Theory) philosophy of sustainable decision-making.
 *
 * @package autonomous
 */

import type { AutonomousModeConfig } from "./config/schema"

// State machine
export {
  AutonomousState,
  StateMetadata,
  VALID_TRANSITIONS,
  isValidTransition,
  isTerminal,
  isRecoverable,
} from "./state/states"
export { StateMachine, createStateMachine } from "./state/state-machine"
export type { TransitionOptions, StateMachineConfig } from "./state/state-machine"
export {
  resourceGuard,
  errorGuard,
  progressGuard,
  OscillationGuard,
  andGuard,
  orGuard,
  notGuard,
  TransitionRecipes,
} from "./state/transitions"
export type { TransitionGuard, TransitionContext } from "./state/transitions"

// Events
export { AutonomousEvent, AutonomousEventHelper } from "./events"

// Decision system
export { DecisionEngine, createDecisionEngine, evaluateDecision } from "./decision/engine"
export type { DecisionEngineConfig, DecisionContext, DecisionResult, AutonomyLevel } from "./decision/engine"
export { DecisionTemplates, buildCriteria, calculateCLOSEFromContext, validateCLOSEScore } from "./decision/criteria"
export type {
  AutonomousDecisionCriteria,
  CLOSEScore,
  DecisionType,
  RiskLevel,
  DecisionRecord,
} from "./decision/criteria"
export { DecisionHistory } from "./decision/history"

// Orchestration
export { Orchestrator, createOrchestrator } from "./orchestration/orchestrator"
export type { OrchestratorConfig, SessionContext } from "./orchestration/orchestrator"
export { PhaseRunner, createPhaseRunner, PhaseTemplates } from "./orchestration/phase-runner"
export type { Phase, PhaseContext, PhaseResult, PhaseRunnerConfig } from "./orchestration/phase-runner"
export { TaskQueue, createTaskQueue } from "./orchestration/task-queue"
export type { Task, TaskPriority, TaskStatus, TaskQueueConfig } from "./orchestration/task-queue"

// Planning
export { RequirementTracker, createRequirementTracker } from "./planning/requirement-tracker"
export type {
  Requirement,
  RequirementStatus,
  AcceptanceCriterion,
  ParseResult,
  RequirementTrackerConfig,
} from "./planning/requirement-tracker"
export { NextStepPlanner, createNextStepPlanner } from "./planning/next-step-planner"
export type {
  CompletionCriteria,
  NextStepPlan,
  NextStepExecutionContext,
  NextStepPlannerConfig,
} from "./planning/next-step-planner"

// Execution
export { ContextManager, createExecutionContext } from "./execution/context"
export type { ExecutionContext, ContextOptions } from "./execution/context"
export { Executor, createExecutor } from "./execution/executor"
export type { ExecutionConfig, TestResult, VerificationResult, TDDCycleResult, TDDPhase } from "./execution/executor"
export { CheckpointManager, createCheckpointManager } from "./execution/checkpoint"
export type { Checkpoint, CheckpointType } from "./execution/checkpoint"
export { AgentInvoker } from "./execution/agent-invoker"
export type { AgentInvocationRequest, AgentInvocationResult, InvocableAgent } from "./execution/agent-invoker"
export { GitOps } from "./execution/git-ops"
export type { GitCheckpoint, GitStatus, GitCommitResult } from "./execution/git-ops"
export { TestRunner } from "./execution/test-runner"

// Sandbox Execution (Phase 3)
export { SandboxExecutor, createSandboxExecutor } from "./execution/sandbox"
export type {
  SandboxLanguage,
  SandboxRequest,
  ResourceLimits,
  SandboxResult,
  ReflectionResult,
} from "./execution/sandbox"

// Enhanced Web Search (Phase 3)
export { EnhancedWebSearch, createEnhancedWebSearch } from "./execution/enhanced-web-search"
export type { WebFetchResult, ExtractedDoc } from "./execution/enhanced-web-search"

// Knowledge Sedimentation (Phase 3)
export {
  KnowledgeSedimentation,
  getKnowledgeSedimentation,
  createKnowledgeSedimentation,
} from "./execution/knowledge-sedimentation"
export type {
  KnowledgeCategory,
  KnowledgeEntry,
  KnowledgeSource,
  ExtractionContext,
  KnowledgeSearchResult,
} from "./execution/knowledge-sedimentation"

// Evolution Loop (Phase 3)
export { EvolutionLoop, createEvolutionLoop, evolveProblem } from "./execution/evolution-loop"
export type {
  AutonomousProblem,
  SolutionAttempt,
  EvolutionResult,
  EvolutionConfig,
} from "./execution/evolution-loop"

// LLM Solver (Phase 1 Enhancement)
export { LLMSolver, getLLMSolver, createLLMSolver } from "./execution/llm-solver"
export type {
  CodeGenerationContext,
  CodeGenerationResult,
  ReflectionContext,
  ReflectionAnalysis,
  LLMSolverConfig,
} from "./execution/llm-solver"

// Memory Writer (Phase 1 Enhancement)
export {
  writeEvolutionToMemory,
  sedimentEvolutionSuccess,
  logEvolutionFailure,
} from "./execution/memory-writer"
export type {
  EvolutionMemoryContext,
  MemoryWriteOptions,
} from "./execution/memory-writer"

// Safety
export { SafetyGuard, parseResourceBudget } from "./safety/constraints"
export type { ResourceBudget, ResourceUsage, SafetyCheckResult, SafetyConfig } from "./safety/constraints"
export { SafetyGuardrails, createGuardrails } from "./safety/guardrails"
export type { GuardrailConfig, LoopPattern } from "./safety/guardrails"
export { RollbackManager, createRollbackManager } from "./safety/rollback"
export type { RollbackTrigger, RollbackOptions, RollbackResult } from "./safety/rollback"
export {
  SafetyIntegration,
  createSafetyIntegration,
  isDestructiveOperation,
  getDestructiveRiskLevel,
} from "./safety/integration"
export type {
  SafetyIntegrationConfig,
  SafetyStatus,
  DestructiveOperation,
  DestructiveCategory,
} from "./safety/integration"

// Metrics
export { MetricsCollector, createMetricsCollector, getSessionMetrics, getAllSessionMetrics } from "./metrics/metrics"
export type { MetricType, MetricData, SessionMetrics, StoredSessionMetrics } from "./metrics/metrics"
export { Scorer, createScorer, calculateScores } from "./metrics/scorer"
export type { QualityScoreBreakdown, CrazinessScoreBreakdown, CrazinessLevel, ScoringWeights } from "./metrics/scorer"
export {
  Reporter,
  createReporter,
  generateSummaryReport,
  generateFullReport,
  getCrazinessDescription,
} from "./metrics/reporter"
export type { Report, ReportType, ReportOptions } from "./metrics/reporter"

// Configuration
export { AutonomousConfig, DEFAULT_AUTONOMOUS_MODE_CONFIG, mergeAutonomousModeConfig } from "./config/config"
export {
  AutonomousModeConfigSchema,
  CloseWeightsSchema,
  CheckpointConfigSchema,
  LoopDetectionConfigSchema,
  SessionConfigSchema,
} from "./config/schema"
export type {
  AutonomyLevel as ConfigAutonomyLevel,
  CloseWeights,
  CheckpointConfig,
  LoopDetectionConfig,
  SessionConfig,
} from "./config/schema"

// Integration
export { AutonomousModeHook } from "./integration/hook"
export { DecisionReporter } from "./integration/reporter"
export type { DecisionSummary } from "./integration/reporter"

// Validate autonomous mode config
export const validateAutonomousModeConfig = (
  config: unknown,
): { success: boolean; data?: AutonomousModeConfig; errors?: string[] } => {
  const { AutonomousModeConfigSchema } = require("./config/schema")
  const result = AutonomousModeConfigSchema.safeParse(config)

  if (result.success) {
    return {
      success: true,
      data: result.data,
    }
  }

  const zodError = result.error
  return {
    success: false,
    errors: zodError.issues.map(
      (e: { path: (string | number)[]; message: string }) => `${e.path.join(".")}: ${e.message}`,
    ),
  }
}

/**
 * Version of Autonomous Mode
 */
export const AUTONOMOUS_VERSION = "0.1.0"
