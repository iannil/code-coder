import * as StructuredLog from "./structured-log"

export function branch(condition: string, value: boolean, context?: Record<string, unknown>): void {
  if (!StructuredLog.isEnabled()) return

  StructuredLog.log({
    eventType: "Branch",
    payload: {
      condition,
      value,
      ...context,
    },
    level: "debug",
  })
}

export function loop(name: string, iteration: number, total?: number, context?: Record<string, unknown>): void {
  if (!StructuredLog.isEnabled()) return

  StructuredLog.log({
    eventType: "Loop",
    payload: {
      name,
      iteration,
      total,
      progress: total ? `${iteration}/${total}` : undefined,
      ...context,
    },
    level: "debug",
  })
}

export interface ApiCallHandle {
  end: (result?: unknown, error?: unknown) => void
}

export function apiCall(name: string, metadata?: Record<string, unknown>): ApiCallHandle {
  if (!StructuredLog.isEnabled()) {
    return {
      end: () => {},
    }
  }

  const startTime = Date.now()

  StructuredLog.log({
    eventType: "API_Call_Start",
    functionName: name,
    payload: metadata ?? {},
    level: "info",
  })

  return {
    end(result?: unknown, error?: unknown) {
      const duration = Date.now() - startTime

      if (error) {
        StructuredLog.log({
          eventType: "API_Call_End",
          functionName: name,
          payload: {
            success: false,
            error: error instanceof Error ? error.message : String(error),
            ...metadata,
          },
          durationMs: duration,
          stackTrace: error instanceof Error ? error.stack : undefined,
          level: "error",
        })
      } else {
        StructuredLog.log({
          eventType: "API_Call_End",
          functionName: name,
          payload: {
            success: true,
            result: result !== undefined ? summarizeResult(result) : undefined,
            ...metadata,
          },
          durationMs: duration,
          level: "info",
        })
      }
    },
  }
}

export function point(name: string, data?: Record<string, unknown>): void {
  if (!StructuredLog.isEnabled()) return

  StructuredLog.log({
    eventType: "Point",
    functionName: name,
    payload: data ?? {},
    level: "debug",
  })
}

function summarizeResult(result: unknown): unknown {
  if (result === null || result === undefined) return result
  if (typeof result === "string") {
    return result.length > 200 ? `${result.slice(0, 200)}... (${result.length} chars)` : result
  }
  if (typeof result === "number" || typeof result === "boolean") return result
  if (Array.isArray(result)) {
    return `[Array(${result.length})]`
  }
  if (typeof result === "object") {
    const keys = Object.keys(result)
    return `{${keys.slice(0, 5).join(", ")}${keys.length > 5 ? `, ... ${keys.length - 5} more` : ""}}`
  }
  return String(result)
}
