import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { LSP } from "@/lsp/index"
import { CodeIndex } from "./code-index"
import path from "path"
import { pathToFileURL, fileURLToPath } from "url"
import z from "zod"

const log = Log.create({ service: "memory.knowledge.call-graph" })

export namespace CallGraph {
  export const CallNode = z.object({
    id: z.string(),
    name: z.string(),
    kind: z.enum(["function", "method", "constructor"]),
    file: z.string(),
    line: z.number(),
    character: z.number(),
    detail: z.string().optional(),
  })
  export type CallNode = z.infer<typeof CallNode>

  export const CallEdge = z.object({
    id: z.string(),
    caller: z.string(),
    callee: z.string(),
    locations: z.array(
      z.object({
        line: z.number(),
        character: z.number(),
      }),
    ),
  })
  export type CallEdge = z.infer<typeof CallEdge>

  export const CallChain = z.object({
    startNode: z.string(),
    maxDepth: z.number(),
    nodes: z.array(CallNode),
    edges: z.array(CallEdge),
    depth: z.record(z.string(), z.number()),
  })
  export type CallChain = z.infer<typeof CallChain>

  export const RecursionInfo = z.object({
    node: CallNode,
    type: z.enum(["direct", "indirect"]),
    cycle: z.array(z.string()),
  })
  export type RecursionInfo = z.infer<typeof RecursionInfo>

  export const Graph = z.object({
    projectID: z.string(),
    nodes: z.array(CallNode),
    edges: z.array(CallEdge),
    incomingMap: z.record(z.string(), z.array(z.string())),
    outgoingMap: z.record(z.string(), z.array(z.string())),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Graph = z.infer<typeof Graph>

  const nodeId = (name: string, file: string, line: number) => `call:${name}:${file}:${line}`

  export async function get(): Promise<Graph | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Graph>(["memory", "knowledge", "call-graph", projectID])
    } catch {
      return undefined
    }
  }

  export async function build(options?: { entryPoints?: string[]; maxFunctions?: number }): Promise<Graph> {
    const projectID = Instance.project.id
    const now = Date.now()
    const maxFunctions = options?.maxFunctions ?? 500

    log.info("building call graph", { projectID })

    const nodes: CallNode[] = []
    const edges: CallEdge[] = []
    const incomingMap: Record<string, string[]> = {}
    const outgoingMap: Record<string, string[]> = {}
    const nodeMap = new Map<string, CallNode>()

    const codeIndex = await CodeIndex.load()
    const functions = options?.entryPoints
      ? codeIndex.functions.filter((f) => options.entryPoints!.some((ep) => f.file.includes(ep)))
      : codeIndex.functions.slice(0, maxFunctions)

    log.info("scanning functions for call relationships", { count: functions.length })

    for (const func of functions) {
      const absolutePath = path.resolve(Instance.worktree, func.file)

      if (!func.line || func.line < 1) continue

      const funcKind = resolveKind(func.type)
      const funcId = nodeId(func.name, func.file, func.line)

      if (!nodeMap.has(funcId)) {
        const callNode: CallNode = {
          id: funcId,
          name: func.name,
          kind: funcKind,
          file: func.file,
          line: func.line,
          character: 0,
          detail: func.signature,
        }
        nodes.push(callNode)
        nodeMap.set(funcId, callNode)
      }

      try {
        const outgoing = await LSP.outgoingCalls({
          file: absolutePath,
          line: func.line - 1,
          character: 0,
        })

        for (const call of outgoing) {
          const calleeNode = extractCallNode(call)
          if (!calleeNode) continue

          const calleeId = calleeNode.id

          if (!nodeMap.has(calleeId)) {
            nodes.push(calleeNode)
            nodeMap.set(calleeId, calleeNode)
          }

          const edgeId = `${funcId}->${calleeId}`
          const locations = extractCallLocations(call)

          edges.push({
            id: edgeId,
            caller: funcId,
            callee: calleeId,
            locations,
          })

          if (!outgoingMap[funcId]) outgoingMap[funcId] = []
          if (!outgoingMap[funcId].includes(calleeId)) {
            outgoingMap[funcId].push(calleeId)
          }

          if (!incomingMap[calleeId]) incomingMap[calleeId] = []
          if (!incomingMap[calleeId].includes(funcId)) {
            incomingMap[calleeId].push(funcId)
          }
        }
      } catch (error) {
        log.debug("failed to get outgoing calls", { func: func.name, error })
      }
    }

    const result: Graph = {
      projectID,
      nodes,
      edges,
      incomingMap,
      outgoingMap,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("call graph built", {
      nodesCount: nodes.length,
      edgesCount: edges.length,
    })

    await save(result)
    return result
  }

  function resolveKind(type: string): "function" | "method" | "constructor" {
    if (type === "method") return "method"
    if (type === "constructor") return "constructor"
    return "function"
  }

  function extractCallNode(lspCallItem: any): CallNode | undefined {
    try {
      const to = lspCallItem.to || lspCallItem
      if (!to?.uri && !to?.name) return undefined

      const uri = to.uri
      const name = to.name
      const range = to.range || to.selectionRange
      const line = range?.start?.line ?? 0
      const character = range?.start?.character ?? 0
      const kind = to.kind === 6 ? "method" : to.kind === 9 ? "constructor" : "function"

      let file: string
      if (uri) {
        const fullPath = fileURLToPath(uri)
        file = path.relative(Instance.worktree, fullPath).replace(/\\/g, "/")
      } else {
        file = "unknown"
      }

      const id = nodeId(name, file, line + 1)

      return {
        id,
        name,
        kind,
        file,
        line: line + 1,
        character,
        detail: to.detail,
      }
    } catch {
      return undefined
    }
  }

  function extractCallLocations(lspCallItem: any): Array<{ line: number; character: number }> {
    const fromRanges = lspCallItem.fromRanges || []
    return fromRanges.map((range: any) => ({
      line: (range.start?.line ?? 0) + 1,
      character: range.start?.character ?? 0,
    }))
  }

  export async function save(graph: Graph): Promise<void> {
    const projectID = Instance.project.id
    graph.time.updated = Date.now()
    await Storage.write(["memory", "knowledge", "call-graph", projectID], graph)
  }

  export async function load(): Promise<Graph> {
    const existing = await get()
    return existing ?? (await build())
  }

  export async function getCallers(input: {
    file: string
    line: number
    character?: number
  }): Promise<CallNode[]> {
    const absolutePath = path.resolve(Instance.worktree, input.file)
    const character = input.character ?? 0

    try {
      const incoming = await LSP.incomingCalls({
        file: absolutePath,
        line: input.line - 1,
        character,
      })

      return incoming.map(extractCallNode).filter(Boolean) as CallNode[]
    } catch (error) {
      log.debug("failed to get callers", { error })
      return []
    }
  }

  export async function getCallees(input: {
    file: string
    line: number
    character?: number
  }): Promise<CallNode[]> {
    const absolutePath = path.resolve(Instance.worktree, input.file)
    const character = input.character ?? 0

    try {
      const outgoing = await LSP.outgoingCalls({
        file: absolutePath,
        line: input.line - 1,
        character,
      })

      return outgoing.map(extractCallNode).filter(Boolean) as CallNode[]
    } catch (error) {
      log.debug("failed to get callees", { error })
      return []
    }
  }

  export async function analyzeCallChain(
    startNodeId: string,
    maxDepth = 5,
    direction: "incoming" | "outgoing" | "both" = "outgoing",
  ): Promise<CallChain> {
    const graph = await load()

    const visitedNodes: CallNode[] = []
    const visitedEdges: CallEdge[] = []
    const depthMap: Record<string, number> = {}
    const visited = new Set<string>()

    const startNode = graph.nodes.find((n) => n.id === startNodeId)
    if (startNode) {
      visitedNodes.push(startNode)
      depthMap[startNodeId] = 0
      visited.add(startNodeId)
    }

    const queue: Array<{ nodeId: string; depth: number }> = [{ nodeId: startNodeId, depth: 0 }]

    while (queue.length > 0) {
      const { nodeId: currentId, depth } = queue.shift()!

      if (depth >= maxDepth) continue

      const neighbors: string[] = []
      if (direction === "outgoing" || direction === "both") {
        neighbors.push(...(graph.outgoingMap[currentId] || []))
      }
      if (direction === "incoming" || direction === "both") {
        neighbors.push(...(graph.incomingMap[currentId] || []))
      }

      for (const neighborId of neighbors) {
        if (!visited.has(neighborId)) {
          visited.add(neighborId)

          const node = graph.nodes.find((n) => n.id === neighborId)
          if (node) {
            visitedNodes.push(node)
            depthMap[neighborId] = depth + 1
          }

          const edge = graph.edges.find(
            (e) => (e.caller === currentId && e.callee === neighborId) || (e.caller === neighborId && e.callee === currentId),
          )
          if (edge) {
            visitedEdges.push(edge)
          }

          queue.push({ nodeId: neighborId, depth: depth + 1 })
        }
      }
    }

    return {
      startNode: startNodeId,
      maxDepth,
      nodes: visitedNodes,
      edges: visitedEdges,
      depth: depthMap,
    }
  }

  export async function findHotspots(limit = 10): Promise<Array<{ node: CallNode; callerCount: number }>> {
    const graph = await load()

    const hotspots = graph.nodes
      .map((node) => ({
        node,
        callerCount: graph.incomingMap[node.id]?.length ?? 0,
      }))
      .filter((h) => h.callerCount > 0)
      .sort((a, b) => b.callerCount - a.callerCount)
      .slice(0, limit)

    return hotspots
  }

  export async function detectRecursion(): Promise<RecursionInfo[]> {
    const graph = await load()
    const recursions: RecursionInfo[] = []
    const visited = new Set<string>()

    for (const node of graph.nodes) {
      if (visited.has(node.id)) continue

      const directCallees = graph.outgoingMap[node.id] || []
      if (directCallees.includes(node.id)) {
        recursions.push({
          node,
          type: "direct",
          cycle: [node.id, node.id],
        })
        visited.add(node.id)
        continue
      }

      const cycle = findCycle(graph, node.id)
      if (cycle && cycle.length > 0) {
        recursions.push({
          node,
          type: "indirect",
          cycle,
        })
        for (const id of cycle) {
          visited.add(id)
        }
      }
    }

    return recursions
  }

  function findCycle(graph: Graph, startId: string): string[] | null {
    const visited = new Set<string>()
    const recursionStack = new Set<string>()
    const path: string[] = []

    function dfs(nodeId: string): string[] | null {
      visited.add(nodeId)
      recursionStack.add(nodeId)
      path.push(nodeId)

      const neighbors = graph.outgoingMap[nodeId] || []
      for (const neighborId of neighbors) {
        if (!visited.has(neighborId)) {
          const cycle = dfs(neighborId)
          if (cycle) return cycle
        } else if (recursionStack.has(neighborId)) {
          const cycleStart = path.indexOf(neighborId)
          if (cycleStart >= 0) {
            return [...path.slice(cycleStart), neighborId]
          }
        }
      }

      path.pop()
      recursionStack.delete(nodeId)
      return null
    }

    return dfs(startId)
  }

  export async function getNodeByName(name: string): Promise<CallNode | undefined> {
    const graph = await load()
    return graph.nodes.find((n) => n.name === name)
  }

  export async function getNodesByFile(file: string): Promise<CallNode[]> {
    const graph = await load()
    return graph.nodes.filter((n) => n.file === file || n.file.endsWith(file))
  }

  export async function getEdgesBetween(nodeAId: string, nodeBId: string): Promise<CallEdge[]> {
    const graph = await load()
    return graph.edges.filter(
      (e) =>
        (e.caller === nodeAId && e.callee === nodeBId) ||
        (e.caller === nodeBId && e.callee === nodeAId),
    )
  }

  export async function getStats(): Promise<{
    totalNodes: number
    totalEdges: number
    averageIncoming: number
    averageOutgoing: number
    maxIncoming: number
    maxOutgoing: number
  }> {
    const graph = await load()

    const incomingCounts = Object.values(graph.incomingMap).map((arr) => arr.length)
    const outgoingCounts = Object.values(graph.outgoingMap).map((arr) => arr.length)

    const avgIncoming = incomingCounts.length > 0 ? incomingCounts.reduce((a, b) => a + b, 0) / incomingCounts.length : 0
    const avgOutgoing = outgoingCounts.length > 0 ? outgoingCounts.reduce((a, b) => a + b, 0) / outgoingCounts.length : 0

    return {
      totalNodes: graph.nodes.length,
      totalEdges: graph.edges.length,
      averageIncoming: Math.round(avgIncoming * 100) / 100,
      averageOutgoing: Math.round(avgOutgoing * 100) / 100,
      maxIncoming: Math.max(0, ...incomingCounts),
      maxOutgoing: Math.max(0, ...outgoingCounts),
    }
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", "call-graph", projectID])
  }

  export async function toMermaid(options?: { maxNodes?: number; direction?: "LR" | "TD" }): Promise<string> {
    const graph = await load()
    const direction = options?.direction ?? "TD"
    const maxNodes = options?.maxNodes ?? 50

    const nodesToShow = graph.nodes.slice(0, maxNodes)
    const nodeIds = new Set(nodesToShow.map((n) => n.id))
    const edgesToShow = graph.edges.filter((e) => nodeIds.has(e.caller) && nodeIds.has(e.callee))

    const lines = [`graph ${direction}`]

    for (const node of nodesToShow) {
      const label = node.name.replace(/"/g, "'")
      const shape = node.kind === "method" ? `[${label}]` : `(${label})`
      lines.push(`  ${sanitizeMermaidId(node.id)}${shape}`)
    }

    for (const edge of edgesToShow) {
      lines.push(`  ${sanitizeMermaidId(edge.caller)} --> ${sanitizeMermaidId(edge.callee)}`)
    }

    return lines.join("\n")
  }

  function sanitizeMermaidId(id: string): string {
    return id.replace(/[^a-zA-Z0-9_]/g, "_").slice(0, 64)
  }
}
