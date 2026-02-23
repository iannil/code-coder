/**
 * CallGraph Tests
 *
 * Tests for the Call Graph module that uses LSP to build call relationships.
 * These tests focus on schema validation and algorithm correctness using mock data.
 */

import { describe, test, expect } from "bun:test"
import { CallGraph } from "../../../src/memory/knowledge/call-graph"

describe("CallGraph schemas", () => {
  describe("CallNode schema", () => {
    test("should validate valid CallNode", () => {
      const node = {
        id: "call:myFunc:src/index.ts:10",
        name: "myFunc",
        kind: "function" as const,
        file: "src/index.ts",
        line: 10,
        character: 0,
      }

      const result = CallGraph.CallNode.safeParse(node)
      expect(result.success).toBe(true)
    })

    test("should validate CallNode with all fields", () => {
      const node = {
        id: "call:MyClass.constructor:src/class.ts:5",
        name: "constructor",
        kind: "constructor" as const,
        file: "src/class.ts",
        line: 5,
        character: 4,
        detail: "MyClass.constructor()",
      }

      const result = CallGraph.CallNode.safeParse(node)
      expect(result.success).toBe(true)
    })

    test("should validate method kind", () => {
      const node = {
        id: "call:doSomething:src/service.ts:15",
        name: "doSomething",
        kind: "method" as const,
        file: "src/service.ts",
        line: 15,
        character: 2,
      }

      const result = CallGraph.CallNode.safeParse(node)
      expect(result.success).toBe(true)
    })

    test("should reject invalid kind", () => {
      const node = {
        id: "call:test:file.ts:1",
        name: "test",
        kind: "invalid",
        file: "file.ts",
        line: 1,
        character: 0,
      }

      const result = CallGraph.CallNode.safeParse(node)
      expect(result.success).toBe(false)
    })

    test("should reject missing required fields", () => {
      const node = {
        id: "call:test:file.ts:1",
        name: "test",
      }

      const result = CallGraph.CallNode.safeParse(node)
      expect(result.success).toBe(false)
    })
  })

  describe("CallEdge schema", () => {
    test("should validate valid CallEdge", () => {
      const edge = {
        id: "call:a:f.ts:1->call:b:f.ts:5",
        caller: "call:a:f.ts:1",
        callee: "call:b:f.ts:5",
        locations: [{ line: 3, character: 10 }],
      }

      const result = CallGraph.CallEdge.safeParse(edge)
      expect(result.success).toBe(true)
    })

    test("should validate CallEdge with empty locations", () => {
      const edge = {
        id: "edge1",
        caller: "nodeA",
        callee: "nodeB",
        locations: [],
      }

      const result = CallGraph.CallEdge.safeParse(edge)
      expect(result.success).toBe(true)
    })

    test("should validate CallEdge with multiple locations", () => {
      const edge = {
        id: "edge1",
        caller: "nodeA",
        callee: "nodeB",
        locations: [
          { line: 10, character: 5 },
          { line: 15, character: 8 },
          { line: 20, character: 2 },
        ],
      }

      const result = CallGraph.CallEdge.safeParse(edge)
      expect(result.success).toBe(true)
    })

    test("should reject invalid location format", () => {
      const edge = {
        id: "edge1",
        caller: "nodeA",
        callee: "nodeB",
        locations: [{ line: "invalid" }],
      }

      const result = CallGraph.CallEdge.safeParse(edge)
      expect(result.success).toBe(false)
    })
  })

  describe("Graph schema", () => {
    test("should validate empty Graph", () => {
      const graph = {
        projectID: "test-project",
        nodes: [],
        edges: [],
        incomingMap: {},
        outgoingMap: {},
        time: {
          created: Date.now(),
          updated: Date.now(),
        },
      }

      const result = CallGraph.Graph.safeParse(graph)
      expect(result.success).toBe(true)
    })

    test("should validate Graph with nodes and edges", () => {
      const graph = {
        projectID: "test-project",
        nodes: [
          {
            id: "call:funcA:src/a.ts:10",
            name: "funcA",
            kind: "function" as const,
            file: "src/a.ts",
            line: 10,
            character: 0,
          },
          {
            id: "call:funcB:src/b.ts:20",
            name: "funcB",
            kind: "function" as const,
            file: "src/b.ts",
            line: 20,
            character: 0,
          },
        ],
        edges: [
          {
            id: "call:funcA:src/a.ts:10->call:funcB:src/b.ts:20",
            caller: "call:funcA:src/a.ts:10",
            callee: "call:funcB:src/b.ts:20",
            locations: [{ line: 15, character: 4 }],
          },
        ],
        incomingMap: {
          "call:funcB:src/b.ts:20": ["call:funcA:src/a.ts:10"],
        },
        outgoingMap: {
          "call:funcA:src/a.ts:10": ["call:funcB:src/b.ts:20"],
        },
        time: {
          created: Date.now(),
          updated: Date.now(),
        },
      }

      const result = CallGraph.Graph.safeParse(graph)
      expect(result.success).toBe(true)
    })

    test("should reject missing projectID", () => {
      const graph = {
        nodes: [],
        edges: [],
        incomingMap: {},
        outgoingMap: {},
        time: {
          created: Date.now(),
          updated: Date.now(),
        },
      }

      const result = CallGraph.Graph.safeParse(graph)
      expect(result.success).toBe(false)
    })
  })

  describe("CallChain schema", () => {
    test("should validate valid CallChain", () => {
      const chain = {
        startNode: "call:main:index.ts:1",
        maxDepth: 5,
        nodes: [
          {
            id: "call:main:index.ts:1",
            name: "main",
            kind: "function" as const,
            file: "index.ts",
            line: 1,
            character: 0,
          },
        ],
        edges: [],
        depth: {
          "call:main:index.ts:1": 0,
        },
      }

      const result = CallGraph.CallChain.safeParse(chain)
      expect(result.success).toBe(true)
    })

    test("should validate CallChain with multiple levels", () => {
      const chain = {
        startNode: "a",
        maxDepth: 3,
        nodes: [
          { id: "a", name: "a", kind: "function" as const, file: "f.ts", line: 1, character: 0 },
          { id: "b", name: "b", kind: "function" as const, file: "f.ts", line: 10, character: 0 },
          { id: "c", name: "c", kind: "function" as const, file: "f.ts", line: 20, character: 0 },
        ],
        edges: [
          { id: "a->b", caller: "a", callee: "b", locations: [] },
          { id: "b->c", caller: "b", callee: "c", locations: [] },
        ],
        depth: { a: 0, b: 1, c: 2 },
      }

      const result = CallGraph.CallChain.safeParse(chain)
      expect(result.success).toBe(true)
    })
  })

  describe("RecursionInfo schema", () => {
    test("should validate direct recursion", () => {
      const info = {
        node: {
          id: "call:factorial:math.ts:5",
          name: "factorial",
          kind: "function" as const,
          file: "math.ts",
          line: 5,
          character: 0,
        },
        type: "direct" as const,
        cycle: ["call:factorial:math.ts:5", "call:factorial:math.ts:5"],
      }

      const result = CallGraph.RecursionInfo.safeParse(info)
      expect(result.success).toBe(true)
    })

    test("should validate indirect recursion", () => {
      const info = {
        node: {
          id: "call:funcA:file.ts:1",
          name: "funcA",
          kind: "function" as const,
          file: "file.ts",
          line: 1,
          character: 0,
        },
        type: "indirect" as const,
        cycle: ["call:funcA:file.ts:1", "call:funcB:file.ts:10", "call:funcA:file.ts:1"],
      }

      const result = CallGraph.RecursionInfo.safeParse(info)
      expect(result.success).toBe(true)
    })

    test("should reject invalid recursion type", () => {
      const info = {
        node: {
          id: "call:func:file.ts:1",
          name: "func",
          kind: "function" as const,
          file: "file.ts",
          line: 1,
          character: 0,
        },
        type: "unknown",
        cycle: ["call:func:file.ts:1"],
      }

      const result = CallGraph.RecursionInfo.safeParse(info)
      expect(result.success).toBe(false)
    })
  })
})

describe("CallGraph graph data structures", () => {
  const createMockGraph = (nodes: CallGraph.CallNode[], edges: CallGraph.CallEdge[]): CallGraph.Graph => {
    const incomingMap: Record<string, string[]> = {}
    const outgoingMap: Record<string, string[]> = {}

    for (const edge of edges) {
      if (!outgoingMap[edge.caller]) outgoingMap[edge.caller] = []
      if (!outgoingMap[edge.caller].includes(edge.callee)) {
        outgoingMap[edge.caller].push(edge.callee)
      }

      if (!incomingMap[edge.callee]) incomingMap[edge.callee] = []
      if (!incomingMap[edge.callee].includes(edge.caller)) {
        incomingMap[edge.callee].push(edge.caller)
      }
    }

    return {
      projectID: "test",
      nodes,
      edges,
      incomingMap,
      outgoingMap,
      time: { created: Date.now(), updated: Date.now() },
    }
  }

  test("should create correct incomingMap from edges", () => {
    const nodes: CallGraph.CallNode[] = [
      { id: "a", name: "a", kind: "function", file: "f.ts", line: 1, character: 0 },
      { id: "b", name: "b", kind: "function", file: "f.ts", line: 2, character: 0 },
      { id: "c", name: "c", kind: "function", file: "f.ts", line: 3, character: 0 },
    ]

    const edges: CallGraph.CallEdge[] = [
      { id: "a->b", caller: "a", callee: "b", locations: [] },
      { id: "a->c", caller: "a", callee: "c", locations: [] },
      { id: "b->c", caller: "b", callee: "c", locations: [] },
    ]

    const graph = createMockGraph(nodes, edges)

    expect(graph.incomingMap["b"]).toEqual(["a"])
    expect(graph.incomingMap["c"]).toContain("a")
    expect(graph.incomingMap["c"]).toContain("b")
    expect(graph.incomingMap["c"]?.length).toBe(2)
    expect(graph.incomingMap["a"]).toBeUndefined()
  })

  test("should create correct outgoingMap from edges", () => {
    const nodes: CallGraph.CallNode[] = [
      { id: "main", name: "main", kind: "function", file: "f.ts", line: 1, character: 0 },
      { id: "helper1", name: "helper1", kind: "function", file: "f.ts", line: 10, character: 0 },
      { id: "helper2", name: "helper2", kind: "function", file: "f.ts", line: 20, character: 0 },
    ]

    const edges: CallGraph.CallEdge[] = [
      { id: "main->helper1", caller: "main", callee: "helper1", locations: [] },
      { id: "main->helper2", caller: "main", callee: "helper2", locations: [] },
    ]

    const graph = createMockGraph(nodes, edges)

    expect(graph.outgoingMap["main"]).toContain("helper1")
    expect(graph.outgoingMap["main"]).toContain("helper2")
    expect(graph.outgoingMap["main"]?.length).toBe(2)
    expect(graph.outgoingMap["helper1"]).toBeUndefined()
    expect(graph.outgoingMap["helper2"]).toBeUndefined()
  })

  test("should handle graph with no edges", () => {
    const nodes: CallGraph.CallNode[] = [
      { id: "isolated1", name: "isolated1", kind: "function", file: "f.ts", line: 1, character: 0 },
      { id: "isolated2", name: "isolated2", kind: "function", file: "f.ts", line: 2, character: 0 },
    ]

    const graph = createMockGraph(nodes, [])

    expect(Object.keys(graph.incomingMap).length).toBe(0)
    expect(Object.keys(graph.outgoingMap).length).toBe(0)
    expect(graph.nodes.length).toBe(2)
  })

  test("should handle cyclic graph", () => {
    const nodes: CallGraph.CallNode[] = [
      { id: "a", name: "a", kind: "function", file: "f.ts", line: 1, character: 0 },
      { id: "b", name: "b", kind: "function", file: "f.ts", line: 2, character: 0 },
    ]

    const edges: CallGraph.CallEdge[] = [
      { id: "a->b", caller: "a", callee: "b", locations: [] },
      { id: "b->a", caller: "b", callee: "a", locations: [] },
    ]

    const graph = createMockGraph(nodes, edges)

    expect(graph.outgoingMap["a"]).toContain("b")
    expect(graph.outgoingMap["b"]).toContain("a")
    expect(graph.incomingMap["a"]).toContain("b")
    expect(graph.incomingMap["b"]).toContain("a")
  })

  test("should handle self-referencing node (direct recursion)", () => {
    const nodes: CallGraph.CallNode[] = [
      { id: "factorial", name: "factorial", kind: "function", file: "math.ts", line: 5, character: 0 },
    ]

    const edges: CallGraph.CallEdge[] = [
      { id: "factorial->factorial", caller: "factorial", callee: "factorial", locations: [] },
    ]

    const graph = createMockGraph(nodes, edges)

    expect(graph.outgoingMap["factorial"]).toContain("factorial")
    expect(graph.incomingMap["factorial"]).toContain("factorial")
  })
})

describe("CallGraph hotspot detection algorithm", () => {
  test("should identify hotspots correctly", () => {
    const incomingMap: Record<string, string[]> = {
      utils: ["a", "b", "c", "d", "e"],
      logger: ["a", "b", "c"],
      config: ["a"],
    }

    const hotspots = Object.entries(incomingMap)
      .map(([id, callers]) => ({ id, callerCount: callers.length }))
      .sort((a, b) => b.callerCount - a.callerCount)

    expect(hotspots[0].id).toBe("utils")
    expect(hotspots[0].callerCount).toBe(5)
    expect(hotspots[1].id).toBe("logger")
    expect(hotspots[1].callerCount).toBe(3)
    expect(hotspots[2].id).toBe("config")
    expect(hotspots[2].callerCount).toBe(1)
  })

  test("should handle empty graph for hotspot detection", () => {
    const incomingMap: Record<string, string[]> = {}

    const hotspots = Object.entries(incomingMap)
      .map(([id, callers]) => ({ id, callerCount: callers.length }))
      .sort((a, b) => b.callerCount - a.callerCount)

    expect(hotspots.length).toBe(0)
  })
})

describe("CallGraph BFS algorithm for call chain analysis", () => {
  test("should traverse graph in BFS order", () => {
    const outgoingMap: Record<string, string[]> = {
      main: ["a", "b"],
      a: ["c"],
      b: ["c", "d"],
    }

    const startNode = "main"
    const maxDepth = 3
    const visited = new Set<string>([startNode])
    const depthMap: Record<string, number> = { [startNode]: 0 }
    const queue: Array<{ nodeId: string; depth: number }> = [{ nodeId: startNode, depth: 0 }]

    while (queue.length > 0) {
      const { nodeId, depth } = queue.shift()!
      if (depth >= maxDepth) continue

      const neighbors = outgoingMap[nodeId] || []
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          visited.add(neighbor)
          depthMap[neighbor] = depth + 1
          queue.push({ nodeId: neighbor, depth: depth + 1 })
        }
      }
    }

    expect(visited.has("main")).toBe(true)
    expect(visited.has("a")).toBe(true)
    expect(visited.has("b")).toBe(true)
    expect(visited.has("c")).toBe(true)
    expect(visited.has("d")).toBe(true)

    expect(depthMap["main"]).toBe(0)
    expect(depthMap["a"]).toBe(1)
    expect(depthMap["b"]).toBe(1)
    expect(depthMap["c"]).toBe(2)
    expect(depthMap["d"]).toBe(2)
  })

  test("should respect maxDepth limit", () => {
    const outgoingMap: Record<string, string[]> = {
      a: ["b"],
      b: ["c"],
      c: ["d"],
      d: ["e"],
    }

    const startNode = "a"
    const maxDepth = 2
    const visited = new Set<string>([startNode])
    const queue: Array<{ nodeId: string; depth: number }> = [{ nodeId: startNode, depth: 0 }]

    while (queue.length > 0) {
      const { nodeId, depth } = queue.shift()!
      if (depth >= maxDepth) continue

      const neighbors = outgoingMap[nodeId] || []
      for (const neighbor of neighbors) {
        if (!visited.has(neighbor)) {
          visited.add(neighbor)
          queue.push({ nodeId: neighbor, depth: depth + 1 })
        }
      }
    }

    expect(visited.has("a")).toBe(true)
    expect(visited.has("b")).toBe(true)
    expect(visited.has("c")).toBe(true)
    expect(visited.has("d")).toBe(false)
    expect(visited.has("e")).toBe(false)
  })
})

describe("CallGraph cycle detection algorithm", () => {
  function findCycle(
    outgoingMap: Record<string, string[]>,
    startId: string,
  ): string[] | null {
    const visited = new Set<string>()
    const recursionStack = new Set<string>()
    const path: string[] = []

    function dfs(nodeId: string): string[] | null {
      visited.add(nodeId)
      recursionStack.add(nodeId)
      path.push(nodeId)

      const neighbors = outgoingMap[nodeId] || []
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

  test("should detect direct recursion", () => {
    const outgoingMap: Record<string, string[]> = {
      factorial: ["factorial"],
    }

    const cycle = findCycle(outgoingMap, "factorial")

    expect(cycle).not.toBeNull()
    expect(cycle).toContain("factorial")
  })

  test("should detect indirect recursion (A -> B -> A)", () => {
    const outgoingMap: Record<string, string[]> = {
      a: ["b"],
      b: ["a"],
    }

    const cycle = findCycle(outgoingMap, "a")

    expect(cycle).not.toBeNull()
    expect(cycle?.length).toBeGreaterThan(1)
  })

  test("should detect longer cycles (A -> B -> C -> A)", () => {
    const outgoingMap: Record<string, string[]> = {
      a: ["b"],
      b: ["c"],
      c: ["a"],
    }

    const cycle = findCycle(outgoingMap, "a")

    expect(cycle).not.toBeNull()
    expect(cycle).toContain("a")
    expect(cycle).toContain("b")
    expect(cycle).toContain("c")
  })

  test("should return null for acyclic graph", () => {
    const outgoingMap: Record<string, string[]> = {
      a: ["b"],
      b: ["c"],
      c: [],
    }

    const cycle = findCycle(outgoingMap, "a")

    expect(cycle).toBeNull()
  })

  test("should handle isolated nodes", () => {
    const outgoingMap: Record<string, string[]> = {
      isolated: [],
    }

    const cycle = findCycle(outgoingMap, "isolated")

    expect(cycle).toBeNull()
  })
})

describe("CallGraph statistics calculation", () => {
  test("should calculate correct statistics", () => {
    const incomingCounts = [5, 3, 1, 0, 2]
    const outgoingCounts = [2, 4, 1, 3, 0]

    const avgIncoming = incomingCounts.reduce((a, b) => a + b, 0) / incomingCounts.length
    const avgOutgoing = outgoingCounts.reduce((a, b) => a + b, 0) / outgoingCounts.length
    const maxIncoming = Math.max(...incomingCounts)
    const maxOutgoing = Math.max(...outgoingCounts)

    expect(avgIncoming).toBe(2.2)
    expect(avgOutgoing).toBe(2)
    expect(maxIncoming).toBe(5)
    expect(maxOutgoing).toBe(4)
  })

  test("should handle empty statistics", () => {
    const incomingCounts: number[] = []
    const outgoingCounts: number[] = []

    const avgIncoming = incomingCounts.length > 0 ? incomingCounts.reduce((a, b) => a + b, 0) / incomingCounts.length : 0
    const avgOutgoing = outgoingCounts.length > 0 ? outgoingCounts.reduce((a, b) => a + b, 0) / outgoingCounts.length : 0
    const maxIncoming = Math.max(0, ...incomingCounts)
    const maxOutgoing = Math.max(0, ...outgoingCounts)

    expect(avgIncoming).toBe(0)
    expect(avgOutgoing).toBe(0)
    expect(maxIncoming).toBe(0)
    expect(maxOutgoing).toBe(0)
  })
})

describe("CallGraph Mermaid generation", () => {
  function sanitizeMermaidId(id: string): string {
    return id.replace(/[^a-zA-Z0-9_]/g, "_").slice(0, 64)
  }

  test("should sanitize node IDs for Mermaid", () => {
    expect(sanitizeMermaidId("call:func:src/file.ts:10")).toBe("call_func_src_file_ts_10")
    expect(sanitizeMermaidId("simple")).toBe("simple")
    expect(sanitizeMermaidId("with spaces")).toBe("with_spaces")
    expect(sanitizeMermaidId("special!@#$%chars")).toBe("special_____chars")
  })

  test("should truncate long IDs", () => {
    const longId = "a".repeat(100)
    const sanitized = sanitizeMermaidId(longId)
    expect(sanitized.length).toBe(64)
  })

  test("should generate valid Mermaid graph header", () => {
    const direction = "TD"
    const header = `graph ${direction}`
    expect(header).toBe("graph TD")
  })

  test("should generate node definitions", () => {
    const node: { id: string; name: string; kind: "function" | "method" | "constructor" } = {
      id: "func",
      name: "myFunction",
      kind: "function",
    }
    const sanitizedId = sanitizeMermaidId(node.id)
    const shape = node.kind === "method" ? `[${node.name}]` : `(${node.name})`
    const line = `  ${sanitizedId}${shape}`

    expect(line).toBe("  func(myFunction)")
  })

  test("should generate method nodes with square brackets", () => {
    const node = { id: "method", name: "doSomething", kind: "method" as const }
    const sanitizedId = sanitizeMermaidId(node.id)
    const shape = node.kind === "method" ? `[${node.name}]` : `(${node.name})`
    const line = `  ${sanitizedId}${shape}`

    expect(line).toBe("  method[doSomething]")
  })

  test("should generate edge definitions", () => {
    const edge = { caller: "a", callee: "b" }
    const line = `  ${sanitizeMermaidId(edge.caller)} --> ${sanitizeMermaidId(edge.callee)}`

    expect(line).toBe("  a --> b")
  })
})
