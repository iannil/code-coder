/**
 * Chaos Engineering Utilities
 *
 * Tools for fault injection, network failure simulation, and recovery testing.
 * Used by chaos-engineering.eval.test.ts and other reliability tests.
 */

// ============================================================================
// Types
// ============================================================================

export type FaultType =
  | "network_timeout"
  | "network_disconnect"
  | "rate_limit"
  | "disk_full"
  | "memory_pressure"
  | "file_lock"
  | "corruption"
  | "partial_write"
  | "concurrent_edit"
  | "deadlock"

export interface FaultInjection {
  type: FaultType
  probability: number
  duration?: number
  targetOperation?: string
}

export interface RecoveryResult {
  recovered: boolean
  attempts: number
  duration: number
  finalState: "healthy" | "degraded" | "failed"
  errors: string[]
}

export interface ChaosSession {
  id: string
  faults: FaultInjection[]
  startTime: number
  endTime?: number
  results: FaultResult[]
}

export interface FaultResult {
  faultType: FaultType
  triggered: boolean
  recoveryResult?: RecoveryResult
  timestamp: number
}

// ============================================================================
// Fault Injection Simulator
// ============================================================================

export class FaultInjector {
  private activeFaults: Map<FaultType, FaultInjection> = new Map()
  private faultHistory: FaultResult[] = []

  /**
   * Register a fault for potential injection
   */
  registerFault(fault: FaultInjection): void {
    this.activeFaults.set(fault.type, fault)
  }

  /**
   * Remove a registered fault
   */
  removeFault(type: FaultType): void {
    this.activeFaults.delete(type)
  }

  /**
   * Clear all faults
   */
  clearFaults(): void {
    this.activeFaults.clear()
  }

  /**
   * Check if a fault should trigger based on probability
   */
  shouldTrigger(type: FaultType): boolean {
    const fault = this.activeFaults.get(type)
    if (!fault) return false
    return Math.random() < fault.probability
  }

  /**
   * Simulate a fault injection
   */
  async injectFault(type: FaultType): Promise<FaultResult> {
    const fault = this.activeFaults.get(type)
    const result: FaultResult = {
      faultType: type,
      triggered: false,
      timestamp: Date.now(),
    }

    if (!fault || !this.shouldTrigger(type)) {
      this.faultHistory.push(result)
      return result
    }

    result.triggered = true

    // Simulate fault duration
    if (fault.duration) {
      await new Promise((resolve) => setTimeout(resolve, fault.duration))
    }

    this.faultHistory.push(result)
    return result
  }

  /**
   * Get fault history
   */
  getHistory(): FaultResult[] {
    return [...this.faultHistory]
  }

  /**
   * Clear fault history
   */
  clearHistory(): void {
    this.faultHistory = []
  }
}

// ============================================================================
// Network Failure Simulator
// ============================================================================

export interface NetworkCondition {
  latencyMs: number
  packetLoss: number
  disconnectProbability: number
  timeoutProbability: number
}

export const NETWORK_CONDITIONS: Record<string, NetworkCondition> = {
  healthy: {
    latencyMs: 5,
    packetLoss: 0,
    disconnectProbability: 0,
    timeoutProbability: 0,
  },
  degraded: {
    latencyMs: 20,
    packetLoss: 0.05,
    disconnectProbability: 0.01,
    timeoutProbability: 0.02,
  },
  unstable: {
    latencyMs: 50,
    packetLoss: 0.15,
    disconnectProbability: 0.05,
    timeoutProbability: 0.1,
  },
  offline: {
    latencyMs: Infinity,
    packetLoss: 1,
    disconnectProbability: 1,
    timeoutProbability: 1,
  },
}

export class NetworkSimulator {
  private condition: NetworkCondition = NETWORK_CONDITIONS.healthy
  private requestCount = 0
  private failedRequests = 0

  /**
   * Set network condition
   */
  setCondition(condition: NetworkCondition | keyof typeof NETWORK_CONDITIONS): void {
    this.condition = typeof condition === "string" ? NETWORK_CONDITIONS[condition] : condition
  }

  /**
   * Simulate a network request
   */
  async simulateRequest<T>(
    operation: () => Promise<T>,
    options: { timeout?: number } = {},
  ): Promise<T> {
    this.requestCount++
    const timeout = options.timeout ?? 30000

    // Check for disconnect
    if (Math.random() < this.condition.disconnectProbability) {
      this.failedRequests++
      throw new NetworkError("Network disconnected", "disconnect")
    }

    // Check for timeout
    if (Math.random() < this.condition.timeoutProbability) {
      this.failedRequests++
      throw new NetworkError("Request timed out", "timeout")
    }

    // Check for packet loss
    if (Math.random() < this.condition.packetLoss) {
      this.failedRequests++
      throw new NetworkError("Packet lost", "packet_loss")
    }

    // Apply latency
    if (this.condition.latencyMs < Infinity) {
      await new Promise((resolve) => setTimeout(resolve, this.condition.latencyMs))
    } else {
      this.failedRequests++
      throw new NetworkError("Network offline", "offline")
    }

    // Execute actual operation with timeout
    return Promise.race([
      operation(),
      new Promise<never>((_, reject) =>
        setTimeout(() => reject(new NetworkError("Operation timed out", "timeout")), timeout),
      ),
    ])
  }

  /**
   * Get network statistics
   */
  getStats(): { requests: number; failed: number; successRate: number } {
    return {
      requests: this.requestCount,
      failed: this.failedRequests,
      successRate: this.requestCount > 0 ? (this.requestCount - this.failedRequests) / this.requestCount : 1,
    }
  }

  /**
   * Reset statistics
   */
  resetStats(): void {
    this.requestCount = 0
    this.failedRequests = 0
  }
}

export class NetworkError extends Error {
  constructor(
    message: string,
    public readonly type: "disconnect" | "timeout" | "packet_loss" | "offline",
  ) {
    super(message)
    this.name = "NetworkError"
  }
}

// ============================================================================
// Resource Pressure Simulator
// ============================================================================

export interface ResourceState {
  memoryUsedMB: number
  memoryLimitMB: number
  diskUsedGB: number
  diskLimitGB: number
  cpuPercent: number
}

export class ResourcePressureSimulator {
  private state: ResourceState = {
    memoryUsedMB: 100,
    memoryLimitMB: 1024,
    diskUsedGB: 10,
    diskLimitGB: 100,
    cpuPercent: 20,
  }

  /**
   * Set resource state
   */
  setState(state: Partial<ResourceState>): void {
    this.state = { ...this.state, ...state }
  }

  /**
   * Check if memory is exhausted
   */
  isMemoryExhausted(): boolean {
    return this.state.memoryUsedMB >= this.state.memoryLimitMB
  }

  /**
   * Check if disk is full
   */
  isDiskFull(): boolean {
    return this.state.diskUsedGB >= this.state.diskLimitGB
  }

  /**
   * Check if system is under pressure
   */
  isUnderPressure(): boolean {
    const memoryRatio = this.state.memoryUsedMB / this.state.memoryLimitMB
    const diskRatio = this.state.diskUsedGB / this.state.diskLimitGB
    return memoryRatio > 0.9 || diskRatio > 0.9 || this.state.cpuPercent > 90
  }

  /**
   * Simulate memory allocation
   */
  allocateMemory(sizeMB: number): { success: boolean; available: number } {
    const available = this.state.memoryLimitMB - this.state.memoryUsedMB
    if (sizeMB > available) {
      return { success: false, available }
    }
    this.state.memoryUsedMB += sizeMB
    return { success: true, available: available - sizeMB }
  }

  /**
   * Simulate disk write
   */
  writeToDisk(sizeGB: number): { success: boolean; available: number } {
    const available = this.state.diskLimitGB - this.state.diskUsedGB
    if (sizeGB > available) {
      return { success: false, available }
    }
    this.state.diskUsedGB += sizeGB
    return { success: true, available: available - sizeGB }
  }

  /**
   * Get current state
   */
  getState(): ResourceState {
    return { ...this.state }
  }

  /**
   * Reset to default state
   */
  reset(): void {
    this.state = {
      memoryUsedMB: 100,
      memoryLimitMB: 1024,
      diskUsedGB: 10,
      diskLimitGB: 100,
      cpuPercent: 20,
    }
  }
}

// ============================================================================
// Data Corruption Simulator
// ============================================================================

export type CorruptionType = "truncate" | "null_bytes" | "encoding" | "partial" | "scramble"

export interface CorruptedData {
  original: string
  corrupted: string
  corruptionType: CorruptionType
  corruptionStart: number
  corruptionLength: number
}

export class DataCorruptionSimulator {
  /**
   * Corrupt data with specified strategy
   */
  corrupt(data: string, type: CorruptionType, severity = 0.1): CorruptedData {
    const length = Math.floor(data.length * severity)
    const start = Math.floor(Math.random() * (data.length - length))

    let corrupted: string

    switch (type) {
      case "truncate":
        corrupted = data.slice(0, start)
        break
      case "null_bytes":
        corrupted = data.slice(0, start) + "\0".repeat(length) + data.slice(start + length)
        break
      case "encoding":
        corrupted = data.slice(0, start) + Buffer.from(data.slice(start, start + length)).toString("base64") + data.slice(start + length)
        break
      case "partial":
        corrupted = data.slice(0, start) + data.slice(start + length)
        break
      case "scramble":
        const scrambled = data.slice(start, start + length).split("").sort(() => Math.random() - 0.5).join("")
        corrupted = data.slice(0, start) + scrambled + data.slice(start + length)
        break
    }

    return {
      original: data,
      corrupted,
      corruptionType: type,
      corruptionStart: start,
      corruptionLength: length,
    }
  }

  /**
   * Check if data appears corrupted
   */
  isCorrupted(data: string): { corrupted: boolean; indicators: string[] } {
    const indicators: string[] = []

    if (data.includes("\0")) {
      indicators.push("null_bytes")
    }

    try {
      JSON.parse(data)
    } catch {
      if (data.startsWith("{") || data.startsWith("[")) {
        indicators.push("invalid_json")
      }
    }

    if (data.length === 0) {
      indicators.push("empty")
    }

    return {
      corrupted: indicators.length > 0,
      indicators,
    }
  }
}

// ============================================================================
// Concurrency Conflict Simulator
// ============================================================================

export interface ConcurrencyConflict {
  type: "write_write" | "read_write" | "deadlock"
  resource: string
  participants: string[]
  timestamp: number
}

export class ConcurrencySimulator {
  private locks: Map<string, string> = new Map()
  private conflicts: ConcurrencyConflict[] = []
  private waitingFor: Map<string, string> = new Map()

  /**
   * Attempt to acquire a lock
   */
  async acquireLock(
    resource: string,
    holder: string,
    timeout = 5000,
  ): Promise<{ acquired: boolean; conflict?: ConcurrencyConflict }> {
    const existingHolder = this.locks.get(resource)

    if (existingHolder && existingHolder !== holder) {
      // Check for deadlock
      this.waitingFor.set(holder, resource)
      if (this.detectDeadlock(holder)) {
        const conflict: ConcurrencyConflict = {
          type: "deadlock",
          resource,
          participants: [holder, existingHolder],
          timestamp: Date.now(),
        }
        this.conflicts.push(conflict)
        this.waitingFor.delete(holder)
        return { acquired: false, conflict }
      }

      // Wait for lock
      const startTime = Date.now()
      while (this.locks.has(resource) && Date.now() - startTime < timeout) {
        await new Promise((resolve) => setTimeout(resolve, 10))
      }

      this.waitingFor.delete(holder)

      if (this.locks.has(resource)) {
        const conflict: ConcurrencyConflict = {
          type: "write_write",
          resource,
          participants: [holder, existingHolder],
          timestamp: Date.now(),
        }
        this.conflicts.push(conflict)
        return { acquired: false, conflict }
      }
    }

    this.locks.set(resource, holder)
    return { acquired: true }
  }

  /**
   * Release a lock
   */
  releaseLock(resource: string, holder: string): boolean {
    if (this.locks.get(resource) === holder) {
      this.locks.delete(resource)
      return true
    }
    return false
  }

  /**
   * Detect deadlock using cycle detection
   */
  private detectDeadlock(start: string): boolean {
    const visited = new Set<string>()
    let current = start

    while (current) {
      if (visited.has(current)) {
        return true
      }
      visited.add(current)

      const waitingForResource = this.waitingFor.get(current)
      if (!waitingForResource) break

      const holder = this.locks.get(waitingForResource)
      if (!holder) break

      current = holder
    }

    return false
  }

  /**
   * Get all conflicts
   */
  getConflicts(): ConcurrencyConflict[] {
    return [...this.conflicts]
  }

  /**
   * Clear all state
   */
  reset(): void {
    this.locks.clear()
    this.conflicts = []
    this.waitingFor.clear()
  }
}

// ============================================================================
// Recovery Testing Utilities
// ============================================================================

export interface RetryConfig {
  maxAttempts: number
  baseDelayMs: number
  maxDelayMs: number
  backoffMultiplier: number
}

export const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 3,
  baseDelayMs: 100,
  maxDelayMs: 5000,
  backoffMultiplier: 2,
}

/**
 * Execute operation with retry and exponential backoff
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  config: Partial<RetryConfig> = {},
): Promise<{ result?: T; attempts: number; errors: string[] }> {
  const cfg = { ...DEFAULT_RETRY_CONFIG, ...config }
  const errors: string[] = []
  let delay = cfg.baseDelayMs

  for (let attempt = 1; attempt <= cfg.maxAttempts; attempt++) {
    try {
      const result = await operation()
      return { result, attempts: attempt, errors }
    } catch (error) {
      errors.push(error instanceof Error ? error.message : String(error))

      if (attempt < cfg.maxAttempts) {
        await new Promise((resolve) => setTimeout(resolve, delay))
        delay = Math.min(delay * cfg.backoffMultiplier, cfg.maxDelayMs)
      }
    }
  }

  return { attempts: cfg.maxAttempts, errors }
}

/**
 * Measure recovery time
 */
export async function measureRecoveryTime(
  failOperation: () => Promise<void>,
  checkRecovered: () => Promise<boolean>,
  maxWaitMs = 10000,
): Promise<{ recovered: boolean; duration: number }> {
  const startTime = Date.now()

  // Trigger failure
  try {
    await failOperation()
  } catch {
    // Expected failure
  }

  // Wait for recovery
  while (Date.now() - startTime < maxWaitMs) {
    try {
      if (await checkRecovered()) {
        return { recovered: true, duration: Date.now() - startTime }
      }
    } catch {
      // Not yet recovered
    }
    await new Promise((resolve) => setTimeout(resolve, 100))
  }

  return { recovered: false, duration: maxWaitMs }
}

// ============================================================================
// Chaos Session Manager
// ============================================================================

export function createChaosSession(faults: FaultInjection[]): ChaosSession {
  return {
    id: `chaos_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
    faults,
    startTime: Date.now(),
    results: [],
  }
}

export function endChaosSession(session: ChaosSession): ChaosSession {
  return {
    ...session,
    endTime: Date.now(),
  }
}
