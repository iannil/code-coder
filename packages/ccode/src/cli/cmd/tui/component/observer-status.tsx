/**
 * Observer Status Component
 *
 * Displays Observer Network status in the TUI including:
 * - Current operating mode (AUTO/MANUAL/HYBRID)
 * - Watcher statuses (running/stopped/error)
 * - Recent observations
 * - Pending escalations
 * - CLOSE scores
 */

import type { WatcherStatus, OperatingMode, Observation } from "@/observer/types"
import type { CLOSEEvaluation } from "@/observer/controller/close-evaluator"
import type { Escalation } from "@/observer/controller/escalation"
import type { ModeControllerStats } from "@/observer/controller/mode"

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ObserverDisplayProps {
  /** Current operating mode */
  mode: OperatingMode
  /** Watcher statuses */
  watchers: WatcherStatus[]
  /** Recent observations (last 5-10) */
  recentObservations?: Observation[]
  /** Pending escalations */
  pendingEscalations?: Escalation[]
  /** Latest CLOSE evaluation */
  closeEvaluation?: CLOSEEvaluation
  /** Mode controller stats */
  stats?: ModeControllerStats
  /** Display in compact mode */
  compact?: boolean
}

// ─────────────────────────────────────────────────────────────────────────────
// Mode Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get mode icon and color
 */
function getModeDisplay(mode: OperatingMode): { icon: string; color: string } {
  switch (mode) {
    case "AUTO":
      return { icon: "🤖", color: "green" }
    case "MANUAL":
      return { icon: "👤", color: "yellow" }
    case "HYBRID":
      return { icon: "🔀", color: "blue" }
  }
}

/**
 * Get mode description
 */
function getModeDescription(mode: OperatingMode): string {
  switch (mode) {
    case "AUTO":
      return "完全自动执行"
    case "MANUAL":
      return "需要人类确认"
    case "HYBRID":
      return "自动执行 + 事后确认"
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Watcher Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Render watcher status
 */
function renderWatcherStatus(watcher: WatcherStatus, compact = false): string {
  const isHealthy = watcher.health === "healthy"
  const statusIcon = watcher.running
    ? isHealthy
      ? "●"
      : "◐"
    : "○"

  const statusColor = watcher.running
    ? isHealthy
      ? "green"
      : "yellow"
    : "gray"

  const name = watcher.type.replace("Watch", "")

  if (compact) {
    return `${statusIcon} ${name}`
  }

  const lines: string[] = []
  lines.push(`${statusIcon} ${watcher.type}`)
  lines.push(`  状态: ${watcher.running ? "运行中" : "已停止"}`)
  lines.push(`  健康: ${watcher.health}`)
  lines.push(`  观察数: ${watcher.observationCount}`)

  if (watcher.lastObservation) {
    const ago = Date.now() - watcher.lastObservation.getTime()
    const agoStr = ago < 60000 ? `${Math.floor(ago / 1000)}秒前` : `${Math.floor(ago / 60000)}分钟前`
    lines.push(`  最近: ${agoStr}`)
  }

  if (watcher.errorCount > 0) {
    lines.push(`  错误数: ${watcher.errorCount}`)
  }

  return lines.join("\n")
}

/**
 * Render all watchers as a compact row
 */
function renderWatchersRow(watchers: WatcherStatus[]): string {
  return watchers.map((w) => renderWatcherStatus(w, true)).join(" │ ")
}

// ─────────────────────────────────────────────────────────────────────────────
// CLOSE Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Render CLOSE evaluation score
 */
function renderCLOSEScore(evaluation: CLOSEEvaluation, compact = false): string {
  const { total, risk, confidence } = evaluation

  if (compact) {
    return `CLOSE: ${total.toFixed(1)}/10 (${(confidence * 100).toFixed(0)}%)`
  }

  const lines: string[] = []
  lines.push("[CLOSE 评估]")
  lines.push(`总分: ${total.toFixed(2)}/10`)
  lines.push(`风险: ${risk.toFixed(2)}/10`)
  lines.push(`置信度: ${(confidence * 100).toFixed(0)}%`)
  lines.push("─".repeat(20))
  lines.push(`C 收敛度: ${evaluation.convergence.score.toFixed(2)}`)
  lines.push(`L 杠杆率: ${evaluation.leverage.score.toFixed(2)}`)
  lines.push(`O 可选性: ${evaluation.optionality.score.toFixed(2)}`)
  lines.push(`S 余量: ${evaluation.surplus.score.toFixed(2)}`)
  lines.push(`E 演化: ${evaluation.evolution.score.toFixed(2)}`)

  return lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Escalations Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Render pending escalations
 */
function renderEscalations(escalations: Escalation[], compact = false): string {
  if (escalations.length === 0) {
    return compact ? "无待处理" : "[无待处理升级]"
  }

  if (compact) {
    return `⚠️ ${escalations.length} 待处理`
  }

  const lines: string[] = []
  lines.push(`[待处理升级: ${escalations.length}]`)

  for (const esc of escalations.slice(0, 3)) {
    const priorityIcon = esc.priority === "critical" ? "🚨"
      : esc.priority === "high" ? "⚠️"
      : esc.priority === "medium" ? "📢"
      : "📝"

    lines.push(`${priorityIcon} ${esc.title}`)

    const expiresIn = esc.expiresAt.getTime() - Date.now()
    if (expiresIn > 0) {
      const minutes = Math.floor(expiresIn / 60000)
      lines.push(`   过期: ${minutes}分钟`)
    } else {
      lines.push(`   已过期`)
    }
  }

  if (escalations.length > 3) {
    lines.push(`... 还有 ${escalations.length - 3} 个`)
  }

  return lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Recent Observations Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Render recent observations
 */
function renderRecentObservations(observations: Observation[], compact = false): string {
  if (observations.length === 0) {
    return compact ? "无观察" : "[无最近观察]"
  }

  if (compact) {
    return `观察: ${observations.length}`
  }

  const lines: string[] = []
  lines.push(`[最近观察: ${observations.length}]`)

  for (const obs of observations.slice(0, 5)) {
    // Determine icon based on watcher type
    const typeIcon = obs.watcherType === "code" ? "💻"
      : obs.watcherType === "world" ? "🌍"
      : obs.watcherType === "self" ? "🔍"
      : "📊"

    const ago = Date.now() - obs.timestamp.getTime()
    const agoStr = ago < 60000 ? `${Math.floor(ago / 1000)}s` : `${Math.floor(ago / 60000)}m`

    lines.push(`${typeIcon} [${obs.watcherType}/${obs.type}] ${agoStr}`)
  }

  return lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Render Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Render complete observer status
 */
export function renderObserverStatus(props: ObserverDisplayProps): string {
  const {
    mode,
    watchers,
    recentObservations,
    pendingEscalations,
    closeEvaluation,
    stats,
    compact,
  } = props

  if (compact) {
    return renderObserverStatusCompact(props)
  }

  const lines: string[] = []
  const { icon } = getModeDisplay(mode)

  lines.push("═".repeat(50))
  lines.push("        OBSERVER NETWORK STATUS")
  lines.push("═".repeat(50))
  lines.push("")

  // Mode section
  lines.push(`模式: ${icon} ${mode} - ${getModeDescription(mode)}`)
  lines.push("")

  // Watchers section
  lines.push("── WATCHERS ──")
  for (const watcher of watchers) {
    lines.push(renderWatcherStatus(watcher))
    lines.push("")
  }

  // CLOSE section
  if (closeEvaluation) {
    lines.push("── CLOSE 评估 ──")
    lines.push(renderCLOSEScore(closeEvaluation))
    lines.push("")
  }

  // Escalations section
  if (pendingEscalations && pendingEscalations.length > 0) {
    lines.push("── 待处理升级 ──")
    lines.push(renderEscalations(pendingEscalations))
    lines.push("")
  }

  // Recent observations section
  if (recentObservations && recentObservations.length > 0) {
    lines.push("── 最近观察 ──")
    lines.push(renderRecentObservations(recentObservations))
    lines.push("")
  }

  // Stats section
  if (stats) {
    lines.push("── 统计 ──")
    lines.push(`模式切换: ${stats.modeSwitches}`)
    lines.push(`运行时间: ${formatUptime(stats.uptime)}`)
    lines.push("")
  }

  lines.push("═".repeat(50))

  return lines.join("\n")
}

/**
 * Render compact status bar
 */
export function renderObserverStatusCompact(props: ObserverDisplayProps): string {
  const { mode, watchers, pendingEscalations, closeEvaluation } = props
  const { icon } = getModeDisplay(mode)

  const parts: string[] = []

  // Mode
  parts.push(`${icon} ${mode}`)

  // Watchers summary
  const runningCount = watchers.filter((w) => w.running).length
  parts.push(`👁️ ${runningCount}/${watchers.length}`)

  // CLOSE score
  if (closeEvaluation) {
    parts.push(renderCLOSEScore(closeEvaluation, true))
  }

  // Escalations
  if (pendingEscalations && pendingEscalations.length > 0) {
    parts.push(renderEscalations(pendingEscalations, true))
  }

  return parts.join(" │ ")
}

/**
 * Render inline status for TUI status line
 */
export function renderObserverStatusBar(props: ObserverDisplayProps): string {
  const { mode, watchers, closeEvaluation } = props
  const { icon } = getModeDisplay(mode)

  const runningCount = watchers.filter((w) => w.running).length
  const watcherStatus = runningCount === watchers.length ? "✓" : `${runningCount}/${watchers.length}`

  let statusBar = `[OBS] ${icon}${mode} W:${watcherStatus}`

  if (closeEvaluation) {
    const riskLevel = closeEvaluation.risk >= 7 ? "⚠️" : closeEvaluation.risk >= 4 ? "○" : "✓"
    statusBar += ` C:${closeEvaluation.total.toFixed(1)}${riskLevel}`
  }

  return statusBar
}

// ─────────────────────────────────────────────────────────────────────────────
// Utilities
// ─────────────────────────────────────────────────────────────────────────────

function formatUptime(ms: number): string {
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  }
  return `${seconds}s`
}
