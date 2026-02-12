/**
 * Performance Testing Utilities
 *
 * Provides helper functions for measuring time, memory, and asserting performance constraints.
 */

export interface TimingResult<T> {
  result: T
  duration: number
}

export interface MemoryResult<T> {
  result: T
  memoryDelta: number
  heapUsedBefore: number
  heapUsedAfter: number
}

/**
 * Measure execution time of an async function
 * @param fn - The async function to measure
 * @returns The result and duration in milliseconds
 */
export async function measureTime<T>(fn: () => Promise<T>): Promise<TimingResult<T>> {
  const start = performance.now()
  const result = await fn()
  const duration = performance.now() - start
  return { result, duration }
}

/**
 * Measure memory usage of an async function
 * @param fn - The async function to measure
 * @returns The result and memory delta in bytes
 */
export async function measureMemory<T>(fn: () => Promise<T>): Promise<MemoryResult<T>> {
  if (typeof Bun !== "undefined") {
    Bun.gc(true)
  }

  const heapUsedBefore = process.memoryUsage().heapUsed
  const result = await fn()

  if (typeof Bun !== "undefined") {
    Bun.gc(true)
  }

  const heapUsedAfter = process.memoryUsage().heapUsed
  const memoryDelta = heapUsedAfter - heapUsedBefore

  return { result, memoryDelta, heapUsedBefore, heapUsedAfter }
}

/**
 * Assert that a duration is within acceptable limits
 * @param duration - The measured duration in milliseconds
 * @param limit - The maximum acceptable duration in milliseconds
 * @param description - Optional description for error messages
 * @throws Error if duration exceeds limit
 */
export function assertPerformance(duration: number, limit: number, description?: string): void {
  if (duration > limit) {
    const desc = description ? ` (${description})` : ""
    throw new Error(`Performance assertion failed${desc}: ${duration.toFixed(2)}ms exceeded limit of ${limit}ms`)
  }
}

/**
 * Assert that memory usage is within acceptable limits
 * @param memoryDelta - The measured memory delta in bytes
 * @param limit - The maximum acceptable memory increase in bytes
 * @param description - Optional description for error messages
 * @throws Error if memory increase exceeds limit
 */
export function assertMemory(memoryDelta: number, limit: number, description?: string): void {
  if (memoryDelta > limit) {
    const desc = description ? ` (${description})` : ""
    const deltaKB = (memoryDelta / 1024).toFixed(2)
    const limitKB = (limit / 1024).toFixed(2)
    throw new Error(`Memory assertion failed${desc}: ${deltaKB}KB exceeded limit of ${limitKB}KB`)
  }
}

/**
 * Convert bytes to human-readable format
 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(2)}MB`
}

/**
 * Run a function multiple times and return average timing
 * @param fn - The async function to benchmark
 * @param iterations - Number of iterations (default 10)
 * @returns Average duration in milliseconds
 */
export async function benchmark<T>(fn: () => Promise<T>, iterations = 10): Promise<{ avg: number; min: number; max: number }> {
  const durations: number[] = []

  for (let i = 0; i < iterations; i++) {
    const { duration } = await measureTime(fn)
    durations.push(duration)
  }

  const sum = durations.reduce((a, b) => a + b, 0)
  return {
    avg: sum / iterations,
    min: Math.min(...durations),
    max: Math.max(...durations),
  }
}

/**
 * Create a mock function that simulates network delay
 */
export function withDelay<T>(fn: () => Promise<T>, delayMs: number): () => Promise<T> {
  return async () => {
    await new Promise((resolve) => setTimeout(resolve, delayMs))
    return fn()
  }
}
