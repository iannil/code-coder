import type { TrackerOptions } from "./types"
import { runWithChildSpan, runWithChildSpanAsync } from "./trace-context"
import * as StructuredLog from "./structured-log"

export function tracked<T extends (...args: any[]) => any>(
  name: string,
  fn: T,
  options: TrackerOptions = {},
): T {
  const { logArgs = true, logResult = true, service } = options

  const wrapper = function (this: any, ...args: Parameters<T>): ReturnType<T> {
    if (!StructuredLog.isEnabled()) {
      return fn.apply(this, args)
    }

    const startTime = Date.now()

    return runWithChildSpan(() => {
      StructuredLog.functionStart(
        name,
        logArgs ? serializeArgs(args) : undefined,
        service,
      )

      try {
        const result = fn.apply(this, args)

        if (result instanceof Promise) {
          return result
            .then((value) => {
              const duration = Date.now() - startTime
              StructuredLog.functionEnd(
                name,
                logResult ? value : undefined,
                duration,
                service,
              )
              return value
            })
            .catch((error) => {
              const duration = Date.now() - startTime
              StructuredLog.functionError(name, error, duration, service)
              throw error
            }) as ReturnType<T>
        }

        const duration = Date.now() - startTime
        StructuredLog.functionEnd(
          name,
          logResult ? result : undefined,
          duration,
          service,
        )
        return result
      } catch (error) {
        const duration = Date.now() - startTime
        StructuredLog.functionError(name, error, duration, service)
        throw error
      }
    })
  }

  return wrapper as T
}

export function trackedAsync<T extends (...args: any[]) => Promise<any>>(
  name: string,
  fn: T,
  options: TrackerOptions = {},
): T {
  const { logArgs = true, logResult = true, service } = options

  const wrapper = async function (this: any, ...args: Parameters<T>): Promise<Awaited<ReturnType<T>>> {
    if (!StructuredLog.isEnabled()) {
      return fn.apply(this, args)
    }

    const startTime = Date.now()

    return runWithChildSpanAsync(async () => {
      StructuredLog.functionStart(
        name,
        logArgs ? serializeArgs(args) : undefined,
        service,
      )

      try {
        const result = await fn.apply(this, args)
        const duration = Date.now() - startTime
        StructuredLog.functionEnd(
          name,
          logResult ? result : undefined,
          duration,
          service,
        )
        return result
      } catch (error) {
        const duration = Date.now() - startTime
        StructuredLog.functionError(name, error, duration, service)
        throw error
      }
    })
  }

  return wrapper as T
}

export function createTracker(defaultService: string) {
  return {
    track<T extends (...args: any[]) => any>(name: string, fn: T, options: Omit<TrackerOptions, "service"> = {}): T {
      return tracked(name, fn, { ...options, service: defaultService })
    },
    trackAsync<T extends (...args: any[]) => Promise<any>>(
      name: string,
      fn: T,
      options: Omit<TrackerOptions, "service"> = {},
    ): T {
      return trackedAsync(name, fn, { ...options, service: defaultService })
    },
  }
}

function serializeArgs(args: unknown[]): Record<string, unknown> {
  const result: Record<string, unknown> = {}
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]
    if (arg === undefined) continue
    if (typeof arg === "object" && arg !== null && !Array.isArray(arg)) {
      const keys = Object.keys(arg)
      if (keys.length <= 5) {
        for (const key of keys) {
          result[key] = (arg as Record<string, unknown>)[key]
        }
      } else {
        result[`arg${i}`] = `[object with ${keys.length} keys]`
      }
    } else {
      result[`arg${i}`] = arg
    }
  }
  return result
}
