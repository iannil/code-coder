/**
 * HITL Approval Queue Dialog
 *
 * Displays pending approval requests and allows users to approve/reject them.
 */

import {
  createSignal,
  createEffect,
  For,
  Show,
  onMount,
  onCleanup,
} from "solid-js"
import { useKeyboard, useTerminalDimensions } from "@opentui/solid"
import { TextAttributes } from "@opentui/core"
import { useDialog } from "@tui/ui/dialog"
import { useTheme } from "@tui/context/theme"
import { useToast } from "@tui/ui/toast"
import {
  HitLClient,
  type ApprovalRequest,
  getApprovalTypeName,
  getRiskLevelIcon,
} from "@/sdk"
import { approvalT, errorT } from "@/config/messages"

interface Props {
  approverId?: string
}

export function DialogApprovalQueue(props: Props) {
  const dialog = useDialog()
  const { theme } = useTheme()
  const toast = useToast()
  const dimensions = useTerminalDimensions()

  const [requests, setRequests] = createSignal<ApprovalRequest[]>([])
  const [loading, setLoading] = createSignal(true)
  const [error, setError] = createSignal<string | null>(null)
  const [selectedIndex, setSelectedIndex] = createSignal(0)
  const [isHealthy, setIsHealthy] = createSignal(false)

  const client = new HitLClient()

  // Polling interval (5 seconds)
  let pollInterval: Timer | null = null

  const fetchPending = async () => {
    try {
      const response = await client.listPending(props.approverId)
      setRequests(response.requests)
      setError(null)
      setIsHealthy(true)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setError(`加载失败: ${message}`)
      setIsHealthy(false)
    } finally {
      setLoading(false)
    }
  }

  onMount(async () => {
    // Check service health first
    const healthy = await client.health()
    setIsHealthy(healthy)

    if (healthy) {
      await fetchPending()
      // Start polling
      pollInterval = setInterval(fetchPending, 5000)
    } else {
      setLoading(false)
      setError("HITL 服务不可用 (zero-gateway:4430)")
    }
  })

  onCleanup(() => {
    if (pollInterval) {
      clearInterval(pollInterval)
    }
  })

  const handleApprove = async () => {
    const request = requests()[selectedIndex()]
    if (!request) return

    try {
      const response = await client.approve(request.id, props.approverId ?? "tui-user")
      if (response.success) {
        toast.show({ variant: "success", message: `${approvalT("approved")}: ${request.title}` })
        await fetchPending()
      } else {
        toast.show({ variant: "error", message: response.error ?? errorT("approve_failed") })
      }
    } catch (err) {
      toast.show({ variant: "error", message: `${errorT("approve_failed")}: ${err}` })
    }
  }

  const handleReject = async () => {
    const request = requests()[selectedIndex()]
    if (!request) return

    try {
      const response = await client.reject(request.id, props.approverId ?? "tui-user", "从 TUI 拒绝")
      if (response.success) {
        toast.show({ variant: "info", message: `${approvalT("rejected")}: ${request.title}` })
        await fetchPending()
      } else {
        toast.show({ variant: "error", message: response.error ?? errorT("reject_failed") })
      }
    } catch (err) {
      toast.show({ variant: "error", message: `${errorT("reject_failed")}: ${err}` })
    }
  }

  useKeyboard((evt) => {
    const name = evt.name?.toLowerCase()

    if (name === "escape") {
      dialog.clear()
      return
    }

    const reqs = requests()
    if (reqs.length === 0) return

    if (name === "up" || name === "k") {
      setSelectedIndex((i) => Math.max(0, i - 1))
    } else if (name === "down" || name === "j") {
      setSelectedIndex((i) => Math.min(reqs.length - 1, i + 1))
    } else if (name === "a") {
      handleApprove()
    } else if (name === "r") {
      handleReject()
    } else if (name === "s" || name === "tab") {
      // Skip to next
      setSelectedIndex((i) => (i + 1) % reqs.length)
    } else if (name === "return") {
      // Show details (could expand to full view)
      const request = reqs[selectedIndex()]
      if (request) {
        toast.show({
          variant: "info",
          message: `${request.description ?? request.title}`,
          duration: 5000,
        })
      }
    }
  })

  // Keep selection in bounds
  createEffect(() => {
    const reqs = requests()
    if (selectedIndex() >= reqs.length && reqs.length > 0) {
      setSelectedIndex(reqs.length - 1)
    }
  })

  const dialogWidth = Math.min(80, dimensions().width - 4)
  const dialogHeight = Math.min(20, dimensions().height - 4)

  const formatTime = (isoString: string): string => {
    const date = new Date(isoString)
    return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })
  }

  return (
    <box
      width={dialogWidth}
      height={dialogHeight}
      flexDirection="column"
      borderStyle="rounded"
      borderColor={theme.border}
      backgroundColor={theme.background}
      padding={1}
    >
      {/* Header */}
      <box flexDirection="row" justifyContent="space-between" marginBottom={1}>
        <text attributes={TextAttributes.BOLD} fg={theme.text}>
          {isHealthy() ? "📋" : "⚠️"} {approvalT("queue_title", { count: requests().length })}
        </text>
        <text fg={theme.textMuted}>[ESC] 关闭</text>
      </box>

      {/* Loading state */}
      <Show when={loading()}>
        <text fg={theme.textMuted}>加载中...</text>
      </Show>

      {/* Error state */}
      <Show when={error()}>
        <text fg={theme.error}>{error()}</text>
      </Show>

      {/* Empty state */}
      <Show when={!loading() && !error() && requests().length === 0}>
        <box flexDirection="column" alignItems="center" marginTop={2}>
          <text fg={theme.textMuted}>{approvalT("queue_empty")}</text>
          <text fg={theme.textMuted} marginTop={1}>
            当 Hands 需要人工确认时，请求会显示在这里
          </text>
        </box>
      </Show>

      {/* Request list */}
      <Show when={!loading() && requests().length > 0}>
        <scrollbox height={dialogHeight - 6} flexGrow={1}>
          <For each={requests()}>
            {(request, index) => {
              const isSelected = () => index() === selectedIndex()
              const typeIcon = () => {
                switch (request.approval_type.type) {
                  case "merge_request":
                    return "🔀"
                  case "trading_command":
                    return "💰"
                  case "config_change":
                    return "⚙️"
                  case "high_cost_operation":
                    return "💵"
                  case "risk_operation":
                    return getRiskLevelIcon(request.approval_type.risk_level)
                  case "tool_execution":
                    return "🛠️"
                }
              }

              return (
                <box
                  flexDirection="column"
                  backgroundColor={isSelected() ? theme.backgroundElement : undefined}
                  padding={1}
                  marginBottom={1}
                >
                  {/* Title row */}
                  <box flexDirection="row" gap={1}>
                    <text fg={isSelected() ? theme.primary : theme.textMuted}>
                      {isSelected() ? "▶" : " "}
                    </text>
                    <text fg={theme.text}>{typeIcon()}</text>
                    <text attributes={TextAttributes.BOLD} fg={theme.text}>
                      {request.title}
                    </text>
                  </box>

                  {/* Details row */}
                  <box flexDirection="row" gap={2} marginLeft={3}>
                    <text fg={theme.textMuted}>
                      [{getApprovalTypeName(request.approval_type)}]
                    </text>
                    <text fg={theme.textMuted}>来源: {request.requester}</text>
                    <text fg={theme.textMuted}>{formatTime(request.created_at)}</text>
                  </box>

                  {/* Description if selected */}
                  <Show when={isSelected() && request.description}>
                    <text fg={theme.textMuted} marginLeft={3} marginTop={1}>
                      {request.description}
                    </text>
                  </Show>
                </box>
              )
            }}
          </For>
        </scrollbox>
      </Show>

      {/* Footer with keybindings */}
      <Show when={requests().length > 0}>
        <box
          flexDirection="row"
          justifyContent="center"
          gap={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.border}
          paddingTop={1}
        >
          <text fg={theme.success}>[a] {approvalT("approve")}</text>
          <text fg={theme.error}>[r] {approvalT("reject")}</text>
          <text fg={theme.textMuted}>[s] 跳过</text>
          <text fg={theme.textMuted}>[↑↓] 导航</text>
          <text fg={theme.textMuted}>[Enter] 详情</text>
        </box>
      </Show>
    </box>
  )
}
