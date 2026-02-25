/**
 * Complex Evaluation Metrics
 *
 * Advanced metrics for measuring reliability, complexity, and performance
 * in complex multi-agent and autonomous scenarios.
 */

import { Statistics } from "./metrics"

// ============================================================================
// Chain Analysis Metrics
// ============================================================================

export interface ChainMetrics {
  totalChains: number
  completedChains: number
  completionRate: number
  averageChainLength: number
  averageChainDuration: number
  contextLossRate: number
  errorRecoveryRate: number
}

export interface ChainExecution {
  chainId: string
  agents: string[]
  completed: boolean
  duration: number
  contextPreserved: boolean
  errors: string[]
  recoveredFromErrors: boolean
}

/**
 * Calculate chain execution metrics
 */
export function calculateChainMetrics(executions: ChainExecution[]): ChainMetrics {
  if (executions.length === 0) {
    return {
      totalChains: 0,
      completedChains: 0,
      completionRate: 0,
      averageChainLength: 0,
      averageChainDuration: 0,
      contextLossRate: 0,
      errorRecoveryRate: 0,
    }
  }

  const completedChains = executions.filter((e) => e.completed).length
  const chainsWithErrors = executions.filter((e) => e.errors.length > 0)
  const chainsWithContextLoss = executions.filter((e) => !e.contextPreserved)
  const recoveredChains = chainsWithErrors.filter((e) => e.recoveredFromErrors)

  return {
    totalChains: executions.length,
    completedChains,
    completionRate: completedChains / executions.length,
    averageChainLength: Statistics.mean(executions.map((e) => e.agents.length)),
    averageChainDuration: Statistics.mean(executions.map((e) => e.duration)),
    contextLossRate: chainsWithContextLoss.length / executions.length,
    errorRecoveryRate: chainsWithErrors.length > 0 ? recoveredChains.length / chainsWithErrors.length : 1,
  }
}

// ============================================================================
// Task Plan Metrics
// ============================================================================

export interface TaskPlanMetrics {
  totalSteps: number
  completedSteps: number
  failedSteps: number
  skippedSteps: number
  completionRate: number
  successRate: number
  averageStepDuration: number
  criticalPathDuration: number
  parallelizationEfficiency: number
}

export interface StepExecution {
  stepId: string
  completed: boolean
  duration: number
  dependencies: string[]
  dependentsCount: number
}

/**
 * Calculate task plan execution metrics
 */
export function calculateTaskPlanMetrics(
  steps: StepExecution[],
  totalDuration: number,
): TaskPlanMetrics {
  if (steps.length === 0) {
    return {
      totalSteps: 0,
      completedSteps: 0,
      failedSteps: 0,
      skippedSteps: 0,
      completionRate: 0,
      successRate: 0,
      averageStepDuration: 0,
      criticalPathDuration: 0,
      parallelizationEfficiency: 0,
    }
  }

  const completedSteps = steps.filter((s) => s.completed).length
  const failedSteps = steps.filter((s) => !s.completed && s.duration > 0).length
  const skippedSteps = steps.filter((s) => !s.completed && s.duration === 0).length

  // Calculate critical path (longest dependency chain)
  const criticalPath = calculateCriticalPath(steps)

  // Calculate sequential duration (if all steps ran in sequence)
  const sequentialDuration = steps.reduce((sum, s) => sum + s.duration, 0)

  // Parallelization efficiency: sequential time / actual time
  const parallelizationEfficiency =
    totalDuration > 0 ? Math.min(sequentialDuration / totalDuration, steps.length) : 1

  return {
    totalSteps: steps.length,
    completedSteps,
    failedSteps,
    skippedSteps,
    completionRate: completedSteps / steps.length,
    successRate: completedSteps / (completedSteps + failedSteps || 1),
    averageStepDuration: Statistics.mean(steps.filter((s) => s.duration > 0).map((s) => s.duration)),
    criticalPathDuration: criticalPath,
    parallelizationEfficiency,
  }
}

/**
 * Calculate critical path duration
 */
function calculateCriticalPath(steps: StepExecution[]): number {
  const stepMap = new Map(steps.map((s) => [s.stepId, s]))
  const memo = new Map<string, number>()

  function getPathDuration(stepId: string): number {
    if (memo.has(stepId)) return memo.get(stepId)!

    const step = stepMap.get(stepId)
    if (!step) return 0

    const depDurations = step.dependencies.map((d) => getPathDuration(d))
    const maxDepDuration = depDurations.length > 0 ? Math.max(...depDurations) : 0
    const totalDuration = maxDepDuration + step.duration

    memo.set(stepId, totalDuration)
    return totalDuration
  }

  return Math.max(...steps.map((s) => getPathDuration(s.stepId)), 0)
}

// ============================================================================
// Decision Distribution Metrics
// ============================================================================

export interface DecisionMetrics {
  totalDecisions: number
  approvalRate: number
  proceedRate: number
  cautionRate: number
  pauseRate: number
  blockRate: number
  averageScore: number
  scoreDistribution: {
    low: number // 0-3
    medium: number // 3-6
    high: number // 6-8
    veryHigh: number // 8-10
  }
  boundaryDecisions: number
  consistencyScore: number
}

export interface DecisionRecord {
  decisionId: string
  score: number
  result: "proceed" | "proceed_with_caution" | "pause" | "block" | "skip"
  approved: boolean
  thresholdDistance: number
}

/**
 * Calculate decision distribution metrics
 */
export function calculateDecisionMetrics(
  decisions: DecisionRecord[],
  approvalThreshold: number,
): DecisionMetrics {
  if (decisions.length === 0) {
    return {
      totalDecisions: 0,
      approvalRate: 0,
      proceedRate: 0,
      cautionRate: 0,
      pauseRate: 0,
      blockRate: 0,
      averageScore: 0,
      scoreDistribution: { low: 0, medium: 0, high: 0, veryHigh: 0 },
      boundaryDecisions: 0,
      consistencyScore: 0,
    }
  }

  const approved = decisions.filter((d) => d.approved).length
  const proceed = decisions.filter((d) => d.result === "proceed").length
  const caution = decisions.filter((d) => d.result === "proceed_with_caution").length
  const pause = decisions.filter((d) => d.result === "pause").length
  const block = decisions.filter((d) => d.result === "block").length

  const scores = decisions.map((d) => d.score)
  const scoreDistribution = {
    low: scores.filter((s) => s < 3).length,
    medium: scores.filter((s) => s >= 3 && s < 6).length,
    high: scores.filter((s) => s >= 6 && s < 8).length,
    veryHigh: scores.filter((s) => s >= 8).length,
  }

  // Boundary decisions: within 0.5 of threshold
  const boundaryDecisions = decisions.filter((d) => Math.abs(d.thresholdDistance) < 0.5).length

  // Consistency score: how consistent are decisions for similar scores
  const consistencyScore = calculateConsistencyScore(decisions)

  return {
    totalDecisions: decisions.length,
    approvalRate: approved / decisions.length,
    proceedRate: proceed / decisions.length,
    cautionRate: caution / decisions.length,
    pauseRate: pause / decisions.length,
    blockRate: block / decisions.length,
    averageScore: Statistics.mean(scores),
    scoreDistribution,
    boundaryDecisions,
    consistencyScore,
  }
}

/**
 * Calculate consistency score for decisions
 */
function calculateConsistencyScore(decisions: DecisionRecord[]): number {
  if (decisions.length < 2) return 1

  // Group decisions by score bucket
  const buckets = new Map<number, DecisionRecord[]>()
  for (const d of decisions) {
    const bucket = Math.floor(d.score)
    const existing = buckets.get(bucket) ?? []
    existing.push(d)
    buckets.set(bucket, existing)
  }

  // Check consistency within each bucket
  let totalBuckets = 0
  let consistentBuckets = 0

  for (const [, records] of buckets) {
    if (records.length < 2) continue
    totalBuckets++

    const firstResult = records[0].approved
    const allSame = records.every((r) => r.approved === firstResult)
    if (allSame) consistentBuckets++
  }

  return totalBuckets > 0 ? consistentBuckets / totalBuckets : 1
}

// ============================================================================
// Memory System Metrics
// ============================================================================

export interface MemoryMetrics {
  readLatency: { p50: number; p95: number; p99: number }
  writeLatency: { p50: number; p95: number; p99: number }
  queryLatency: { p50: number; p95: number; p99: number }
  consistencyRate: number
  recoveryRate: number
  dataIntegrity: number
}

export interface MemoryOperation {
  type: "read" | "write" | "query"
  latency: number
  success: boolean
  dataConsistent?: boolean
}

/**
 * Calculate memory system metrics
 */
export function calculateMemoryMetrics(operations: MemoryOperation[]): MemoryMetrics {
  const reads = operations.filter((o) => o.type === "read")
  const writes = operations.filter((o) => o.type === "write")
  const queries = operations.filter((o) => o.type === "query")

  const getLatencyPercentiles = (ops: MemoryOperation[]) => {
    const latencies = ops.map((o) => o.latency).sort((a, b) => a - b)
    return {
      p50: Statistics.percentile(latencies, 50),
      p95: Statistics.percentile(latencies, 95),
      p99: Statistics.percentile(latencies, 99),
    }
  }

  const successfulOps = operations.filter((o) => o.success)
  const consistentOps = operations.filter((o) => o.dataConsistent !== false)

  return {
    readLatency: getLatencyPercentiles(reads),
    writeLatency: getLatencyPercentiles(writes),
    queryLatency: getLatencyPercentiles(queries),
    consistencyRate: operations.length > 0 ? consistentOps.length / operations.length : 1,
    recoveryRate: operations.length > 0 ? successfulOps.length / operations.length : 1,
    dataIntegrity: operations.length > 0 ? consistentOps.length / operations.length : 1,
  }
}

// ============================================================================
// Chaos Engineering Metrics
// ============================================================================

export interface ChaosMetrics {
  faultsInjected: number
  faultsRecovered: number
  recoveryRate: number
  averageRecoveryTime: number
  maxRecoveryTime: number
  systemDegradation: number
  gracefulDegradation: boolean
}

export interface ChaosEvent {
  faultType: string
  recovered: boolean
  recoveryTime: number
  systemHealthBefore: number
  systemHealthAfter: number
}

/**
 * Calculate chaos engineering metrics
 */
export function calculateChaosMetrics(events: ChaosEvent[]): ChaosMetrics {
  if (events.length === 0) {
    return {
      faultsInjected: 0,
      faultsRecovered: 0,
      recoveryRate: 0,
      averageRecoveryTime: 0,
      maxRecoveryTime: 0,
      systemDegradation: 0,
      gracefulDegradation: true,
    }
  }

  const recovered = events.filter((e) => e.recovered)
  const recoveryTimes = recovered.map((e) => e.recoveryTime)

  // Calculate degradation (average health drop)
  const degradations = events.map((e) => e.systemHealthBefore - e.systemHealthAfter)
  const avgDegradation = Statistics.mean(degradations)

  // Graceful degradation: system didn't crash (health stayed above 0.2)
  const gracefulDegradation = events.every((e) => e.systemHealthAfter > 0.2)

  return {
    faultsInjected: events.length,
    faultsRecovered: recovered.length,
    recoveryRate: recovered.length / events.length,
    averageRecoveryTime: recoveryTimes.length > 0 ? Statistics.mean(recoveryTimes) : 0,
    maxRecoveryTime: recoveryTimes.length > 0 ? Math.max(...recoveryTimes) : 0,
    systemDegradation: avgDegradation,
    gracefulDegradation,
  }
}

// ============================================================================
// Performance Benchmarking
// ============================================================================

export interface BenchmarkResult {
  name: string
  iterations: number
  totalDuration: number
  averageDuration: number
  minDuration: number
  maxDuration: number
  standardDeviation: number
  throughput: number
}

/**
 * Run a benchmark
 */
export async function runBenchmark(
  name: string,
  operation: () => Promise<void> | void,
  iterations: number,
): Promise<BenchmarkResult> {
  const durations: number[] = []

  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    await operation()
    durations.push(performance.now() - start)
  }

  const totalDuration = durations.reduce((a, b) => a + b, 0)

  return {
    name,
    iterations,
    totalDuration,
    averageDuration: Statistics.mean(durations),
    minDuration: Math.min(...durations),
    maxDuration: Math.max(...durations),
    standardDeviation: Statistics.stdDev(durations),
    throughput: (iterations / totalDuration) * 1000, // ops/second
  }
}

// ============================================================================
// Evaluation Summary
// ============================================================================

export interface EvaluationDimension {
  name: string
  score: number
  weight: number
  passed: boolean
  details: string[]
}

export interface ComplexEvaluationSummary {
  overallScore: number
  dimensions: EvaluationDimension[]
  passRate: number
  criticalFailures: string[]
  recommendations: string[]
}

/**
 * Generate evaluation summary
 */
export function generateEvaluationSummary(dimensions: EvaluationDimension[]): ComplexEvaluationSummary {
  const totalWeight = dimensions.reduce((sum, d) => sum + d.weight, 0)
  const weightedScore = dimensions.reduce((sum, d) => sum + d.score * d.weight, 0)
  const overallScore = totalWeight > 0 ? weightedScore / totalWeight : 0

  const passedDimensions = dimensions.filter((d) => d.passed)
  const failedDimensions = dimensions.filter((d) => !d.passed)

  const criticalFailures = failedDimensions
    .filter((d) => d.weight >= 1.5)
    .flatMap((d) => d.details)

  const recommendations = failedDimensions.map(
    (d) => `Improve ${d.name}: Current score ${d.score.toFixed(2)}, target ≥ 0.7`,
  )

  return {
    overallScore,
    dimensions,
    passRate: dimensions.length > 0 ? passedDimensions.length / dimensions.length : 0,
    criticalFailures,
    recommendations,
  }
}

// ============================================================================
// Parallel Execution Analysis
// ============================================================================

export interface ParallelExecutionMetrics {
  totalTasks: number
  parallelBatches: number
  maxConcurrency: number
  speedupFactor: number
  efficiency: number
  loadBalance: number
}

export interface TaskTiming {
  taskId: string
  startTime: number
  endTime: number
  batchId: number
}

/**
 * Analyze parallel execution
 */
export function analyzeParallelExecution(timings: TaskTiming[]): ParallelExecutionMetrics {
  if (timings.length === 0) {
    return {
      totalTasks: 0,
      parallelBatches: 0,
      maxConcurrency: 0,
      speedupFactor: 0,
      efficiency: 0,
      loadBalance: 0,
    }
  }

  // Group by batch
  const batches = new Map<number, TaskTiming[]>()
  for (const t of timings) {
    const batch = batches.get(t.batchId) ?? []
    batch.push(t)
    batches.set(t.batchId, batch)
  }

  // Calculate sequential time
  const sequentialTime = timings.reduce((sum, t) => sum + (t.endTime - t.startTime), 0)

  // Calculate actual parallel time
  const minStart = Math.min(...timings.map((t) => t.startTime))
  const maxEnd = Math.max(...timings.map((t) => t.endTime))
  const parallelTime = maxEnd - minStart

  // Max concurrency: maximum tasks running at any point
  let maxConcurrency = 0
  for (const [, batch] of batches) {
    maxConcurrency = Math.max(maxConcurrency, batch.length)
  }

  // Load balance: variance in batch sizes
  const batchSizes = Array.from(batches.values()).map((b) => b.length)
  const loadBalance = batchSizes.length > 1 ? 1 - (Statistics.stdDev(batchSizes) / Statistics.mean(batchSizes)) : 1

  const speedupFactor = parallelTime > 0 ? sequentialTime / parallelTime : 1
  const efficiency = maxConcurrency > 0 ? speedupFactor / maxConcurrency : 1

  return {
    totalTasks: timings.length,
    parallelBatches: batches.size,
    maxConcurrency,
    speedupFactor,
    efficiency,
    loadBalance: Math.max(0, loadBalance),
  }
}
