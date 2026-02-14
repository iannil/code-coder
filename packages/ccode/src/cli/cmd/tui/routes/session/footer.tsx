import {
  createMemo,
  Match,
  onCleanup,
  onMount,
  Show,
  Switch,
  createSignal,
} from "solid-js"
import { useTheme } from "../../context/theme"
import { useSync } from "../../context/sync"
import { useDirectory } from "../../context/directory"
import { useConnected } from "../../component/dialog-model"
import { createStore } from "solid-js/store"
import { useRoute } from "../../context/route"
import { formatPreciseTime, getToolDuration } from "../../util/execution-time"
import { Spinner } from "../../ui/progress-bar"
import type { ToolPart, Part } from "@/types"
import { MessageV2 } from "@/session/message-v2"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous"

// ============================================================================
// Autonomous State
// ============================================================================

interface AutonomousState {
  active: boolean
  sessionId?: string
  level?: string
  state?: string
  qualityScore?: number
  crazinessScore?: number
  safe?: boolean
  iteration?: number
  tasksCompleted?: number
  tasksTotal?: number
}

export function Footer() {
  const { theme } = useTheme()
  const sync = useSync()
  const route = useRoute()
  const mcp = createMemo(() => Object.values(sync.data.mcp).filter((x) => x.status === "connected").length)
  const mcpError = createMemo(() => Object.values(sync.data.mcp).some((x) => x.status === "failed"))
  const lsp = createMemo(() => Object.keys(sync.data.lsp))
  const permissions = createMemo(() => {
    if (route.data.type !== "session") return []
    return sync.data.permission[route.data.sessionID] ?? []
  })
  const directory = useDirectory()
  const connected = useConnected()

  const [now, setNow] = createSignal(Date.now())

  // Autonomous mode state
  const [autonomousState, setAutonomousState] = createSignal<AutonomousState>({ active: false })

  const currentSessionID = createMemo(() => {
    if (route.data.type === "session") return route.data.sessionID
    return undefined
  })

  // Get the current running tool from the last assistant message
  const runningTool = createMemo(() => {
    const sessionID = currentSessionID()
    if (!sessionID) return undefined
    const messages = sync.data.message[sessionID] ?? []
    const lastAssistant = messages.findLast((m) => m.role === "assistant")
    if (!lastAssistant) return undefined
    const parts = sync.data.part[lastAssistant.id] ?? []
    const runningPart = parts.find(
      (p): p is ToolPart => p.type === "tool" && (p.state.status === "running" || p.state.status === "pending"),
    )
    if (!runningPart) return undefined
    const duration = getToolDuration(runningPart)
    return {
      tool: runningPart.tool,
      elapsed: duration.elapsed,
      part: runningPart,
    }
  })

  // Model API call status
  const modelCallStatus = createMemo(() => {
    const sessionID = currentSessionID()
    if (!sessionID) return undefined
    const messages = sync.data.message[sessionID] ?? []
    const lastAssistant = messages.findLast((m) => m.role === "assistant")
    if (!lastAssistant) return undefined

    // Check if message is still being generated (model call in progress)
    const isGenerating = !lastAssistant.time.completed
    if (isGenerating) {
      const parts = sync.data.part[lastAssistant.id] ?? []
      const startTime = lastAssistant.time.created
      const elapsed = (now() - startTime) / 1000

      // Check for step-start part
      const stepStart = parts.find((p): p is MessageV2.StepStartPart => p.type === "step-start")
      const stepFinish = parts.find((p): p is MessageV2.StepFinishPart => p.type === "step-finish")

      return {
        type: "generating",
        elapsed,
        hasStepStart: !!stepStart,
        hasStepFinish: !!stepFinish,
        tokens: lastAssistant.tokens,
      }
    }

    return undefined
  })

  const toolCount = createMemo(() => {
    const sessionID = currentSessionID()
    if (!sessionID) return { pending: 0, completed: 0 }
    const messages = sync.data.message[sessionID] ?? []
    const lastAssistant = messages.findLast((m) => m.role === "assistant")
    if (!lastAssistant) return { pending: 0, completed: 0 }
    const parts = sync.data.part[lastAssistant.id] ?? []
    const toolParts = parts.filter((p): p is ToolPart => p.type === "tool")
    return {
      pending: toolParts.filter((p) => p.state.status === "pending" || p.state.status === "running").length,
      completed: toolParts.filter((p) => p.state.status === "completed").length,
    }
  })

  const statusText = createMemo(() => {
    const tool = runningTool()
    const modelStatus = modelCallStatus()

    // Priority: Model call status > Tool status
    if (modelStatus) {
      if (modelStatus.type === "generating") {
        const timeText = modelStatus.elapsed > 0 ? ` (${modelStatus.elapsed.toFixed(1)}s)` : ""
        return `◈ Generating${timeText}`
      }
    }

    if (tool) {
      const timeText = tool.elapsed > 0 ? ` (${formatPreciseTime(tool.elapsed)})` : ""
      return `${tool.tool}${timeText}`
    }
    return undefined
  })

  // Token usage display for current message
  const tokenDisplay = createMemo(() => {
    const modelStatus = modelCallStatus()
    if (!modelStatus) return undefined
    const tokens = modelStatus.tokens
    const total = tokens.input + tokens.output + tokens.reasoning
    if (total === 0) return undefined
    const cacheTotal = tokens.cache.read + tokens.cache.write
    const cacheText = cacheTotal > 0 ? ` +${cacheTotal} cache` : ""
    return `${total.toLocaleString()}${cacheText}`
  })

  const [store, setStore] = createStore({
    welcome: false,
  })

  onMount(() => {
    // Track all timeouts to ensure proper cleanup
    const timeouts: ReturnType<typeof setTimeout>[] = []
    const unsubscribes: Array<() => void> = []

    // Update time every 100ms for smooth real-time updates
    const timeInterval = setInterval(() => setNow(Date.now()), 100)
    timeouts.push(timeInterval)

    // Subscribe to autonomous events
    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SessionStarted, (event) => {
        setAutonomousState({
          active: true,
          sessionId: event.properties.sessionId,
          level: event.properties.autonomyLevel,
          state: "PLANNING",
          iteration: 0,
        })
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.StateChanged, (event) => {
        setAutonomousState((prev) => ({
          ...prev,
          state: event.properties.to,
        }))
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.IterationStarted, (event) => {
        setAutonomousState((prev) => ({
          ...prev,
          iteration: event.properties.iteration,
        }))
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.MetricsUpdated, (event) => {
        const metrics = event.properties.metrics
        setAutonomousState((prev) => ({
          ...prev,
          qualityScore: metrics.qualityScore,
          crazinessScore: metrics.crazinessScore,
          tasksCompleted: metrics.tasksCompleted,
          tasksTotal: metrics.tasksTotal,
        }))
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SafetyTriggered, (event) => {
        const severity = event.properties.severity
        setAutonomousState((prev) => ({
          ...prev,
          safe: severity !== "critical" && severity !== "high",
        }))
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SessionCompleted, () => {
        setAutonomousState({ active: false })
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SessionFailed, () => {
        setAutonomousState({ active: false })
      }),
    )

    function tick() {
      if (connected()) return
      if (!store.welcome) {
        setStore("welcome", true)
        timeouts.push(setTimeout(() => tick(), 5000))
        return
      }

      if (store.welcome) {
        setStore("welcome", false)
        timeouts.push(setTimeout(() => tick(), 10_000))
        return
      }
    }
    timeouts.push(setTimeout(() => tick(), 10_000))

    onCleanup(() => {
      timeouts.forEach(clearTimeout)
      clearInterval(timeInterval)
      unsubscribes.forEach((unsub) => unsub())
    })
  })

  // Autonomous mode status display
  const autonomousStatusDisplay = createMemo(() => {
    const state = autonomousState()
    if (!state.active) return null

    const levelColor =
      state.level === "lunatic"
        ? theme.error
        : state.level === "insane"
          ? theme.warning
          : state.level === "crazy"
            ? theme.accent
            : state.level === "wild"
              ? theme.primary
              : theme.textMuted

    const safeIcon = state.safe !== false ? "✓" : "!"
    const safeColor = state.safe !== false ? theme.success : theme.error

    const progressText =
      state.tasksCompleted !== undefined && state.tasksTotal !== undefined
        ? ` ${state.tasksCompleted}/${state.tasksTotal}`
        : ""

    return {
      level: state.level?.toUpperCase() ?? "AUTO",
      levelColor,
      state: state.state ?? "IDLE",
      safeIcon,
      safeColor,
      qualityScore: state.qualityScore ?? 0,
      crazinessScore: state.crazinessScore ?? 0,
      iteration: state.iteration ?? 0,
      progressText,
    }
  })

  return (
    <box flexDirection="row" justifyContent="space-between" gap={1} flexShrink={0}>
      <box flexDirection="row" gap={2}>
        <text fg={theme.textMuted}>{directory()}</text>
        <Show when={statusText()}>
          <box flexDirection="row" gap={1}>
            <Spinner />
            <text fg={theme.accent}>{statusText()}</text>
            <Show when={tokenDisplay()}>
              <text fg={theme.textMuted}> | {tokenDisplay()} tokens</text>
            </Show>
            <Show when={toolCount().pending > 1}>
              <text fg={theme.textMuted}>
                (+{String(toolCount().pending - 1)} more)
              </text>
            </Show>
          </box>
        </Show>
      </box>
      <box gap={2} flexDirection="row" flexShrink={0}>
        <Show when={autonomousStatusDisplay()}>
          {(status) => (
            <box flexDirection="row" gap={1}>
              <text>
                <span style={{ fg: status().safeColor }}>{status().safeIcon}</span>
              </text>
              <text fg={status().levelColor}>{status().level}</text>
              <text fg={theme.textMuted}>
                {status().state}{status().progressText}
              </text>
              <Show when={status().qualityScore > 0}>
                <text fg={theme.textMuted}>Q:{String(Math.round(status().qualityScore))}</text>
              </Show>
            </box>
          )}
        </Show>
        <Switch>
          <Match when={store.welcome}>
            <text fg={theme.text}>
              Get started <span style={{ fg: theme.textMuted }}>/connect</span>
            </text>
          </Match>
          <Match when={connected()}>
            <Show when={permissions().length > 0}>
              <text fg={theme.warning}>
                <span style={{ fg: theme.warning }}>△</span> {String(permissions().length)} Permission
                {permissions().length > 1 ? "s" : ""}
              </text>
            </Show>
            <text fg={theme.text}>
              <span style={{ fg: lsp().length > 0 ? theme.success : theme.textMuted }}>•</span> {String(lsp().length)} LSP
            </text>
            <Show when={mcp()}>
              <text fg={theme.text}>
                <Switch>
                  <Match when={mcpError()}>
                    <span style={{ fg: theme.error }}>⊙ </span>
                  </Match>
                  <Match when={true}>
                    <span style={{ fg: theme.success }}>⊙ </span>
                  </Match>
                </Switch>
                                {String(mcp())} MCP
              </text>
            </Show>
            <text fg={theme.textMuted}>/status</text>
          </Match>
        </Switch>
      </box>
    </box>
  )
}
