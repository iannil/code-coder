import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { AutonomousState } from "../state/states"
import { createOrchestrator, type SessionContext } from "../orchestration/orchestrator"
import { AutonomousConfig } from "../config/config"

const log = Log.create({ service: "autonomous.agent" })

export interface AutonomousModeAgent {
  start(request: string): Promise<void>
  process(request: string): Promise<{
    success: boolean
    result: {
      qualityScore: number
      crazinessScore: number
      duration: number
      tokensUsed: number
      costUSD: number
    } | null
  }>
  pause(): Promise<void>
  resume(): Promise<boolean>
  stop(): Promise<void>
  getState(): AutonomousState
  getStatus(): {
    state: AutonomousState
    taskStats: ReturnType<ReturnType<typeof createOrchestrator>["getTaskStats"]>
    decisionHistory: ReturnType<ReturnType<typeof createOrchestrator>["getDecisionHistory"]>
  }
}

export function createAutonomousModeAgent(sessionId: string, requestId: string): AutonomousModeAgent {
  const context: SessionContext = {
    sessionId,
    requestId,
    request: "",
    startTime: Date.now(),
  }

  const configPromise = AutonomousConfig.get()

  let orchestrator: ReturnType<typeof createOrchestrator> | null = null
  let initialized = false

  return {
    async start(request: string): Promise<void> {
      if (initialized) {
        log.warn("Autonomous Mode already started", { sessionId })
        return
      }

      log.info("Starting Autonomous Mode agent", { sessionId, request })

      const config = await configPromise

      orchestrator = createOrchestrator(context, {
        autonomyLevel: config.autonomyLevel,
        resourceBudget: config.resourceLimits,
        unattended: config.unattended,
      })

      await orchestrator.start(request)

      initialized = true
    },

    async process(request: string): Promise<{
      success: boolean
      result: {
        qualityScore: number
        crazinessScore: number
        duration: number
        tokensUsed: number
        costUSD: number
      } | null
    }> {
      if (!orchestrator || !initialized) {
        throw new Error("Autonomous Mode not initialized. Call start() first.")
      }

      context.request = request

      log.info("Autonomous Mode processing request", { sessionId, request })

      const result = await orchestrator.process(request)

      return result
    },

    async pause(): Promise<void> {
      if (!orchestrator) {
        throw new Error("Autonomous Mode not initialized")
      }

      log.info("Pausing Autonomous Mode", { sessionId })

      await orchestrator.pause()
    },

    async resume(): Promise<boolean> {
      if (!orchestrator) {
        throw new Error("Autonomous Mode not initialized")
      }

      log.info("Resuming Autonomous Mode", { sessionId })

      return orchestrator.resume()
    },

    async stop(): Promise<void> {
      if (!orchestrator) {
        throw new Error("Autonomous Mode not initialized")
      }

      log.info("Stopping Autonomous Mode", { sessionId })

      await orchestrator.stop()
      initialized = false
    },

    getState(): AutonomousState {
      if (!orchestrator) {
        return AutonomousState.IDLE
      }
      return orchestrator.getState()
    },

    getStatus() {
      if (!orchestrator) {
        return {
          state: AutonomousState.IDLE,
          taskStats: {
            total: 0,
            pending: 0,
            running: 0,
            completed: 0,
            failed: 0,
            skipped: 0,
            blocked: 0,
          },
          decisionHistory: [],
        }
      }
      return {
        state: orchestrator.getState(),
        taskStats: orchestrator.getTaskStats(),
        decisionHistory: orchestrator.getDecisionHistory(),
      }
    },
  }
}

export async function startAutonomousMode(
  sessionId: string,
  requestId: string,
  request: string,
): Promise<{
  agent: AutonomousModeAgent
  result: Awaited<ReturnType<AutonomousModeAgent["process"]>>
}> {
  const agent = createAutonomousModeAgent(sessionId, requestId)
  await agent.start(request)
  const result = await agent.process(request)
  return { agent, result }
}
