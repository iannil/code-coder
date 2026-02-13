import { Identifier } from "../../id/id"
import { KnowledgeSchema } from "./schema"

/**
 * Dynamically import Storage to avoid circular dependencies.
 */
async function getStorage() {
  return (await import("../../storage/storage")).Storage
}

// Local storage interface to avoid circular dependency
const storage = {
  write: async (key: string[], content: unknown) =>
    (await getStorage()).write(key, content),
  read: async <T>(key: string[]) =>
    (await getStorage()).read<T>(key),
  list: async (prefix: string[]) =>
    (await getStorage()).list(prefix),
  remove: async (key: string[]) =>
    (await getStorage()).remove(key),
}

export namespace ArgumentChain {
  const STORAGE_PREFIX = ["document_argument"]

  /**
   * Generate a unique argument chain ID.
   */
  export function createID(): string {
    return Identifier.create("argument", false)
  }

  /**
   * Create a new argument chain.
   */
  export async function create(
    documentID: string,
    input: Omit<KnowledgeSchema.ArgumentChain, "id" | "createdAt" | "updatedAt">,
  ): Promise<KnowledgeSchema.ArgumentChain> {
    const now = Date.now()
    const chain: KnowledgeSchema.ArgumentChain = {
      id: createID(),
      ...input,
      createdAt: now,
      updatedAt: now,
    }

    await storage.write([...STORAGE_PREFIX, documentID, chain.id], chain)
    return chain
  }

  /**
   * Get an argument chain by ID.
   */
  export async function get(
    documentID: string,
    chainID: string,
  ): Promise<KnowledgeSchema.ArgumentChain | null> {
    try {
      return await storage.read<KnowledgeSchema.ArgumentChain>([...STORAGE_PREFIX, documentID, chainID])
    } catch {
      return null
    }
  }

  /**
   * Get all argument chains for a document.
   */
  export async function list(documentID: string): Promise<KnowledgeSchema.ArgumentChain[]> {
    const keys = await storage.list([...STORAGE_PREFIX, documentID])
    const chains: KnowledgeSchema.ArgumentChain[] = []

    for (const key of keys) {
      try {
        const chain = await storage.read<KnowledgeSchema.ArgumentChain>(key)
        if (chain) chains.push(chain)
      } catch {
        // Skip invalid entries
      }
    }

    return chains.sort((a, b) => b.createdAt - a.createdAt)
  }

  /**
   * Get argument chains for a specific chapter.
   */
  export async function listByChapter(
    documentID: string,
    chapterID: string,
  ): Promise<KnowledgeSchema.ArgumentChain[]> {
    const chains = await list(documentID)
    return chains.filter((c) => c.chapterID === chapterID)
  }

  /**
   * Update an argument chain.
   */
  export async function update(
    documentID: string,
    chainID: string,
    updates: Partial<Omit<KnowledgeSchema.ArgumentChain, "id" | "createdAt">>,
  ): Promise<KnowledgeSchema.ArgumentChain | null> {
    const existing = await get(documentID, chainID)
    if (!existing) return null

    const updated: KnowledgeSchema.ArgumentChain = {
      ...existing,
      ...updates,
      id: existing.id,
      createdAt: existing.createdAt,
      updatedAt: Date.now(),
    }

    await storage.write([...STORAGE_PREFIX, documentID, chainID], updated)
    return updated
  }

  /**
   * Delete an argument chain.
   */
  export async function remove(documentID: string, chainID: string): Promise<boolean> {
    // Remove from other chains' counterArguments
    const allChains = await list(documentID)
    for (const chain of allChains) {
      if (chain.counterArguments.includes(chainID)) {
        await update(documentID, chain.id, {
          counterArguments: chain.counterArguments.filter((id) => id !== chainID),
        })
      }
    }

    await storage.remove([...STORAGE_PREFIX, documentID, chainID])
    return true
  }

  /**
   * Link two argument chains as counter-arguments.
   */
  export async function linkCounterArguments(
    documentID: string,
    chainID: string,
    counterChainIDs: string[],
  ): Promise<void> {
    const chain = await get(documentID, chainID)
    if (!chain) throw new Error("Argument chain not found")

    const existingCounters = new Set(chain.counterArguments)
    for (const counterID of counterChainIDs) {
      existingCounters.add(counterID)

      // Also add reverse reference
      const counterChain = await get(documentID, counterID)
      if (counterChain && !counterChain.counterArguments.includes(chainID)) {
        await update(documentID, counterID, {
          counterArguments: [...counterChain.counterArguments, chainID],
        })
      }
    }

    await update(documentID, chainID, {
      counterArguments: Array.from(existingCounters),
    })
  }

  /**
   * Add a reasoning step to an argument chain.
   */
  export async function addReasoningStep(
    documentID: string,
    chainID: string,
    step: Omit<KnowledgeSchema.ReasoningStep, "step"> & { step: string },
  ): Promise<KnowledgeSchema.ArgumentChain | null> {
    const chain = await get(documentID, chainID)
    if (!chain) return null

    const reasoningStep: KnowledgeSchema.ReasoningStep = {
      step: step.step,
      supportsPremise: step.supportsPremise,
      evidence: step.evidence ?? [],
    }

    return update(documentID, chainID, {
      reasoningSteps: [...chain.reasoningSteps, reasoningStep],
    })
  }

  /**
   * Validate an argument chain for logical consistency.
   */
  export async function validate(
    documentID: string,
    chainID: string,
  ): Promise<{
    isValid: boolean
    issues: Array<{ type: string; description: string; severity: "low" | "medium" | "high" }>
  }> {
    const chain = await get(documentID, chainID)
    if (!chain) {
      return {
        isValid: false,
        issues: [{ type: "not_found", description: "Argument chain not found", severity: "high" }],
      }
    }

    const issues: Array<{ type: string; description: string; severity: "low" | "medium" | "high" }> = []

    // Check if premise is empty
    if (!chain.premise || chain.premise.trim().length === 0) {
      issues.push({
        type: "missing_premise",
        description: "Argument chain lacks a premise",
        severity: "high",
      })
    }

    // Check if conclusion is empty
    if (!chain.conclusion || chain.conclusion.trim().length === 0) {
      issues.push({
        type: "missing_conclusion",
        description: "Argument chain lacks a conclusion",
        severity: "high",
      })
    }

    // Check if there are reasoning steps
    if (chain.reasoningSteps.length === 0) {
      issues.push({
        type: "missing_steps",
        description: "Argument chain lacks reasoning steps",
        severity: "medium",
      })
    }

    // Check for circular reasoning
    const premiseLower = chain.premise.toLowerCase()
    const conclusionLower = chain.conclusion.toLowerCase()
    if (premiseLower === conclusionLower || premiseLower.includes(conclusionLower)) {
      issues.push({
        type: "circular_reasoning",
        description: "Conclusion appears to be the same as the premise",
        severity: "high",
      })
    }

    // Check if any step contradicts the premise
    for (const step of chain.reasoningSteps) {
      if (step.supportsPremise === false) {
        issues.push({
          type: "contradictory_step",
          description: `Reasoning step "${step.step.slice(0, 50)}..." contradicts the premise`,
          severity: "medium",
        })
      }
    }

    return {
      isValid: issues.filter((i) => i.severity === "high").length === 0,
      issues,
    }
  }

  /**
   * Detect circular reasoning across all argument chains in a document.
   */
  export async function detectCircularReasoning(
    documentID: string,
  ): Promise<Array<{ path: string[]; description: string }>> {
    const chains = await list(documentID)
    const cycles: Array<{ path: string[]; description: string }> = []

    // Build a dependency graph
    const graph = new Map<string, string[]>()
    for (const chain of chains) {
      const relatedIDs = [...chain.counterArguments]
      for (const step of chain.reasoningSteps) {
        for (const evidenceID of step.evidence) {
          if (!relatedIDs.includes(evidenceID)) {
            relatedIDs.push(evidenceID)
          }
        }
      }
      graph.set(chain.id, relatedIDs)
    }

    // Detect cycles using DFS
    const visited = new Set<string>()
    const recStack = new Set<string>()

    function dfs(nodeID: string, path: string[]): void {
      visited.add(nodeID)
      recStack.add(nodeID)
      path.push(nodeID)

      const neighbors = graph.get(nodeID) || []
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          dfs(neighbor, [...path])
        } else if (recStack.has(neighbor)) {
          // Found a cycle
          const cycleStart = path.indexOf(neighbor)
          const cyclePath = [...path.slice(cycleStart), neighbor]
          cycles.push({
            path: cyclePath,
            description: `Circular reasoning detected through ${cyclePath.length} argument(s)`,
          })
        }
      }

      recStack.delete(nodeID)
    }

    for (const chainID of chains.map((c) => c.id)) {
      if (!visited.has(chainID)) {
        dfs(chainID, [])
      }
    }

    return cycles
  }

  /**
   * Check argument coherence across all chains.
   */
  export async function checkCoherence(
    documentID: string,
  ): Promise<{
    overallScore: number // 0-1
    issues: Array<{ chainID: string; description: string; severity: "low" | "medium" | "high" }>
  }> {
    const chains = await list(documentID)
    const issues: Array<{ chainID: string; description: string; severity: "low" | "medium" | "high" }> = []

    // Validate each chain
    for (const chain of chains) {
      const validation = await validate(documentID, chain.id)
      for (const issue of validation.issues) {
        issues.push({
          chainID: chain.id,
          description: issue.description,
          severity: issue.severity,
        })
      }
    }

    // Check for contradictions between chains
    for (let i = 0; i < chains.length; i++) {
      for (let j = i + 1; j < chains.length; j++) {
        const chainA = chains[i]
        const chainB = chains[j]

        // Check if conclusions contradict
        if (areContradictory(chainA.conclusion, chainB.conclusion)) {
          issues.push({
            chainID: chainA.id,
            description: `Conclusion contradicts argument ${chainB.id}`,
            severity: "medium",
          })
        }
      }
    }

    // Calculate overall score
    const validChains = chains.length - issues.filter((i) => i.severity === "high").length
    const overallScore = chains.length > 0 ? validChains / chains.length : 1

    return { overallScore, issues }
  }

  /**
   * Export argument chains as formatted text for AI context.
   */
  export async function exportForAI(
    documentID: string,
    options: {
      chapterID?: string
      maxChains?: number
    } = {},
  ): Promise<string> {
    let chains = await list(documentID)

    if (options.chapterID) {
      chains = chains.filter((c) => c.chapterID === options.chapterID)
    }

    chains = chains.slice(0, options.maxChains ?? 10)

    const lines: string[] = []
    lines.push("# Argument Chains")

    for (const chain of chains) {
      lines.push("")
      lines.push(`## Argument: ${chain.id.slice(0, 8)}`)
      lines.push(`**Status:** ${chain.status}`)
      lines.push(`**Premise:** ${chain.premise}`)
      lines.push(`**Conclusion:** ${chain.conclusion}`)

      if (chain.reasoningSteps.length > 0) {
        lines.push("**Reasoning Steps:**")
        for (let i = 0; i < chain.reasoningSteps.length; i++) {
          const step = chain.reasoningSteps[i]
          const support = step.supportsPremise === false ? " ❌" : " ✓"
          lines.push(`${i + 1}.${support} ${step.step}`)
        }
      }

      if (chain.counterArguments.length > 0) {
        lines.push(`**Counter-Arguments:** ${chain.counterArguments.length} referenced`)
      }

      lines.push("")
    }

    return lines.join("\n")
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /**
   * Check if two statements are contradictory.
   */
  function areContradictory(a: string, b: string): boolean {
    const lowerA = a.toLowerCase()
    const lowerB = b.toLowerCase()

    // Direct negation patterns
    const negationPatterns = [
      /^(not|never|no|none|neither)/i,
    ]

    // If one starts with negation and contains similar content
    for (const pattern of negationPatterns) {
      if (pattern.test(a) && lowerB.includes(lowerA.replace(/^(not |never |no |none |neither )/i, ""))) {
        return true
      }
      if (pattern.test(b) && lowerA.includes(lowerB.replace(/^(not |never |no |none |neither )/i, ""))) {
        return true
      }
    }

    // Antonym pairs
    const antonyms = [
      ["always", "never"],
      ["all", "none"],
      ["true", "false"],
      ["good", "bad"],
      ["increases", "decreases"],
      ["gain", "loss"],
    ]

    for (const [a1, a2] of antonyms) {
      if (lowerA.includes(a1) && lowerB.includes(a2)) return true
      if (lowerA.includes(a2) && lowerB.includes(a1)) return true
    }

    return false
  }
}
