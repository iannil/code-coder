/**
 * Native Graph Bindings (Fail-Fast Mode)
 *
 * Provides native Rust implementations for graph operations.
 * Throws error if native bindings are unavailable - no fallback.
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "memory.knowledge.native" })

// ============================================================================
// Type Definitions (must match NAPI types)
// ============================================================================

export interface NapiDecisionNode {
  id: string
  sessionId: string
  agentId: string
  prompt: string
  reasoning: string
  confidence: number
  timestamp: string
  context: string | null
}

export interface NapiActionNode {
  id: string
  decisionId: string
  actionType: string
  description: string
  input: string | null
  output: string | null
  timestamp: string
  duration: number | null
}

export interface NapiOutcomeNode {
  id: string
  actionId: string
  status: "success" | "partial" | "failure"
  description: string
  metrics: string | null
  feedback: string | null
  timestamp: string
}

export interface NapiCausalChain {
  decision: NapiDecisionNode
  actions: NapiActionNode[]
  outcomes: NapiOutcomeNode[]
}

export interface NapiCausalQuery {
  agentId?: string
  sessionId?: string
  actionType?: string
  status?: string
  minConfidence?: number
  dateFrom?: string
  dateTo?: string
  limit?: number
}

export interface NapiCausalStats {
  totalDecisions: number
  totalActions: number
  totalOutcomes: number
  totalEdges: number
  successRate: number
  avgConfidence: number
}

export interface NapiCallNode {
  id: string
  name: string
  kind: "function" | "method" | "constructor"
  file: string
  line: number
  character: number
  detail: string | null
}

export interface NapiRecursionInfo {
  node: NapiCallNode
  recursionType: "direct" | "indirect"
  cycle: string[]
}

export interface NapiSemanticNode {
  id: string
  nodeType: string
  name: string
  file: string
  metadata: string | null
}

export interface NapiSemanticStats {
  totalNodes: number
  totalEdges: number
  files: number
  functions: number
  classes: number
  interfaces: number
  hasCycles: boolean
}

export interface NapiPathResult {
  found: boolean
  path: string[]
  nodesVisited: number
}

export interface NapiCycleResult {
  hasCycles: boolean
  cycles: string[][]
}

// ============================================================================
// Handle Interfaces
// ============================================================================

export interface CausalGraphHandle {
  recordDecision(
    sessionId: string,
    agentId: string,
    prompt: string,
    reasoning: string,
    confidence: number,
    context: string | null,
  ): NapiDecisionNode
  recordAction(
    decisionId: string,
    actionType: string,
    description: string,
    input: string | null,
    output: string | null,
    duration: number | null,
  ): NapiActionNode
  recordOutcome(
    actionId: string,
    status: string,
    description: string,
    metrics: string | null,
    feedback: string | null,
  ): NapiOutcomeNode
  getDecision(id: string): NapiDecisionNode | null
  getDecisions(): NapiDecisionNode[]
  getCausalChain(decisionId: string): NapiCausalChain | null
  query(query: NapiCausalQuery): NapiCausalChain[]
  getSuccessRate(agentId: string | null): number
  getStats(): NapiCausalStats
  hasCycles(): boolean
  toJson(): string
}

export interface CallGraphHandle {
  addFunction(
    name: string,
    file: string,
    line: number,
    character: number,
    kind: string,
    detail: string | null,
  ): NapiCallNode
  addCall(callerId: string, calleeId: string, line: number, character: number): string | null
  getNode(id: string): NapiCallNode | null
  getNodes(): NapiCallNode[]
  getCallers(calleeId: string): NapiCallNode[]
  getCallees(callerId: string): NapiCallNode[]
  detectRecursion(): NapiRecursionInfo[]
  getEntryPoints(): NapiCallNode[]
  getLeafFunctions(): NapiCallNode[]
  toJson(): string
}

export interface SemanticGraphHandle {
  addFunction(name: string, file: string, signature: string | null, exported: boolean): NapiSemanticNode
  addClass(name: string, file: string, extendsClass: string | null, methods: string[]): NapiSemanticNode
  addInterface(name: string, file: string, extendsInterface: string | null): NapiSemanticNode
  addFile(path: string): NapiSemanticNode
  addImport(importer: string, imported: string): string | null
  addExtends(child: string, parent: string): string | null
  addImplements(implementor: string, interfaceId: string): string | null
  addContains(container: string, contained: string): string | null
  getNode(id: string): NapiSemanticNode | null
  getNodes(): NapiSemanticNode[]
  getNodesByType(nodeType: string): NapiSemanticNode[]
  getImports(entityId: string): NapiSemanticNode[]
  getImporters(entityId: string): NapiSemanticNode[]
  getInheritanceChain(classId: string): NapiSemanticNode[]
  hasCircularDependencies(): boolean
  stats(): NapiSemanticStats
  toJson(): string
}

export interface GraphEngineHandle {
  addNode(id: string, nodeType: string, payload: string): string
  getNode(id: string): { id: string; nodeType: string; payload: string } | null
  containsNode(id: string): boolean
  addEdge(source: string, target: string, relationship: string): string | null
  getSuccessors(id: string): string[]
  getPredecessors(id: string): string[]
  nodeCount(): number
  edgeCount(): number
  bfs(start: string): string[]
  dfs(start: string): string[]
  findPath(from: string, to: string): NapiPathResult
  hasCycles(): boolean
  detectCycles(): NapiCycleResult
  topologicalSort(): string[]
  getReachable(start: string, maxDepth: number): string[]
  toJson(): string
  clear(): void
}

// ============================================================================
// Native Bindings Interface
// ============================================================================

interface NativeGraphBindings {
  CausalGraphHandle: new (projectId: string) => CausalGraphHandle
  CallGraphHandle: new (projectId: string) => CallGraphHandle
  SemanticGraphHandle: new (projectId: string) => SemanticGraphHandle
  GraphEngineHandle: new () => GraphEngineHandle
}

// ============================================================================
// Native Bindings Loader (Fail-Fast)
// ============================================================================

let nativeBindings: NativeGraphBindings | null = null
let loadAttempted = false

/**
 * Load native graph bindings. Throws if unavailable.
 * @throws Error if native bindings cannot be loaded
 */
async function loadNativeBindings(): Promise<NativeGraphBindings> {
  if (loadAttempted && nativeBindings) return nativeBindings

  try {
    const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>

    if (
      typeof bindings.CausalGraphHandle === "function" &&
      typeof bindings.CallGraphHandle === "function" &&
      typeof bindings.SemanticGraphHandle === "function" &&
      typeof bindings.GraphEngineHandle === "function"
    ) {
      nativeBindings = bindings as unknown as NativeGraphBindings
      log.info("Loaded native graph bindings")
      loadAttempted = true
      return nativeBindings
    }
  } catch (e) {
    loadAttempted = true
    throw new Error(`Native bindings required: @codecoder-ai/core graph functions not available: ${e}`)
  }

  loadAttempted = true
  throw new Error("Native bindings required: @codecoder-ai/core graph functions not available")
}

// ============================================================================
// Public API (Fail-Fast)
// ============================================================================

/**
 * Check if native graph bindings are available
 */
export async function isNativeAvailable(): Promise<boolean> {
  try {
    await loadNativeBindings()
    return true
  } catch {
    return false
  }
}

/**
 * Create a new CausalGraph.
 * @throws Error if native bindings unavailable
 */
export async function createCausalGraph(projectId: string): Promise<CausalGraphHandle> {
  const bindings = await loadNativeBindings()
  return new bindings.CausalGraphHandle(projectId)
}

/**
 * Create a new CallGraph.
 * @throws Error if native bindings unavailable
 */
export async function createCallGraph(projectId: string): Promise<CallGraphHandle> {
  const bindings = await loadNativeBindings()
  return new bindings.CallGraphHandle(projectId)
}

/**
 * Create a new SemanticGraph.
 * @throws Error if native bindings unavailable
 */
export async function createSemanticGraph(projectId: string): Promise<SemanticGraphHandle> {
  const bindings = await loadNativeBindings()
  return new bindings.SemanticGraphHandle(projectId)
}

/**
 * Create a new GraphEngine.
 * @throws Error if native bindings unavailable
 */
export async function createGraphEngine(): Promise<GraphEngineHandle> {
  const bindings = await loadNativeBindings()
  return new bindings.GraphEngineHandle()
}

/**
 * Restore a CausalGraph from JSON.
 * @throws Error if native bindings unavailable
 */
export async function restoreCausalGraphFromJson(json: string): Promise<CausalGraphHandle> {
  const bindings = await loadNativeBindings()
  const CausalGraphHandleClass = bindings.CausalGraphHandle as unknown as {
    fromJson(json: string): CausalGraphHandle
  }
  if (typeof CausalGraphHandleClass.fromJson !== "function") {
    throw new Error("CausalGraphHandle.fromJson is not available")
  }
  return CausalGraphHandleClass.fromJson(json)
}

/**
 * Restore a CallGraph from JSON.
 * @throws Error if native bindings unavailable
 */
export async function restoreCallGraphFromJson(json: string): Promise<CallGraphHandle> {
  const bindings = await loadNativeBindings()
  const CallGraphHandleClass = bindings.CallGraphHandle as unknown as {
    fromJson(json: string): CallGraphHandle
  }
  if (typeof CallGraphHandleClass.fromJson !== "function") {
    throw new Error("CallGraphHandle.fromJson is not available")
  }
  return CallGraphHandleClass.fromJson(json)
}

/**
 * Restore a SemanticGraph from JSON.
 * @throws Error if native bindings unavailable
 */
export async function restoreSemanticGraphFromJson(json: string): Promise<SemanticGraphHandle> {
  const bindings = await loadNativeBindings()
  const SemanticGraphHandleClass = bindings.SemanticGraphHandle as unknown as {
    fromJson(json: string): SemanticGraphHandle
  }
  if (typeof SemanticGraphHandleClass.fromJson !== "function") {
    throw new Error("SemanticGraphHandle.fromJson is not available")
  }
  return SemanticGraphHandleClass.fromJson(json)
}
