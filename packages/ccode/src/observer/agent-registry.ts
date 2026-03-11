/**
 * Observer Agent Registry
 *
 * Provides helper functions to query agents by their observer capabilities.
 * Enables the Observer Network to discover which agents can contribute
 * to which watcher types.
 *
 * @module observer/agent-registry
 */

import { Agent } from "@/agent/agent"
import { Log } from "@/util/log"
import type { WatcherType } from "@/sdk/types"

const log = Log.create({ service: "observer.agent-registry" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ObserverAgentInfo {
  name: string
  description?: string
  canWatch: WatcherType[]
  contributeToConsensus: boolean
  reportToMeta: boolean
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get all agents with observer capabilities.
 */
export async function getObserverAgents(): Promise<ObserverAgentInfo[]> {
  const agents = await Agent.list()

  return agents
    .filter((agent) => agent.observerCapability)
    .map((agent) => ({
      name: agent.name,
      description: agent.description,
      canWatch: agent.observerCapability!.canWatch as WatcherType[],
      contributeToConsensus: agent.observerCapability!.contributeToConsensus,
      reportToMeta: agent.observerCapability!.reportToMeta,
    }))
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
  const agent = await Agent.get(agentName)
  return !!agent?.observerCapability
}

/**
 * Get observer capability for a specific agent.
 */
export async function getAgentObserverCapability(agentName: string): Promise<Agent.ObserverCapability | null> {
  const agent = await Agent.get(agentName)
  return agent?.observerCapability ?? null
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
