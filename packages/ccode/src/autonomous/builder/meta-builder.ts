/**
 * Meta Builder
 *
 * Orchestrates the complete concept build flow:
 * Gap Detection → CLOSE Evaluation → Generation → Validation → Approval → Registration
 *
 * This is the "meta-capability" that enables CodeCoder to extend itself.
 *
 * @package autonomous/builder
 */

import { Log } from "@/util/log"
import { nanoid } from "nanoid"

import {
  type ConceptType,
  type GapDetectionResult,
  type BuildRequest,
  type BuildResult,
  type BuildContext,
  type BuildConstraints,
  type BuildPhaseResult,
  type GeneratorInput,
  type GeneratedConcept,
  CONCEPT_METADATA,
  AUTONOMY_CONCEPT_GATES,
  isConceptAllowed,
  createSelfBuildingCriteria,
} from "./types"
import { getConceptInventory } from "./concept-inventory"
import { getGapDetector, type GapDetector, type TaskFailure } from "./gap-detector"
import { generateConcept } from "./generators"
import { validateConcept } from "./validation"
import { registerConcept } from "./registration"
import { DecisionEngine, createDecisionEngine, type DecisionResult } from "../decision/engine"
import { buildCriteria, type CLOSEScore } from "../decision/criteria"
import type { AutonomyLevel } from "../decision/engine"

const log = Log.create({ service: "autonomous.builder.meta-builder" })

// ============================================================================
// Configuration
// ============================================================================

export interface MetaBuilderConfig {
  /** Default autonomy level */
  autonomyLevel: AutonomyLevel
  /** Minimum CLOSE score to proceed */
  closeThreshold: number
  /** Enable auto-approval for low-risk concepts */
  enableAutoApproval: boolean
  /** Callback for requesting human approval */
  onApprovalRequest?: (concept: GeneratedConcept, closeScore: CLOSEScore) => Promise<boolean>
  /** Maximum build attempts */
  maxBuildAttempts: number
  /** Enable dry-run mode (don't actually register) */
  dryRun: boolean
}

const DEFAULT_CONFIG: MetaBuilderConfig = {
  autonomyLevel: "crazy",
  closeThreshold: 5.5,
  enableAutoApproval: true,
  maxBuildAttempts: 3,
  dryRun: false,
}

// ============================================================================
// Meta Builder
// ============================================================================

export class MetaBuilder {
  private config: MetaBuilderConfig
  private gapDetector: GapDetector
  private decisionEngine: DecisionEngine | null = null
  private buildHistory: Map<string, BuildResult> = new Map()

  constructor(config: Partial<MetaBuilderConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.gapDetector = getGapDetector()
  }

  /**
   * Initialize the meta builder
   */
  async initialize(): Promise<void> {
    await this.gapDetector.initialize()
    this.decisionEngine = createDecisionEngine({
      autonomyLevel: this.config.autonomyLevel,
    })
  }

  /**
   * Build a concept from a detected gap
   */
  async build(request: BuildRequest): Promise<BuildResult> {
    const startTime = Date.now()
    const phases: BuildPhaseResult[] = []

    log.info("Starting concept build", {
      gapId: request.gap.id,
      type: request.gap.type,
      description: request.gap.description.slice(0, 100),
    })

    // Phase 1: Evaluation
    const evaluationPhase = await this.runPhase("evaluation", async () => {
      return this.evaluateBuildDecision(request)
    })
    phases.push(evaluationPhase)

    if (!evaluationPhase.output || !(evaluationPhase.output as DecisionResult).approved) {
      return this.buildFailureResult(
        request.gap,
        phases,
        Date.now() - startTime,
        "Build declined by CLOSE evaluation",
        false,
      )
    }

    const decision = evaluationPhase.output as DecisionResult

    // Phase 2: Generation
    const generationPhase = await this.runPhase("generation", async () => {
      return this.generateConcept(request)
    })
    phases.push(generationPhase)

    if (generationPhase.status === "failed" || !generationPhase.output) {
      return this.buildFailureResult(
        request.gap,
        phases,
        Date.now() - startTime,
        generationPhase.error ?? "Generation failed",
        false,
      )
    }

    const concept = generationPhase.output as GeneratedConcept

    // Phase 3: Validation
    const validationPhase = await this.runPhase("validation", async () => {
      return validateConcept(concept)
    })
    phases.push(validationPhase)

    const validationResult = validationPhase.output as { success: boolean; errors?: Array<{ message: string }> }
    if (!validationResult?.success) {
      return this.buildFailureResult(
        request.gap,
        phases,
        Date.now() - startTime,
        `Validation failed: ${validationResult?.errors?.map((e) => e.message).join(", ")}`,
        false,
      )
    }

    // Phase 4: Approval
    const approvalRequired = this.isApprovalRequired(request.gap.type)
    let approvalGranted = !approvalRequired

    if (approvalRequired) {
      const approvalPhase = await this.runPhase("approval", async () => {
        return this.requestApproval(concept, decision.score)
      })
      phases.push(approvalPhase)
      approvalGranted = approvalPhase.output as boolean
    } else {
      phases.push({
        phase: "approval",
        status: "skipped",
        durationMs: 0,
        output: true,
      })
    }

    if (!approvalGranted) {
      return this.buildFailureResult(
        request.gap,
        phases,
        Date.now() - startTime,
        "Build declined - approval not granted",
        true,
        approvalRequired,
      )
    }

    // Phase 5: Registration
    if (this.config.dryRun) {
      phases.push({
        phase: "registration",
        status: "skipped",
        durationMs: 0,
        output: { dryRun: true },
      })

      return this.buildSuccessResult(
        concept,
        request.gap,
        decision.score,
        phases,
        Date.now() - startTime,
        "Build completed (dry run - not registered)",
        approvalRequired,
        approvalGranted,
      )
    }

    const registrationPhase = await this.runPhase("registration", async () => {
      return registerConcept(concept)
    })
    phases.push(registrationPhase)

    const regResult = registrationPhase.output as { success: boolean; error?: string }
    if (!regResult?.success) {
      return this.buildFailureResult(
        request.gap,
        phases,
        Date.now() - startTime,
        regResult?.error ?? "Registration failed",
        approvalRequired,
        approvalGranted,
      )
    }

    // Success!
    const result = this.buildSuccessResult(
      concept,
      request.gap,
      decision.score,
      phases,
      Date.now() - startTime,
      `Successfully built ${request.gap.type}: ${concept.identifier}`,
      approvalRequired,
      approvalGranted,
      regResult,
    )

    // Store in history
    this.buildHistory.set(request.gap.id, result)

    // Clear the resolved gap
    this.gapDetector.clearGap(request.gap.id)

    log.info("Build completed successfully", {
      gapId: request.gap.id,
      conceptId: concept.identifier,
      type: concept.type,
      durationMs: result.durationMs,
    })

    return result
  }

  /**
   * Convenience method: detect gap from failure and build if appropriate
   */
  async buildFromFailure(failure: TaskFailure, context: Partial<BuildContext> = {}): Promise<BuildResult | null> {
    // Detect gap
    const gap = await this.gapDetector.detectFromFailure(failure)

    if (!gap) {
      log.debug("No buildable gap detected from failure")
      return null
    }

    // Check autonomy gate
    if (!isConceptAllowed(gap.type, this.config.autonomyLevel)) {
      log.info("Gap type not allowed at current autonomy level", {
        type: gap.type,
        autonomyLevel: this.config.autonomyLevel,
      })
      return null
    }

    // Build context
    const buildContext: BuildContext = {
      sessionId: failure.sessionId,
      workingDir: context.workingDir,
      triggeredBy: "system",
      taskDescription: failure.description,
      errorMessage: failure.errorMessage,
      technology: failure.technology,
      ...context,
    }

    // Build
    return this.build({
      gap,
      context: buildContext,
      constraints: {
        autonomyLevel: this.config.autonomyLevel,
      },
    })
  }

  /**
   * Convenience method: detect gap from query and build if appropriate
   */
  async buildFromQuery(
    query: string,
    context: Partial<BuildContext> = {},
  ): Promise<BuildResult | null> {
    // Detect gap
    const gap = await this.gapDetector.detectFromQuery(query, {
      sessionId: context.sessionId,
      technology: context.technology,
      isUserRequest: context.triggeredBy === "user",
    })

    if (!gap) {
      log.debug("No buildable gap detected from query")
      return null
    }

    // Check autonomy gate
    if (!isConceptAllowed(gap.type, this.config.autonomyLevel)) {
      log.info("Gap type not allowed at current autonomy level", {
        type: gap.type,
        autonomyLevel: this.config.autonomyLevel,
      })
      return null
    }

    // Build context
    const buildContext: BuildContext = {
      sessionId: context.sessionId ?? `session_${nanoid(8)}`,
      triggeredBy: context.triggeredBy ?? "user",
      taskDescription: query,
      ...context,
    }

    return this.build({
      gap,
      context: buildContext,
      constraints: {
        autonomyLevel: this.config.autonomyLevel,
      },
    })
  }

  /**
   * Get build history
   */
  getBuildHistory(): BuildResult[] {
    return Array.from(this.buildHistory.values())
  }

  /**
   * Get a specific build result
   */
  getBuildResult(gapId: string): BuildResult | null {
    return this.buildHistory.get(gapId) ?? null
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async runPhase<T>(
    phase: BuildPhaseResult["phase"],
    fn: () => Promise<T>,
  ): Promise<BuildPhaseResult> {
    const startTime = Date.now()

    try {
      const output = await fn()
      return {
        phase,
        status: "completed",
        durationMs: Date.now() - startTime,
        output,
      }
    } catch (error) {
      log.error("Build phase failed", { phase, error })
      return {
        phase,
        status: "failed",
        durationMs: Date.now() - startTime,
        error: String(error),
      }
    }
  }

  private async evaluateBuildDecision(request: BuildRequest): Promise<DecisionResult> {
    if (!this.decisionEngine) {
      this.decisionEngine = createDecisionEngine({
        autonomyLevel: this.config.autonomyLevel,
      })
    }

    // Build CLOSE criteria
    const criteria = buildCriteria(createSelfBuildingCriteria(request.gap, request.context))

    // Build decision context
    const context = {
      sessionId: request.context.sessionId,
      currentState: "building",
      errorCount: 0,
      recentDecisions: [],
    }

    // Evaluate
    return this.decisionEngine.evaluate(criteria, context)
  }

  private async generateConcept(request: BuildRequest): Promise<GeneratedConcept> {
    const inventory = getConceptInventory()
    const allConcepts = await inventory.all()

    const input: GeneratorInput = {
      gap: request.gap,
      context: request.context,
      existingConcepts: allConcepts.map((c) => c.identifier),
      hints: request.constraints as Record<string, unknown>,
    }

    return generateConcept(request.gap.type, input)
  }

  private isApprovalRequired(type: ConceptType): boolean {
    // Check concept metadata
    const meta = CONCEPT_METADATA[type]
    if (meta.requiresApproval) {
      return true
    }

    // Check auto-approval setting
    if (!this.config.enableAutoApproval) {
      return true
    }

    return !meta.autoApprovable
  }

  private async requestApproval(concept: GeneratedConcept, closeScore: CLOSEScore): Promise<boolean> {
    // If callback provided, use it
    if (this.config.onApprovalRequest) {
      return this.config.onApprovalRequest(concept, closeScore)
    }

    // Auto-approve if allowed
    const meta = CONCEPT_METADATA[concept.type]
    if (this.config.enableAutoApproval && meta.autoApprovable) {
      log.info("Auto-approved concept", {
        identifier: concept.identifier,
        type: concept.type,
        closeScore: closeScore.total,
      })
      return true
    }

    // Default: deny if no approval mechanism
    log.warn("No approval mechanism available, denying", {
      identifier: concept.identifier,
      type: concept.type,
    })
    return false
  }

  private buildSuccessResult(
    concept: GeneratedConcept,
    gap: GapDetectionResult,
    closeScore: CLOSEScore,
    phases: BuildPhaseResult[],
    durationMs: number,
    summary: string,
    approvalRequired: boolean,
    approvalGranted?: boolean,
    registration?: unknown,
  ): BuildResult {
    return {
      success: true,
      concept,
      gap,
      closeScore,
      phases,
      durationMs,
      summary,
      approvalRequired,
      approvalGranted,
      registration: registration as BuildResult["registration"],
    }
  }

  private buildFailureResult(
    gap: GapDetectionResult,
    phases: BuildPhaseResult[],
    durationMs: number,
    summary: string,
    approvalRequired: boolean,
    approvalGranted?: boolean,
  ): BuildResult {
    return {
      success: false,
      gap,
      closeScore: gap.closeScore,
      phases,
      durationMs,
      summary,
      approvalRequired,
      approvalGranted,
    }
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

let metaBuilderInstance: MetaBuilder | null = null

/**
 * Get the global meta builder instance
 */
export function getMetaBuilder(): MetaBuilder {
  if (!metaBuilderInstance) {
    metaBuilderInstance = new MetaBuilder()
  }
  return metaBuilderInstance
}

/**
 * Create a new meta builder instance
 */
export function createMetaBuilder(config?: Partial<MetaBuilderConfig>): MetaBuilder {
  return new MetaBuilder(config)
}
