import { Log } from "./log"

/**
 * Node.js internal process methods (not part of public API)
 * These are used for debugging event loop state
 */
interface ProcessWithInternals {
  _getActiveHandles(): unknown[]
  _getActiveRequests(): unknown[]
}

/** Type-safe access to Node.js internal process methods */
function getProcessInternals(): ProcessWithInternals {
  return process as unknown as ProcessWithInternals
}

export namespace EventLoop {
  export async function wait() {
    const internals = getProcessInternals()
    return new Promise<void>((resolve) => {
      const check = () => {
        const active = [...internals._getActiveHandles(), ...internals._getActiveRequests()]
        Log.Default.info("eventloop", {
          active,
        })
        if (internals._getActiveHandles().length === 0 && internals._getActiveRequests().length === 0) {
          resolve()
        } else {
          setImmediate(check)
        }
      }
      check()
    })
  }
}
