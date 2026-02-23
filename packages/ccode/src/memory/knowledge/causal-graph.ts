/**
 * Causal Graph
 *
 * Core implementation for the Causal Graph system.
 * Tracks Decision → Action → Outcome chains to enable causal analysis
 * of agent decisions and their results.
 *
 * Part of Phase 16: 因果链图数据库 (Causal Graph)
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import type {
  CausalGraphData,
  DecisionNode,
  ActionNode,
  OutcomeNode,
  CausalEdge,
  CausalChain,
  RecordDecisionRequest,
  RecordActionRequest,
  RecordOutcomeRequest,
  CausalRelationship,
  CausalStats,
  CausalQuery,
} from "./causal-types"

const log = Log.create({ service: "memory.knowledge.causal-graph" })

export namespace CausalGraph {
  // ============================================================================
  // ID Generators
  // ============================================================================

  const generateId = (prefix: string) =>
    `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`

  const decisionId = () => generateId("dec")
  const actionId = () => generateId("act")
  const outcomeId = () => generateId("out")
  const edgeId = (source: string, target: string) => `${source}->${target}`

  // ============================================================================
  // Storage Operations
  // ============================================================================

  export async function get(): Promise<CausalGraphData | undefined> {
    const projectId = Instance.project.id
    try {
      return await Storage.read<CausalGraphData>(["memory", "knowledge", "causal-graph", projectId])
    } catch {
      return undefined
    }
  }

  export async function create(): Promise<CausalGraphData> {
    const projectId = Instance.project.id
    const now = Date.now()

    const graph: CausalGraphData = {
      projectId,
      nodes: {
        decisions: [],
        actions: [],
        outcomes: [],
      },
      edges: [],
      adjacencyMap: {
        outgoing: {},
        incoming: {},
      },
      time: {
        created: now,
        updated: now,
      },
    }

    await save(graph)
    return graph
  }

  export async function save(graph: CausalGraphData): Promise<void> {
    const projectId = Instance.project.id
    graph.time.updated = Date.now()
    await Storage.write(["memory", "knowledge", "causal-graph", projectId], graph)
  }

  export async function load(): Promise<CausalGraphData> {
    const existing = await get()
    return existing ?? (await create())
  }

  export async function invalidate(): Promise<void> {
    const projectId = Instance.project.id
    await Storage.remove(["memory", "knowledge", "causal-graph", projectId])
  }

  // ============================================================================
  // Record Operations
  // ============================================================================

  /**
   * Record a decision made by an agent
   */
  export async function recordDecision(input: RecordDecisionRequest): Promise<DecisionNode> {
    const graph = await load()
    const id = decisionId()
    const timestamp = new Date().toISOString()

    const node: DecisionNode = {
      id,
      type: "decision",
      sessionId: input.sessionId,
      agentId: input.agentId,
      prompt: input.prompt,
      reasoning: input.reasoning,
      confidence: input.confidence,
      timestamp,
      context: input.context,
    }

    graph.nodes.decisions.push(node)
    graph.adjacencyMap.outgoing[id] = []
    graph.adjacencyMap.incoming[id] = []

    log.info("recorded decision", { id, agentId: input.agentId })
    await save(graph)

    return node
  }

  /**
   * Record an action taken as a result of a decision
   */
  export async function recordAction(input: RecordActionRequest): Promise<ActionNode> {
    const graph = await load()

    // Verify decision exists
    const decision = graph.nodes.decisions.find((d) => d.id === input.decisionId)
    if (!decision) {
      throw new Error(`Decision "${input.decisionId}" not found`)
    }

    const id = actionId()
    const timestamp = new Date().toISOString()

    const node: ActionNode = {
      id,
      type: "action",
      decisionId: input.decisionId,
      actionType: input.actionType,
      description: input.description,
      input: input.input,
      output: input.output,
      timestamp,
      duration: input.duration,
    }

    graph.nodes.actions.push(node)

    // Create edge from decision to action
    const edge: CausalEdge = {
      id: edgeId(input.decisionId, id),
      source: input.decisionId,
      target: id,
      relationship: "causes",
      weight: decision.confidence,
    }
    graph.edges.push(edge)

    // Update adjacency map
    graph.adjacencyMap.outgoing[id] = []
    graph.adjacencyMap.incoming[id] = [input.decisionId]
    if (!graph.adjacencyMap.outgoing[input.decisionId]) {
      graph.adjacencyMap.outgoing[input.decisionId] = []
    }
    graph.adjacencyMap.outgoing[input.decisionId].push(id)

    log.info("recorded action", { id, decisionId: input.decisionId, type: input.actionType })
    await save(graph)

    return node
  }

  /**
   * Record the outcome of an action
   */
  export async function recordOutcome(input: RecordOutcomeRequest): Promise<OutcomeNode> {
    const graph = await load()

    // Verify action exists
    const action = graph.nodes.actions.find((a) => a.id === input.actionId)
    if (!action) {
      throw new Error(`Action "${input.actionId}" not found`)
    }

    const id = outcomeId()
    const timestamp = new Date().toISOString()

    const node: OutcomeNode = {
      id,
      type: "outcome",
      actionId: input.actionId,
      status: input.status,
      description: input.description,
      metrics: input.metrics,
      feedback: input.feedback,
      timestamp,
    }

    graph.nodes.outcomes.push(node)

    // Determine edge weight based on outcome status
    const weight = input.status === "success" ? 1.0 : input.status === "partial" ? 0.5 : 0.0

    // Create edge from action to outcome
    const edge: CausalEdge = {
      id: edgeId(input.actionId, id),
      source: input.actionId,
      target: id,
      relationship: "results_in",
      weight,
      metadata: input.metrics,
    }
    graph.edges.push(edge)

    // Update adjacency map
    graph.adjacencyMap.outgoing[id] = []
    graph.adjacencyMap.incoming[id] = [input.actionId]
    if (!graph.adjacencyMap.outgoing[input.actionId]) {
      graph.adjacencyMap.outgoing[input.actionId] = []
    }
    graph.adjacencyMap.outgoing[input.actionId].push(id)

    log.info("recorded outcome", { id, actionId: input.actionId, status: input.status })
    await save(graph)

    return node
  }

  // ============================================================================
  // Query Operations
  // ============================================================================

  /**
   * Get a complete causal chain starting from a decision
   */
  export async function getCausalChain(decisionId: string): Promise<CausalChain | null> {
    const graph = await load()

    const decision = graph.nodes.decisions.find((d) => d.id === decisionId)
    if (!decision) return null

    const actions: ActionNode[] = []
    const outcomes: OutcomeNode[] = []
    const edges: CausalEdge[] = []

    // Find all actions linked to this decision
    const actionIds = graph.adjacencyMap.outgoing[decisionId] || []
    for (const actionId of actionIds) {
      const action = graph.nodes.actions.find((a) => a.id === actionId)
      if (action) {
        actions.push(action)

        // Find edge
        const edge = graph.edges.find((e) => e.source === decisionId && e.target === actionId)
        if (edge) edges.push(edge)

        // Find outcomes linked to this action
        const outcomeIds = graph.adjacencyMap.outgoing[actionId] || []
        for (const outcomeId of outcomeIds) {
          const outcome = graph.nodes.outcomes.find((o) => o.id === outcomeId)
          if (outcome) {
            outcomes.push(outcome)
            const outcomeEdge = graph.edges.find((e) => e.source === actionId && e.target === outcomeId)
            if (outcomeEdge) edges.push(outcomeEdge)
          }
        }
      }
    }

    return { decision, actions, outcomes, edges }
  }

  /**
   * Get all causal chains for a session
   */
  export async function getCausalChainsForSession(sessionId: string): Promise<CausalChain[]> {
    const graph = await load()
    const chains: CausalChain[] = []

    const sessionDecisions = graph.nodes.decisions.filter((d) => d.sessionId === sessionId)

    for (const decision of sessionDecisions) {
      const chain = await getCausalChain(decision.id)
      if (chain) chains.push(chain)
    }

    return chains
  }

  /**
   * Query the causal graph with filters
   */
  export async function query(input: CausalQuery): Promise<CausalChain[]> {
    const graph = await load()
    const chains: CausalChain[] = []
    const limit = input.limit ?? 100

    let decisions = graph.nodes.decisions

    // Apply filters
    if (input.agentId) {
      decisions = decisions.filter((d) => d.agentId === input.agentId)
    }
    if (input.sessionId) {
      decisions = decisions.filter((d) => d.sessionId === input.sessionId)
    }
    if (input.minConfidence !== undefined) {
      decisions = decisions.filter((d) => d.confidence >= input.minConfidence!)
    }
    if (input.dateFrom) {
      decisions = decisions.filter((d) => d.timestamp >= input.dateFrom!)
    }
    if (input.dateTo) {
      decisions = decisions.filter((d) => d.timestamp <= input.dateTo!)
    }

    // Apply limit
    decisions = decisions.slice(0, limit)

    // Build chains for filtered decisions
    for (const decision of decisions) {
      const chain = await getCausalChain(decision.id)
      if (!chain) continue

      // Additional filtering by action type and outcome status
      if (input.actionType) {
        chain.actions = chain.actions.filter((a) => a.actionType === input.actionType)
      }
      if (input.status) {
        chain.outcomes = chain.outcomes.filter((o) => o.status === input.status)
      }

      if (chain.actions.length > 0 || chain.outcomes.length > 0) {
        chains.push(chain)
      }
    }

    return chains
  }

  /**
   * Get decision node by ID
   */
  export async function getDecision(id: string): Promise<DecisionNode | undefined> {
    const graph = await load()
    return graph.nodes.decisions.find((d) => d.id === id)
  }

  /**
   * Get action node by ID
   */
  export async function getAction(id: string): Promise<ActionNode | undefined> {
    const graph = await load()
    return graph.nodes.actions.find((a) => a.id === id)
  }

  /**
   * Get outcome node by ID
   */
  export async function getOutcome(id: string): Promise<OutcomeNode | undefined> {
    const graph = await load()
    return graph.nodes.outcomes.find((o) => o.id === id)
  }

  /**
   * Get all decisions by agent
   */
  export async function getDecisionsByAgent(agentId: string): Promise<DecisionNode[]> {
    const graph = await load()
    return graph.nodes.decisions.filter((d) => d.agentId === agentId)
  }

  /**
   * Get success rate for a specific agent
   */
  export async function getSuccessRate(agentId?: string): Promise<number> {
    const graph = await load()

    let decisions = graph.nodes.decisions
    if (agentId) {
      decisions = decisions.filter((d) => d.agentId === agentId)
    }

    if (decisions.length === 0) return 0

    // Find all outcomes for these decisions
    const decisionIds = new Set(decisions.map((d) => d.id))
    const relevantActions = graph.nodes.actions.filter((a) => decisionIds.has(a.decisionId))
    const actionIds = new Set(relevantActions.map((a) => a.id))
    const relevantOutcomes = graph.nodes.outcomes.filter((o) => actionIds.has(o.actionId))

    if (relevantOutcomes.length === 0) return 0

    const successCount = relevantOutcomes.filter((o) => o.status === "success").length
    return successCount / relevantOutcomes.length
  }

  // ============================================================================
  // Statistics
  // ============================================================================

  /**
   * Get comprehensive statistics about the causal graph
   */
  export async function getStats(): Promise<CausalStats> {
    const graph = await load()

    const { decisions, actions, outcomes } = graph.nodes

    // Calculate overall success rate
    const successOutcomes = outcomes.filter((o) => o.status === "success").length
    const successRate = outcomes.length > 0 ? successOutcomes / outcomes.length : 0

    // Calculate average confidence
    const avgConfidence =
      decisions.length > 0
        ? decisions.reduce((sum, d) => sum + d.confidence, 0) / decisions.length
        : 0

    // Group by agent
    const agentStats = new Map<string, { count: number; successCount: number }>()
    for (const decision of decisions) {
      const stat = agentStats.get(decision.agentId) || { count: 0, successCount: 0 }
      stat.count++

      // Find outcomes for this decision
      const decisionActions = actions.filter((a) => a.decisionId === decision.id)
      for (const action of decisionActions) {
        const actionOutcomes = outcomes.filter((o) => o.actionId === action.id)
        stat.successCount += actionOutcomes.filter((o) => o.status === "success").length
      }

      agentStats.set(decision.agentId, stat)
    }

    const topAgents = Array.from(agentStats.entries())
      .map(([agentId, stat]) => ({
        agentId,
        decisionCount: stat.count,
        successRate: stat.count > 0 ? stat.successCount / stat.count : 0,
      }))
      .sort((a, b) => b.decisionCount - a.decisionCount)
      .slice(0, 10)

    // Action type distribution
    const actionTypeDistribution: Record<string, number> = {}
    for (const action of actions) {
      actionTypeDistribution[action.actionType] = (actionTypeDistribution[action.actionType] || 0) + 1
    }

    return {
      totalDecisions: decisions.length,
      totalActions: actions.length,
      totalOutcomes: outcomes.length,
      totalEdges: graph.edges.length,
      successRate,
      avgConfidence,
      topAgents,
      actionTypeDistribution,
    }
  }

  // ============================================================================
  // Visualization
  // ============================================================================

  /**
   * Generate Mermaid diagram for causal graph
   */
  export async function toMermaid(options?: {
    maxNodes?: number
    decisionId?: string
  }): Promise<string> {
    const graph = await load()
    const maxNodes = options?.maxNodes ?? 50
    const lines = ["graph TD"]

    let decisions = graph.nodes.decisions
    if (options?.decisionId) {
      decisions = decisions.filter((d) => d.id === options.decisionId)
    }
    decisions = decisions.slice(0, maxNodes)

    const includedNodeIds = new Set<string>()

    // Add decision nodes
    for (const decision of decisions) {
      const label = sanitizeLabel(`${decision.agentId}: ${decision.prompt.slice(0, 30)}...`)
      lines.push(`  ${sanitizeId(decision.id)}{{${label}}}`)
      includedNodeIds.add(decision.id)
    }

    // Add action nodes and edges
    for (const action of graph.nodes.actions) {
      if (!includedNodeIds.has(action.decisionId)) continue
      if (includedNodeIds.size >= maxNodes) break

      const label = sanitizeLabel(`${action.actionType}: ${action.description.slice(0, 25)}...`)
      lines.push(`  ${sanitizeId(action.id)}[${label}]`)
      lines.push(`  ${sanitizeId(action.decisionId)} -->|causes| ${sanitizeId(action.id)}`)
      includedNodeIds.add(action.id)
    }

    // Add outcome nodes and edges
    for (const outcome of graph.nodes.outcomes) {
      if (!includedNodeIds.has(outcome.actionId)) continue
      if (includedNodeIds.size >= maxNodes) break

      const statusEmoji = outcome.status === "success" ? "✓" : outcome.status === "failure" ? "✗" : "~"
      const label = sanitizeLabel(`${statusEmoji} ${outcome.description.slice(0, 25)}...`)
      const shape = outcome.status === "success" ? `(${label})` : outcome.status === "failure" ? `((${label}))` : `[${label}]`
      lines.push(`  ${sanitizeId(outcome.id)}${shape}`)
      lines.push(`  ${sanitizeId(outcome.actionId)} -->|results_in| ${sanitizeId(outcome.id)}`)
      includedNodeIds.add(outcome.id)
    }

    // Style nodes by type
    lines.push("")
    lines.push("  classDef decision fill:#f9f,stroke:#333,stroke-width:2px")
    lines.push("  classDef action fill:#bbf,stroke:#333")
    lines.push("  classDef success fill:#bfb,stroke:#333")
    lines.push("  classDef failure fill:#fbb,stroke:#333")

    return lines.join("\n")
  }

  // ============================================================================
  // Helpers
  // ============================================================================

  function sanitizeId(id: string): string {
    return id.replace(/[^a-zA-Z0-9_]/g, "_").slice(0, 64)
  }

  function sanitizeLabel(label: string): string {
    return label.replace(/"/g, "'").replace(/[<>]/g, "")
  }
}
