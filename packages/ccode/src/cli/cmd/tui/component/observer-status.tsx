/**
 * Observer Status Component
 *
 * Displays Observer Network status in the TUI including:
 * - Current gear preset (P/N/D/S/M)
 * - Three dials visualization (Observe/Decide/Act)
 * - Current operating mode (AUTO/MANUAL/HYBRID)
 * - Watcher statuses (running/stopped/error)
 * - Recent observations
 * - Pending escalations
 * - CLOSE scores
 */

import type {
  WatcherStatus,
  OperatingMode,
  Observation,
  GearPreset,
  DialValues,
  CLOSEEvaluation,
  Escalation,
  ModeControllerStats,
} from "@/sdk/types"
import { GEAR_INFO } from "@/sdk/types"

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ObserverDisplayProps {
  /** Current operating mode */
  mode: OperatingMode
  /** Current gear preset */
  gear?: GearPreset
  /** Dial values (observe, decide, act) */
  dials?: DialValues
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
// Gear Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get gear icon and color
 */
export function getGearDisplay(gear: GearPreset): { icon: string; color: string; label: string } {
  const info = GEAR_INFO[gear]
  switch (gear) {
    case "P":
      return { icon: "P", color: "gray", label: info.name }
    case "N":
      return { icon: "N", color: "yellow", label: info.name }
    case "D":
      return { icon: "D", color: "green", label: info.name }
    case "S":
      return { icon: "S", color: "red", label: info.name }
    case "M":
      return { icon: "M", color: "blue", label: info.name }
  }
}

/**
 * Render gear selector indicator
 */
export function renderGearIndicator(currentGear: GearPreset, compact = false): string {
  const gears: GearPreset[] = ["P", "N", "D", "S", "M"]

  if (compact) {
    return `[${currentGear}]`
  }

  const indicator = gears
    .map((g) => {
      if (g === currentGear) {
        return `[${g}]`
      }
      return ` ${g} `
    })
    .join("")

  return `◀ ${indicator} ▶`
}

/**
 * Render gear with description
 */
export function renderGearWithDescription(gear: GearPreset): string {
  const info = GEAR_INFO[gear]
  return `[${gear}] ${info.name}: ${info.description}`
}

// ─────────────────────────────────────────────────────────────────────────────
// Dial Display
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Render a single dial as a progress bar
 */
function renderDialBar(label: string, value: number, width = 20): string {
  const filled = Math.round((value / 100) * width)
  const empty = width - filled
  const bar = "█".repeat(filled) + "░".repeat(empty)
  return `${label.padEnd(8)} ${bar} ${value.toString().padStart(3)}%`
}

/**
 * Render three dials as compact inline
 */
export function renderDialsCompact(dials: DialValues): string {
  const o = dials.observe.toString().padStart(3)
  const d = dials.decide.toString().padStart(3)
  const a = dials.act.toString().padStart(3)
  return `O:${o} D:${d} A:${a}`
}

/**
 * Render three dials with bars
 */
export function renderDials(dials: DialValues): string {
  const lines: string[] = []
  lines.push(renderDialBar("Observe", dials.observe))
  lines.push(renderDialBar("Decide", dials.decide))
  lines.push(renderDialBar("Act", dials.act))
  return lines.join("\n")
}

/**
 * Render dial knobs ASCII art
 */
export function renderDialKnobs(dials: DialValues): string {
  // Convert value to knob position (8 positions around the dial)
  const getKnobChar = (value: number): string => {
    const normalized = Math.round((value / 100) * 7)
    const chars = ["○", "◔", "◑", "◕", "●", "◕", "◑", "◔"]
    return chars[normalized] ?? "○"
  }

  const lines: string[] = []
  lines.push("  ┌─────────────────────┐")
  lines.push(`  │  ${getKnobChar(dials.observe)} Observe: ${dials.observe.toString().padStart(3)}%  │`)
  lines.push(`  │  ${getKnobChar(dials.decide)} Decide:  ${dials.decide.toString().padStart(3)}%  │`)
  lines.push(`  │  ${getKnobChar(dials.act)} Act:     ${dials.act.toString().padStart(3)}%  │`)
  lines.push("  └─────────────────────┘")
  return lines.join("\n")
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
    gear,
    dials,
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

  // Gear section (new)
  if (gear) {
    lines.push("── GEAR ──")
    lines.push(renderGearIndicator(gear))
    lines.push(renderGearWithDescription(gear))
    lines.push("")
  }

  // Dials section (new)
  if (dials) {
    lines.push("── DIALS ──")
    lines.push(renderDials(dials))
    lines.push("")
  }

  // Mode section
  lines.push("── MODE ──")
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
    lines.push(`档位: ${stats.currentGear}`)
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
  const { mode, gear, dials, watchers, pendingEscalations, closeEvaluation } = props
  const { icon } = getModeDisplay(mode)

  const parts: string[] = []

  // Gear (new)
  if (gear) {
    parts.push(renderGearIndicator(gear, true))
  }

  // Mode
  parts.push(`${icon} ${mode}`)

  // Dials (new)
  if (dials) {
    parts.push(renderDialsCompact(dials))
  }

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
 * Render inline status for TUI status line (enhanced with gear)
 */
export function renderObserverStatusBar(props: ObserverDisplayProps): string {
  const { mode, gear, dials, watchers, closeEvaluation } = props
  const { icon } = getModeDisplay(mode)

  const runningCount = watchers.filter((w) => w.running).length
  const watcherStatus = runningCount === watchers.length ? "✓" : `${runningCount}/${watchers.length}`

  // Start with gear if available
  let statusBar = gear ? `[${gear}] ` : ""
  statusBar += `${icon}${mode} W:${watcherStatus}`

  // Add dials summary if available
  if (dials) {
    const avgDial = Math.round((dials.observe + dials.decide + dials.act) / 3)
    statusBar += ` D:${avgDial}%`
  }

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

// ─────────────────────────────────────────────────────────────────────────────
// Gear Status Data Type (for SolidJS integration)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Gear status data for TUI display
 */
export interface GearStatusData {
  gear: GearPreset
  dials: DialValues
  mode: OperatingMode
  watchersRunning: number
  watchersTotal: number
  closeScore?: number
  riskLevel: "safe" | "moderate" | "high"
}

/**
 * Get gear color based on gear preset
 */
export function getGearColor(gear: GearPreset): string {
  switch (gear) {
    case "P":
      return "gray"
    case "N":
      return "yellow"
    case "D":
      return "green"
    case "S":
      return "red"
    case "M":
      return "blue"
  }
}

/**
 * Get dial bar character based on value (0-100)
 */
export function getDialChar(value: number): string {
  if (value >= 80) return "█"
  if (value >= 60) return "▓"
  if (value >= 40) return "▒"
  if (value >= 20) return "░"
  return "·"
}

/**
 * Format gear status for inline display
 * Example: "[D] ▓▒░ HYBRID W:4/4 C:7.2✓"
 */
export function formatGearStatusInline(status: GearStatusData): string {
  const gearChar = `[${status.gear}]`
  const dialChars = `${getDialChar(status.dials.observe)}${getDialChar(status.dials.decide)}${getDialChar(status.dials.act)}`
  const watcherStatus = status.watchersRunning === status.watchersTotal
    ? "✓"
    : `${status.watchersRunning}/${status.watchersTotal}`
  const closeText = status.closeScore !== undefined
    ? ` C:${status.closeScore.toFixed(1)}${status.riskLevel === "safe" ? "✓" : status.riskLevel === "high" ? "⚠" : "○"}`
    : ""

  return `${gearChar} ${dialChars} ${status.mode} W:${watcherStatus}${closeText}`
}
