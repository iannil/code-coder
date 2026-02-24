/**
 * Complex Scenario Fixtures
 *
 * Mock data and fixtures for complex multi-agent and real-world scenario tests.
 */

import { generateTestId } from "./mock-candidates"
import type { AutonomyLevel, ComplexityLevel } from "../config"

// ============================================================================
// Types
// ============================================================================

export interface AgentChainScenario {
  id: string
  name: string
  agents: string[]
  expectedDuration: number
  complexity: ComplexityLevel
  requiresContext: boolean
}

export interface MockAgentResult {
  agentId: string
  success: boolean
  output: string
  duration: number
  context?: Record<string, unknown>
}

export interface TaskPlanScenario {
  id: string
  stepCount: number
  complexity: ComplexityLevel
  expectedCompletionRate: number
  steps: TaskStep[]
}

export interface TaskStep {
  id: string
  subject: string
  description: string
  dependencies: string[]
  agent: string
  priority: "critical" | "high" | "medium" | "low"
}

export interface ToolChainScenario {
  id: string
  depth: number
  tools: string[]
  expectedSuccess: boolean
}

export interface CausalChainScenario {
  id: string
  length: number
  decisions: MockDecision[]
  actions: MockAction[]
  outcomes: MockOutcome[]
}

export interface MockDecision {
  id: string
  agentId: string
  prompt: string
  confidence: number
}

export interface MockAction {
  id: string
  decisionId: string
  actionType: string
  description: string
}

export interface MockOutcome {
  id: string
  actionId: string
  status: "success" | "partial" | "failure"
  description: string
}

export interface RefactoringScenario {
  id: string
  name: string
  description: string
  affectedFiles: string[]
  complexity: ComplexityLevel
  expectedChanges: number
}

// ============================================================================
// Agent Chain Scenarios
// ============================================================================

export function createAgentChainScenario(
  agents: string[],
  overrides: Partial<AgentChainScenario> = {},
): AgentChainScenario {
  return {
    id: generateTestId("chain"),
    name: `${agents.length}-Agent Chain`,
    agents,
    expectedDuration: agents.length * 1000,
    complexity: agents.length <= 2 ? "low" : agents.length <= 4 ? "medium" : "high",
    requiresContext: true,
    ...overrides,
  }
}

export const AGENT_CHAIN_SCENARIOS: AgentChainScenario[] = [
  createAgentChainScenario(["build", "code-reviewer", "security-reviewer", "tdd-guide"], {
    name: "Standard 4-Agent Chain",
    expectedDuration: 10000,
    complexity: "high",
  }),
  createAgentChainScenario(["explore", "architect"], {
    name: "Planning Chain",
    expectedDuration: 5000,
    complexity: "medium",
  }),
  createAgentChainScenario(["writer", "expander", "proofreader"], {
    name: "Content Creation Chain",
    expectedDuration: 8000,
    complexity: "medium",
  }),
  createAgentChainScenario(["decision", "architect", "build", "code-reviewer"], {
    name: "Decision-Driven Chain",
    expectedDuration: 12000,
    complexity: "high",
  }),
  createAgentChainScenario(
    ["explore", "architect", "tdd-guide", "build", "code-reviewer", "security-reviewer"],
    {
      name: "Deep 6-Agent Chain",
      expectedDuration: 20000,
      complexity: "extreme",
    },
  ),
]

export function createMockAgentResult(
  agentId: string,
  overrides: Partial<MockAgentResult> = {},
): MockAgentResult {
  return {
    agentId,
    success: true,
    output: `Agent ${agentId} completed successfully`,
    duration: 1000 + Math.random() * 2000,
    context: { timestamp: Date.now(), agentId },
    ...overrides,
  }
}

// ============================================================================
// Task Plan Scenarios
// ============================================================================

export function createTaskStep(
  index: number,
  dependencies: string[] = [],
  overrides: Partial<TaskStep> = {},
): TaskStep {
  const agents = ["explore", "architect", "build", "code-reviewer", "tdd-guide", "security-reviewer"]
  return {
    id: `step_${index}`,
    subject: `Task Step ${index}`,
    description: `Execute step ${index} of the task plan`,
    dependencies,
    agent: agents[index % agents.length],
    priority: index < 5 ? "critical" : index < 20 ? "high" : index < 50 ? "medium" : "low",
    ...overrides,
  }
}

export function createTaskPlanScenario(stepCount: number): TaskPlanScenario {
  const steps: TaskStep[] = []

  for (let i = 0; i < stepCount; i++) {
    // Create dependencies: each step depends on up to 2 previous steps
    const dependencies: string[] = []
    if (i > 0) dependencies.push(`step_${i - 1}`)
    if (i > 5 && i % 5 === 0) dependencies.push(`step_${Math.floor(i / 2)}`)

    steps.push(createTaskStep(i, dependencies))
  }

  const complexity: ComplexityLevel =
    stepCount <= 10 ? "low" : stepCount <= 30 ? "medium" : stepCount <= 70 ? "high" : "extreme"

  return {
    id: generateTestId("plan"),
    stepCount,
    complexity,
    expectedCompletionRate: complexity === "extreme" ? 0.9 : 0.95,
    steps,
  }
}

export const TASK_PLAN_SCENARIOS = {
  small: createTaskPlanScenario(10),
  medium: createTaskPlanScenario(30),
  large: createTaskPlanScenario(50),
  extreme: createTaskPlanScenario(100),
}

// ============================================================================
// Tool Chain Scenarios
// ============================================================================

export function createToolChainScenario(depth: number): ToolChainScenario {
  const availableTools = ["read", "edit", "write", "bash", "grep", "glob", "task"]
  const tools: string[] = []

  for (let i = 0; i < depth; i++) {
    tools.push(availableTools[i % availableTools.length])
  }

  return {
    id: generateTestId("toolchain"),
    depth,
    tools,
    expectedSuccess: depth <= 15,
  }
}

export const TOOL_CHAIN_SCENARIOS = {
  shallow: createToolChainScenario(3),
  medium: createToolChainScenario(7),
  deep: createToolChainScenario(10),
  extreme: createToolChainScenario(15),
}

// ============================================================================
// Causal Chain Scenarios
// ============================================================================

export function createCausalChainScenario(length: number): CausalChainScenario {
  const decisions: MockDecision[] = []
  const actions: MockAction[] = []
  const outcomes: MockOutcome[] = []

  for (let i = 0; i < length; i++) {
    const decisionId = `dec_${i}`
    const actionId = `act_${i}`
    const outcomeId = `out_${i}`

    decisions.push({
      id: decisionId,
      agentId: `agent_${i % 5}`,
      prompt: `Decision prompt ${i}: Should we proceed with action ${i}?`,
      confidence: 0.5 + Math.random() * 0.5,
    })

    actions.push({
      id: actionId,
      decisionId,
      actionType: ["tool_execution", "code_change", "file_operation"][i % 3],
      description: `Action ${i}: Execute operation`,
    })

    outcomes.push({
      id: outcomeId,
      actionId,
      status: i % 10 === 9 ? "failure" : i % 5 === 4 ? "partial" : "success",
      description: `Outcome ${i}: Operation result`,
    })
  }

  return {
    id: generateTestId("causal"),
    length,
    decisions,
    actions,
    outcomes,
  }
}

export const CAUSAL_CHAIN_SCENARIOS = {
  short: createCausalChainScenario(10),
  medium: createCausalChainScenario(50),
  long: createCausalChainScenario(100),
  veryLong: createCausalChainScenario(500),
}

// ============================================================================
// Real-World Refactoring Scenarios
// ============================================================================

export const REFACTORING_SCENARIOS: RefactoringScenario[] = [
  {
    id: "refactor_rename_module",
    name: "Rename Module Across Files",
    description: "Rename a module from 'oldName' to 'newName' across 20 files",
    affectedFiles: Array.from({ length: 20 }, (_, i) => `src/module${i}/index.ts`),
    complexity: "high",
    expectedChanges: 60,
  },
  {
    id: "refactor_extract_service",
    name: "Extract Service Layer",
    description: "Extract common business logic into a service layer",
    affectedFiles: [
      "src/controllers/user.ts",
      "src/controllers/auth.ts",
      "src/services/user.ts",
      "src/services/auth.ts",
      "src/index.ts",
    ],
    complexity: "medium",
    expectedChanges: 25,
  },
  {
    id: "refactor_api_migration",
    name: "API Version Migration",
    description: "Migrate from v1 API to v2 API across entire codebase",
    affectedFiles: Array.from({ length: 15 }, (_, i) => `src/api/endpoints/${i}.ts`),
    complexity: "high",
    expectedChanges: 100,
  },
]

// ============================================================================
// Memory Pressure Scenarios
// ============================================================================

export interface MemoryPressureScenario {
  id: string
  embeddingCount: number
  dailyNoteCount: number
  callGraphNodes: number
  causalDecisions: number
  expectedLatency: number
}

export const MEMORY_PRESSURE_SCENARIOS: MemoryPressureScenario[] = [
  {
    id: "memory_light",
    embeddingCount: 100,
    dailyNoteCount: 5,
    callGraphNodes: 50,
    causalDecisions: 20,
    expectedLatency: 100,
  },
  {
    id: "memory_moderate",
    embeddingCount: 1000,
    dailyNoteCount: 30,
    callGraphNodes: 500,
    causalDecisions: 100,
    expectedLatency: 250,
  },
  {
    id: "memory_heavy",
    embeddingCount: 5000,
    dailyNoteCount: 90,
    callGraphNodes: 1000,
    causalDecisions: 500,
    expectedLatency: 500,
  },
  {
    id: "memory_extreme",
    embeddingCount: 10000,
    dailyNoteCount: 180,
    callGraphNodes: 2000,
    causalDecisions: 1000,
    expectedLatency: 1000,
  },
]

// ============================================================================
// Decision Distribution Scenarios
// ============================================================================

export interface DecisionDistributionScenario {
  autonomyLevel: AutonomyLevel
  decisionCount: number
  expectedApprovalRate: { min: number; max: number }
  expectedProceedRate: { min: number; max: number }
}

export const DECISION_DISTRIBUTION_SCENARIOS: DecisionDistributionScenario[] = [
  {
    autonomyLevel: "lunatic",
    decisionCount: 100,
    expectedApprovalRate: { min: 0.85, max: 1.0 },
    expectedProceedRate: { min: 0.75, max: 1.0 },
  },
  {
    autonomyLevel: "insane",
    decisionCount: 100,
    expectedApprovalRate: { min: 0.75, max: 0.95 },
    expectedProceedRate: { min: 0.65, max: 0.90 },
  },
  {
    autonomyLevel: "crazy",
    decisionCount: 100,
    expectedApprovalRate: { min: 0.65, max: 0.85 },
    expectedProceedRate: { min: 0.55, max: 0.80 },
  },
  {
    autonomyLevel: "wild",
    decisionCount: 100,
    expectedApprovalRate: { min: 0.50, max: 0.75 },
    expectedProceedRate: { min: 0.40, max: 0.70 },
  },
  {
    autonomyLevel: "bold",
    decisionCount: 100,
    expectedApprovalRate: { min: 0.35, max: 0.60 },
    expectedProceedRate: { min: 0.25, max: 0.55 },
  },
  {
    autonomyLevel: "timid",
    decisionCount: 100,
    expectedApprovalRate: { min: 0.15, max: 0.40 },
    expectedProceedRate: { min: 0.10, max: 0.35 },
  },
]
