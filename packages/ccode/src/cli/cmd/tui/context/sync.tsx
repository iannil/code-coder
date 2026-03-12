import type {
  Message,
  AgentInfo,
  ProviderInfo,
  Session,
  Snapshot,
  Part,
  Config,
  TodoInfo,
  CommandInfo,
  PermissionRequest,
  QuestionRequest,
  LspStatus,
  McpStatus,
  McpResource,
  FormatterStatus,
  SessionStatusInfo,
  ProviderListResponse,
  ProviderAuthMethod,
  VcsInfo,
} from "@/types"

// Event payload types for type-safe event handling
type PermissionRepliedPayload = { sessionID: string; requestID: string }
type QuestionRepliedPayload = { sessionID: string; requestID: string }
type QuestionRejectedPayload = { sessionID: string; requestID: string }
type TodoUpdatedPayload = { sessionID: string; todos: TodoInfo[] }
type SessionDiffPayload = { sessionID: string; diff: Snapshot.FileDiff[] }
type SessionDeletedPayload = { info: Session.Info }
type SessionUpdatedPayload = { info: Session.Info }
type SessionStatusPayload = { sessionID: string; status: SessionStatusInfo }
type MessageUpdatedPayload = { info: Message }
type MessageRemovedPayload = { sessionID: string; messageID: string }
type MessagePartUpdatedPayload = { part: Part; delta?: string }
type MessagePartRemovedPayload = { sessionID: string; messageID: string; partID: string }
type VcsBranchUpdatedPayload = { branch?: string }
import { createStore, produce, reconcile } from "solid-js/store"
import { useSDK } from "@tui/context/sdk"
import { Binary } from "@codecoder-ai/core/util/binary"
import { createSimpleContext } from "./helper"
import { useExit } from "./exit"
import { useArgs } from "./args"
import { batch, onMount } from "solid-js"
import { Log } from "@/util/log"
import type { Path } from "@/types"
import { GlobalErrorHandler } from "@/util/global-error-handler"

export const { use: useSync, provider: SyncProvider } = createSimpleContext({
  name: "Sync",
  init: () => {
    const [store, setStore] = createStore<{
      status: "loading" | "partial" | "complete"
      provider: ProviderInfo[]
      provider_default: Record<string, string>
      provider_next: ProviderListResponse
      provider_auth: Record<string, ProviderAuthMethod[]>
      agent: AgentInfo[]
      command: CommandInfo[]
      permission: {
        [sessionID: string]: PermissionRequest[]
      }
      question: {
        [sessionID: string]: QuestionRequest[]
      }
      config: Config
      session: Session.Info[]
      session_status: {
        [sessionID: string]: SessionStatusInfo
      }
      session_diff: {
        [sessionID: string]: Snapshot.FileDiff[]
      }
      todo: {
        [sessionID: string]: TodoInfo[]
      }
      message: {
        [sessionID: string]: Message[]
      }
      part: {
        [messageID: string]: Part[]
      }
      lsp: LspStatus[]
      mcp: {
        [key: string]: McpStatus
      }
      mcp_resource: {
        [key: string]: McpResource
      }
      formatter: FormatterStatus[]
      vcs: VcsInfo | undefined
      path: Path
    }>({
      provider_next: {
        all: [],
        default: {},
        connected: [],
      },
      provider_auth: {},
      config: {},
      status: "loading",
      agent: [],
      permission: {},
      question: {},
      command: [],
      provider: [],
      provider_default: {},
      session: [],
      session_status: {},
      session_diff: {},
      todo: {},
      message: {},
      part: {},
      lsp: [],
      mcp: {},
      mcp_resource: {},
      formatter: [],
      vcs: undefined,
      path: { home: "", state: "", config: "", worktree: "", directory: "" },
    })

    const sdk = useSDK()

    sdk.event.listen((e) => {
      const event = e.details
      // Track all sync events for debugging
      GlobalErrorHandler.addContext(`sync:${event.type}`, event.properties)

      switch (event.type) {
        case "server.instance.disposed":
          bootstrap()
          break
        case "permission.replied": {
          const props = event.properties as PermissionRepliedPayload
          const requests = store.permission[props.sessionID]
          if (!requests) break
          const match = Binary.search(requests, props.requestID, (r) => r.id)
          if (!match.found) break
          setStore(
            "permission",
            props.sessionID,
            produce((draft) => {
              draft.splice(match.index, 1)
            }),
          )
          break
        }

        case "permission.asked": {
          const request = event.properties as PermissionRequest
          const requests = store.permission[request.sessionID]
          if (!requests) {
            setStore("permission", request.sessionID, [request])
            break
          }
          const match = Binary.search(requests, request.id, (r) => r.id)
          if (match.found) {
            setStore("permission", request.sessionID, match.index, reconcile(request))
            break
          }
          setStore(
            "permission",
            request.sessionID,
            produce((draft) => {
              draft.splice(match.index, 0, request)
            }),
          )
          break
        }

        case "question.replied":
        case "question.rejected": {
          const props = event.properties as QuestionRepliedPayload
          const requests = store.question[props.sessionID]
          if (!requests) break
          const match = Binary.search(requests, props.requestID, (r) => r.id)
          if (!match.found) break
          setStore(
            "question",
            props.sessionID,
            produce((draft) => {
              draft.splice(match.index, 1)
            }),
          )
          break
        }

        case "question.asked": {
          const request = event.properties as QuestionRequest
          const requests = store.question[request.sessionID]
          if (!requests) {
            setStore("question", request.sessionID, [request])
            break
          }
          const match = Binary.search(requests, request.id, (r) => r.id)
          if (match.found) {
            setStore("question", request.sessionID, match.index, reconcile(request))
            break
          }
          setStore(
            "question",
            request.sessionID,
            produce((draft) => {
              draft.splice(match.index, 0, request)
            }),
          )
          break
        }

        case "todo.updated": {
          const props = event.properties as TodoUpdatedPayload
          setStore("todo", props.sessionID, props.todos)
          break
        }

        case "session.diff": {
          const props = event.properties as SessionDiffPayload
          setStore("session_diff", props.sessionID, props.diff)
          break
        }

        case "session.deleted": {
          const props = event.properties as SessionDeletedPayload
          const result = Binary.search(store.session, props.info.id, (s) => s.id)
          if (result.found) {
            setStore(
              "session",
              produce((draft) => {
                draft.splice(result.index, 1)
              }),
            )
          }
          break
        }
        case "session.updated": {
          const props = event.properties as SessionUpdatedPayload
          const result = Binary.search(store.session, props.info.id, (s) => s.id)
          if (result.found) {
            setStore("session", result.index, reconcile(props.info))
            break
          }
          setStore(
            "session",
            produce((draft) => {
              draft.splice(result.index, 0, props.info)
            }),
          )
          break
        }

        case "session.status": {
          const props = event.properties as SessionStatusPayload
          setStore("session_status", props.sessionID, props.status)
          break
        }

        case "message.updated": {
          const props = event.properties as MessageUpdatedPayload
          const messages = store.message[props.info.sessionID]
          if (!messages) {
            setStore("message", props.info.sessionID, [props.info])
            break
          }
          const result = Binary.search(messages, props.info.id, (m) => m.id)
          if (result.found) {
            setStore("message", props.info.sessionID, result.index, reconcile(props.info))
            break
          }
          setStore(
            "message",
            props.info.sessionID,
            produce((draft) => {
              draft.splice(result.index, 0, props.info)
            }),
          )
          const updated = store.message[props.info.sessionID]
          if (updated.length > 100) {
            const oldest = updated[0]
            batch(() => {
              setStore(
                "message",
                props.info.sessionID,
                produce((draft) => {
                  draft.shift()
                }),
              )
              setStore(
                "part",
                produce((draft) => {
                  delete draft[oldest.id]
                }),
              )
            })
          }
          break
        }
        case "message.removed": {
          const props = event.properties as MessageRemovedPayload
          const messages = store.message[props.sessionID]
          const result = Binary.search(messages, props.messageID, (m) => m.id)
          if (result.found) {
            setStore(
              "message",
              props.sessionID,
              produce((draft) => {
                draft.splice(result.index, 1)
              }),
            )
          }
          break
        }
        case "message.part.updated": {
          const props = event.properties as MessagePartUpdatedPayload
          // Extra debugging for part updates since they drive most UI rendering
          const part = props.part
          GlobalErrorHandler.addContext("sync:part.detail", {
            type: part.type,
            id: part.id,
            messageID: part.messageID,
            tool: (part as any).tool,
            state: (part as any).state?.status,
          })
          const parts = store.part[props.part.messageID]
          if (!parts) {
            setStore("part", props.part.messageID, [props.part])
            break
          }
          const result = Binary.search(parts, props.part.id, (p) => p.id)
          if (result.found) {
            setStore("part", props.part.messageID, result.index, reconcile(props.part))
            break
          }
          setStore(
            "part",
            props.part.messageID,
            produce((draft) => {
              draft.splice(result.index, 0, props.part)
            }),
          )
          break
        }

        case "message.part.removed": {
          const props = event.properties as MessagePartRemovedPayload
          const parts = store.part[props.messageID]
          const result = Binary.search(parts, props.partID, (p) => p.id)
          if (result.found)
            setStore(
              "part",
              props.messageID,
              produce((draft) => {
                draft.splice(result.index, 1)
              }),
            )
          break
        }

        case "lsp.updated": {
          sdk.client.lsp.status().then((x: { data?: any[] }) => setStore("lsp", x.data!))
          break
        }

        case "vcs.branch.updated": {
          const props = event.properties as VcsBranchUpdatedPayload
          setStore("vcs", { branch: props.branch })
          break
        }
      }
    })

    const exit = useExit()
    const args = useArgs()

    async function bootstrap() {
      Log.Default.debug("bootstrapping")
      const start = Date.now() - 30 * 24 * 60 * 60 * 1000
      const sessionListPromise = sdk.client.session
        .list({ start: start })
        .then((x: any) =>
          setStore(
            "session",
            reconcile((x.data ?? []).toSorted((a: { id: string }, b: { id: string }) => a.id.localeCompare(b.id))),
          ),
        )

      // blocking - include session.list when continuing a session
      const blockingRequests: Promise<unknown>[] = [
        sdk.client.config.providers({}, { throwOnError: true }).then((x: any) => {
          batch(() => {
            setStore("provider", reconcile(x.data!.providers))
            setStore("provider_default", reconcile(x.data!.default))
          })
        }),
        sdk.client.provider.list({}, { throwOnError: true }).then((x: any) => {
          batch(() => {
            setStore("provider_next", reconcile(x.data!))
          })
        }),
        sdk.client.app.agents({}, { throwOnError: true }).then((x: any) => setStore("agent", reconcile(x.data ?? []))),
        sdk.client.config.get({}, { throwOnError: true }).then((x: any) => setStore("config", reconcile(x.data!))),
        ...(args.continue ? [sessionListPromise] : []),
      ]

      await Promise.all(blockingRequests)
        .then(() => {
          if (store.status !== "complete") setStore("status", "partial")
          // non-blocking
          Promise.all([
            ...(args.continue ? [] : [sessionListPromise]),
            sdk.client.command.list().then((x: any) => setStore("command", reconcile(x.data ?? []))),
            sdk.client.lsp.status().then((x: any) => setStore("lsp", reconcile(x.data!))),
            sdk.client.mcp.status().then((x: any) => setStore("mcp", reconcile(x.data!))),
            sdk.client.experimental.resource.list().then((x: any) => setStore("mcp_resource", reconcile(x.data ?? {}))),
            sdk.client.formatter.status().then((x: any) => setStore("formatter", reconcile(x.data!))),
            sdk.client.session.status().then((x: any) => {
              setStore("session_status", reconcile(x.data!))
            }),
            sdk.client.provider.auth().then((x: any) => setStore("provider_auth", reconcile(x.data ?? {}))),
            sdk.client.vcs.get().then((x: any) => setStore("vcs", reconcile(x.data))),
            sdk.client.path.get().then((x: any) => setStore("path", reconcile(x.data!))),
          ]).then(() => {
            setStore("status", "complete")
          })
        })
        .catch(async (e) => {
          Log.Default.error("tui bootstrap failed", {
            error: e instanceof Error ? e.message : String(e),
            name: e instanceof Error ? e.name : undefined,
            stack: e instanceof Error ? e.stack : undefined,
          })
          await exit(e)
        })
    }

    onMount(() => {
      bootstrap()
    })

    const fullSyncedSessions = new Set<string>()
    const result = {
      data: store,
      set: setStore,
      get status() {
        return store.status
      },
      get ready() {
        return store.status !== "loading"
      },
      session: {
        get(sessionID: string) {
          const match = Binary.search(store.session, sessionID, (s) => s.id)
          if (match.found) return store.session[match.index]
          return undefined
        },
        status(sessionID: string) {
          const session = result.session.get(sessionID)
          if (!session) return "idle"
          if (session.time.compacting) return "compacting"
          const messages = store.message[sessionID] ?? []
          const last = messages.at(-1)
          if (!last) return "idle"
          if (last.role === "user") return "working"
          return last.time.completed ? "idle" : "working"
        },
        async sync(sessionID: string) {
          if (fullSyncedSessions.has(sessionID)) return
          const [session, messages, todo, diff] = await Promise.all([
            sdk.client.session.get({ sessionID }, { throwOnError: true }),
            sdk.client.session.messages({ sessionID, limit: 100 }),
            sdk.client.session.todo({ sessionID }),
            sdk.client.session.diff({ sessionID }),
          ])
          setStore(
            produce((draft) => {
              const match = Binary.search(draft.session, sessionID, (s) => s.id)
              if (match.found) draft.session[match.index] = session.data!
              if (!match.found) draft.session.splice(match.index, 0, session.data!)
              draft.todo[sessionID] = todo.data ?? []
              draft.message[sessionID] = messages.data!.map((x: any) => x.info)
              for (const message of messages.data!) {
                draft.part[message.info.id] = message.parts
              }
              draft.session_diff[sessionID] = diff.data ?? []
            }),
          )
          fullSyncedSessions.add(sessionID)
        },
      },
      bootstrap,
    }
    return result
  },
})
