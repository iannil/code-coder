/**
 * Execution time formatting and calculation utilities
 */

export function formatExecutionTime(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  if (ms < 3600000) {
    const minutes = Math.floor(ms / 60000)
    const seconds = Math.round((ms % 60000) / 1000)
    return `${minutes}m ${seconds}s`
  }
  const hours = Math.floor(ms / 3600000)
  const minutes = Math.floor((ms % 3600000) / 60000)
  return `${hours}h ${minutes}m`
}

export function formatPreciseTime(ms: number): string {
  if (ms < 1000) return `${ms.toFixed(0)}ms`
  if (ms < 10000) return `${(ms / 1000).toFixed(2)}s`
  return `${(ms / 1000).toFixed(1)}s`
}

export function getElapsedTime(start: number): number {
  return Date.now() - start
}

export function getElapsedTimeFromPart(
  part: { state: { status: string; time?: { start?: number; end?: number } } },
): number {
  const time = part.state.time
  if (!time) return 0
  const start = time.start ?? 0
  if (time.end) return time.end - start
  if (time.start) return getElapsedTime(start)
  return 0
}

export function isToolRunning(
  part: { state: { status: string; time?: { start?: number } } },
): boolean {
  return part.state.status === "running" || part.state.status === "pending"
}

export function getToolDuration(
  part: { state: { status: string; time?: { start?: number; end?: number } } },
): { elapsed: number; isRunning: boolean } {
  const time = part.state.time
  if (!time) return { elapsed: 0, isRunning: false }
  const isRunning = part.state.status === "running" || part.state.status === "pending"
  const start = time.start ?? 0
  if (time.end) return { elapsed: time.end - start, isRunning: false }
  if (time.start) return { elapsed: getElapsedTime(start), isRunning }
  return { elapsed: 0, isRunning: false }
}

export function getEstimatedTimeRemaining(
  start: number,
  progress?: number,
): number | undefined {
  if (!progress || progress <= 0 || progress >= 100) return undefined
  const elapsed = getElapsedTime(start)
  const estimatedTotal = (elapsed / progress) * 100
  return Math.max(0, estimatedTotal - elapsed)
}

export function getCompletionPercentage(
  part: { state: { status: string; time?: { start?: number; end?: number } } },
): number {
  if (part.state.status === "completed") return 100
  if (part.state.status === "error") return 100
  if (part.state.status === "pending") return 0
  if (part.state.status === "running") {
    const time = part.state.time
    if (!time?.start) return 0
    return 50
  }
  return 0
}
