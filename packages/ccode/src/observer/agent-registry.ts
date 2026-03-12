/**
 * Observer Agent Registry
 *
 * Provides helper functions to query agents by their observer capabilities.
 * Enables the Observer Network to discover which agents can contribute
 * to which watcher types.
 *
 * @module observer/agent-registry
 */

import { listAgentsFiltered } from "@/sdk"
import { Log } from "@/util/log"
import type { WatcherType } from "@/sdk/types"

const log = Log.create({ service: "observer.agent-registry" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ObserverCapability {
  canWatch: WatcherType[]
  contributeToConsensus: boolean
  reportToMeta: boolean
}

export interface ObserverAgentInfo {
  name: string
  description?: string
  canWatch: WatcherType[]
  contributeToConsensus: boolean
  reportToMeta: boolean
}

// ─────────────────────────────────────────────────────────────────────────────
// Static Observer Capability Mapping
// Defines which agents have observer capabilities (static configuration)
// ─────────────────────────────────────────────────────────────────────────────

const OBSERVER_CAPABILITIES: Record<string, ObserverCapability> = {
  explore: {
    canWatch: ["code"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  "code-reviewer": {
    canWatch: ["self"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  "security-reviewer": {
    canWatch: ["self"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  "tdd-guide": {
    canWatch: ["self"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  architect: {
    canWatch: ["code"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  observer: {
    canWatch: ["meta"],
    contributeToConsensus: true,
    reportToMeta: false,
  },
  decision: {
    canWatch: ["self"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  macro: {
    canWatch: ["world"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  trader: {
    canWatch: ["world"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  picker: {
    canWatch: ["world"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  "value-analyst": {
    canWatch: ["world"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  verifier: {
    canWatch: ["self"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  autonomous: {
    canWatch: ["self"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
  "feasibility-assess": {
    canWatch: ["code"],
    contributeToConsensus: true,
    reportToMeta: true,
  },
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get all agents with observer capabilities.
 */
export async function getObserverAgents(): Promise<ObserverAgentInfo[]> {
  const agents = await listAgentsFiltered()
  const result: ObserverAgentInfo[] = []

  for (const agent of agents) {
    const capability = OBSERVER_CAPABILITIES[agent.name]
    if (capability) {
      result.push({
        name: agent.name,
        description: agent.description,
        canWatch: capability.canWatch,
        contributeToConsensus: capability.contributeToConsensus,
        reportToMeta: capability.reportToMeta,
      })
    }
  }

  return result
}

/**
 * Get agents that can contribute to a specific watcher type.
 */
export async function getAgentsForWatcher(watcherType: WatcherType): Promise<ObserverAgentInfo[]> {
  const observerAgents = await getObserverAgents()
  return observerAgents.filter((agent) => agent.canWatch.includes(watcherType))
}

/**
 * Get agents that contribute to consensus.
 */
export async function getConsensusAgents(): Promise<ObserverAgentInfo[]> {
  const observerAgents = await getObserverAgents()
  return observerAgents.filter((agent) => agent.contributeToConsensus)
}

/**
 * Get agents that report to MetaWatch.
 */
export async function getMetaReportingAgents(): Promise<ObserverAgentInfo[]> {
  const observerAgents = await getObserverAgents()
  return observerAgents.filter((agent) => agent.reportToMeta)
}

/**
 * Check if a specific agent has observer capability.
 */
export async function hasObserverCapability(agentName: string): Promise<boolean> {
  return agentName in OBSERVER_CAPABILITIES
}

/**
 * Get observer capability for a specific agent.
 */
export async function getAgentObserverCapability(agentName: string): Promise<ObserverCapability | null> {
  return OBSERVER_CAPABILITIES[agentName] ?? null
}

/**
 * Log summary of observer-capable agents.
 */
export async function logObserverAgentSummary(): Promise<void> {
  const observerAgents = await getObserverAgents()

  if (observerAgents.length === 0) {
    log.info("No observer-capable agents found")
    return
  }

  const byWatcher: Record<WatcherType, string[]> = {
    code: [],
    world: [],
    self: [],
    meta: [],
  }

  for (const agent of observerAgents) {
    for (const watcherType of agent.canWatch) {
      byWatcher[watcherType].push(agent.name)
    }
  }

  log.info("Observer-capable agents", {
    total: observerAgents.length,
    byWatcher,
    consensusContributors: observerAgents.filter((a) => a.contributeToConsensus).length,
    metaReporters: observerAgents.filter((a) => a.reportToMeta).length,
  })
}
