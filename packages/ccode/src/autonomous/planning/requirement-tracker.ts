import { Log } from "@/util/log"

const log = Log.create({ service: "autonomous.requirement-tracker" })

/**
 * Requirement status
 */
export type RequirementStatus = "pending" | "in_progress" | "completed" | "blocked"

/**
 * Requirement acceptance criterion
 */
export interface AcceptanceCriterion {
  id: string
  description: string
  status: "pending" | "passed" | "failed"
  verifiedAt?: number
}

/**
 * Requirement extracted from user request
 */
export interface Requirement {
  id: string
  description: string
  status: RequirementStatus
  priority: "critical" | "high" | "medium" | "low"
  acceptanceCriteria: AcceptanceCriterion[]
  createdAt: number
  updatedAt: number
  dependencies: string[] // IDs of requirements this depends on
  estimatedCycles?: number
  source?: "original" | "derived" // original = from user request, derived = discovered during execution
}

/**
 * Parse result for requirements
 */
export interface ParseResult {
  requirements: Requirement[]
  implicitRequirements: string[]
  clarificationsNeeded: string[]
}

/**
 * Requirement tracking configuration
 */
export interface RequirementTrackerConfig {
  enableImplicitDetection: boolean
  autoDeriveRequirements: boolean
}

/**
 * Requirement tracker for parsing and tracking requirements
 *
 * Parses user requests into structured requirements and tracks their completion
 */
export class RequirementTracker {
  private requirements: Map<string, Requirement> = new Map()
  private config: RequirementTrackerConfig
  private sessionId: string

  constructor(sessionId: string, config: Partial<RequirementTrackerConfig> = {}) {
    this.sessionId = sessionId
    this.config = {
      enableImplicitDetection: true,
      autoDeriveRequirements: true,
      ...config,
    }
  }

  /**
   * Parse requirements from user request
   */
  parseRequirements(request: string): ParseResult {
    log.info("Parsing requirements from request", { sessionId: this.sessionId })

    const requirements: Requirement[] = []
    const implicitRequirements: string[] = []
    const clarificationsNeeded: string[] = []

    // Extract explicit requirements using pattern matching
    const patterns = this.getRequirementPatterns()

    for (const pattern of patterns) {
      const matches = request.matchAll(pattern.regex)
      for (const match of matches) {
        const description = match.groups?.description || match[1]
        if (description) {
          const requirement: Requirement = {
            id: this.generateId(),
            description: description.trim(),
            status: "pending",
            priority: pattern.priority,
            acceptanceCriteria: this.generateDefaultCriteria(description),
            createdAt: Date.now(),
            updatedAt: Date.now(),
            dependencies: [],
            source: "original",
          }
          requirements.push(requirement)
        }
      }
    }

    // If no explicit requirements found, treat the entire request as one requirement
    if (requirements.length === 0) {
      const mainRequirement: Requirement = {
        id: this.generateId(),
        description: request.trim(),
        status: "pending",
        priority: "high",
        acceptanceCriteria: this.generateDefaultCriteria(request),
        createdAt: Date.now(),
        updatedAt: Date.now(),
        dependencies: [],
        source: "original",
      }
      requirements.push(mainRequirement)
    }

    // Detect implicit requirements
    if (this.config.enableImplicitDetection) {
      const implicit = this.detectImplicitRequirements(request)
      implicitRequirements.push(...implicit)
    }

    // Store requirements
    for (const req of requirements) {
      this.requirements.set(req.id, req)
    }

    log.info("Requirements parsed", {
      sessionId: this.sessionId,
      count: requirements.length,
      implicit: implicitRequirements.length,
    })

    return { requirements, implicitRequirements, clarificationsNeeded }
  }

  /**
   * Update requirement status
   */
  updateStatus(requirementId: string, status: RequirementStatus): void {
    const requirement = this.requirements.get(requirementId)
    if (!requirement) {
      log.warn("Requirement not found", { requirementId })
      return
    }

    requirement.status = status
    requirement.updatedAt = Date.now()

    log.info("Requirement status updated", {
      sessionId: this.sessionId,
      requirementId,
      status,
    })

    // If completed, mark all acceptance criteria as passed
    if (status === "completed") {
      for (const criterion of requirement.acceptanceCriteria) {
        if (criterion.status === "pending") {
          criterion.status = "passed"
          criterion.verifiedAt = Date.now()
        }
      }
    }
  }

  /**
   * Update acceptance criterion status
   */
  updateCriterionStatus(requirementId: string, criterionId: string, status: "passed" | "failed"): void {
    const requirement = this.requirements.get(requirementId)
    if (!requirement) {
      return
    }

    const criterion = requirement.acceptanceCriteria.find((c) => c.id === criterionId)
    if (!criterion) {
      return
    }

    criterion.status = status
    criterion.verifiedAt = Date.now()
    requirement.updatedAt = Date.now()

    // Update requirement status based on criteria
    this.updateRequirementStatusFromCriteria(requirement)
  }

  /**
   * Add a derived requirement discovered during execution
   */
  addDerivedRequirement(description: string, priority: Requirement["priority"] = "medium"): string {
    const requirement: Requirement = {
      id: this.generateId(),
      description,
      status: "pending",
      priority,
      acceptanceCriteria: this.generateDefaultCriteria(description),
      createdAt: Date.now(),
      updatedAt: Date.now(),
      dependencies: [],
      source: "derived",
    }

    this.requirements.set(requirement.id, requirement)

    log.info("Derived requirement added", {
      sessionId: this.sessionId,
      requirementId: requirement.id,
      description,
    })

    return requirement.id
  }

  /**
   * Check if all requirements are completed
   */
  allRequirementsCompleted(): boolean {
    const pending = this.getPendingRequirements()
    return pending.length === 0
  }

  /**
   * Get pending requirements
   */
  getPendingRequirements(): Requirement[] {
    return Array.from(this.requirements.values()).filter(
      (r) => r.status !== "completed" || !this.allCriteriaPassed(r),
    )
  }

  /**
   * Get requirement by ID
   */
  getRequirement(id: string): Requirement | undefined {
    return this.requirements.get(id)
  }

  /**
   * Get all requirements
   */
  getAllRequirements(): Requirement[] {
    return Array.from(this.requirements.values())
  }

  /**
   * Get requirements by status
   */
  getRequirementsByStatus(status: RequirementStatus): Requirement[] {
    return Array.from(this.requirements.values()).filter((r) => r.status === status)
  }

  /**
   * Get completion percentage
   */
  getCompletionPercentage(): number {
    const total = this.requirements.size
    if (total === 0) return 100

    const completed = Array.from(this.requirements.values()).filter(
      (r) => r.status === "completed" && this.allCriteriaPassed(r),
    ).length

    return Math.round((completed / total) * 100)
  }

  /**
   * Get statistics
   */
  getStats(): {
    total: number
    completed: number
    inProgress: number
    pending: number
    blocked: number
    completionPercentage: number
  } {
    const requirements = Array.from(this.requirements.values())
    const completed = requirements.filter((r) => r.status === "completed").length
    const inProgress = requirements.filter((r) => r.status === "in_progress").length
    const pending = requirements.filter((r) => r.status === "pending").length
    const blocked = requirements.filter((r) => r.status === "blocked").length

    return {
      total: requirements.length,
      completed,
      inProgress,
      pending,
      blocked,
      completionPercentage: this.getCompletionPercentage(),
    }
  }

  /**
   * Serialize tracker state
   */
  serialize(): {
    requirements: Requirement[]
    stats: ReturnType<RequirementTracker["getStats"]>
  } {
    return {
      requirements: Array.from(this.requirements.values()),
      stats: this.getStats(),
    }
  }

  /**
   * Generate a unique ID
   */
  private generateId(): string {
    return `req-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
  }

  /**
   * Generate default acceptance criteria for a requirement
   */
  private generateDefaultCriteria(description: string): AcceptanceCriterion[] {
    return [
      {
        id: this.generateId(),
        description: `Implementation matches: "${description.slice(0, 100)}${description.length > 100 ? "..." : ""}"`,
        status: "pending",
      },
      {
        id: this.generateId(),
        description: "Code follows project style guidelines",
        status: "pending",
      },
      {
        id: this.generateId(),
        description: "Tests cover the implemented functionality",
        status: "pending",
      },
    ]
  }

  /**
   * Detect implicit requirements from the request
   */
  private detectImplicitRequirements(request: string): string[] {
    const implicit: string[] = []
    const lowerRequest = request.toLowerCase()

    // Test coverage requirement
    if (lowerRequest.includes("test") || lowerRequest.includes("function")) {
      implicit.push("Ensure test coverage meets 80% threshold")
    }

    // Error handling requirement
    if (lowerRequest.includes("api") || lowerRequest.includes("endpoint")) {
      implicit.push("Implement proper error handling and validation")
    }

    // Documentation requirement
    if (lowerRequest.includes("feature") || lowerRequest.includes("implement")) {
      implicit.push("Update relevant documentation")
    }

    return implicit
  }

  /**
   * Update requirement status based on acceptance criteria
   */
  private updateRequirementStatusFromCriteria(requirement: Requirement): void {
    if (this.allCriteriaPassed(requirement)) {
      requirement.status = "completed"
    } else if (this.hasFailedCriteria(requirement)) {
      requirement.status = "blocked"
    } else if (this.hasInProgressCriteria(requirement)) {
      requirement.status = "in_progress"
    }
  }

  /**
   * Check if all acceptance criteria have passed
   */
  private allCriteriaPassed(requirement: Requirement): boolean {
    return requirement.acceptanceCriteria.every((c) => c.status === "passed")
  }

  /**
   * Check if any acceptance criteria have failed
   */
  private hasFailedCriteria(requirement: Requirement): boolean {
    return requirement.acceptanceCriteria.some((c) => c.status === "failed")
  }

  /**
   * Check if any acceptance criteria are in progress
   */
  private hasInProgressCriteria(requirement: Requirement): boolean {
    return requirement.acceptanceCriteria.some((c) => c.status === "pending")
  }

  /**
   * Get requirement patterns for parsing
   */
  private getRequirementPatterns(): Array<{
    regex: RegExp
    priority: Requirement["priority"]
  }> {
    return [
      // Critical requirements
      {
        regex: /(?:must|shall|require(?:ment)?|critical(?:ally)?)[:\s]+(?:to\s+)?(?:implement|build|create|add)?\s*(?<description>[^.!?]+)/gi,
        priority: "critical",
      },
      // High priority
      {
        regex: /(?:should|high\s+priority)[:\s]+(?:to\s+)?(?:implement|build|create|add)?\s*(?<description>[^.!?]+)/gi,
        priority: "high",
      },
      // Medium priority
      {
        regex: /(?:could|would\s+be\s+nice|medium\s+priority)[:\s]+(?:to\s+)?(?:implement|build|create|add)?\s*(?<description>[^.!?]+)/gi,
        priority: "medium",
      },
      // Low priority
      {
        regex: /(?:might|could\s+consider|low\s+priority|optional)[:\s]+(?:to\s+)?(?:implement|build|create|add)?\s*(?<description>[^.!?]+)/gi,
        priority: "low",
      },
    ]
  }
}

/**
 * Create a requirement tracker
 */
export function createRequirementTracker(
  sessionId: string,
  config?: Partial<RequirementTrackerConfig>,
): RequirementTracker {
  return new RequirementTracker(sessionId, config)
}
