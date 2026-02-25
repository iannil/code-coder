import { render, useKeyboard, useRenderer, useTerminalDimensions } from "@opentui/solid"
import { Clipboard } from "@tui/util/clipboard"
import { TextAttributes } from "@opentui/core"

import { RouteProvider, useRoute } from "@tui/context/route"
import { Switch, Match, createEffect, untrack, ErrorBoundary, createSignal, onMount, batch, Show, on } from "solid-js"
import { VERSION } from "@/version"
import { Flag } from "@/flag/flag"
import { DialogProvider, useDialog } from "@tui/ui/dialog"
import { DialogProvider as DialogProviderList } from "@tui/component/dialog-provider"
import { SDKProvider, useSDK } from "@tui/context/sdk"
import { SyncProvider, useSync } from "@tui/context/sync"
import { LocalProvider, useLocal } from "@tui/context/local"
import { DialogModel, useConnected } from "@tui/component/dialog-model"
import { DialogMcp } from "@tui/component/dialog-mcp"
import { DialogStatus } from "@tui/component/dialog-status"
import { DialogThemeList } from "@tui/component/dialog-theme-list"
import { DialogHelp } from "./ui/dialog-help"
import { CommandProvider, useCommandDialog } from "@tui/component/dialog-command"
import { DialogAgent } from "@tui/component/dialog-agent"
import { DialogSessionList } from "@tui/component/dialog-session-list"
import { KeybindProvider } from "@tui/context/keybind"
import { ThemeProvider, useTheme } from "@tui/context/theme"
import { Home } from "@tui/routes/home"
import { Session } from "@tui/routes/session"
import { PromptHistoryProvider } from "./component/prompt/history"
import { FrecencyProvider } from "./component/prompt/frecency"
import { PromptStashProvider } from "./component/prompt/stash"
import { DialogAlert } from "./ui/dialog-alert"
import { ToastProvider, useToast } from "./ui/toast"
import { ExitProvider, useExit } from "./context/exit"
import { Session as SessionApi } from "@/session"
import { TuiEvent } from "./event"
import { KVProvider, useKV } from "./context/kv"
import { Provider } from "@/provider/provider"
import { ArgsProvider, useArgs, type Args } from "./context/args"
import open from "open"
import { writeHeapSnapshot } from "v8"
import { PromptRefProvider, usePromptRef } from "./context/prompt"
import { Log } from "@/util/log"
import { GlobalErrorHandler } from "@/util/global-error-handler"
import * as fs from "fs"

async function getTerminalBackgroundColor(): Promise<"dark" | "light"> {
  // Skip terminal background detection to avoid hanging
  // iTerm2 and some terminals may not respond properly to the escape sequence
  return "dark"
}

import type { EventSource } from "./context/sdk"

export function tui(input: {
  url: string
  args: Args
  directory?: string
  fetch?: typeof fetch
  events?: EventSource
  onExit?: () => Promise<void>
}) {
  // promise to prevent immediate exit
  return new Promise<void>(async (resolve) => {
    const mode = await getTerminalBackgroundColor()
    const onExit = async () => {
      await input.onExit?.()
      resolve()
    }

    // Log TUI errors to dev.log with enhanced context
    const logTuiError = (error: Error) => {
      // Gather context information
      const context: Record<string, unknown> = {
        url: input.url,
        directory: input.directory,
        mode,
        timestamp: Date.now(),
        processMemory: process.memoryUsage(),
      }

      // Try to extract more info from error
      if (error.message.includes("TextNodeRenderable")) {
        context.hint = "A non-string value was passed to a text element. Check for numbers, undefined, or objects being rendered directly."
      }

      GlobalErrorHandler.logError("TUI Fatal Error", error, context)
      Log.Default.error("tui_fatal", {
        name: error.name,
        message: error.message,
        stack: error.stack,
        context,
      })
    }

    render(
      () => {
        return (
          <ErrorBoundary
            fallback={(error, reset) => {
              logTuiError(error)
              return <ErrorComponent error={error} reset={reset} onExit={onExit} mode={mode} />
            }}
          >
            <ArgsProvider {...input.args}>
              <ExitProvider onExit={onExit}>
                <KVProvider>
                    <ToastProvider>
                    <RouteProvider>
                      <SDKProvider
                        url={input.url}
                        directory={input.directory}
                        fetch={input.fetch}
                        events={input.events}
                      >
                        <SyncProvider>
                          <ThemeProvider mode={mode}>
                            <LocalProvider>
                              <KeybindProvider>
                                <PromptStashProvider>
                                  <DialogProvider>
                                    <CommandProvider>
                                      <FrecencyProvider>
                                        <PromptHistoryProvider>
                                          <PromptRefProvider>
                                            <App />
                                          </PromptRefProvider>
                                        </PromptHistoryProvider>
                                      </FrecencyProvider>
                                    </CommandProvider>
                                  </DialogProvider>
                                </PromptStashProvider>
                              </KeybindProvider>
                            </LocalProvider>
                          </ThemeProvider>
                        </SyncProvider>
                      </SDKProvider>
                    </RouteProvider>
                  </ToastProvider>
                </KVProvider>
              </ExitProvider>
            </ArgsProvider>
          </ErrorBoundary>
        )
      },
      {
        targetFps: 60,
        gatherStats: false,
        exitOnCtrlC: false,
        useKittyKeyboard: {},
        consoleOptions: {
          keyBindings: [{ name: "y", ctrl: true, action: "copy-selection" }],
          onCopySelection: (text) => {
            Clipboard.copy(text).catch((error) => {
              console.error(`Failed to copy console selection to clipboard: ${error}`)
            })
          },
        },
      },
    )
  })
}

function App() {
  const route = useRoute()
  const dimensions = useTerminalDimensions()
  const renderer = useRenderer()
  renderer.disableStdoutInterception()
  const dialog = useDialog()
  const local = useLocal()
  const kv = useKV()
  const command = useCommandDialog()
  const sdk = useSDK()
  const toast = useToast()
  const { theme, mode, setMode } = useTheme()
  const sync = useSync()
  const exit = useExit()
  const promptRef = usePromptRef()

  // Wire up console copy-to-clipboard via opentui's onCopySelection callback
  renderer.console.onCopySelection = async (text: string) => {
    if (!text || text.length === 0) return

    await Clipboard.copy(text)
      .then(() => toast.show({ message: "Copied to clipboard", variant: "info" }))
      .catch(toast.error)
    renderer.clearSelection()
  }
  const [terminalTitleEnabled, setTerminalTitleEnabled] = createSignal(kv.get("terminal_title_enabled", true))

  createEffect(() => {
    Log.Default.debug(JSON.stringify(route.data))
  })

  // Update terminal window title based on current route and session
  createEffect(() => {
    if (!terminalTitleEnabled() || Flag.CCODE_DISABLE_TERMINAL_TITLE) return

    if (route.data.type === "home") {
      renderer.setTerminalTitle("CodeCoder")
      return
    }

    if (route.data.type === "session") {
      const session = sync.session.get(route.data.sessionID)
      if (!session || SessionApi.isDefaultTitle(session.title)) {
        renderer.setTerminalTitle("CodeCoder")
        return
      }

      // Truncate title to 40 chars max
      const title = session.title.length > 40 ? session.title.slice(0, 37) + "..." : session.title
      renderer.setTerminalTitle(`OC | ${title}`)
    }
  })

  const args = useArgs()
  onMount(() => {
    batch(() => {
      if (args.agent) local.agent.set(args.agent)
      if (args.model) {
        const { providerID, modelID } = Provider.parseModel(args.model)
        if (!providerID || !modelID)
          return toast.show({
            variant: "warning",
            message: `Invalid model format: ${args.model}`,
            duration: 3000,
          })
        local.model.set({ providerID, modelID }, { recent: true })
      }
      if (args.sessionID) {
        route.navigate({
          type: "session",
          sessionID: args.sessionID,
        })
      }
    })
  })

  let continued = false
  createEffect(() => {
    // When using -c, session list is loaded in blocking phase, so we can navigate at "partial"
    if (continued || sync.status === "loading" || !args.continue) return
    const match = sync.data.session
      .toSorted((a, b) => b.time.updated - a.time.updated)
      .find((x) => x.parentID === undefined)?.id
    if (match) {
      continued = true
      route.navigate({ type: "session", sessionID: match })
    }
  })

  createEffect(
    on(
      () => sync.status === "complete" && sync.data.provider.length === 0,
      (isEmpty, wasEmpty) => {
        // only trigger when we transition into an empty-provider state
        if (!isEmpty || wasEmpty) return
        dialog.replace(() => <DialogProviderList />)
      },
    ),
  )

  const connected = useConnected()
  command.register(() => [
    {
      title: "Switch session",
      value: "session.list",
      keybind: "session_list",
      category: "Session",
      suggested: sync.data.session.length > 0,
      slash: {
        name: "sessions",
        aliases: ["resume", "continue"],
      },
      onSelect: () => {
        dialog.replace(() => <DialogSessionList />)
      },
    },
    {
      title: "New session",
      suggested: route.data.type === "session",
      value: "session.new",
      keybind: "session_new",
      category: "Session",
      slash: {
        name: "new",
        aliases: ["clear"],
      },
      onSelect: () => {
        const current = promptRef.current
        // Don't require focus - if there's any text, preserve it
        const currentPrompt = current?.current?.input ? current.current : undefined
        local.agent.reset()
        route.navigate({
          type: "home",
          initialPrompt: currentPrompt,
        })
        dialog.clear()
      },
    },
    {
      title: "Switch model",
      value: "model.list",
      keybind: "model_list",
      suggested: true,
      category: "Agent",
      slash: {
        name: "models",
      },
      onSelect: () => {
        dialog.replace(() => <DialogModel />)
      },
    },
    {
      title: "Model cycle",
      value: "model.cycle_recent",
      keybind: "model_cycle_recent",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.model.cycle(1)
      },
    },
    {
      title: "Model cycle reverse",
      value: "model.cycle_recent_reverse",
      keybind: "model_cycle_recent_reverse",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.model.cycle(-1)
      },
    },
    {
      title: "Favorite cycle",
      value: "model.cycle_favorite",
      keybind: "model_cycle_favorite",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.model.cycleFavorite(1)
      },
    },
    {
      title: "Favorite cycle reverse",
      value: "model.cycle_favorite_reverse",
      keybind: "model_cycle_favorite_reverse",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.model.cycleFavorite(-1)
      },
    },
    {
      title: "Switch agent",
      value: "agent.list",
      keybind: "agent_list",
      category: "Agent",
      slash: {
        name: "agents",
      },
      onSelect: () => {
        dialog.replace(() => <DialogAgent />)
      },
    },
    {
      title: "Toggle MCPs",
      value: "mcp.list",
      category: "Agent",
      slash: {
        name: "mcps",
      },
      onSelect: () => {
        dialog.replace(() => <DialogMcp />)
      },
    },
    {
      title: "Agent cycle",
      value: "agent.cycle",
      keybind: "agent_cycle",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.agent.move(1)
      },
    },
    {
      title: "Variant cycle",
      value: "variant.cycle",
      keybind: "variant_cycle",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.model.variant.cycle()
      },
    },
    {
      title: "Agent cycle reverse",
      value: "agent.cycle.reverse",
      keybind: "agent_cycle_reverse",
      category: "Agent",
      hidden: true,
      onSelect: () => {
        local.agent.move(-1)
      },
    },
    {
      title: "Connect provider",
      value: "provider.connect",
      suggested: !connected(),
      slash: {
        name: "connect",
      },
      onSelect: () => {
        dialog.replace(() => <DialogProviderList />)
      },
      category: "Provider",
    },
    {
      title: "View status",
      keybind: "status_view",
      value: "codecoder.status",
      slash: {
        name: "status",
      },
      onSelect: () => {
        dialog.replace(() => <DialogStatus />)
      },
      category: "System",
    },
    {
      title: "Switch theme",
      value: "theme.switch",
      keybind: "theme_list",
      slash: {
        name: "themes",
      },
      onSelect: () => {
        dialog.replace(() => <DialogThemeList />)
      },
      category: "System",
    },
    {
      title: "Toggle appearance",
      value: "theme.switch_mode",
      onSelect: (dialog) => {
        setMode(mode() === "dark" ? "light" : "dark")
        dialog.clear()
      },
      category: "System",
    },
    {
      title: "Help",
      value: "help.show",
      slash: {
        name: "help",
      },
      onSelect: () => {
        dialog.replace(() => <DialogHelp />)
      },
      category: "System",
    },
    {
      title: "Open docs",
      value: "docs.open",
      onSelect: () => {
        open("https://code-coder.com/docs").catch(() => {})
        dialog.clear()
      },
      category: "System",
    },
    {
      title: "Open WebUI",
      value: "webui.open",
      onSelect: () => {
        open(sdk.url).catch(() => {})
        dialog.clear()
      },
      category: "System",
    },
    {
      title: "Exit app",
      value: "app.exit",
      slash: {
        name: "exit",
        aliases: ["quit", "q"],
      },
      onSelect: () => exit(),
      category: "System",
    },
    {
      title: "Toggle debug panel",
      category: "System",
      value: "app.debug",
      onSelect: (dialog) => {
        renderer.toggleDebugOverlay()
        dialog.clear()
      },
    },
    {
      title: "Toggle console",
      category: "System",
      value: "app.console",
      onSelect: (dialog) => {
        renderer.console.toggle()
        dialog.clear()
      },
    },
    {
      title: "Write heap snapshot",
      category: "System",
      value: "app.heap_snapshot",
      onSelect: (dialog) => {
        const path = writeHeapSnapshot()
        toast.show({
          variant: "info",
          message: `Heap snapshot written to ${path}`,
          duration: 5000,
        })
        dialog.clear()
      },
    },
    {
      title: "Suspend terminal",
      value: "terminal.suspend",
      keybind: "terminal_suspend",
      category: "System",
      hidden: true,
      onSelect: () => {
        process.once("SIGCONT", () => {
          renderer.resume()
        })

        renderer.suspend()
        // pid=0 means send the signal to all processes in the process group
        process.kill(0, "SIGTSTP")
      },
    },
    {
      title: terminalTitleEnabled() ? "Disable terminal title" : "Enable terminal title",
      value: "terminal.title.toggle",
      keybind: "terminal_title_toggle",
      category: "System",
      onSelect: (dialog) => {
        setTerminalTitleEnabled((prev) => {
          const next = !prev
          kv.set("terminal_title_enabled", next)
          if (!next) renderer.setTerminalTitle("")
          return next
        })
        dialog.clear()
      },
    },
  ])

  createEffect(() => {
    const currentModel = local.model.current()
    if (!currentModel) return
    if (currentModel.providerID === "openrouter" && !kv.get("openrouter_warning", false)) {
      untrack(() => {
        DialogAlert.show(
          dialog,
          "Warning",
          "While openrouter is a convenient way to access LLMs your request will often be routed to subpar providers that do not work well in our testing.\n\nFor reliable access to models check out CodeCoder Zen\nhttps://code-coder.com/zen",
        ).then(() => kv.set("openrouter_warning", true))
      })
    }
  })

  sdk.event.on(TuiEvent.CommandExecute.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.CommandExecute", evt.properties)
    command.trigger(evt.properties.command)
  })

  sdk.event.on(TuiEvent.ToastShow.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.ToastShow", evt.properties)
    toast.show({
      title: evt.properties.title,
      message: evt.properties.message,
      variant: evt.properties.variant,
      duration: evt.properties.duration,
    })
  })

  sdk.event.on(TuiEvent.SessionSelect.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.SessionSelect", evt.properties)
    route.navigate({
      type: "session",
      sessionID: evt.properties.sessionID,
    })
  })

  sdk.event.on(TuiEvent.ModelCall.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.ModelCall", evt.properties)
    const { providerID, modelID, agent, sessionID } = evt.properties
    // Defer toast.show to avoid interfering with event processing
    setTimeout(() => {
      toast.show({
        variant: "info",
        message: `[${String(agent)}] ${String(providerID)}/${String(modelID)}`,
        duration: 3000,
      })
    }, 0)
  })

  // Track writer agent progress for long-form tasks
  sdk.event.on(TuiEvent.WriterProgress.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.WriterProgress", evt.properties)
    const { action, chapter, total, message } = evt.properties
    let progressMessage = ""

    switch (action) {
      case "outline":
        progressMessage = `📋 Outline: ${total || 0} chapters planned`
        break
      case "chapter_start":
        progressMessage = `✍️  Chapter ${chapter}/${total}...`
        break
      case "chapter_complete":
        progressMessage = `✅ Chapter ${chapter}/${total} complete`
        break
      case "complete":
        progressMessage = `🎉 Writing complete!`
        break
      case "error":
        progressMessage = `⚠️ Error: ${message || "Unknown"}`
        break
    }

    if (progressMessage) {
      toast.show({
        variant: action === "error" ? "error" : action === "chapter_complete" ? "success" : "info",
        message: progressMessage,
        duration: action === "error" ? 8000 : 2000,
      })
    }
  })

  // Track expander agent execution stats for long-form content generation
  sdk.event.on(TuiEvent.WriterStats.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.WriterStats", evt.properties)
    const { status, agentType, elapsedSeconds, wordsGenerated, filesWritten, writesPending, isStalled } = evt.properties

    const minutes = Math.floor(elapsedSeconds / 60)
    const seconds = elapsedSeconds % 60
    const timeStr = minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`

    // Format word count with K suffix for large numbers
    const formatWords = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n))

    let message = ""
    let variant: "info" | "warning" | "success" = "info"

    switch (status) {
      case "started":
        message = `📝 [${agentType}] 开始生成...`
        break
      case "running":
        message = `📊 [${agentType}] ${timeStr} | ${formatWords(wordsGenerated)}字`
        if (filesWritten > 0) {
          message += ` | ${filesWritten}文件已写入`
        }
        if (writesPending > 0) {
          message += ` (${writesPending}待写入)`
        }
        if (isStalled) {
          message += " ⚠️ 响应缓慢"
          variant = "warning"
        }
        break
      case "completed":
        message = `✅ [${agentType}] 完成 | ${timeStr} | ${formatWords(wordsGenerated)}字`
        if (filesWritten > 0) {
          message += ` | ${filesWritten}文件`
        }
        variant = "success"
        break
    }

    toast.show({
      variant,
      message,
      duration: status === "running" ? 5000 : 3000,
    })
  })

  // Track chapter draft saves for progress protection
  sdk.event.on(TuiEvent.ChapterDraftSaved.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.ChapterDraftSaved", evt.properties)
    const { wordsWritten, saveCount } = evt.properties

    const formatWords = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n))

    toast.show({
      variant: "info",
      message: `💾 草稿已保存 (${saveCount}) | ${formatWords(wordsWritten)}字`,
      duration: 2000,
    })
  })

  sdk.event.on(TuiEvent.ChapterDraftFinalized.type, (evt) => {
    GlobalErrorHandler.addContext("TuiEvent.ChapterDraftFinalized", evt.properties)
    const { wordsWritten, totalSaves } = evt.properties

    const formatWords = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n))

    toast.show({
      variant: "success",
      message: `✅ 章节已完成 | ${formatWords(wordsWritten)}字 | 共${totalSaves}次保存`,
      duration: 3000,
    })
  })

  sdk.event.on(SessionApi.Event.Deleted.type, (evt) => {
    GlobalErrorHandler.addContext("SessionApi.Event.Deleted", evt.properties)
    if (route.data.type === "session" && route.data.sessionID === evt.properties.info.id) {
      route.navigate({ type: "home" })
      toast.show({
        variant: "info",
        message: "The current session was deleted",
      })
    }
  })

  sdk.event.on(SessionApi.Event.Error.type, (evt) => {
    GlobalErrorHandler.addContext("SessionApi.Event.Error", evt.properties)
    const error = evt.properties.error
    if (error && typeof error === "object" && error.name === "MessageAbortedError") return
    const message = (() => {
      if (!error) return "An error occurred"

      if (typeof error === "object" && error !== null && !Array.isArray(error)) {
        const data = (error as any).data
        if (typeof data === "object" && data !== null && !Array.isArray(data) && "message" in data && typeof data.message === "string") {
          return data.message
        }
      }
      return String(error)
    })()

    toast.show({
      variant: "error",
      message,
      duration: 5000,
    })
  })

  return (
    <box
      width={dimensions().width}
      height={dimensions().height}
      backgroundColor={theme.background}
      onMouseUp={async () => {
        if (Flag.CCODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT) {
          renderer.clearSelection()
          return
        }
        const text = renderer.getSelection()?.getSelectedText()
        if (text && text.length > 0) {
          await Clipboard.copy(text)
            .then(() => toast.show({ message: "Copied to clipboard", variant: "info" }))
            .catch(toast.error)
          renderer.clearSelection()
        }
      }}
    >
      <Switch>
        <Match when={route.data.type === "home"}>
          <Home />
        </Match>
        <Match when={route.data.type === "session"}>
          <Session />
        </Match>
      </Switch>
    </box>
  )
}

function ErrorComponent(props: {
  error: Error
  reset: () => void
  onExit: () => Promise<void>
  mode?: "dark" | "light"
}) {
  const term = useTerminalDimensions()
  const renderer = useRenderer()

  const handleExit = async () => {
    renderer.setTerminalTitle("")
    renderer.destroy()
    props.onExit()
  }

  useKeyboard((evt) => {
    if (evt.ctrl && evt.name === "c") {
      handleExit()
    }
  })
  const [copied, setCopied] = createSignal(false)

  const issueURL = new URL("https://github.com/iannil/code-coder/issues/new?template=bug-report.yml")

  // Choose safe fallback colors per mode since theme context may not be available
  const isLight = props.mode === "light"
  const colors = {
    bg: isLight ? "#ffffff" : "#0a0a0a",
    text: isLight ? "#1a1a1a" : "#eeeeee",
    muted: isLight ? "#8a8a8a" : "#808080",
    primary: isLight ? "#3b7dd8" : "#fab283",
  }

  if (props.error.message) {
    issueURL.searchParams.set("title", `opentui: fatal: ${props.error.message}`)
  }

  if (props.error.stack) {
    issueURL.searchParams.set(
      "description",
      "```\n" + props.error.stack.substring(0, 6000 - issueURL.toString().length) + "...\n```",
    )
  }

  issueURL.searchParams.set("codecoder-version", VERSION)

  const copyIssueURL = () => {
    Clipboard.copy(issueURL.toString()).then(() => {
      setCopied(true)
    })
  }

  return (
    <box flexDirection="column" gap={1} backgroundColor={colors.bg}>
      <box flexDirection="row" gap={1} alignItems="center">
        <text attributes={TextAttributes.BOLD} fg={colors.text}>
          Please report an issue.
        </text>
        <box onMouseUp={copyIssueURL} backgroundColor={colors.primary} padding={1}>
          <text attributes={TextAttributes.BOLD} fg={colors.bg}>
            Copy issue URL (exception info pre-filled)
          </text>
        </box>
        {copied() && <text fg={colors.muted}>Successfully copied</text>}
      </box>
      <box flexDirection="row" gap={2} alignItems="center">
        <text fg={colors.text}>A fatal error occurred!</text>
        <box onMouseUp={props.reset} backgroundColor={colors.primary} padding={1}>
          <text fg={colors.bg}>Reset TUI</text>
        </box>
        <box onMouseUp={handleExit} backgroundColor={colors.primary} padding={1}>
          <text fg={colors.bg}>Exit</text>
        </box>
      </box>
      <scrollbox height={Math.floor(term().height * 0.7)}>
        <text fg={colors.muted}>{props.error.stack}</text>
      </scrollbox>
      <text fg={colors.text}>{props.error.message}</text>
    </box>
  )
}
