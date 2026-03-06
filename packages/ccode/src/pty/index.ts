import { BusEvent } from "@/bus/bus-event"
import { Bus } from "@/bus"
import z from "zod"
import { Identifier } from "../id/id"
import { Log } from "@/util/log"
import type { WSContext } from "hono/ws"
import { Instance } from "../project/instance"
import { Shell } from "@/shell/shell"
import { spawnPty, type NapiPtyConfig, type PtySessionHandleType } from "@codecoder-ai/core"

export namespace Pty {
  const log = Log.create({ service: "pty" })

  const BUFFER_LIMIT = 1024 * 1024 * 2
  const BUFFER_CHUNK = 64 * 1024
  const POLL_INTERVAL_MS = 50

  export const Info = z
    .object({
      id: Identifier.schema("pty"),
      title: z.string(),
      command: z.string(),
      args: z.array(z.string()),
      cwd: z.string(),
      status: z.enum(["running", "exited"]),
      pid: z.number(),
    })
    .meta({ ref: "Pty" })

  export type Info = z.infer<typeof Info>

  export const CreateInput = z.object({
    command: z.string().optional(),
    args: z.array(z.string()).optional(),
    cwd: z.string().optional(),
    title: z.string().optional(),
    env: z.record(z.string(), z.string()).optional(),
  })

  export type CreateInput = z.infer<typeof CreateInput>

  export const UpdateInput = z.object({
    title: z.string().optional(),
    size: z
      .object({
        rows: z.number(),
        cols: z.number(),
      })
      .optional(),
  })

  export type UpdateInput = z.infer<typeof UpdateInput>

  export const Event = {
    Created: BusEvent.define("pty.created", z.object({ info: Info })),
    Updated: BusEvent.define("pty.updated", z.object({ info: Info })),
    Exited: BusEvent.define("pty.exited", z.object({ id: Identifier.schema("pty"), exitCode: z.number() })),
    Deleted: BusEvent.define("pty.deleted", z.object({ id: Identifier.schema("pty") })),
  }

  interface ActiveSession {
    info: Info
    handle: PtySessionHandleType
    buffer: string
    subscribers: Set<WSContext>
    pollTimer: Timer | null
    exitCheckTimer: Timer | null
  }

  const state = Instance.state(
    () => new Map<string, ActiveSession>(),
    async (sessions) => {
      for (const session of sessions.values()) {
        try {
          if (session.pollTimer) clearInterval(session.pollTimer)
          if (session.exitCheckTimer) clearInterval(session.exitCheckTimer)
          session.handle.kill()
        } catch {
          // Process may already be dead - ignore
        }
        for (const ws of session.subscribers) {
          ws.close()
        }
      }
      sessions.clear()
    },
  )

  export function list() {
    return Array.from(state().values()).map((s) => s.info)
  }

  export function get(id: string) {
    return state().get(id)?.info
  }

  export async function create(input: CreateInput) {
    const id = Identifier.create("pty", false)
    const command = input.command || Shell.preferred()
    const args = input.args || []
    if (command.endsWith("sh")) {
      args.push("-l")
    }

    const cwd = input.cwd || Instance.directory
    const env = {
      ...process.env,
      ...input.env,
      TERM: "xterm-256color",
      CCODE_TERMINAL: "1",
    } as Record<string, string>
    log.info("creating session", { id, cmd: command, args, cwd })

    // Check if native PTY is available
    if (!spawnPty) {
      throw new Error("Native PTY support not available. Ensure zero-core is built with PTY feature.")
    }

    // Build command string for native PTY
    const fullCommand = args.length > 0 ? `${command} ${args.join(" ")}` : command

    const config: NapiPtyConfig = {
      cols: 80,
      rows: 24,
      shell: fullCommand,
      cwd,
      env,
      inheritEnv: false, // We provide full env above
    }

    const handle = spawnPty(config)
    const nativeInfo = handle.info()

    const info: Info = {
      id,
      title: input.title || `Terminal ${id.slice(-4)}`,
      command,
      args,
      cwd,
      status: "running",
      // Native PTY doesn't expose PID directly, use a placeholder based on session ID hash
      pid: Math.abs(hashCode(nativeInfo.id)),
    }

    const session: ActiveSession = {
      info,
      handle,
      buffer: "",
      subscribers: new Set(),
      pollTimer: null,
      exitCheckTimer: null,
    }
    state().set(id, session)

    // Start polling for output
    session.pollTimer = setInterval(() => {
      try {
        const data = session.handle.read()
        if (data && data.length > 0) {
          const dataStr = data.toString("utf-8")
          let open = false
          for (const ws of session.subscribers) {
            if (ws.readyState !== 1) {
              session.subscribers.delete(ws)
              continue
            }
            open = true
            ws.send(dataStr)
          }
          if (!open) {
            session.buffer += dataStr
            if (session.buffer.length > BUFFER_LIMIT) {
              session.buffer = session.buffer.slice(-BUFFER_LIMIT)
            }
          }
        }
      } catch {
        // Read error - process may have exited
      }
    }, POLL_INTERVAL_MS)

    // Start checking for exit
    session.exitCheckTimer = setInterval(() => {
      if (!session.handle.isRunning()) {
        const exitCode = session.handle.exitCode() ?? 0
        log.info("session exited", { id, exitCode })
        session.info.status = "exited"

        // Clean up timers
        if (session.pollTimer) clearInterval(session.pollTimer)
        if (session.exitCheckTimer) clearInterval(session.exitCheckTimer)
        session.pollTimer = null
        session.exitCheckTimer = null

        // Notify subscribers
        for (const ws of session.subscribers) {
          ws.close()
        }
        session.subscribers.clear()
        Bus.publish(Event.Exited, { id, exitCode })
        state().delete(id)
      }
    }, POLL_INTERVAL_MS * 2)

    Bus.publish(Event.Created, { info })
    return info
  }

  export async function update(id: string, input: UpdateInput) {
    const session = state().get(id)
    if (!session) return
    if (input.title) {
      session.info.title = input.title
    }
    if (input.size) {
      session.handle.resize(input.size.cols, input.size.rows)
    }
    Bus.publish(Event.Updated, { info: session.info })
    return session.info
  }

  export async function remove(id: string) {
    const session = state().get(id)
    if (!session) return
    log.info("removing session", { id })

    // Clean up timers
    if (session.pollTimer) clearInterval(session.pollTimer)
    if (session.exitCheckTimer) clearInterval(session.exitCheckTimer)

    try {
      session.handle.kill()
    } catch {
      // Process may already be dead - ignore
    }
    for (const ws of session.subscribers) {
      ws.close()
    }
    state().delete(id)
    Bus.publish(Event.Deleted, { id })
  }

  export function resize(id: string, cols: number, rows: number) {
    const session = state().get(id)
    if (session && session.info.status === "running") {
      session.handle.resize(cols, rows)
    }
  }

  export function write(id: string, data: string) {
    const session = state().get(id)
    if (session && session.info.status === "running") {
      session.handle.write(Buffer.from(data))
    }
  }

  export function connect(id: string, ws: WSContext) {
    const session = state().get(id)
    if (!session) {
      ws.close()
      return
    }
    log.info("client connected to session", { id })
    session.subscribers.add(ws)
    if (session.buffer) {
      const buffer = session.buffer.length <= BUFFER_LIMIT ? session.buffer : session.buffer.slice(-BUFFER_LIMIT)
      session.buffer = ""
      try {
        for (let i = 0; i < buffer.length; i += BUFFER_CHUNK) {
          ws.send(buffer.slice(i, i + BUFFER_CHUNK))
        }
      } catch {
        session.subscribers.delete(ws)
        session.buffer = buffer
        ws.close()
        return
      }
    }
    return {
      onMessage: (message: string | ArrayBuffer) => {
        session.handle.write(Buffer.from(String(message)))
      },
      onClose: () => {
        log.info("client disconnected from session", { id })
        session.subscribers.delete(ws)
      },
    }
  }

  // Simple hash function for generating pseudo-PID from session ID
  function hashCode(str: string): number {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = ((hash << 5) - hash) + char
      hash = hash & hash // Convert to 32bit integer
    }
    return hash
  }
}
