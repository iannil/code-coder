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

export namespace KnowledgeNode {
  const STORAGE_PREFIX = ["document_knowledge"]

  // Type aliases for internal use
  type KnowledgeNode = KnowledgeSchema.KnowledgeNode
  type KnowledgeNodeType = KnowledgeSchema.KnowledgeNodeType

  /**
   * Generate a unique knowledge node ID.
   */
  export function createID(): string {
    return Identifier.create("knowledge", false)
  }

  /**
   * Create a new knowledge node.
   */
  export async function create(
    documentID: string,
    input: Omit<KnowledgeNode, "id" | "createdAt" | "updatedAt">,
  ): Promise<KnowledgeNode> {
    const now = Date.now()
    const node: KnowledgeNode = {
      id: createID(),
      ...input,
      createdAt: now,
      updatedAt: now,
    }

    await storage.write([...STORAGE_PREFIX, documentID, node.id], node)
    return node
  }

  /**
   * Get a knowledge node by ID.
   */
  export async function get(
    documentID: string,
    nodeID: string,
  ): Promise<KnowledgeNode | null> {
    try {
      return await storage.read<KnowledgeNode>([...STORAGE_PREFIX, documentID, nodeID])
    } catch {
      return null
    }
  }

  /**
   * Get all knowledge nodes for a document.
   */
  export async function list(documentID: string): Promise<KnowledgeNode[]> {
    const keys = await storage.list([...STORAGE_PREFIX, documentID])
    const nodes: KnowledgeNode[] = []

    for (const key of keys) {
      try {
        const node = await storage.read<KnowledgeNode>(key)
        if (node) nodes.push(node)
      } catch {
        // Skip invalid entries
      }
    }

    return nodes.sort((a, b) => b.createdAt - a.createdAt)
  }

  /**
   * Get knowledge nodes by type.
   */
  export async function listByType(
    documentID: string,
    type: KnowledgeNodeType,
  ): Promise<KnowledgeNode[]> {
    const nodes = await list(documentID)
    return nodes.filter((n) => n.type === type)
  }

  /**
   * Get knowledge nodes for a specific chapter.
   */
  export async function listByChapter(
    documentID: string,
    chapterID: string,
  ): Promise<KnowledgeNode[]> {
    const nodes = await list(documentID)
    return nodes.filter((n) => n.chapterID === chapterID)
  }

  /**
   * Update a knowledge node (immutable pattern).
   */
  export async function update(
    documentID: string,
    nodeID: string,
    updates: Partial<Omit<KnowledgeNode, "id" | "createdAt">>,
  ): Promise<KnowledgeNode | null> {
    const existing = await get(documentID, nodeID)
    if (!existing) return null

    const updated: KnowledgeNode = {
      ...existing,
      ...updates,
      id: existing.id, // Preserve ID
      createdAt: existing.createdAt, // Preserve creation time
      updatedAt: Date.now(),
    }

    await storage.write([...STORAGE_PREFIX, documentID, nodeID], updated)
    return updated
  }

  /**
   * Delete a knowledge node.
   */
  export async function remove(documentID: string, nodeID: string): Promise<boolean> {
    // Check if node exists
    const node = await get(documentID, nodeID)
    if (!node) return false

    // Check if other nodes reference this node
    const allNodes = await list(documentID)
    const dependents = allNodes.filter((n) => n.derivedFrom.includes(nodeID))
    if (dependents.length > 0) {
      throw new Error(
        `Cannot delete node ${nodeID}: ${dependents.length} node(s) depend on it`,
      )
    }

    await storage.remove([...STORAGE_PREFIX, documentID, nodeID])
    return true
  }

  /**
   * Search knowledge nodes by content.
   */
  export async function search(
    documentID: string,
    query: string,
  ): Promise<KnowledgeNode[]> {
    const nodes = await list(documentID)
    const lowerQuery = query.toLowerCase()

    return nodes.filter(
      (n) =>
        n.content.toLowerCase().includes(lowerQuery) ||
        Object.values(n.attributes).some((v) => {
          const sv = v as string
          return sv.toLowerCase().includes(lowerQuery)
        }),
    )
  }

  /**
   * Find related nodes via derivedFrom references.
   */
  export async function getRelated(
    documentID: string,
    nodeID: string,
    maxDepth: number = 2,
  ): Promise<KnowledgeNode[]> {
    const visited = new Set<string>([nodeID])
    const related: KnowledgeNode[] = []
    const queue: Array<{ id: string; depth: number }> = [{ id: nodeID, depth: 0 }]

    while (queue.length > 0) {
      const { id, depth } = queue.shift()!
      if (depth >= maxDepth) continue

      const node = await get(documentID, id)
      if (!node) continue

      // Add derived nodes
      for (const derivedID of node.derivedFrom) {
        if (visited.has(derivedID)) continue

        visited.add(derivedID)
        const derivedNode = await get(documentID, derivedID)
        if (derivedNode) {
          related.push(derivedNode)
          queue.push({ id: derivedID, depth: depth + 1 })
        }
      }

      // Find nodes that reference this node
      const allNodes = await list(documentID)
      for (const other of allNodes) {
        if (other.derivedFrom.includes(id) && !visited.has(other.id)) {
          visited.add(other.id)
          related.push(other)
          queue.push({ id: other.id, depth: depth + 1 })
        }
      }
    }

    return related
  }

  /**
   * Build a knowledge graph adjacency list from nodes.
   */
  export async function buildKnowledgeGraph(
    documentID: string,
  ): Promise<Record<string, string[]>> {
    const nodes = await list(documentID)
    const graph: Record<string, string[]> = {}

    // Initialize with all nodes
    for (const node of nodes) {
      graph[node.id] = []
    }

    // Build edges
    for (const node of nodes) {
      for (const sourceID of node.derivedFrom) {
        if (!graph[sourceID]) {
          graph[sourceID] = []
        }
        if (!graph[sourceID].includes(node.id)) {
          graph[sourceID].push(node.id)
        }
      }
    }

    return graph
  }

  /**
   * Export nodes as structured data for AI context.
   */
  export async function exportForAI(
    documentID: string,
    options: {
      types?: KnowledgeNodeType[]
      maxTokens?: number
      includeRelationships?: boolean
    } = {},
  ): Promise<string> {
    const nodes = await list(documentID)

    let filtered = nodes
    if (options.types && options.types.length > 0) {
      filtered = nodes.filter((n) => options.types!.includes(n.type))
    }

    // Simple token estimation (rough approximation)
    const estimateTokens = (text: string): number => {
      return Math.ceil(text.length / 3)
    }

    let result = ""
    let usedTokens = 0
    const maxTokens = options.maxTokens ?? 10000

    for (const node of filtered) {
      const nodeText = formatNode(node, options.includeRelationships ?? false)
      const tokens = estimateTokens(nodeText)

      if (usedTokens + tokens > maxTokens) break

      result += nodeText + "\n\n"
      usedTokens += tokens
    }

    return result
  }

  /**
   * Format a single node for AI context.
   */
  function formatNode(
    node: KnowledgeNode,
    includeRelationships: boolean,
  ): string {
    const emoji: Record<KnowledgeNodeType, string> = {
      principle: "📜",
      concept: "💡",
      argument: "⚖️",
      evidence: "📊",
      conclusion: "✅",
      character: "👤",
      location: "📍",
      world_rule: "🌍",
    }

    const lines: string[] = []
    lines.push(`${emoji[node.type]} **${node.type.toUpperCase()}:** ${node.content.slice(0, 100)}${node.content.length > 100 ? "..." : ""}`)

    if (node.confidence < 1) {
      lines.push(`   Confidence: ${(node.confidence * 100).toFixed(0)}%`)
    }

    if (Object.keys(node.attributes).length > 0) {
      const attrs = Object.entries(node.attributes)
        .slice(0, 3)
        .map(([k, v]) => `${k}: ${v}`)
        .join(", ")
      lines.push(`   Attributes: ${attrs}`)
    }

    if (includeRelationships && node.derivedFrom.length > 0) {
      lines.push(`   Derived from: ${node.derivedFrom.length} node(s)`)
    }

    return lines.join("\n")
  }

  /**
   * Validate knowledge graph for circular references.
   */
  export async function detectCycles(
    documentID: string,
  ): Promise<string[][]> {
    const graph = await buildKnowledgeGraph(documentID)
    const cycles: string[][] = []
    const visited = new Set<string>()
    const recStack = new Set<string>()

    function dfs(nodeID: string, path: string[]): void {
      visited.add(nodeID)
      recStack.add(nodeID)
      path.push(nodeID)

      for (const neighbor of graph[nodeID] || []) {
        if (!visited.has(neighbor)) {
          dfs(neighbor, [...path])
        } else if (recStack.has(neighbor)) {
          // Found a cycle
          const cycleStart = path.indexOf(neighbor)
          cycles.push([...path.slice(cycleStart), neighbor])
        }
      }

      recStack.delete(nodeID)
    }

    for (const nodeID of Object.keys(graph)) {
      if (!visited.has(nodeID)) {
        dfs(nodeID, [])
      }
    }

    return cycles
  }

  /**
   * Merge duplicate knowledge nodes.
   */
  export async function mergeDuplicates(
    documentID: string,
    similarityThreshold: number = 0.85,
  ): Promise<{ merged: number; ids: string[][] }> {
    const nodes = await list(documentID)
    const mergedIds: string[][] = []
    let mergedCount = 0

    // Group by type for comparison
    const byType = new Map<KnowledgeNodeType, KnowledgeNode[]>()
    for (const node of nodes) {
      if (!byType.has(node.type)) {
        byType.set(node.type, [])
      }
      byType.get(node.type)!.push(node)
    }

    // Simple similarity check (content-based)
    for (const [, typeNodes] of byType.entries()) {
      const processed = new Set<string>()

      for (const node of typeNodes) {
        if (processed.has(node.id)) continue

        const duplicates: string[] = []
        const nodeContent = node.content.toLowerCase()

        for (const other of typeNodes) {
          if (other.id === node.id || processed.has(other.id)) continue

          const otherContent = other.content.toLowerCase()
          const similarity = calculateSimilarity(nodeContent, otherContent)

          if (similarity >= similarityThreshold) {
            duplicates.push(other.id)
            processed.add(other.id)
          }
        }

        if (duplicates.length > 0) {
          mergedIds.push([node.id, ...duplicates])
          mergedCount += duplicates.length

          // Merge attributes and derivedFrom
          const allDerived = new Set(node.derivedFrom)
          for (const dupId of duplicates) {
            const dupNode = await get(documentID, dupId)
            if (dupNode) {
              dupNode.derivedFrom.forEach((d: string) => allDerived.add(d))
            }
          }

          await update(documentID, node.id, {
            derivedFrom: Array.from(allDerived),
          })

          // Delete duplicates
          for (const dupId of duplicates) {
            await storage.remove([...STORAGE_PREFIX, documentID, dupId])
          }
        }

        processed.add(node.id)
      }
    }

    return { merged: mergedCount, ids: mergedIds }
  }

  /**
   * Calculate Jaccard similarity between two strings.
   */
  function calculateSimilarity(a: string, b: string): number {
    const wordsA = new Set(a.split(/\s+/))
    const wordsB = new Set(b.split(/\s+/))

    const intersection = new Set([...wordsA].filter((x) => wordsB.has(x)))
    const union = new Set([...wordsA, ...wordsB])

    return intersection.size / union.size
  }
}
