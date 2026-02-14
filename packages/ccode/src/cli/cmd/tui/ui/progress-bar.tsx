import { Show, type JSX } from "solid-js"
import { RGBA, TextAttributes } from "@opentui/core"
import { useTheme } from "@tui/context/theme"

export interface ProgressBarProps {
  progress: number
  width?: number
  animated?: boolean
  color?: RGBA
  showPercentage?: boolean
  showLabel?: boolean
  label?: string
}

export function ProgressBar(props: ProgressBarProps) {
  const { theme } = useTheme()

  const clampedProgress = () => Math.max(0, Math.min(100, props.progress))
  const filled = () => Math.round((props.width ?? 20) * (clampedProgress() / 100))
  const empty = () => (props.width ?? 20) - filled()

  const barColor = () => props.color ?? theme.accent

  const animationFrames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
  const frameIndex = () => Math.floor(Date.now() / 100) % animationFrames.length
  const spinner = () => (props.animated ? animationFrames[frameIndex()] : "")

  return (
    <box flexDirection="row" gap={1} alignItems="center">
      <Show when={props.animated}>
        <text fg={barColor()}>{spinner()}</text>
      </Show>
      <Show when={props.showLabel && props.label}>
        <text fg={theme.text}>{props.label}</text>
      </Show>
      <box flexDirection="row" gap={0} backgroundColor={theme.backgroundElement}>
        <text fg={barColor()}>{"█".repeat(filled())}</text>
        <text fg={theme.textMuted}>{"░".repeat(empty())}</text>
      </box>
      <Show when={props.showPercentage}>
        <text fg={theme.textMuted}>{String(Math.round(clampedProgress()))}%</text>
      </Show>
    </box>
  )
}

export interface SpinnerProps {
  color?: RGBA
}

export function Spinner(props: SpinnerProps) {
  const { theme } = useTheme()
  const color = () => props.color ?? theme.accent

  const animationFrames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
  const frameIndex = () => Math.floor(Date.now() / 80) % animationFrames.length
  const frame = () => animationFrames[frameIndex()]

  return <text fg={color()}>{frame()}</text>
}
