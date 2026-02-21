import { Log } from "@/util/log"
import { Hook } from "@/hook"
import { Bus } from "@/bus"
import { Session } from "@/session"
import { BootstrapTypes } from "./types"
import { Triggers } from "./triggers"
import { CostTracker } from "./cost-tracker"

const log = Log.create({ service: "bootstrap.hooks" })

/**
 * BootstrapHooks integrates the bootstrap system with the existing hook infrastructure.
 */
export namespace BootstrapHooks {
  let initialized = false
  let enabled = true

  /**
   * Enable or disable bootstrap hooks
   */
  export function setEnabled(value: boolean): void {
    enabled = value
    log.info("bootstrap hooks", { enabled })
  }

  /**
   * Initialize bootstrap hooks
   */
  export async function init(): Promise<void> {
    if (initialized) return
    initialized = true

    log.info("initializing bootstrap hooks")

    // Subscribe to session events
    Bus.subscribe(Session.Event.Created, async (event) => {
      if (!enabled) return
      Triggers.startSession(event.properties.info.id)
    })

    // Note: Session end is handled via the Stop lifecycle hook
    // or explicit cleanup
  }

  /**
   * Hook handler for PostToolUse
   * Called after each tool execution to track patterns
   */
  export async function handlePostToolUse(ctx: Hook.Context): Promise<void> {
    if (!enabled || !ctx.sessionID) return

    const toolCall: BootstrapTypes.ToolCallRecord = {
      id: `tc_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      tool: ctx.tool ?? "unknown",
      input: (ctx.input ?? {}) as Record<string, unknown>,
      output: ctx.output?.slice(0, 1000),
      timestamp: Date.now(),
    }

    // Track for cost metrics
    await CostTracker.record({
      sessionId: ctx.sessionID,
      operation: ctx.tool ?? "unknown",
      inputTokens: estimateTokens(JSON.stringify(ctx.input ?? {})),
      outputTokens: estimateTokens(ctx.output ?? ""),
      reasoningSteps: 1,
      duration: 0, // Would need timing from actual execution
      timestamp: Date.now(),
    })

    // Trigger pattern detection
    await Triggers.onPostToolUse({
      sessionId: ctx.sessionID,
      toolCall,
    })
  }

  /**
   * Hook handler for Stop
   * Called when session ends
   */
  export async function handleStop(ctx: Hook.Context): Promise<void> {
    if (!enabled || !ctx.sessionID) return

    await Triggers.onSessionEnd({
      sessionId: ctx.sessionID,
    })

    // Flush cost metrics
    await CostTracker.flush()
  }

  /**
   * Estimate token count (rough approximation)
   */
  function estimateTokens(text: string): number {
    return Math.ceil(text.length / 4)
  }

  /**
   * Get hook definitions for registration
   */
  export function getHookDefinitions(): {
    PostToolUse: Record<string, Hook.HookDefinition>
    Stop: Record<string, Hook.HookDefinition>
  } {
    return {
      PostToolUse: {
        "bootstrap-tracker": {
          description: "Track tool calls for skill extraction",
          actions: [
            {
              type: "run_command",
              async: true,
            },
          ],
        },
      },
      Stop: {
        "bootstrap-session-end": {
          description: "Process session for skill candidates",
          actions: [
            {
              type: "run_command",
              async: true,
            },
          ],
        },
      },
    }
  }

  /**
   * Create a runnable hook function for PostToolUse
   */
  export function createPostToolUseHandler(): (ctx: Hook.Context) => Promise<Hook.RunResult> {
    return async (ctx: Hook.Context): Promise<Hook.RunResult> => {
      try {
        await handlePostToolUse(ctx)
        return { blocked: false }
      } catch (error) {
        log.warn("PostToolUse hook error", { error })
        return { blocked: false }
      }
    }
  }

  /**
   * Create a runnable hook function for Stop
   */
  export function createStopHandler(): (ctx: Hook.Context) => Promise<Hook.RunResult> {
    return async (ctx: Hook.Context): Promise<Hook.RunResult> => {
      try {
        await handleStop(ctx)
        return { blocked: false }
      } catch (error) {
        log.warn("Stop hook error", { error })
        return { blocked: false }
      }
    }
  }
}
