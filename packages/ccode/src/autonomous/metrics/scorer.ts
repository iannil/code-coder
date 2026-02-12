import { Log } from "@/util/log"
import type { SessionMetrics } from "./metrics"

const log = Log.create({ service: "autonomous.scorer" })

/**
 * Craziness level categories
 */
export type CrazinessLevel = "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"

/**
 * Quality score breakdown
 */
export interface QualityScoreBreakdown {
  overall: number
  testCoverage: number
  codeQuality: number
  decisionQuality: number
  efficiency: number
  safety: number
}

/**
 * Craziness score breakdown
 */
export interface CrazinessScoreBreakdown {
  overall: number
  level: CrazinessLevel
  autonomy: number
  selfCorrection: number
  speed: number
  riskTaking: number
}

/**
 * Scoring weights
 */
export interface ScoringWeights {
  quality: {
    testCoverage: number
    codeQuality: number
    decisionQuality: number
    efficiency: number
    safety: number
  }
  craziness: {
    autonomy: number
    selfCorrection: number
    speed: number
    riskTaking: number
  }
}

/**
 * Default scoring weights
 */
const DEFAULT_WEIGHTS: ScoringWeights = {
  quality: {
    testCoverage: 0.25,
    codeQuality: 0.25,
    decisionQuality: 0.2,
    efficiency: 0.15,
    safety: 0.15,
  },
  craziness: {
    autonomy: 0.35,
    selfCorrection: 0.25,
    speed: 0.2,
    riskTaking: 0.2,
  },
}

/**
 * Score thresholds for craziness levels
 */
const CRAZINESS_THRESHOLDS: Record<CrazinessLevel, { min: number; max: number }> = {
  lunatic: { min: 90, max: 100 },
  insane: { min: 75, max: 89 },
  crazy: { min: 60, max: 74 },
  wild: { min: 40, max: 59 },
  bold: { min: 20, max: 39 },
  timid: { min: 0, max: 19 },
}

/**
 * Scorer for calculating quality and craziness scores
 *
 * Analyzes session metrics to produce meaningful scores
 */
export class Scorer {
  private weights: ScoringWeights

  constructor(weights?: Partial<ScoringWeights>) {
    this.weights = {
      quality: { ...DEFAULT_WEIGHTS.quality, ...weights?.quality },
      craziness: { ...DEFAULT_WEIGHTS.craziness, ...weights?.craziness },
    }
  }

  /**
   * Calculate quality score from session metrics
   */
  calculateQuality(metrics: SessionMetrics): QualityScoreBreakdown {
    // Test coverage score
    const testCoverage = this.calculateTestCoverageScore(metrics)

    // Code quality score
    const codeQuality = this.calculateCodeQualityScore(metrics)

    // Decision quality score
    const decisionQuality = this.calculateDecisionQualityScore(metrics)

    // Efficiency score
    const efficiency = this.calculateEfficiencyScore(metrics)

    // Safety score
    const safety = this.calculateSafetyScore(metrics)

    // Overall quality score (weighted)
    const overall =
      testCoverage * this.weights.quality.testCoverage +
      codeQuality * this.weights.quality.codeQuality +
      decisionQuality * this.weights.quality.decisionQuality +
      efficiency * this.weights.quality.efficiency +
      safety * this.weights.quality.safety

    return {
      overall: Math.round(overall),
      testCoverage: Math.round(testCoverage),
      codeQuality: Math.round(codeQuality),
      decisionQuality: Math.round(decisionQuality),
      efficiency: Math.round(efficiency),
      safety: Math.round(safety),
    }
  }

  /**
   * Calculate craziness score from session metrics
   */
  calculateCraziness(metrics: SessionMetrics): CrazinessScoreBreakdown {
    // Autonomy score
    const autonomy = this.calculateAutonomyScore(metrics)

    // Self-correction score
    const selfCorrection = this.calculateSelfCorrectionScore(metrics)

    // Speed score
    const speed = this.calculateSpeedScore(metrics)

    // Risk-taking score
    const riskTaking = this.calculateRiskTakingScore(metrics)

    // Overall craziness score (weighted)
    const overall =
      autonomy * this.weights.craziness.autonomy +
      selfCorrection * this.weights.craziness.selfCorrection +
      speed * this.weights.craziness.speed +
      riskTaking * this.weights.craziness.riskTaking

    // Determine level
    const level = this.getCrazinessLevel(overall)

    return {
      overall: Math.round(overall),
      level,
      autonomy: Math.round(autonomy),
      selfCorrection: Math.round(selfCorrection),
      speed: Math.round(speed),
      riskTaking: Math.round(riskTaking),
    }
  }

  /**
   * Calculate test coverage score
   */
  private calculateTestCoverageScore(metrics: SessionMetrics): number {
    const { tests, tdd } = metrics

    // Pass rate (40 points max)
    const passRateScore = tests.passRate * 40

    // TDD cycle completion (30 points max)
    const tddScore =
      tdd.cycles > 0
        ? (tdd.redPassed / tdd.cycles) * 10 +
          (tdd.greenPassed / tdd.cycles) * 10 +
          (tdd.refactorPassed / tdd.cycles) * 10
        : 0

    // Test count (30 points max) - running tests is good
    const testCountScore = Math.min(30, tests.run * 2)

    return Math.min(100, passRateScore + tddScore + testCountScore)
  }

  /**
   * Calculate code quality score
   */
  private calculateCodeQualityScore(metrics: SessionMetrics): number {
    // Base score on task completion rate
    const { tasks } = metrics
    const completionRate = tasks.total > 0 ? tasks.completed / tasks.total : 0

    // High completion rate = good code quality
    return completionRate * 100
  }

  /**
   * Calculate decision quality score
   */
  private calculateDecisionQualityScore(metrics: SessionMetrics): number {
    const { decisions } = metrics

    if (decisions.total === 0) {
      return 50 // Neutral score if no decisions
    }

    // High approval rate = good decision quality
    const approvalRate = decisions.approved / decisions.total

    // High average score = good decision quality
    const scoreFactor = decisions.averageScore / 10

    return (approvalRate * 0.6 + scoreFactor * 0.4) * 100
  }

  /**
   * Calculate efficiency score
   */
  private calculateEfficiencyScore(metrics: SessionMetrics): number {
    const { tasks, resources, duration } = metrics

    // Efficiency = tasks completed per minute
    const tasksPerMinute = duration > 0 ? tasks.completed / (duration / 60000) : 0

    // Score: 0.5 tasks/min = 50 points, 2 tasks/min = 100 points
    const taskScore = Math.min(100, tasksPerMinute * 50)

    // Resource efficiency (tokens per task)
    const tokensPerTask = tasks.completed > 0 ? resources.tokensUsed / tasks.completed : 0
    const resourceScore = Math.max(0, 100 - tokensPerTask / 1000)

    return taskScore * 0.7 + resourceScore * 0.3
  }

  /**
   * Calculate safety score
   */
  private calculateSafetyScore(metrics: SessionMetrics): number {
    const { safety, tasks } = metrics

    // Start with 100, deduct for issues
    let score = 100

    // Rollbacks reduce score (10 points each)
    score -= safety.rollbacks * 10

    // Loops detected reduce score (15 points each)
    score -= safety.loopsDetected * 15

    // Warnings reduce score (5 points each)
    score -= safety.warnings * 5

    // Failed tasks reduce score (5 points each)
    score -= tasks.failed * 5

    return Math.max(0, score)
  }

  /**
   * Calculate autonomy score
   */
  private calculateAutonomyScore(metrics: SessionMetrics): number {
    const { decisions } = metrics

    if (decisions.total === 0) {
      return 50 // Neutral
    }

    // High autonomy = few pauses/blocks
    const interventionRate = (decisions.paused + decisions.blocked) / decisions.total
    return (1 - interventionRate) * 100
  }

  /**
   * Calculate self-correction score
   */
  private calculateSelfCorrectionScore(metrics: SessionMetrics): number {
    const { safety, tasks } = metrics

    // Self-correction = detecting and fixing issues
    // Rollbacks indicate self-correction in action
    const rollbackScore = Math.min(50, safety.rollbacks * 15)

    // Fixed tasks (failed then recovered)
    // This would be tracked separately in a full implementation

    return rollbackScore + 50 // Base 50 + rollback bonus
  }

  /**
   * Calculate speed score
   */
  private calculateSpeedScore(metrics: SessionMetrics): number {
    const { tasks, duration } = metrics

    if (duration === 0 || tasks.completed === 0) {
      return 50
    }

    // Speed = tasks completed per minute
    const tasksPerMinute = tasks.completed / (duration / 60000)

    // 0.1 tasks/min = 10 points, 1+ tasks/min = 100 points
    return Math.min(100, tasksPerMinute * 100)
  }

  /**
   * Calculate risk-taking score
   */
  private calculateRiskTakingScore(metrics: SessionMetrics): number {
    const { decisions } = metrics

    if (decisions.total === 0) {
      return 50
    }

    // Risk-taking = making decisions without pausing
    // High average CLOSE score with low pause rate = good risk-taking
    const scoreFactor = decisions.averageScore / 10
    const approvalRate = decisions.approved / decisions.total

    return (scoreFactor * 0.5 + approvalRate * 0.5) * 100
  }

  /**
   * Get craziness level from score
   */
  private getCrazinessLevel(score: number): CrazinessLevel {
    for (const [level, thresholds] of Object.entries(CRAZINESS_THRESHOLDS)) {
      if (score >= thresholds.min && score <= thresholds.max) {
        return level as CrazinessLevel
      }
    }
    return "timid"
  }

  /**
   * Get score description
   */
  getLevelDescription(level: CrazinessLevel): string {
    const descriptions: Record<CrazinessLevel, string> = {
      lunatic: "Completely autonomous,疯狂到令人担忧 - operates without any human intervention",
      insane: "Highly autonomous, almost不需要干预 - handles most complex tasks independently",
      crazy: "Significantly autonomous,偶尔需要帮助 - can work independently with periodic check-ins",
      wild: "Partially autonomous,需要定期确认 - requires regular human guidance",
      bold: "Cautiously autonomous,频繁暂停 - asks for permission often",
      timid: "Barely autonomous,几乎无法自主 - requires constant supervision",
    }
    return descriptions[level]
  }

  /**
   * Update weights
   */
  updateWeights(weights: Partial<ScoringWeights>): void {
    if (weights.quality) {
      this.weights.quality = { ...this.weights.quality, ...weights.quality }
    }
    if (weights.craziness) {
      this.weights.craziness = { ...this.weights.craziness, ...weights.craziness }
    }
  }

  /**
   * Get current weights
   */
  getWeights(): ScoringWeights {
    return {
      quality: { ...this.weights.quality },
      craziness: { ...this.weights.craziness },
    }
  }
}

/**
 * Create a scorer
 */
export function createScorer(weights?: Partial<ScoringWeights>): Scorer {
  return new Scorer(weights)
}

/**
 * Quick score calculation (convenience function)
 */
export function calculateScores(metrics: SessionMetrics): {
  quality: QualityScoreBreakdown
  craziness: CrazinessScoreBreakdown
} {
  const scorer = new Scorer()
  return {
    quality: scorer.calculateQuality(metrics),
    craziness: scorer.calculateCraziness(metrics),
  }
}
