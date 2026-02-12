import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { CodeIndex } from "./code-index"
import z from "zod"

const log = Log.create({ service: "memory.knowledge.semantic-graph" })

export namespace SemanticGraph {
  export const Node = z.object({
    id: z.string(),
    type: z.enum(["function", "class", "interface", "type", "enum", "component", "file", "module"]),
    name: z.string(),
    file: z.string(),
    metadata: z.record(z.string(), z.any()).optional(),
  })
  export type Node = z.infer<typeof Node>

  export const Edge = z.object({
    id: z.string(),
    source: z.string(),
    target: z.string(),
    type: z.enum([
      "imports",
      "exports",
      "extends",
      "implements",
      "calls",
      "instantiates",
      "references",
      "contains",
      "related",
    ]),
    weight: z.number().min(0).max(1),
    metadata: z.record(z.string(), z.any()).optional(),
  })
  export type Edge = z.infer<typeof Edge>

  export const Graph = z.object({
    projectID: z.string(),
    nodes: z.array(Node),
    edges: z.array(Edge),
    adjacency: z.record(z.string(), z.array(z.string())),
    reverseAdjacency: z.record(z.string(), z.array(z.string())),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Graph = z.infer<typeof Graph>

  const nodeId = (type: string, name: string, file: string) => `${type}:${name}:${file}`

  export async function get(): Promise<Graph | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Graph>(["memory", "knowledge", "semantic-graph", projectID])
    } catch {
      return undefined
    }
  }

  export async function build(): Promise<Graph> {
    const projectID = Instance.project.id
    const now = Date.now()

    log.info("building semantic graph", { projectID })

    const codeIndex = await CodeIndex.load()

    const nodes: Node[] = []
    const edges: Edge[] = []
    const adjacency: Record<string, string[]> = {}
    const reverseAdjacency: Record<string, string[]> = {}

    for (const func of codeIndex.functions) {
      const id = nodeId("function", func.name, func.file)
      nodes.push({
        id,
        type: "function",
        name: func.name,
        file: func.file,
        metadata: {
          signature: func.signature,
          exported: func.exported,
        },
      })
    }

    for (const cls of codeIndex.classes) {
      const id = nodeId("class", cls.name, cls.file)
      nodes.push({
        id,
        type: "class",
        name: cls.name,
        file: cls.file,
        metadata: {
          extends: cls.extends,
          methods: cls.methods,
          properties: cls.properties,
        },
      })

      if (cls.extends) {
        const extendsId = nodeId("class", cls.extends, cls.file)
        addEdge(edges, adjacency, reverseAdjacency, {
          id: `${id}:extends:${extendsId}`,
          source: id,
          target: extendsId,
          type: "extends",
          weight: 0.9,
        })
      }

      if (cls.implements) {
        for (const iface of cls.implements) {
          const ifaceId = nodeId("interface", iface, cls.file)
          addEdge(edges, adjacency, reverseAdjacency, {
            id: `${id}:implements:${ifaceId}`,
            source: id,
            target: ifaceId,
            type: "implements",
            weight: 0.8,
          })
        }
      }
    }

    for (const iface of codeIndex.interfaces) {
      const id = nodeId("interface", iface.name, iface.file)
      nodes.push({
        id,
        type: "interface",
        name: iface.name,
        file: iface.file,
        metadata: {
          exported: iface.exported,
        },
      })

      if (iface.extends) {
        for (const parent of iface.extends) {
          const parentId = nodeId("interface", parent, iface.file)
          addEdge(edges, adjacency, reverseAdjacency, {
            id: `${id}:extends:${parentId}`,
            source: id,
            target: parentId,
            type: "extends",
            weight: 0.9,
          })
        }
      }
    }

    for (const typeInfo of codeIndex.types) {
      const id = nodeId(typeInfo.kind, typeInfo.name, typeInfo.file)
      nodes.push({
        id,
        type: typeInfo.kind,
        name: typeInfo.name,
        file: typeInfo.file,
        metadata: {
          definition: typeInfo.definition,
        },
      })
    }

    for (const imp of codeIndex.imports) {
      const fileId = nodeId("file", imp.file, imp.file)

      if (!nodes.find((n) => n.id === fileId)) {
        nodes.push({
          id: fileId,
          type: "file",
          name: imp.file,
          file: imp.file,
        })
      }

      for (const imported of imp.imports) {
        const targetId = nodeId("unknown", imported, imp.source)

        addEdge(edges, adjacency, reverseAdjacency, {
          id: `${fileId}:imports:${targetId}`,
          source: fileId,
          target: targetId,
          type: "imports",
          weight: 0.7,
        })
      }
    }

    for (const exp of codeIndex.exports) {
      const fileId = nodeId("file", exp.file, exp.file)
      const targetId = nodeId("unknown", exp.name, exp.file)

      addEdge(edges, adjacency, reverseAdjacency, {
        id: `${fileId}:exports:${targetId}`,
        source: fileId,
        target: targetId,
        type: "exports",
        weight: 0.7,
      })
    }

    const result: Graph = {
      projectID,
      nodes,
      edges,
      adjacency,
      reverseAdjacency,
      time: {
        created: now,
        updated: now,
      },
    }

    log.info("semantic graph built", {
      nodesCount: nodes.length,
      edgesCount: edges.length,
    })

    await save(result)
    return result
  }

  function addEdge(
    edges: Edge[],
    adjacency: Record<string, string[]>,
    reverseAdjacency: Record<string, string[]>,
    edge: Edge,
  ): void {
    edges.push(edge)

    if (!adjacency[edge.source]) adjacency[edge.source] = []
    if (!adjacency[edge.source].includes(edge.target)) {
      adjacency[edge.source].push(edge.target)
    }

    if (!reverseAdjacency[edge.target]) reverseAdjacency[edge.target] = []
    if (!reverseAdjacency[edge.target].includes(edge.source)) {
      reverseAdjacency[edge.target].push(edge.source)
    }
  }

  export async function save(graph: Graph): Promise<void> {
    const projectID = Instance.project.id
    graph.time.updated = Date.now()
    await Storage.write(["memory", "knowledge", "semantic-graph", projectID], graph)
  }

  export async function load(): Promise<Graph> {
    let existing = await get()
    if (!existing) {
      existing = await build()
    }
    return existing
  }

  export async function findRelatedNodes(nodeId: string, maxDepth = 2): Promise<Node[]> {
    const graph = await load()
    const visited = new Set<string>([nodeId])
    const queue: [string, number][] = [[nodeId, 0]]
    const related: Node[] = []

    while (queue.length > 0) {
      const [currentId, depth] = queue.shift()!

      if (depth >= maxDepth) continue

      const neighbors = graph.adjacency[currentId] || []
      for (const neighborId of neighbors) {
        if (!visited.has(neighborId)) {
          visited.add(neighborId)
          const node = graph.nodes.find((n) => n.id === neighborId)
          if (node) related.push(node)
          queue.push([neighborId, depth + 1])
        }
      }

      const reverseNeighbors = graph.reverseAdjacency[currentId] || []
      for (const neighborId of reverseNeighbors) {
        if (!visited.has(neighborId)) {
          visited.add(neighborId)
          const node = graph.nodes.find((n) => n.id === neighborId)
          if (node) related.push(node)
          queue.push([neighborId, depth + 1])
        }
      }
    }

    return related
  }

  export async function findPath(fromId: string, toId: string): Promise<string[] | null> {
    const graph = await load()

    const visited = new Set<string>()
    const queue: [string, string[]][] = [[fromId, []]]
    visited.add(fromId)

    while (queue.length > 0) {
      const [currentId, path] = queue.shift()!

      if (currentId === toId) {
        return [...path, toId]
      }

      const neighbors = graph.adjacency[currentId] || []
      for (const neighborId of neighbors) {
        if (!visited.has(neighborId)) {
          visited.add(neighborId)
          queue.push([neighborId, [...path, currentId]])
        }
      }
    }

    return null
  }

  export async function findShortestPath(fromId: string, toId: string): Promise<string[] | null> {
    const graph = await load()

    const visited = new Set<string>()
    const queue: [string, string[]][] = [[fromId, []]]
    visited.add(fromId)

    while (queue.length > 0) {
      const [currentId, path] = queue.shift()!

      if (currentId === toId) {
        return [...path, toId]
      }

      const neighbors = graph.adjacency[currentId] || []
      for (const neighborId of neighbors) {
        if (!visited.has(neighborId)) {
          visited.add(neighborId)
          const newPath = [...path, currentId]
          if (neighborId === toId) {
            return [...newPath, toId]
          }
          queue.push([neighborId, newPath])
        }
      }
    }

    return null
  }

  export async function getCallGraph(functionName: string): Promise<{
    calls: string[]
    calledBy: string[]
  }> {
    const graph = await load()
    const funcId = graph.nodes.find((n) => n.name === functionName && n.type === "function")?.id

    if (!funcId) {
      return { calls: [], calledBy: [] }
    }

    return {
      calls: graph.adjacency[funcId] || [],
      calledBy: graph.reverseAdjacency[funcId] || [],
    }
  }

  export async function getDependencyGraph(moduleId: string): Promise<{
    dependsOn: string[]
    dependents: string[]
  }> {
    const graph = await load()

    return {
      dependsOn: graph.adjacency[moduleId] || [],
      dependents: graph.reverseAdjacency[moduleId] || [],
    }
  }

  export async function findCycles(): Promise<string[][]> {
    const graph = await load()
    const cycles: string[][] = []
    const visited = new Set<string>()
    const recursionStack = new Set<string>()

    function dfs(nodeId: string, path: string[]): void {
      visited.add(nodeId)
      recursionStack.add(nodeId)
      path.push(nodeId)

      const neighbors = graph.adjacency[nodeId] || []
      for (const neighborId of neighbors) {
        if (!visited.has(neighborId)) {
          dfs(neighborId, [...path])
        } else if (recursionStack.has(neighborId)) {
          const cycleStart = path.indexOf(neighborId)
          if (cycleStart >= 0) {
            cycles.push([...path.slice(cycleStart), neighborId])
          }
        }
      }

      recursionStack.delete(nodeId)
    }

    for (const node of graph.nodes) {
      if (!visited.has(node.id)) {
        dfs(node.id, [])
      }
    }

    return cycles
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", "semantic-graph", projectID])
  }
}
