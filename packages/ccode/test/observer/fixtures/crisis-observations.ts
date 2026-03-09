/**
 * Crisis Observation Fixtures
 *
 * Test data for simulating a production crisis response scenario.
 * Provides phased observations that escalate from normal → crisis → recovery.
 *
 * @module test/observer/fixtures/crisis-observations
 */

import type {
  Observation,
  CodeObservation,
  WorldObservation,
  SelfObservation,
  MetaObservation,
} from "@/observer/types"

/**
 * Phase definition for crisis simulation.
 */
export interface CrisisPhase {
  name: string
  description: string
  observations: Observation[]
}

/**
 * Generate unique observation ID.
 */
function genId(prefix: string): string {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
}

/**
 * Create Phase 1: Normal Operation observations.
 * System is healthy, all metrics within normal range.
 */
export function createNormalOperationObservations(): Observation[] {
  const timestamp = new Date()

  return [
    // Code watcher: healthy build
    {
      id: genId("obs_code"),
      timestamp,
      watcherId: "code_watch_1",
      watcherType: "code",
      confidence: 0.95,
      tags: ["ci", "build"],
      type: "build_status",
      source: "ci/github-actions",
      change: {
        action: "modify",
        before: { status: "passing" },
        after: { status: "passing" },
      },
      impact: {
        scope: "project",
        severity: "low",
        affectedFiles: [],
      },
    } as CodeObservation,

    // World watcher: stable API
    {
      id: genId("obs_world"),
      timestamp,
      watcherId: "world_watch_1",
      watcherType: "world",
      confidence: 0.9,
      tags: ["api", "monitoring"],
      type: "api_change",
      source: "api-monitor/external",
      data: {
        title: "External API Status",
        summary: "All external APIs responding normally",
        content: { latency: 120, status: "healthy" },
      },
      relevance: 0.8,
      sentiment: "positive",
    } as WorldObservation,

    // Self watcher: healthy agent
    {
      id: genId("obs_self"),
      timestamp,
      watcherId: "self_watch_1",
      watcherType: "self",
      confidence: 0.92,
      tags: ["agent", "health"],
      type: "agent_behavior",
      agentId: "build_agent",
      observation: {
        action: "code_review",
        success: true,
        duration: 1500,
      },
      quality: {
        closeScore: 8.2,
        accuracy: 0.95,
        efficiency: 0.88,
      },
    } as SelfObservation,

    // Meta watcher: system healthy
    {
      id: genId("obs_meta"),
      timestamp,
      watcherId: "meta_watch_1",
      watcherType: "meta",
      confidence: 0.95,
      tags: ["system", "health"],
      type: "system_health",
      assessment: {
        health: "healthy",
        coverage: 0.92,
        accuracy: 0.9,
        latency: 45,
      },
      recommendations: [],
      issues: [],
    } as MetaObservation,
  ]
}

/**
 * Create Phase 2: Crisis Emergence observations.
 * Early warning signs - API latency increase, build warnings.
 */
export function createCrisisEmergenceObservations(): Observation[] {
  const timestamp = new Date()

  return [
    // World watcher: API latency increase (warning sign)
    {
      id: genId("obs_world"),
      timestamp,
      watcherId: "world_watch_1",
      watcherType: "world",
      confidence: 0.85,
      tags: ["api", "latency", "warning"],
      type: "trend",
      source: "api-monitor/external",
      data: {
        title: "API Latency Increase",
        summary: "External API response time increased by 300%",
        content: {
          currentLatency: 2000,
          baselineLatency: 500,
          threshold: 1000,
          trend: "increasing",
        },
      },
      relevance: 0.9,
      sentiment: "negative",
    } as WorldObservation,

    // Code watcher: build warnings increasing
    {
      id: genId("obs_code"),
      timestamp,
      watcherId: "code_watch_1",
      watcherType: "code",
      confidence: 0.88,
      tags: ["ci", "warnings"],
      type: "build_status",
      source: "ci/github-actions",
      change: {
        action: "modify",
        before: { warnings: 3 },
        after: { warnings: 15 },
      },
      impact: {
        scope: "project",
        severity: "medium",
        affectedFiles: ["src/api/client.ts", "src/services/cache.ts"],
      },
    } as CodeObservation,

    // Self watcher: quality degrading
    {
      id: genId("obs_self"),
      timestamp,
      watcherId: "self_watch_1",
      watcherType: "self",
      confidence: 0.8,
      tags: ["quality", "degradation"],
      type: "quality_metric",
      agentId: "build_agent",
      observation: {
        action: "code_review",
        success: true,
        duration: 3500, // slower
      },
      quality: {
        closeScore: 6.0, // dropped
        accuracy: 0.82,
        efficiency: 0.65,
      },
    } as SelfObservation,
  ]
}

/**
 * Create Phase 3: Crisis Escalation observations.
 * Critical issues detected - error rates spiking, service degradation.
 */
export function createCrisisEscalationObservations(): Observation[] {
  const timestamp = new Date()

  return [
    // Self watcher: error rate spike (critical)
    {
      id: genId("obs_self"),
      timestamp,
      watcherId: "self_watch_1",
      watcherType: "self",
      confidence: 0.95,
      tags: ["error", "spike", "critical"],
      type: "error_pattern",
      agentId: "build_agent",
      observation: {
        action: "api_call",
        success: false,
        duration: 5000,
        error: "Timeout: External API unresponsive",
      },
      quality: {
        closeScore: 3.5, // critical drop
        accuracy: 0.5,
        efficiency: 0.3,
      },
    } as SelfObservation,

    // World watcher: external service down
    {
      id: genId("obs_world"),
      timestamp,
      watcherId: "world_watch_1",
      watcherType: "world",
      confidence: 0.98,
      tags: ["api", "outage", "critical"],
      type: "api_change",
      source: "api-monitor/external",
      data: {
        title: "External API Outage",
        summary: "Critical external dependency experiencing outage",
        content: {
          status: "down",
          errorRate: 0.85,
          lastSuccess: timestamp.getTime() - 300000,
        },
      },
      relevance: 1.0,
      sentiment: "negative",
    } as WorldObservation,

    // Code watcher: build failing
    {
      id: genId("obs_code"),
      timestamp,
      watcherId: "code_watch_1",
      watcherType: "code",
      confidence: 0.99,
      tags: ["ci", "failure", "critical"],
      type: "build_status",
      source: "ci/github-actions",
      change: {
        action: "modify",
        before: { status: "passing" },
        after: { status: "failing" },
      },
      impact: {
        scope: "project",
        severity: "critical",
        affectedFiles: ["src/api/client.ts"],
      },
    } as CodeObservation,

    // Meta watcher: system degraded
    {
      id: genId("obs_meta"),
      timestamp,
      watcherId: "meta_watch_1",
      watcherType: "meta",
      confidence: 0.95,
      tags: ["system", "degraded", "critical"],
      type: "system_health",
      assessment: {
        health: "failing",
        coverage: 0.6,
        accuracy: 0.55,
        latency: 500,
      },
      recommendations: [
        "Switch to MANUAL mode immediately",
        "Investigate external API dependency",
        "Consider fallback mechanisms",
      ],
      issues: [
        {
          type: "dependency_failure",
          severity: "critical",
          description: "External API dependency is unresponsive",
        },
        {
          type: "error_rate_spike",
          severity: "high",
          description: "Agent error rate has exceeded 50%",
        },
      ],
    } as MetaObservation,
  ]
}

/**
 * Create Phase 4: Recovery observations.
 * Issues being addressed, metrics stabilizing.
 */
export function createRecoveryObservations(): Observation[] {
  const timestamp = new Date()

  return [
    // World watcher: API recovering
    {
      id: genId("obs_world"),
      timestamp,
      watcherId: "world_watch_1",
      watcherType: "world",
      confidence: 0.85,
      tags: ["api", "recovery"],
      type: "api_change",
      source: "api-monitor/external",
      data: {
        title: "API Recovery In Progress",
        summary: "External API showing signs of recovery",
        content: {
          status: "recovering",
          errorRate: 0.15,
          latency: 800,
        },
      },
      relevance: 0.9,
      sentiment: "positive",
    } as WorldObservation,

    // Self watcher: error rate decreasing
    {
      id: genId("obs_self"),
      timestamp,
      watcherId: "self_watch_1",
      watcherType: "self",
      confidence: 0.88,
      tags: ["recovery", "stabilizing"],
      type: "agent_behavior",
      agentId: "build_agent",
      observation: {
        action: "retry_api_call",
        success: true,
        duration: 1200,
      },
      quality: {
        closeScore: 5.5,
        accuracy: 0.75,
        efficiency: 0.7,
      },
    } as SelfObservation,

    // Meta watcher: system stabilizing
    {
      id: genId("obs_meta"),
      timestamp,
      watcherId: "meta_watch_1",
      watcherType: "meta",
      confidence: 0.85,
      tags: ["system", "stabilizing"],
      type: "system_health",
      assessment: {
        health: "degraded",
        coverage: 0.75,
        accuracy: 0.78,
        latency: 120,
      },
      recommendations: [
        "Continue monitoring",
        "Consider switching back to HYBRID mode",
      ],
      issues: [
        {
          type: "partial_recovery",
          severity: "medium",
          description: "System recovering but not fully stable",
        },
      ],
    } as MetaObservation,
  ]
}

/**
 * Create Phase 5: Full Recovery observations.
 * System back to normal, all metrics healthy.
 */
export function createFullRecoveryObservations(): Observation[] {
  const timestamp = new Date()

  return [
    // All watchers reporting healthy
    {
      id: genId("obs_world"),
      timestamp,
      watcherId: "world_watch_1",
      watcherType: "world",
      confidence: 0.95,
      tags: ["api", "recovered"],
      type: "api_change",
      source: "api-monitor/external",
      data: {
        title: "API Fully Recovered",
        summary: "External API back to normal operation",
        content: {
          status: "healthy",
          errorRate: 0.01,
          latency: 100,
        },
      },
      relevance: 0.85,
      sentiment: "positive",
    } as WorldObservation,

    {
      id: genId("obs_code"),
      timestamp,
      watcherId: "code_watch_1",
      watcherType: "code",
      confidence: 0.95,
      tags: ["ci", "passing"],
      type: "build_status",
      source: "ci/github-actions",
      change: {
        action: "modify",
        before: { status: "failing" },
        after: { status: "passing" },
      },
      impact: {
        scope: "project",
        severity: "low",
        affectedFiles: [],
      },
    } as CodeObservation,

    {
      id: genId("obs_self"),
      timestamp,
      watcherId: "self_watch_1",
      watcherType: "self",
      confidence: 0.92,
      tags: ["recovered", "healthy"],
      type: "agent_behavior",
      agentId: "build_agent",
      observation: {
        action: "code_review",
        success: true,
        duration: 1400,
      },
      quality: {
        closeScore: 8.0,
        accuracy: 0.92,
        efficiency: 0.88,
      },
    } as SelfObservation,

    {
      id: genId("obs_meta"),
      timestamp,
      watcherId: "meta_watch_1",
      watcherType: "meta",
      confidence: 0.95,
      tags: ["system", "healthy"],
      type: "system_health",
      assessment: {
        health: "healthy",
        coverage: 0.9,
        accuracy: 0.9,
        latency: 50,
      },
      recommendations: [
        "System recovered - recommend switching back to D mode",
      ],
      issues: [],
    } as MetaObservation,
  ]
}

/**
 * Get all crisis phases for full simulation.
 */
export function getAllCrisisPhases(): CrisisPhase[] {
  return [
    {
      name: "normal",
      description: "Phase 1: Normal Operation - System healthy",
      observations: createNormalOperationObservations(),
    },
    {
      name: "emergence",
      description: "Phase 2: Crisis Emergence - Early warning signs",
      observations: createCrisisEmergenceObservations(),
    },
    {
      name: "escalation",
      description: "Phase 3: Crisis Escalation - Critical issues",
      observations: createCrisisEscalationObservations(),
    },
    {
      name: "recovery",
      description: "Phase 4: Recovery - Issues being addressed",
      observations: createRecoveryObservations(),
    },
    {
      name: "full_recovery",
      description: "Phase 5: Full Recovery - Back to normal",
      observations: createFullRecoveryObservations(),
    },
  ]
}

/**
 * Get observations for crisis detection (phases 2-3).
 * Useful for testing pattern and anomaly detection.
 */
export function createCrisisObservations(): Observation[] {
  return [
    ...createCrisisEmergenceObservations(),
    ...createCrisisEscalationObservations(),
  ]
}

/**
 * Create a minimal set of observations for quick tests.
 */
export function createMinimalCrisisSet(): Observation[] {
  const timestamp = new Date()

  return [
    // One critical observation that should trigger mode switch
    {
      id: genId("obs_self"),
      timestamp,
      watcherId: "self_watch_1",
      watcherType: "self",
      confidence: 0.95,
      tags: ["error", "critical"],
      type: "error_pattern",
      agentId: "build_agent",
      observation: {
        action: "api_call",
        success: false,
        duration: 10000,
        error: "Critical failure: Service unavailable",
      },
      quality: {
        closeScore: 2.0,
        accuracy: 0.3,
        efficiency: 0.2,
      },
    } as SelfObservation,

    // Meta observation confirming system failure
    {
      id: genId("obs_meta"),
      timestamp,
      watcherId: "meta_watch_1",
      watcherType: "meta",
      confidence: 0.98,
      tags: ["critical", "failing"],
      type: "system_health",
      assessment: {
        health: "failing",
        coverage: 0.4,
        accuracy: 0.35,
        latency: 1000,
      },
      recommendations: ["Immediate human intervention required"],
      issues: [
        {
          type: "system_failure",
          severity: "critical",
          description: "Multiple critical failures detected",
        },
      ],
    } as MetaObservation,
  ]
}
