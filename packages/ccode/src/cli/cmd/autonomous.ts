import type { Argv } from "yargs"
import { cmd } from "./cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../ui"
import { Log } from "@/util/log"
import { bootstrap } from "../bootstrap"
import { Identifier } from "@/id/id"
import {
  createOrchestrator,
  type OrchestratorConfig,
  type SessionContext,
  type ResourceBudget,
  parseResourceBudget,
  AutonomousEvent,
  AutonomousEventHelper,
} from "@/autonomous"
import { Bus } from "@/bus"

const log = Log.create({ service: "cli.autonomous" })

// ============================================================================
// Types
// ============================================================================

type AutonomyLevel = "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"

interface AutonomousArgs {
  request: string
  "autonomy-level"?: AutonomyLevel
  budget?: string
  unattended?: boolean
  "max-tokens"?: number
  "max-cost"?: number
}

// ============================================================================
// Autonomy Level Descriptions (Chinese + English)
// ============================================================================

const AUTONOMY_DESCRIPTIONS: Record<AutonomyLevel, { cn: string; en: string }> = {
  lunatic: {
    cn: "完全自主，疯狂到令人担忧 - 无任何人工干预",
    en: "Fully autonomous, worryingly crazy - no human intervention",
  },
  insane: {
    cn: "高度自主，几乎不需要干预",
    en: "Highly autonomous, barely needs intervention",
  },
  crazy: {
    cn: "显著自主，偶需帮助",
    en: "Significantly autonomous, occasionally needs help",
  },
  wild: {
    cn: "部分自主，需定期确认",
    en: "Partially autonomous, needs regular confirmation",
  },
  bold: {
    cn: "谨慎自主，频繁暂停",
    en: "Cautiously autonomous, frequent pauses",
  },
  timid: {
    cn: "几乎无法自主 - 需要持续监督",
    en: "Barely autonomous - needs constant supervision",
  },
}

// ============================================================================
// Autonomous Command
// ============================================================================

export const AutonomousCommand = cmd({
  command: "autonomous <request>",
  describe: "Run CodeCoder in autonomous mode with full self-direction capability",
  builder: (yargs: Argv) => {
    return yargs
      .positional("request", {
        type: "string",
        describe: "The request or task for autonomous execution",
        demandOption: true,
      })
      .option("autonomy-level", {
        type: "string",
        choices: ["lunatic", "insane", "crazy", "wild", "bold", "timid"] as const,
        default: "crazy" as AutonomyLevel,
        describe: "Autonomy level (how much freedom to give the agent)",
      })
      .option("budget", {
        type: "string",
        describe: "Resource budget string (e.g., 'tokens:100000,cost:1.0,time:3600')",
      })
      .option("max-tokens", {
        type: "number",
        default: 500000,
        describe: "Maximum tokens to consume",
      })
      .option("max-cost", {
        type: "number",
        default: 5.0,
        describe: "Maximum cost in USD",
      })
      .option("unattended", {
        type: "boolean",
        default: false,
        describe: "Run without any user interaction (auto-continue on blocks)",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      await runAutonomous(args as AutonomousArgs)
    })
  },
})

// ============================================================================
// Main Execution
// ============================================================================

async function runAutonomous(args: AutonomousArgs): Promise<void> {
  const autonomyLevel = (args["autonomy-level"] ?? "crazy") as AutonomyLevel

  prompts.intro(`🚀 CodeCoder Autonomous Mode - ${autonomyLevel.toUpperCase()}`)

  // Show autonomy level description
  const desc = AUTONOMY_DESCRIPTIONS[autonomyLevel]
  prompts.log.info(`${desc.cn}\n${desc.en}`)

  // Parse resource budget
  const resourceBudget: ResourceBudget = args.budget
    ? parseResourceBudget(args.budget)
    : {
        maxTokens: args["max-tokens"] ?? 500000,
        maxCostUSD: args["max-cost"] ?? 5.0,
        maxDurationMinutes: 60, // 1 hour default
        maxFilesChanged: 50,
        maxActions: 1000,
      }

  // Confirm before proceeding for high autonomy levels
  if (!args.unattended && (autonomyLevel === "lunatic" || autonomyLevel === "insane")) {
    const confirmed = await prompts.confirm({
      message: `⚠️  You selected ${autonomyLevel.toUpperCase()} mode. Are you sure?`,
      initialValue: false,
    })

    if (prompts.isCancel(confirmed) || !confirmed) {
      prompts.cancel("Operation cancelled")
      return
    }
  }

  // Create session context
  const sessionId = Identifier.ascending("session")
  const requestId = Identifier.ascending("session") // Use session prefix for request ID

  const context: SessionContext = {
    sessionId,
    requestId,
    request: args.request,
    startTime: Date.now(),
  }

  // Create orchestrator config
  const config: OrchestratorConfig = {
    autonomyLevel,
    resourceBudget,
    unattended: args.unattended ?? false,
  }

  // Setup event listeners for progress display
  const spinner = prompts.spinner()
  let currentState = "IDLE"
  let iteration = 0

  const cleanup = await setupEventListeners({
    sessionId,
    onStateChange: (from, to) => {
      currentState = to
      spinner.message(`State: ${to}`)
    },
    onIterationStart: (iter) => {
      iteration = iter
      spinner.message(`Iteration ${iter} starting...`)
    },
    onProgress: (msg) => {
      spinner.message(msg)
    },
  })

  try {
    // Create and start orchestrator
    const orchestrator = createOrchestrator(context, config)

    spinner.start(`Starting autonomous session [${autonomyLevel}]...`)

    await orchestrator.start(args.request)

    spinner.message("Processing request...")

    const result = await orchestrator.process(args.request)

    spinner.stop(result.success ? "✓ Completed" : "✗ Failed")

    // Display results
    if (result.success && result.result) {
      displayResults(result.result)
    } else {
      prompts.log.error("Autonomous execution did not complete successfully")
      const state = orchestrator.getState()
      prompts.log.warn(`Final state: ${state}`)
    }
  } catch (error) {
    spinner.stop("✗ Error")
    log.error("Autonomous execution error", {
      error: error instanceof Error ? error.message : String(error),
    })
    prompts.log.error(error instanceof Error ? error.message : String(error))
  } finally {
    cleanup()
  }

  prompts.outro("Autonomous session ended")
}

// ============================================================================
// Event Listeners Setup
// ============================================================================

interface EventCallbacks {
  sessionId: string
  onStateChange: (from: string, to: string) => void
  onIterationStart: (iteration: number) => void
  onProgress: (message: string) => void
}

async function setupEventListeners(callbacks: EventCallbacks): Promise<() => void> {
  const unsubscribes: Array<() => void> = []

  // State change events
  unsubscribes.push(
    Bus.subscribe(AutonomousEvent.StateChanged, (event) => {
      if (event.properties.from && event.properties.to) {
        callbacks.onStateChange(event.properties.from, event.properties.to)
      }
    }),
  )

  // Iteration events
  unsubscribes.push(
    Bus.subscribe(AutonomousEvent.IterationStarted, (event) => {
      if (event.properties.sessionId === callbacks.sessionId) {
        callbacks.onIterationStart(event.properties.iteration)
      }
    }),
  )

  // Task events
  unsubscribes.push(
    Bus.subscribe(AutonomousEvent.TaskStarted, (event) => {
      if (event.properties.sessionId === callbacks.sessionId) {
        callbacks.onProgress(`Task started: ${event.properties.taskId}`)
      }
    }),
  )

  unsubscribes.push(
    Bus.subscribe(AutonomousEvent.TaskCompleted, (event) => {
      if (event.properties.sessionId === callbacks.sessionId) {
        const status = event.properties.success ? "✓" : "✗"
        callbacks.onProgress(`Task ${status}: ${event.properties.taskId}`)
      }
    }),
  )

  // Decision events
  unsubscribes.push(
    Bus.subscribe(AutonomousEvent.DecisionMade, (event) => {
      if (event.properties.sessionId === callbacks.sessionId) {
        const status = event.properties.approved ? "approved" : "blocked"
        callbacks.onProgress(`Decision ${status}: ${event.properties.type}`)
      }
    }),
  )

  // Resource warnings
  unsubscribes.push(
    Bus.subscribe(AutonomousEvent.ResourceWarning, (event) => {
      if (event.properties.sessionId === callbacks.sessionId) {
        callbacks.onProgress(`⚠ Resource warning: ${event.properties.resource} at ${event.properties.percentage}%`)
      }
    }),
  )

  return () => {
    unsubscribes.forEach((unsub) => unsub())
  }
}

// ============================================================================
// Results Display
// ============================================================================

function displayResults(result: {
  success: boolean
  qualityScore: number
  crazinessScore: number
  duration: number
  tokensUsed: number
  costUSD: number
  iterationsCompleted?: number
}): void {
  prompts.log.success("Autonomous execution completed!")

  const minutes = Math.floor(result.duration / 60000)
  const seconds = Math.floor((result.duration % 60000) / 1000)

  const lines = [
    "",
    "═══════════════════════════════════════════════════",
    "              AUTONOMOUS MODE REPORT",
    "═══════════════════════════════════════════════════",
    "",
    `  Quality Score:    ${result.qualityScore.toFixed(1)}/100`,
    `  Craziness Score:  ${result.crazinessScore.toFixed(1)}/100`,
    "",
    `  Duration:         ${minutes}m ${seconds}s`,
    `  Tokens Used:      ${result.tokensUsed.toLocaleString()}`,
    `  Cost:             $${result.costUSD.toFixed(4)}`,
    "",
  ]

  if (result.iterationsCompleted !== undefined) {
    lines.push(`  Iterations:       ${result.iterationsCompleted}`)
    lines.push("")
  }

  lines.push("═══════════════════════════════════════════════════")

  for (const line of lines) {
    prompts.log.message(line)
  }
}
