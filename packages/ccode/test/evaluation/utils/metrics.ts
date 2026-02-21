/**
 * Evaluation metrics calculation utilities
 *
 * Provides functions to calculate and aggregate evaluation metrics
 * for the Bootstrap Flywheel system.
 */

import type { BootstrapTypes } from "@/bootstrap/types"

/**
 * Metric result with statistical information
 */
export interface MetricResult {
  name: string
  value: number
  target: number
  passed: boolean
  details?: string
}

/**
 * Aggregated evaluation results
 */
export interface EvaluationSummary {
  dimension: string
  metrics: MetricResult[]
  overallScore: number
  passRate: number
}

/**
 * Calculate confidence calibration error
 * Measures how well predicted confidence matches actual success rate
 */
export function calculateCalibrationError(
  predictions: Array<{ confidence: number; actual: boolean }>,
): number {
  if (predictions.length === 0) return 0

  // Group by confidence buckets (0.1 intervals)
  const buckets = new Map<number, { predicted: number; actual: number; count: number }>()

  for (const p of predictions) {
    const bucket = Math.floor(p.confidence * 10) / 10
    const existing = buckets.get(bucket) ?? { predicted: 0, actual: 0, count: 0 }
    existing.predicted += p.confidence
    existing.actual += p.actual ? 1 : 0
    existing.count++
    buckets.set(bucket, existing)
  }

  // Calculate average absolute error across buckets
  let totalError = 0
  let totalWeight = 0

  for (const [_, bucket] of buckets) {
    const avgPredicted = bucket.predicted / bucket.count
    const avgActual = bucket.actual / bucket.count
    totalError += Math.abs(avgPredicted - avgActual) * bucket.count
    totalWeight += bucket.count
  }

  return totalWeight > 0 ? totalError / totalWeight : 0
}

/**
 * Calculate precision for resource suggestions
 */
export function calculatePrecision(
  suggestions: string[],
  relevant: string[],
): number {
  if (suggestions.length === 0) return 0

  const relevantSet = new Set(relevant.map((r) => r.toLowerCase()))
  const hits = suggestions.filter((s) => relevantSet.has(s.toLowerCase())).length

  return hits / suggestions.length
}

/**
 * Calculate recall for resource suggestions
 */
export function calculateRecall(
  suggestions: string[],
  relevant: string[],
): number {
  if (relevant.length === 0) return 1 // No relevant items means perfect recall

  const suggestedSet = new Set(suggestions.map((s) => s.toLowerCase()))
  const hits = relevant.filter((r) => suggestedSet.has(r.toLowerCase())).length

  return hits / relevant.length
}

/**
 * Calculate F1 score (harmonic mean of precision and recall)
 */
export function calculateF1Score(precision: number, recall: number): number {
  if (precision + recall === 0) return 0
  return (2 * precision * recall) / (precision + recall)
}

/**
 * Calculate pattern extraction success rate
 */
export function calculateExtractionSuccessRate(
  extractions: Array<{ triggered: boolean; valid: boolean }>,
): number {
  const triggered = extractions.filter((e) => e.triggered)
  if (triggered.length === 0) return 0

  const valid = triggered.filter((e) => e.valid).length
  return valid / triggered.length
}

/**
 * Calculate convergence time (number of uses to reach threshold)
 */
export function calculateConvergenceTime(
  confidenceHistory: number[],
  threshold: number,
): number {
  const convergenceIndex = confidenceHistory.findIndex((c) => c >= threshold)
  return convergenceIndex === -1 ? confidenceHistory.length : convergenceIndex + 1
}

/**
 * Calculate self-correction success rate
 */
export function calculateSelfCorrectionRate(
  corrections: Array<{ before: number; after: number }>,
): number {
  if (corrections.length === 0) return 0

  const improvements = corrections.filter((c) => c.after > c.before).length
  return improvements / corrections.length
}

/**
 * Calculate overall evolution cycle success rate
 */
export function calculateEvolutionSuccessRate(
  cycles: Array<{
    candidateCreated: boolean
    verificationPassed: boolean
    promoted: boolean
  }>,
): number {
  if (cycles.length === 0) return 0

  const successful = cycles.filter(
    (c) => c.candidateCreated && c.verificationPassed && c.promoted,
  ).length

  return successful / cycles.length
}

/**
 * Calculate asymmetric learning ratio
 * Returns the ratio of failure impact to success impact
 */
export function calculateAsymmetricLearningRatio(
  initialConfidence: number,
  afterSuccess: number,
  afterFailure: number,
): number {
  const successDelta = afterSuccess - initialConfidence
  const failureDelta = initialConfidence - afterFailure

  if (successDelta === 0) return failureDelta > 0 ? Infinity : 0
  return failureDelta / successDelta
}

/**
 * Aggregate multiple metric results into a summary
 */
export function aggregateMetrics(
  dimension: string,
  metrics: MetricResult[],
): EvaluationSummary {
  const passedCount = metrics.filter((m) => m.passed).length
  const passRate = metrics.length > 0 ? passedCount / metrics.length : 0

  // Calculate overall score as weighted average of (value/target)
  const totalWeight = metrics.length
  const weightedSum = metrics.reduce((sum, m) => {
    const normalized = m.target > 0 ? Math.min(m.value / m.target, 1) : m.passed ? 1 : 0
    return sum + normalized
  }, 0)

  const overallScore = totalWeight > 0 ? weightedSum / totalWeight : 0

  return {
    dimension,
    metrics,
    overallScore,
    passRate,
  }
}

/**
 * Create a metric result
 */
export function createMetricResult(
  name: string,
  value: number,
  target: number,
  comparator: "gte" | "lte" | "eq" = "gte",
  details?: string,
): MetricResult {
  let passed: boolean
  switch (comparator) {
    case "gte":
      passed = value >= target
      break
    case "lte":
      passed = value <= target
      break
    case "eq":
      passed = Math.abs(value - target) < 0.001
      break
  }

  return { name, value, target, passed, details }
}

/**
 * Statistics helper functions
 */
export namespace Statistics {
  export function mean(values: number[]): number {
    if (values.length === 0) return 0
    return values.reduce((a, b) => a + b, 0) / values.length
  }

  export function variance(values: number[]): number {
    if (values.length < 2) return 0
    const avg = mean(values)
    return values.reduce((sum, v) => sum + (v - avg) ** 2, 0) / (values.length - 1)
  }

  export function stdDev(values: number[]): number {
    return Math.sqrt(variance(values))
  }

  export function median(values: number[]): number {
    if (values.length === 0) return 0
    const sorted = [...values].sort((a, b) => a - b)
    const mid = Math.floor(sorted.length / 2)
    return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2
  }

  export function percentile(values: number[], p: number): number {
    if (values.length === 0) return 0
    const sorted = [...values].sort((a, b) => a - b)
    const index = (p / 100) * (sorted.length - 1)
    const lower = Math.floor(index)
    const upper = Math.ceil(index)
    if (lower === upper) return sorted[lower]
    return sorted[lower] * (upper - index) + sorted[upper] * (index - lower)
  }
}
