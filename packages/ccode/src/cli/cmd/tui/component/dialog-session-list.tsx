import { useDialog } from "@tui/ui/dialog"
import { DialogSelect } from "@tui/ui/dialog-select"
import { useRoute } from "@tui/context/route"
import { useSync } from "@tui/context/sync"
import { createMemo, createSignal, createResource, onMount, onCleanup, Show } from "solid-js"
import { Locale } from "@/util/locale"
import { useKeybind } from "../context/keybind"
import { useTheme } from "../context/theme"
import { useSDK } from "../context/sdk"
import { DialogSessionRename } from "./dialog-session-rename"
import { useKV } from "../context/kv"
import { createDebouncedSignal } from "../util/signal"
import { Session } from "@/session"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous"
import "opentui-spinner/solid"

// ============================================================================
// Autonomous Session Tracking
// ============================================================================

interface AutonomousSessionInfo {
  level: string
  state: string
  tasksCompleted?: number
  tasksTotal?: number
}

export function DialogSessionList() {
  const dialog = useDialog()
  const route = useRoute()
  const sync = useSync()
  const keybind = useKeybind()
  const { theme } = useTheme()
  const sdk = useSDK()
  const kv = useKV()

  const [toDelete, setToDelete] = createSignal<string>()
  const [search, setSearch] = createDebouncedSignal("", 150)
  const [autonomousSessions, setAutonomousSessions] = createSignal<Map<string, AutonomousSessionInfo>>(new Map())

  const [searchResults] = createResource(search, async (query) => {
    if (!query) return undefined
    const result = await sdk.client.session.list({ search: query, limit: 30 })
    return result.data ?? []
  })

  const currentSessionID = createMemo(() => (route.data.type === "session" ? route.data.sessionID : undefined))

  const spinnerFrames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
  const crazyFrames = ["🔥", "⚡", "💥", "✨", "🚀"]

  const sessions = createMemo(() => searchResults() ?? sync.data.session)

  const options = createMemo(() => {
    const today = new Date().toDateString()
    const deleting = toDelete()
    const autoSessions = autonomousSessions()
    return sessions()
      .filter((x: Session.Info) => x.parentID === undefined)
      .toSorted((a: Session.Info, b: Session.Info) => b.time.updated - a.time.updated)
      .map((x: Session.Info) => {
        const date = new Date(x.time.updated)
        let category = date.toDateString()
        if (category === today) {
          category = "Today"
        }
        const isDeleting = deleting === x.id
        const status = sync.data.session_status?.[x.id]
        const isWorking = status?.type === "busy"
        const autoInfo = autoSessions.get(x.id)
        const isAutonomous = autoInfo !== undefined

        // Determine gutter content
        let gutter: any = undefined
        if (isAutonomous) {
          // Autonomous mode indicator with level
          const levelColor =
            autoInfo.level === "lunatic"
              ? theme.error
              : autoInfo.level === "insane"
                ? theme.warning
                : autoInfo.level === "crazy"
                  ? theme.accent
                  : theme.primary
          gutter = (
            <Show when={kv.get("animations_enabled", true)} fallback={<text fg={levelColor}>[AUTO]</text>}>
              <box flexDirection="row" gap={1}>
                <spinner frames={crazyFrames} interval={200} color={levelColor} />
                <text fg={levelColor}>{String(autoInfo.level?.toUpperCase() ?? "AUTO")}</text>
              </box>
            </Show>
          )
        } else if (isWorking) {
          gutter = (
            <Show when={kv.get("animations_enabled", true)} fallback={<text fg={theme.textMuted}>[⋯]</text>}>
              <spinner frames={spinnerFrames} interval={80} color={theme.primary} />
            </Show>
          )
        }

        return {
          title: isDeleting ? `Press ${keybind.print("session_delete")} again to confirm` : x.title,
          bg: isDeleting ? theme.error : undefined,
          value: x.id,
          category,
          footer: Locale.time(x.time.updated),
          gutter,
        }
      })
  })

  onMount(() => {
    dialog.setSize("large")

    // Subscribe to autonomous events
    const unsubscribes: Array<() => void> = []

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SessionStarted, (event) => {
        setAutonomousSessions((prev) => {
          const next = new Map(prev)
          next.set(event.properties.sessionId, {
            level: event.properties.autonomyLevel,
            state: "PLANNING",
          })
          return next
        })
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.StateChanged, (event) => {
        // Try to find the session by state change and update it
        setAutonomousSessions((prev) => {
          const next = new Map(prev)
          for (const [sessionId, info] of next.entries()) {
            next.set(sessionId, {
              ...info,
              state: event.properties.to,
            })
          }
          return next
        })
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.MetricsUpdated, (event) => {
        setAutonomousSessions((prev) => {
          const next = new Map(prev)
          const existing = next.get(event.properties.sessionId)
          if (existing) {
            next.set(event.properties.sessionId, {
              ...existing,
              tasksCompleted: event.properties.metrics.tasksCompleted,
              tasksTotal: event.properties.metrics.tasksTotal,
            })
          }
          return next
        })
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SessionCompleted, (event) => {
        setAutonomousSessions((prev) => {
          const next = new Map(prev)
          next.delete(event.properties.sessionId)
          return next
        })
      }),
    )

    unsubscribes.push(
      Bus.subscribe(AutonomousEvent.SessionFailed, (event) => {
        setAutonomousSessions((prev) => {
          const next = new Map(prev)
          next.delete(event.properties.sessionId)
          return next
        })
      }),
    )

    onCleanup(() => {
      unsubscribes.forEach((unsub) => unsub())
    })
  })

  const handleDelete = async (sessionID: string) => {
    const current = toDelete()
    if (current === sessionID) {
      console.log(`[DEBUG SessionList] DELETING session ${sessionID}`)
      await sdk.client.session.delete({ sessionID })
      setToDelete(undefined)
      return
    }
    console.log(`[DEBUG SessionList] Setting toDelete=${sessionID}`)
    setToDelete(sessionID)
  }

  return (
    <DialogSelect
      title="Sessions"
      options={options()}
      skipFilter={true}
      current={currentSessionID()}
      onFilter={setSearch}
      onMove={() => {
        // Don't reset delete confirm when moving - allow user to move away and back
      }}
      onSelect={(option) => {
        route.navigate({
          type: "session",
          sessionID: option.value,
        })
        dialog.clear()
      }}
      keybind={[
        {
          keybind: keybind.all.session_delete?.[0],
          title: "delete",
          onTrigger: (option) => handleDelete(option.value as string),
        },
        {
          keybind: keybind.all.session_rename?.[0],
          title: "rename",
          onTrigger: async (option) => {
            dialog.replace(() => <DialogSessionRename session={option.value} />)
          },
        },
      ]}
    />
  )
}
