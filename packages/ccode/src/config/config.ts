import { Log } from "@/util/log"
import path from "path"
import { pathToFileURL } from "url"
import os from "os"
import z from "zod"
import { Filesystem } from "@/util/filesystem"
import { ModelsDev } from "../provider/models"
import { mergeDeep, pipe, unique } from "remeda"
import { Global } from "../global"
import fs from "fs/promises"
import { lazy } from "@/util/lazy"
import { NamedError } from "@codecoder-ai/util/error"
import { Flag } from "../flag/flag"
import {
  type ParseError as JsoncParseError,
  applyEdits,
  modify,
  parse as parseJsonc,
  printParseErrorCode,
} from "jsonc-parser"
import { Instance } from "../project/instance"
import { LSPServer } from "../lsp/server"
import { BunProc } from "@/bun"
import { ConfigMarkdown } from "./markdown"
import { existsSync } from "fs"
import { Bus } from "@/bus"

export namespace Config {
  const log = Log.create({ service: "config" })

  // Custom merge function that concatenates array fields instead of replacing them
  function mergeConfigConcatArrays(target: Info, source: Info): Info {
    const merged = mergeDeep(target, source)
    if (target.instructions && source.instructions) {
      merged.instructions = Array.from(new Set([...target.instructions, ...source.instructions]))
    }
    return merged
  }

  export const state = Instance.state(async () => {
    let result: Info = {}

    // Global user config overrides remote config
    result = mergeConfigConcatArrays(result, await global())

    // Custom config path overrides global
    if (Flag.CCODE_CONFIG) {
      result = mergeConfigConcatArrays(result, await loadFile(Flag.CCODE_CONFIG))
      log.debug("loaded custom config", { path: Flag.CCODE_CONFIG })
    }

    // Project config has highest precedence (overrides global and remote)
    if (!Flag.CCODE_DISABLE_PROJECT_CONFIG) {
      for (const file of ["codecoder.jsonc", "codecoder.json", "codecoder.jsonc", "codecoder.json"]) {
        const found = await Filesystem.findUp(file, Instance.directory, Instance.worktree)
        for (const resolved of found.toReversed()) {
          result = mergeConfigConcatArrays(result, await loadFile(resolved))
        }
      }
    }

    // Inline config content has highest precedence
    if (Flag.CCODE_CONFIG_CONTENT) {
      result = mergeConfigConcatArrays(result, JSON.parse(Flag.CCODE_CONFIG_CONTENT))
      log.debug("loaded custom config from CCODE_CONFIG_CONTENT")
    }

    result.agent = result.agent || {}
    result.mode = result.mode || {}

    const directories = [
      Global.Path.config,
      // Only scan project .codecoder/ directories when project discovery is enabled
      ...(!Flag.CCODE_DISABLE_PROJECT_CONFIG
        ? await Array.fromAsync(
            Filesystem.up({
              targets: [".codecoder"],
              start: Instance.directory,
              stop: Instance.worktree,
            }),
          )
        : []),
      // Always scan ~/.codecoder/ (user home directory)
      ...(await Array.fromAsync(
        Filesystem.up({
          targets: [".codecoder"],
          start: Global.Path.home,
          stop: Global.Path.home,
        }),
      )),
    ]

    if (Flag.CCODE_CONFIG_DIR) {
      directories.push(Flag.CCODE_CONFIG_DIR)
      log.debug("loading config from CCODE_CONFIG_DIR", { path: Flag.CCODE_CONFIG_DIR })
    }

    for (const dir of unique(directories)) {
      if (dir.endsWith(".codecoder") || dir === Flag.CCODE_CONFIG_DIR) {
        for (const file of ["config.jsonc", "codecoder.jsonc", "codecoder.json"]) {
          log.debug(`loading config from ${path.join(dir, file)}`)
          result = mergeConfigConcatArrays(result, await loadFile(path.join(dir, file)))
          // to satisfy the type checker
          result.agent ??= {}
          result.mode ??= {}
        }
      }

      const exists = existsSync(path.join(dir, "node_modules"))
      const installing = installDependencies(dir)
      if (!exists) await installing

      result.command = mergeDeep(result.command ?? {}, await loadCommand(dir))
      result.agent = mergeDeep(result.agent, await loadAgent(dir))
      result.agent = mergeDeep(result.agent, await loadMode(dir))
    }

    // Migrate deprecated mode field to agent field
    for (const [name, mode] of Object.entries(result.mode)) {
      result.agent = mergeDeep(result.agent ?? {}, {
        [name]: {
          ...mode,
          mode: "primary" as const,
        },
      })
    }

    if (Flag.CCODE_PERMISSION) {
      result.permission = mergeDeep(result.permission ?? {}, JSON.parse(Flag.CCODE_PERMISSION))
    }

    // Backwards compatibility: legacy top-level `tools` config
    if (result.tools) {
      const perms: Record<string, Config.PermissionAction> = {}
      for (const [tool, enabled] of Object.entries(result.tools)) {
        const action: Config.PermissionAction = enabled ? "allow" : "deny"
        if (tool === "write" || tool === "edit" || tool === "patch" || tool === "multiedit") {
          perms.edit = action
          continue
        }
        perms[tool] = action
      }
      result.permission = mergeDeep(perms, result.permission ?? {})
    }

    if (!result.username) result.username = os.userInfo().username

    if (!result.keybinds) result.keybinds = Info.shape.keybinds.parse({})

    // Apply flag overrides for compaction settings
    if (Flag.CCODE_DISABLE_AUTOCOMPACT) {
      result.compaction = { ...result.compaction, auto: false }
    }
    if (Flag.CCODE_DISABLE_PRUNE) {
      result.compaction = { ...result.compaction, prune: false }
    }

    return {
      config: result,
      directories,
    }
  })

  export async function installDependencies(dir: string) {
    const pkg = path.join(dir, "package.json")

    if (!(await Bun.file(pkg).exists())) {
      await Bun.write(pkg, "{}")
    }

    const gitignore = path.join(dir, ".gitignore")
    const hasGitIgnore = await Bun.file(gitignore).exists()
    if (!hasGitIgnore) await Bun.write(gitignore, ["node_modules", "package.json", "bun.lock", ".gitignore"].join("\n"))

    // Install any additional dependencies defined in the package.json
    await BunProc.run(["install"], { cwd: dir }).catch(() => {})
  }

  function rel(item: string, patterns: string[]) {
    for (const pattern of patterns) {
      const index = item.indexOf(pattern)
      if (index === -1) continue
      return item.slice(index + pattern.length)
    }
  }

  function trim(file: string) {
    const ext = path.extname(file)
    return ext.length ? file.slice(0, -ext.length) : file
  }

  const COMMAND_GLOB = new Bun.Glob("{command,commands}/**/*.md")
  async function loadCommand(dir: string) {
    const result: Record<string, Command> = {}
    for await (const item of COMMAND_GLOB.scan({
      absolute: true,
      followSymlinks: true,
      dot: true,
      cwd: dir,
    })) {
      const md = await ConfigMarkdown.parse(item).catch(async (err) => {
        const message = ConfigMarkdown.FrontmatterError.isInstance(err)
          ? err.data.message
          : `Failed to parse command ${item}`
        const { Session } = await import("@/session")
        Bus.publish(Session.Event.Error, { error: new NamedError.Unknown({ message }).toObject() })
        log.error("failed to load command", { command: item, err })
        return undefined
      })
      if (!md) continue

      const patterns = ["/.codecoder/commands/", "/.codecoder/command/", "/command/", "/commands/"]
      const file = rel(item, patterns) ?? path.basename(item)
      const name = trim(file)

      const config = {
        name,
        ...md.data,
        template: md.content.trim(),
      }
      const parsed = Command.safeParse(config)
      if (parsed.success) {
        result[config.name] = parsed.data
        continue
      }
      throw new InvalidError({ path: item, issues: parsed.error.issues }, { cause: parsed.error })
    }
    return result
  }

  const AGENT_GLOB = new Bun.Glob("{agent,agents}/**/*.md")
  async function loadAgent(dir: string) {
    const result: Record<string, Agent> = {}

    for await (const item of AGENT_GLOB.scan({
      absolute: true,
      followSymlinks: true,
      dot: true,
      cwd: dir,
    })) {
      const md = await ConfigMarkdown.parse(item).catch(async (err) => {
        const message = ConfigMarkdown.FrontmatterError.isInstance(err)
          ? err.data.message
          : `Failed to parse agent ${item}`
        const { Session } = await import("@/session")
        Bus.publish(Session.Event.Error, { error: new NamedError.Unknown({ message }).toObject() })
        log.error("failed to load agent", { agent: item, err })
        return undefined
      })
      if (!md) continue

      const patterns = ["/.codecoder/agents/", "/.codecoder/agent/", "/agent/", "/agents/"]
      const file = rel(item, patterns) ?? path.basename(item)
      const agentName = trim(file)

      const config = {
        name: agentName,
        ...md.data,
        prompt: md.content.trim(),
      }
      const parsed = Agent.safeParse(config)
      if (parsed.success) {
        result[config.name] = parsed.data
        continue
      }
      throw new InvalidError({ path: item, issues: parsed.error.issues }, { cause: parsed.error })
    }
    return result
  }

  const MODE_GLOB = new Bun.Glob("{mode,modes}/*.md")
  async function loadMode(dir: string) {
    const result: Record<string, Agent> = {}
    for await (const item of MODE_GLOB.scan({
      absolute: true,
      followSymlinks: true,
      dot: true,
      cwd: dir,
    })) {
      const md = await ConfigMarkdown.parse(item).catch(async (err) => {
        const message = ConfigMarkdown.FrontmatterError.isInstance(err)
          ? err.data.message
          : `Failed to parse mode ${item}`
        const { Session } = await import("@/session")
        Bus.publish(Session.Event.Error, { error: new NamedError.Unknown({ message }).toObject() })
        log.error("failed to load mode", { mode: item, err })
        return undefined
      })
      if (!md) continue

      const config = {
        name: path.basename(item, ".md"),
        ...md.data,
        prompt: md.content.trim(),
      }
      const parsed = Agent.safeParse(config)
      if (parsed.success) {
        result[config.name] = {
          ...parsed.data,
          mode: "primary" as const,
        }
        continue
      }
    }
    return result
  }

  export const McpLocal = z
    .object({
      type: z.literal("local").describe("Type of MCP server connection"),
      command: z.string().array().describe("Command and arguments to run the MCP server"),
      environment: z
        .record(z.string(), z.string())
        .optional()
        .describe("Environment variables to set when running the MCP server"),
      enabled: z.boolean().optional().describe("Enable or disable the MCP server on startup"),
      timeout: z
        .number()
        .int()
        .positive()
        .optional()
        .describe("Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."),
    })
    .strict()
    .meta({
      ref: "McpLocalConfig",
    })

  export const McpOAuth = z
    .object({
      clientId: z
        .string()
        .optional()
        .describe("OAuth client ID. If not provided, dynamic client registration (RFC 7591) will be attempted."),
      clientSecret: z.string().optional().describe("OAuth client secret (if required by the authorization server)"),
      scope: z.string().optional().describe("OAuth scopes to request during authorization"),
    })
    .strict()
    .meta({
      ref: "McpOAuthConfig",
    })
  export type McpOAuth = z.infer<typeof McpOAuth>

  export const McpRemote = z
    .object({
      type: z.literal("remote").describe("Type of MCP server connection"),
      url: z.string().describe("URL of the remote MCP server"),
      enabled: z.boolean().optional().describe("Enable or disable the MCP server on startup"),
      headers: z.record(z.string(), z.string()).optional().describe("Headers to send with the request"),
      oauth: z
        .union([McpOAuth, z.literal(false)])
        .optional()
        .describe(
          "OAuth authentication configuration for the MCP server. Set to false to disable OAuth auto-detection.",
        ),
      timeout: z
        .number()
        .int()
        .positive()
        .optional()
        .describe("Timeout in ms for MCP server requests. Defaults to 5000 (5 seconds) if not specified."),
    })
    .strict()
    .meta({
      ref: "McpRemoteConfig",
    })

  export const Mcp = z.discriminatedUnion("type", [McpLocal, McpRemote])
  export type Mcp = z.infer<typeof Mcp>

  export const RedisConfig = z
    .object({
      url: z.string().default("redis://localhost:4410").describe("Redis connection URL"),
      password: z.string().optional().describe("Redis password"),
      db: z.number().int().min(0).max(15).default(0).describe("Redis database number"),
      keyPrefix: z.string().default("codecoder:").describe("Key prefix for all Redis keys"),
      connectTimeout: z.number().int().positive().default(5000).describe("Connection timeout in ms"),
      commandTimeout: z.number().int().positive().default(3000).describe("Command timeout in ms"),
      maxRetriesPerRequest: z.number().int().min(0).default(3).describe("Max retries per request"),
    })
    .strict()
    .optional()
    .describe("Redis configuration for conversation store")
  export type RedisConfig = z.infer<typeof RedisConfig>

  export const McpDisabled = z
    .object({
      enabled: z.boolean(),
    })
    .strict()
  export type McpDisabled = z.infer<typeof McpDisabled>

  export const McpServerConfig = z
    .object({
      apiKey: z.string().optional().describe("API key for MCP server authentication"),
      port: z.number().int().positive().optional().describe("Default port for HTTP transport"),
      defaultTransport: z.enum(["stdio", "http"]).optional().describe("Default transport mode"),
      resources: z.array(z.string()).optional().describe("Glob patterns for additional resources to expose"),
    })
    .strict()
    .meta({
      ref: "McpServerConfig",
    })
  export type McpServerConfig = z.infer<typeof McpServerConfig>

  export const PermissionAction = z.enum(["ask", "allow", "deny"]).meta({
    ref: "PermissionActionConfig",
  })
  export type PermissionAction = z.infer<typeof PermissionAction>

  export const PermissionObject = z.record(z.string(), PermissionAction).meta({
    ref: "PermissionObjectConfig",
  })
  export type PermissionObject = z.infer<typeof PermissionObject>

  export const PermissionRule = z.union([PermissionAction, PermissionObject]).meta({
    ref: "PermissionRuleConfig",
  })
  export type PermissionRule = z.infer<typeof PermissionRule>

  // Capture original key order before zod reorders, then rebuild in original order
  const permissionPreprocess = (val: unknown) => {
    if (typeof val === "object" && val !== null && !Array.isArray(val)) {
      return { __originalKeys: Object.keys(val), ...val }
    }
    return val
  }

  const permissionTransform = (x: unknown): Record<string, PermissionRule> => {
    if (typeof x === "string") return { "*": x as PermissionAction }
    const obj = x as { __originalKeys?: string[] } & Record<string, unknown>
    const { __originalKeys, ...rest } = obj
    if (!__originalKeys) return rest as Record<string, PermissionRule>
    const result: Record<string, PermissionRule> = {}
    for (const key of __originalKeys) {
      if (key in rest) result[key] = rest[key] as PermissionRule
    }
    return result
  }

  export const Permission = z
    .preprocess(
      permissionPreprocess,
      z
        .object({
          __originalKeys: z.string().array().optional(),
          read: PermissionRule.optional(),
          edit: PermissionRule.optional(),
          glob: PermissionRule.optional(),
          grep: PermissionRule.optional(),
          list: PermissionRule.optional(),
          bash: PermissionRule.optional(),
          task: PermissionRule.optional(),
          external_directory: PermissionRule.optional(),
          todowrite: PermissionAction.optional(),
          todoread: PermissionAction.optional(),
          question: PermissionAction.optional(),
          webfetch: PermissionAction.optional(),
          websearch: PermissionAction.optional(),
          codesearch: PermissionAction.optional(),
          lsp: PermissionRule.optional(),
          doom_loop: PermissionAction.optional(),
        })
        .catchall(PermissionRule)
        .or(PermissionAction),
    )
    .transform(permissionTransform)
    .meta({
      ref: "PermissionConfig",
    })
  export type Permission = z.infer<typeof Permission>

  export const Command = z.object({
    template: z.string(),
    description: z.string().optional(),
    agent: z.string().optional(),
    model: z.string().optional(),
    subtask: z.boolean().optional(),
  })
  export type Command = z.infer<typeof Command>

  export const Agent = z
    .object({
      model: z.string().optional(),
      temperature: z.number().optional(),
      top_p: z.number().optional(),
      prompt: z.string().optional(),
      tools: z.record(z.string(), z.boolean()).optional().describe("@deprecated Use 'permission' field instead"),
      disable: z.boolean().optional(),
      description: z.string().optional().describe("Description of when to use the agent"),
      mode: z.enum(["subagent", "primary", "all"]).optional(),
      hidden: z
        .boolean()
        .optional()
        .describe("Hide this subagent from the @ autocomplete menu (default: false, only applies to mode: subagent)"),
      options: z.record(z.string(), z.any()).optional(),
      color: z
        .string()
        .regex(/^#[0-9a-fA-F]{6}$/, "Invalid hex color format")
        .optional()
        .describe("Hex color code for the agent (e.g., #FF5733)"),
      steps: z
        .number()
        .int()
        .positive()
        .optional()
        .describe("Maximum number of agentic iterations before forcing text-only response"),
      maxSteps: z.number().int().positive().optional().describe("@deprecated Use 'steps' field instead."),
      permission: Permission.optional(),
    })
    .catchall(z.any())
    .transform((agent, ctx) => {
      const knownKeys = new Set([
        "name",
        "model",
        "prompt",
        "description",
        "temperature",
        "top_p",
        "mode",
        "hidden",
        "color",
        "steps",
        "maxSteps",
        "options",
        "permission",
        "disable",
        "tools",
      ])

      // Extract unknown properties into options
      const options: Record<string, unknown> = { ...agent.options }
      for (const [key, value] of Object.entries(agent)) {
        if (!knownKeys.has(key)) options[key] = value
      }

      // Convert legacy tools config to permissions
      const permission: Permission = {}
      for (const [tool, enabled] of Object.entries(agent.tools ?? {})) {
        const action = enabled ? "allow" : "deny"
        // write, edit, patch, multiedit all map to edit permission
        if (tool === "write" || tool === "edit" || tool === "patch" || tool === "multiedit") {
          permission.edit = action
        } else {
          permission[tool] = action
        }
      }
      Object.assign(permission, agent.permission)

      // Convert legacy maxSteps to steps
      const steps = agent.steps ?? agent.maxSteps

      return { ...agent, options, permission, steps } as typeof agent & {
        options?: Record<string, unknown>
        permission?: Permission
        steps?: number
      }
    })
    .meta({
      ref: "AgentConfig",
    })
  export type Agent = z.infer<typeof Agent>

  export const Keybinds = z
    .object({
      leader: z.string().optional().default("ctrl+x").describe("Leader key for keybind combinations"),
      app_exit: z.string().optional().default("ctrl+c,ctrl+d,<leader>q").describe("Exit the application"),
      editor_open: z.string().optional().default("<leader>e").describe("Open external editor"),
      theme_list: z.string().optional().default("<leader>t").describe("List available themes"),
      sidebar_toggle: z.string().optional().default("<leader>b").describe("Toggle sidebar"),
      scrollbar_toggle: z.string().optional().default("none").describe("Toggle session scrollbar"),
      username_toggle: z.string().optional().default("none").describe("Toggle username visibility"),
      status_view: z.string().optional().default("<leader>s").describe("View status"),
      session_export: z.string().optional().default("<leader>x").describe("Export session to editor"),
      session_new: z.string().optional().default("<leader>n").describe("Create a new session"),
      session_list: z.string().optional().default("<leader>l").describe("List all sessions"),
      session_timeline: z.string().optional().default("<leader>g").describe("Show session timeline"),
      session_fork: z.string().optional().default("none").describe("Fork session from message"),
      session_rename: z.string().optional().default("ctrl+r").describe("Rename session"),
      session_delete: z.string().optional().default("ctrl+d").describe("Delete session"),
      stash_delete: z.string().optional().default("ctrl+d").describe("Delete stash entry"),
      model_provider_list: z.string().optional().default("ctrl+a").describe("Open provider list from model dialog"),
      model_favorite_toggle: z.string().optional().default("ctrl+f").describe("Toggle model favorite status"),
      session_interrupt: z.string().optional().default("escape").describe("Interrupt current session"),
      session_compact: z.string().optional().default("<leader>c").describe("Compact the session"),
      messages_page_up: z.string().optional().default("pageup,ctrl+alt+b").describe("Scroll messages up by one page"),
      messages_page_down: z
        .string()
        .optional()
        .default("pagedown,ctrl+alt+f")
        .describe("Scroll messages down by one page"),
      messages_line_up: z.string().optional().default("ctrl+alt+y").describe("Scroll messages up by one line"),
      messages_line_down: z.string().optional().default("ctrl+alt+e").describe("Scroll messages down by one line"),
      messages_half_page_up: z.string().optional().default("ctrl+alt+u").describe("Scroll messages up by half page"),
      messages_half_page_down: z
        .string()
        .optional()
        .default("ctrl+alt+d")
        .describe("Scroll messages down by half page"),
      messages_first: z.string().optional().default("ctrl+g,home").describe("Navigate to first message"),
      messages_last: z.string().optional().default("ctrl+alt+g,end").describe("Navigate to last message"),
      messages_next: z.string().optional().default("none").describe("Navigate to next message"),
      messages_previous: z.string().optional().default("none").describe("Navigate to previous message"),
      messages_last_user: z.string().optional().default("none").describe("Navigate to last user message"),
      messages_copy: z.string().optional().default("<leader>y").describe("Copy message"),
      messages_undo: z.string().optional().default("<leader>u").describe("Undo message"),
      messages_redo: z.string().optional().default("<leader>r").describe("Redo message"),
      messages_toggle_conceal: z
        .string()
        .optional()
        .default("<leader>h")
        .describe("Toggle code block concealment in messages"),
      tool_details: z.string().optional().default("none").describe("Toggle tool details visibility"),
      model_list: z.string().optional().default("<leader>m").describe("List available models"),
      model_cycle_recent: z.string().optional().default("f2").describe("Next recently used model"),
      model_cycle_recent_reverse: z.string().optional().default("shift+f2").describe("Previous recently used model"),
      model_cycle_favorite: z.string().optional().default("none").describe("Next favorite model"),
      model_cycle_favorite_reverse: z.string().optional().default("none").describe("Previous favorite model"),
      command_list: z.string().optional().default("ctrl+p").describe("List available commands"),
      agent_list: z.string().optional().default("<leader>a").describe("List agents"),
      agent_cycle: z.string().optional().default("tab").describe("Next agent"),
      agent_cycle_reverse: z.string().optional().default("shift+tab").describe("Previous agent"),
      variant_cycle: z.string().optional().default("ctrl+t").describe("Cycle model variants"),
      input_clear: z.string().optional().default("ctrl+c").describe("Clear input field"),
      input_paste: z.string().optional().default("ctrl+v").describe("Paste from clipboard"),
      input_submit: z.string().optional().default("return").describe("Submit input"),
      input_newline: z
        .string()
        .optional()
        .default("shift+return,ctrl+return,alt+return,ctrl+j")
        .describe("Insert newline in input"),
      input_move_left: z.string().optional().default("left,ctrl+b").describe("Move cursor left in input"),
      input_move_right: z.string().optional().default("right,ctrl+f").describe("Move cursor right in input"),
      input_move_up: z.string().optional().default("up").describe("Move cursor up in input"),
      input_move_down: z.string().optional().default("down").describe("Move cursor down in input"),
      input_select_left: z.string().optional().default("shift+left").describe("Select left in input"),
      input_select_right: z.string().optional().default("shift+right").describe("Select right in input"),
      input_select_up: z.string().optional().default("shift+up").describe("Select up in input"),
      input_select_down: z.string().optional().default("shift+down").describe("Select down in input"),
      input_line_home: z.string().optional().default("ctrl+a").describe("Move to start of line in input"),
      input_line_end: z.string().optional().default("ctrl+e").describe("Move to end of line in input"),
      input_select_line_home: z
        .string()
        .optional()
        .default("ctrl+shift+a")
        .describe("Select to start of line in input"),
      input_select_line_end: z.string().optional().default("ctrl+shift+e").describe("Select to end of line in input"),
      input_visual_line_home: z.string().optional().default("alt+a").describe("Move to start of visual line in input"),
      input_visual_line_end: z.string().optional().default("alt+e").describe("Move to end of visual line in input"),
      input_select_visual_line_home: z
        .string()
        .optional()
        .default("alt+shift+a")
        .describe("Select to start of visual line in input"),
      input_select_visual_line_end: z
        .string()
        .optional()
        .default("alt+shift+e")
        .describe("Select to end of visual line in input"),
      input_buffer_home: z.string().optional().default("home").describe("Move to start of buffer in input"),
      input_buffer_end: z.string().optional().default("end").describe("Move to end of buffer in input"),
      input_select_buffer_home: z
        .string()
        .optional()
        .default("shift+home")
        .describe("Select to start of buffer in input"),
      input_select_buffer_end: z.string().optional().default("shift+end").describe("Select to end of buffer in input"),
      input_delete_line: z.string().optional().default("ctrl+shift+d").describe("Delete line in input"),
      input_delete_to_line_end: z.string().optional().default("ctrl+k").describe("Delete to end of line in input"),
      input_delete_to_line_start: z.string().optional().default("ctrl+u").describe("Delete to start of line in input"),
      input_backspace: z.string().optional().default("backspace,shift+backspace").describe("Backspace in input"),
      input_delete: z.string().optional().default("ctrl+d,delete,shift+delete").describe("Delete character in input"),
      input_undo: z.string().optional().default("ctrl+-,super+z").describe("Undo in input"),
      input_redo: z.string().optional().default("ctrl+.,super+shift+z").describe("Redo in input"),
      input_word_forward: z
        .string()
        .optional()
        .default("alt+f,alt+right,ctrl+right")
        .describe("Move word forward in input"),
      input_word_backward: z
        .string()
        .optional()
        .default("alt+b,alt+left,ctrl+left")
        .describe("Move word backward in input"),
      input_select_word_forward: z
        .string()
        .optional()
        .default("alt+shift+f,alt+shift+right")
        .describe("Select word forward in input"),
      input_select_word_backward: z
        .string()
        .optional()
        .default("alt+shift+b,alt+shift+left")
        .describe("Select word backward in input"),
      input_delete_word_forward: z
        .string()
        .optional()
        .default("alt+d,alt+delete,ctrl+delete")
        .describe("Delete word forward in input"),
      input_delete_word_backward: z
        .string()
        .optional()
        .default("ctrl+w,ctrl+backspace,alt+backspace")
        .describe("Delete word backward in input"),
      history_previous: z.string().optional().default("up").describe("Previous history item"),
      history_next: z.string().optional().default("down").describe("Next history item"),
      session_child_cycle: z.string().optional().default("<leader>right").describe("Next child session"),
      session_child_cycle_reverse: z.string().optional().default("<leader>left").describe("Previous child session"),
      session_parent: z.string().optional().default("<leader>up").describe("Go to parent session"),
      terminal_suspend: z.string().optional().default("ctrl+z").describe("Suspend terminal"),
      terminal_title_toggle: z.string().optional().default("none").describe("Toggle terminal title"),
      tips_toggle: z.string().optional().default("<leader>h").describe("Toggle tips on home screen"),
    })
    .strict()
    .meta({
      ref: "KeybindsConfig",
    })

  export const TUI = z.object({
    scroll_speed: z.number().min(0.001).optional().describe("TUI scroll speed"),
    scroll_acceleration: z
      .object({
        enabled: z.boolean().describe("Enable scroll acceleration"),
      })
      .optional()
      .describe("Scroll acceleration settings"),
    diff_style: z
      .enum(["auto", "stacked"])
      .optional()
      .describe("Control diff rendering style: 'auto' adapts to terminal width, 'stacked' always shows single column"),
  })

  // ══════════════════════════════════════════════════════════════════════
  // ZeroBot Configuration (shared with ZeroBot daemon)
  // ══════════════════════════════════════════════════════════════════════

  export const ZeroBotObservability = z
    .object({
      backend: z.enum(["none", "log", "prometheus", "otel"]).optional().describe("Observability backend"),
    })
    .strict()
    .meta({ ref: "ZeroBotObservabilityConfig" })

  export const ZeroBotAutonomy = z
    .object({
      level: z
        .enum(["readonly", "supervised", "full"])
        .optional()
        .describe("Autonomy level: readonly (observe only), supervised (requires approval), full (unrestricted)"),
      workspace_only: z.boolean().optional().describe("Restrict operations to workspace directory"),
      allowed_commands: z.array(z.string()).optional().describe("Whitelist of allowed shell commands"),
      forbidden_paths: z.array(z.string()).optional().describe("Paths that are never accessible"),
      max_actions_per_hour: z.number().int().positive().optional().describe("Rate limit for autonomous actions"),
      max_cost_per_day_cents: z.number().int().positive().optional().describe("Daily cost limit in cents"),
    })
    .strict()
    .meta({ ref: "ZeroBotAutonomyConfig" })

  export const ZeroBotRuntime = z
    .object({
      kind: z.enum(["native", "docker", "cloudflare"]).optional().describe("Runtime environment"),
    })
    .strict()
    .meta({ ref: "ZeroBotRuntimeConfig" })

  export const ZeroBotReliability = z
    .object({
      provider_retries: z.number().int().positive().optional().describe("Retries per provider before failover"),
      provider_backoff_ms: z.number().int().positive().optional().describe("Base backoff in milliseconds"),
      fallback_providers: z.array(z.string()).optional().describe("Fallback provider chain"),
      channel_initial_backoff_secs: z
        .number()
        .int()
        .positive()
        .optional()
        .describe("Initial backoff for channel restarts"),
      channel_max_backoff_secs: z.number().int().positive().optional().describe("Maximum backoff for channel restarts"),
    })
    .strict()
    .meta({ ref: "ZeroBotReliabilityConfig" })

  export const ZeroBotHeartbeat = z
    .object({
      enabled: z.boolean().optional().describe("Enable heartbeat monitoring"),
      interval_minutes: z.number().int().positive().optional().describe("Heartbeat interval in minutes"),
    })
    .strict()
    .meta({ ref: "ZeroBotHeartbeatConfig" })

  export const ZeroBotMemory = z
    .object({
      backend: z.enum(["sqlite", "markdown", "none"]).optional().describe("Memory storage backend"),
      auto_save: z.boolean().optional().describe("Auto-save conversation context"),
      hygiene_enabled: z.boolean().optional().describe("Enable memory hygiene (archiving + cleanup)"),
      embedding_provider: z.string().optional().describe("Embedding provider: none, openai, or custom:URL"),
      embedding_model: z.string().optional().describe("Embedding model name"),
    })
    .strict()
    .meta({ ref: "ZeroBotMemoryConfig" })

  export const ZeroBotGateway = z
    .object({
      port: z.number().int().positive().optional().describe("Gateway port (default: 3000)"),
      host: z.string().optional().describe("Gateway host (default: 127.0.0.1)"),
      require_pairing: z.boolean().optional().describe("Require pairing before accepting requests"),
      allow_public_bind: z.boolean().optional().describe("Allow binding to non-localhost without a tunnel"),
    })
    .strict()
    .meta({ ref: "ZeroBotGatewayConfig" })

  export const ZeroBotTunnelCloudflare = z.object({ token: z.string().describe("Cloudflare Tunnel token") }).strict()
  export const ZeroBotTunnelTailscale = z
    .object({
      funnel: z.boolean().optional().describe("Use Tailscale Funnel (public) vs Serve (tailnet only)"),
      hostname: z.string().optional().describe("Optional hostname override"),
    })
    .strict()
  export const ZeroBotTunnelNgrok = z
    .object({
      auth_token: z.string().describe("ngrok auth token"),
      domain: z.string().optional().describe("Optional custom domain"),
    })
    .strict()

  export const ZeroBotTunnel = z
    .object({
      provider: z
        .enum(["none", "cloudflare", "tailscale", "ngrok", "custom"])
        .optional()
        .describe("Tunnel provider for remote access"),
      cloudflare: ZeroBotTunnelCloudflare.optional(),
      tailscale: ZeroBotTunnelTailscale.optional(),
      ngrok: ZeroBotTunnelNgrok.optional(),
    })
    .strict()
    .meta({ ref: "ZeroBotTunnelConfig" })

  export const ZeroBotTelegramVoice = z
    .object({
      enabled: z.boolean().optional().describe("Enable voice message support"),
      stt_provider: z.string().optional().describe("Speech-to-text provider: openai, uniapi, groq, deepinfra, compatible"),
      stt_model: z.string().optional().describe("Speech-to-text model"),
      stt_api_key: z.string().optional().describe("Speech-to-text API key"),
      stt_base_url: z.string().optional().describe("Base URL for OpenAI-compatible STT providers"),
    })
    .strict()

  export const ZeroBotTelegram = z
    .object({
      bot_token: z.string().describe("Telegram bot token"),
      allowed_users: z.array(z.string()).optional().describe("Allowed Telegram usernames"),
      voice: ZeroBotTelegramVoice.optional().describe("Voice message configuration"),
    })
    .strict()

  export const ZeroBotDiscord = z
    .object({
      bot_token: z.string().describe("Discord bot token"),
      guild_id: z.string().optional().describe("Discord guild/server ID"),
      allowed_users: z.array(z.string()).optional().describe("Allowed Discord user IDs"),
    })
    .strict()

  export const ZeroBotSlack = z
    .object({
      bot_token: z.string().describe("Slack bot token"),
      app_token: z.string().optional().describe("Slack app token for Socket Mode"),
      channel_id: z.string().optional().describe("Default Slack channel ID"),
    })
    .strict()

  export const ZeroBotWhatsApp = z
    .object({
      access_token: z.string().describe("Meta Business API access token"),
      phone_number_id: z.string().describe("WhatsApp phone number ID"),
      verify_token: z.string().describe("Webhook verification token"),
      allowed_numbers: z.array(z.string()).optional().describe("Allowed phone numbers (E.164 format)"),
    })
    .strict()

  export const ZeroBotFeishu = z
    .object({
      enabled: z.boolean().optional().describe("Enable Feishu channel"),
      app_id: z.string().describe("App ID from Feishu Open Platform"),
      app_secret: z.string().describe("App Secret from Feishu Open Platform"),
      encrypt_key: z.string().optional().describe("Encrypt key for event callback decryption"),
      verification_token: z.string().optional().describe("Verification token for event callback verification"),
      allowed_users: z.array(z.string()).optional().describe("Allowed user open_ids or '*' for all"),
      use_lark_api: z.boolean().optional().describe("Use Lark API instead of Feishu API (for international users)"),
    })
    .strict()

  export const ZeroBotChannels = z
    .object({
      cli: z.boolean().optional().describe("Enable CLI channel"),
      telegram: ZeroBotTelegram.optional(),
      discord: ZeroBotDiscord.optional(),
      slack: ZeroBotSlack.optional(),
      whatsapp: ZeroBotWhatsApp.optional(),
      feishu: ZeroBotFeishu.optional(),
    })
    .strict()
    .meta({ ref: "ZeroBotChannelsConfig" })

  export const ZeroBotBrowser = z
    .object({
      enabled: z.boolean().optional().describe("Enable browser tool"),
      allowed_domains: z.array(z.string()).optional().describe("Allowed domains for browsing"),
    })
    .strict()
    .meta({ ref: "ZeroBotBrowserConfig" })

  export const ZeroBotIdentity = z
    .object({
      format: z.enum(["openclaw", "aieos"]).optional().describe("Identity format"),
      aieos_path: z.string().optional().describe("Path to AIEOS JSON file"),
    })
    .strict()
    .meta({ ref: "ZeroBotIdentityConfig" })

  export const ZeroBotCodeCoder = z
    .object({
      enabled: z.boolean().optional().describe("Enable CodeCoder integration"),
      endpoint: z.string().optional().describe("CodeCoder API endpoint (default: http://localhost:4400)"),
    })
    .strict()
    .meta({ ref: "ZeroBotCodeCoderConfig" })

  export const ZeroBotSession = z
    .object({
      enabled: z.boolean().optional().describe("Whether session management is enabled (default: true)"),
      context_window: z.number().int().positive().optional().describe("Model context window size in tokens (default: 128000)"),
      compact_threshold: z.number().min(0).max(1).optional().describe("Compact when usage exceeds this ratio (default: 0.8)"),
      keep_recent: z.number().int().positive().optional().describe("Number of recent messages to always keep (default: 5)"),
    })
    .strict()
    .meta({ ref: "ZeroBotSessionConfig" })

  export const ZeroBotTts = z
    .object({
      enabled: z.boolean().optional().describe("Enable TTS responses (default: false)"),
      provider: z.string().optional().describe("TTS provider: openai, elevenlabs, compatible (default: openai)"),
      api_key: z.string().optional().describe("API key for TTS provider"),
      model: z.string().optional().describe("TTS model name (e.g., tts-1 for OpenAI)"),
      voice: z.string().optional().describe("Voice ID (e.g., alloy for OpenAI)"),
      default_voice: z.string().optional().describe("Voice ID (alias for voice)"),
      base_url: z.string().optional().describe("Base URL for OpenAI-compatible TTS providers"),
    })
    .strict()
    .meta({ ref: "ZeroBotTtsConfig" })

  export const ZeroBot = z
    .object({
      // Provider configuration (can reuse CodeCoder's provider config)
      default_provider: z.string().optional().describe("Default LLM provider (uses CodeCoder provider config for keys)"),
      default_model: z.string().optional().describe("Default model name"),
      default_temperature: z.number().min(0).max(2).optional().describe("Default temperature (0.0-2.0)"),

      // ZeroBot-specific configuration
      workspace_dir: z.string().optional().describe("ZeroBot workspace directory"),

      // Subsystem configuration
      observability: ZeroBotObservability.optional(),
      autonomy: ZeroBotAutonomy.optional(),
      runtime: ZeroBotRuntime.optional(),
      reliability: ZeroBotReliability.optional(),
      heartbeat: ZeroBotHeartbeat.optional(),
      memory: ZeroBotMemory.optional(),
      gateway: ZeroBotGateway.optional(),
      tunnel: ZeroBotTunnel.optional(),
      channels: ZeroBotChannels.optional(),
      browser: ZeroBotBrowser.optional(),
      identity: ZeroBotIdentity.optional(),
      codecoder: ZeroBotCodeCoder.optional().describe("CodeCoder integration for accessing AI agents"),
      session: ZeroBotSession.optional().describe("Session/conversation context management configuration"),
      tts: ZeroBotTts.optional().describe("Text-to-Speech configuration for voice responses"),
    })
    .strict()
    .meta({ ref: "ZeroBotConfig" })
  export type ZeroBot = z.infer<typeof ZeroBot>

  export const Server = z
    .object({
      port: z.number().int().positive().optional().describe("Port to listen on"),
      hostname: z.string().optional().describe("Hostname to listen on"),
      mdns: z.boolean().optional().describe("Enable mDNS service discovery"),
      cors: z.array(z.string()).optional().describe("Additional domains to allow for CORS"),
      apiKey: z.string().optional().describe("API key for authenticating incoming requests"),
    })
    .strict()
    .meta({
      ref: "ServerConfig",
    })

  // ══════════════════════════════════════════════════════════════════════
  // Network Configuration (Unified with Rust zero-* services)
  // ══════════════════════════════════════════════════════════════════════

  export const Network = z
    .object({
      bind: z.string().optional().describe("Bind address for all services (default: 127.0.0.1)"),
      public_url: z.string().nullable().optional().describe("Public URL for callbacks (optional)"),
    })
    .strict()
    .meta({ ref: "NetworkConfig" })
  export type Network = z.infer<typeof Network>

  // ══════════════════════════════════════════════════════════════════════
  // Services Port Configuration (Unified with Rust zero-* services)
  // ══════════════════════════════════════════════════════════════════════

  export const ServicePortConfig = z
    .object({
      port: z.number().int().positive().optional().describe("Port number for the service"),
    })
    .strict()
    .meta({ ref: "ServicePortConfig" })
  export type ServicePortConfig = z.infer<typeof ServicePortConfig>

  export const Services = z
    .object({
      codecoder: ServicePortConfig.optional().describe("CodeCoder API service (default port: 4400)"),
      gateway: ServicePortConfig.optional().describe("Gateway service (default port: 4430)"),
      channels: ServicePortConfig.optional().describe("Channels service (default port: 4431)"),
      workflow: ServicePortConfig.optional().describe("Workflow service (default port: 4432)"),
      trading: ServicePortConfig.optional().describe("Trading service (default port: 4434)"),
    })
    .strict()
    .meta({ ref: "ServicesConfig" })
  export type Services = z.infer<typeof Services>

  export const Vault = z
    .object({
      enabled: z.boolean().optional().describe("Enable the credential vault (default: true)"),
      autoInject: z.boolean().optional().describe("Auto-inject credentials into HTTP requests (default: true)"),
    })
    .strict()
    .meta({
      ref: "VaultConfig",
    })

  export const Layout = z.enum(["auto", "stretch"]).meta({
    ref: "LayoutConfig",
  })
  export type Layout = z.infer<typeof Layout>

  // ══════════════════════════════════════════════════════════════════════
  // Provider Settings (global LLM settings within provider._settings)
  // ══════════════════════════════════════════════════════════════════════

  export const ProviderSettings = z
    .object({
      default: z.string().optional().describe("Default model in provider/model format (e.g., deepseek/deepseek-chat)"),
      retries: z.number().int().min(0).optional().describe("Number of retries for failed requests"),
      backoff_ms: z.number().int().positive().optional().describe("Base backoff in milliseconds"),
      fallbacks: z.array(z.string()).optional().describe("Fallback model chain"),
    })
    .strict()
    .meta({ ref: "ProviderSettingsConfig" })
  export type ProviderSettings = z.infer<typeof ProviderSettings>

  // ══════════════════════════════════════════════════════════════════════
  // LLM Configuration (shared format with Rust services)
  // @deprecated Use provider._settings instead
  // ══════════════════════════════════════════════════════════════════════

  export const LlmProviderConfig = z
    .object({
      base_url: z.string().optional().describe("Base URL for the provider API"),
      models: z.array(z.string()).optional().describe("List of available model IDs"),
      whitelist: z.array(z.string()).optional().describe("Only enable these models"),
      blacklist: z.array(z.string()).optional().describe("Disable these models"),
      region: z.string().optional().describe("AWS region for Bedrock"),
      profile: z.string().optional().describe("AWS profile for Bedrock"),
      endpoint: z.string().optional().describe("Custom API endpoint"),
      variants: z
        .record(
          z.string(),
          z.record(z.string(), z.any()),
        )
        .optional()
        .describe("Model variant configurations"),
    })
    .catchall(z.any())
    .meta({ ref: "LlmProviderConfig" })
  export type LlmProviderConfig = z.infer<typeof LlmProviderConfig>

  export const Llm = z
    .object({
      default: z.string().optional().describe("Default model in provider/model format (e.g., deepseek/deepseek-chat)"),
      retries: z.number().int().min(0).optional().describe("Number of retries for failed requests"),
      backoff_ms: z.number().int().positive().optional().describe("Base backoff in milliseconds"),
      fallbacks: z.array(z.string()).optional().describe("Fallback model chain"),
      providers: z
        .record(z.string(), LlmProviderConfig)
        .optional()
        .describe("Provider-specific configuration"),
      ollama: z
        .object({
          base_url: z.string().optional(),
          default_model: z.string().optional(),
          timeout_secs: z.number().int().positive().optional(),
        })
        .optional()
        .describe("Ollama local model configuration"),
    })
    .strict()
    .meta({ ref: "LlmConfig" })
  export type Llm = z.infer<typeof Llm>

  export const Secrets = z
    .object({
      llm: z
        .record(z.string(), z.string().nullable())
        .optional()
        .describe("API keys for LLM providers, keyed by provider ID"),
      channels: z.record(z.string(), z.string().nullable()).optional(),
      external: z.record(z.string(), z.string().nullable()).optional(),
    })
    .strict()
    .meta({ ref: "SecretsConfig" })
  export type Secrets = z.infer<typeof Secrets>

  // ══════════════════════════════════════════════════════════════════════
  // Provider Configuration (original format)
  // ══════════════════════════════════════════════════════════════════════

  export const Provider = ModelsDev.Provider.partial()
    .extend({
      whitelist: z.array(z.string()).optional(),
      blacklist: z.array(z.string()).optional(),
      models: z
        .record(
          z.string(),
          ModelsDev.Model.partial().extend({
            variants: z
              .record(
                z.string(),
                z
                  .object({
                    disabled: z.boolean().optional().describe("Disable this variant for the model"),
                  })
                  .catchall(z.any()),
              )
              .optional()
              .describe("Variant-specific configuration"),
          }),
        )
        .optional(),
      options: z
        .object({
          apiKey: z.string().optional(),
          baseURL: z.string().optional(),
          enterpriseUrl: z.string().optional().describe("GitHub Enterprise URL for copilot authentication"),
          setCacheKey: z.boolean().optional().describe("Enable promptCacheKey for this provider (default false)"),
          timeout: z
            .union([
              z
                .number()
                .int()
                .positive()
                .describe(
                  "Timeout in milliseconds for requests to this provider. Default is 300000 (5 minutes). Set to false to disable timeout.",
                ),
              z.literal(false).describe("Disable timeout for this provider entirely."),
            ])
            .optional()
            .describe(
              "Timeout in milliseconds for requests to this provider. Default is 300000 (5 minutes). Set to false to disable timeout.",
            ),
        })
        .catchall(z.any())
        .optional(),
    })
    .strict()
    .meta({
      ref: "ProviderConfig",
    })
  export type Provider = z.infer<typeof Provider>

  export const Info = z
    .object({
      $schema: z.string().optional().describe("JSON schema reference for configuration validation"),
      theme: z.string().optional().describe("Theme name to use for the interface"),
      keybinds: Keybinds.optional().describe("Custom keybind configurations"),
      logLevel: Log.Level.optional().describe("Log level"),
      tui: TUI.optional().describe("TUI specific settings"),
      server: Server.optional().describe("Server configuration for codecoder serve and web commands"),
      network: Network.optional().describe("Network configuration (unified with Rust zero-* services)"),
      services: Services.optional().describe("Services port configuration (unified with Rust zero-* services)"),
      redis: RedisConfig.describe("Redis configuration for conversation store"),
      command: z
        .record(z.string(), Command)
        .optional()
        .describe("Command configuration, see https://code-coder.com/docs/commands"),
      watcher: z
        .object({
          ignore: z.array(z.string()).optional(),
        })
        .optional(),
      snapshot: z.boolean().optional(),
      autoupdate: z
        .union([z.boolean(), z.literal("notify")])
        .optional()
        .describe(
          "Automatically update to the latest version. Set to true to auto-update, false to disable, or 'notify' to show update notifications",
        ),
      disabled_providers: z.array(z.string()).optional().describe("Disable providers that are loaded automatically"),
      enabled_providers: z
        .array(z.string())
        .optional()
        .describe("When set, ONLY these providers will be enabled. All other providers will be ignored"),
      model: z.string().describe("Model to use in the format of provider/model, eg anthropic/claude-2").optional(),
      small_model: z
        .string()
        .describe("Small model to use for tasks like title generation in the format of provider/model")
        .optional(),
      default_agent: z
        .string()
        .optional()
        .describe(
          "Default agent to use when none is specified. Must be a primary agent. Falls back to 'build' if not set or if the specified agent is invalid.",
        ),
      username: z
        .string()
        .optional()
        .describe("Custom username to display in conversations instead of system username"),
      mode: z
        .object({
          build: Agent.optional(),
          plan: Agent.optional(),
        })
        .catchall(Agent)
        .optional()
        .describe("@deprecated Use `agent` field instead."),
      agent: z
        .object({
          // primary
          plan: Agent.optional(),
          build: Agent.optional(),
          // subagent
          general: Agent.optional(),
          explore: Agent.optional(),
          // specialized
          title: Agent.optional(),
          summary: Agent.optional(),
          compaction: Agent.optional(),
        })
        .catchall(Agent)
        .optional()
        .describe("Agent configuration, see https://code-coder.com/docs/agents"),
      mcp: z
        .object({
          server: McpServerConfig.optional().describe("MCP server configuration for 'mcp serve' command"),
        })
        .catchall(
          z.union([
            Mcp,
            McpDisabled,
          ]),
        )
        .optional()
        .describe("MCP (Model Context Protocol) server configurations"),
      formatter: z
        .union([
          z.literal(false),
          z.record(
            z.string(),
            z.object({
              disabled: z.boolean().optional(),
              command: z.array(z.string()).optional(),
              environment: z.record(z.string(), z.string()).optional(),
              extensions: z.array(z.string()).optional(),
            }),
          ),
        ])
        .optional(),
      lsp: z
        .union([
          z.literal(false),
          z.record(
            z.string(),
            z.union([
              z.object({
                disabled: z.literal(true),
              }),
              z.object({
                command: z.array(z.string()),
                extensions: z.array(z.string()).optional(),
                disabled: z.boolean().optional(),
                env: z.record(z.string(), z.string()).optional(),
                initialization: z.record(z.string(), z.any()).optional(),
              }),
            ]),
          ),
        ])
        .optional()
        .refine(
          (data) => {
            if (!data) return true
            if (typeof data === "boolean") return true
            const serverIds = new Set(Object.values(LSPServer).map((s) => s.id))

            return Object.entries(data).every(([id, config]) => {
              if (config.disabled) return true
              if (serverIds.has(id)) return true
              return Boolean(config.extensions)
            })
          },
          {
            error: "For custom LSP servers, 'extensions' array is required.",
          },
        ),
      instructions: z.array(z.string()).optional().describe("Additional instruction files or patterns to include"),
      layout: Layout.optional().describe("@deprecated Always uses stretch layout."),
      permission: Permission.optional(),
      tools: z.record(z.string(), z.boolean()).optional(),
      enterprise: z
        .object({
          url: z.string().optional().describe("Enterprise URL"),
        })
        .optional(),
      compaction: z
        .object({
          auto: z.boolean().optional().describe("Enable automatic compaction when context is full (default: true)"),
          prune: z.boolean().optional().describe("Enable pruning of old tool outputs (default: true)"),
        })
        .optional(),
      experimental: z
        .object({
          hook: z
            .object({
              file_edited: z
                .record(
                  z.string(),
                  z
                    .object({
                      command: z.string().array(),
                      environment: z.record(z.string(), z.string()).optional(),
                    })
                    .array(),
                )
                .optional(),
              session_completed: z
                .object({
                  command: z.string().array(),
                  environment: z.record(z.string(), z.string()).optional(),
                })
                .array()
                .optional(),
            })
            .optional(),
          chatMaxRetries: z.number().optional().describe("Number of retries for chat completions on failure"),
          disable_paste_summary: z.boolean().optional(),
          batch_tool: z.boolean().optional().describe("Enable the batch tool"),
          openTelemetry: z
            .boolean()
            .optional()
            .describe("Enable OpenTelemetry spans for AI SDK calls (using the 'experimental_telemetry' flag)"),
          primary_tools: z
            .array(z.string())
            .optional()
            .describe("Tools that should only be available to primary agents."),
          continue_loop_on_deny: z.boolean().optional().describe("Continue the agent loop when a tool call is denied"),
          mcp_timeout: z
            .number()
            .int()
            .positive()
            .optional()
            .describe("Timeout in milliseconds for model context protocol (MCP) requests"),
          observability: z
            .object({
              enabled: z.boolean().optional().describe("Enable observability logging (default: true)"),
              level: z
                .enum(["debug", "info", "warn", "error"])
                .optional()
                .describe("Minimum log level for observability (default: info)"),
              sampling: z
                .number()
                .min(0)
                .max(1)
                .optional()
                .describe("Sampling rate for observability logs (0.0-1.0, default: 1.0)"),
            })
            .optional()
            .describe("Observability configuration for structured logging and tracing"),
        })
        .optional(),
      autonomousMode: z
        .object({
          enabled: z.boolean().optional().describe("Enable Autonomous Mode autonomous execution"),
          autonomyLevel: z
            .enum(["lunatic", "insane", "crazy", "wild", "bold", "timid"])
            .optional()
            .describe("Autonomy level for Autonomous Mode"),
          unattended: z.boolean().optional().describe("Run in unattended mode (no user present)"),
          resourceLimits: z
            .object({
              maxTokens: z.number().optional().describe("Maximum tokens to consume"),
              maxCostUSD: z.number().optional().describe("Maximum cost in USD"),
              maxDurationMinutes: z.number().optional().describe("Maximum duration in minutes"),
              maxFilesChanged: z.number().optional().describe("Maximum files to change"),
              maxActions: z.number().optional().describe("Maximum actions to perform"),
            })
            .optional()
            .describe("Resource limits for Autonomous Mode"),
        })
        .optional()
        .describe("Autonomous Mode autonomous execution configuration"),
      vault: Vault.optional().describe("Credential vault configuration"),
      zerobot: ZeroBot.optional().describe("ZeroBot daemon and channel configuration"),
      llm: Llm.optional().describe("LLM configuration shared with Rust services"),
      secrets: Secrets.optional().describe("API secrets for providers and services"),
    })
    .passthrough() // Allow unknown keys for Rust services (gateway, channels, workflow, codecoder)
    .meta({
      ref: "Config",
    })

  export type Info = z.output<typeof Info>

  export const global = lazy(async () => {
    let result: Info = pipe(
      {},
      mergeDeep(await loadFile(path.join(Global.Path.config, "config.json"))),
      mergeDeep(await loadFile(path.join(Global.Path.config, "codecoder.json"))),
      mergeDeep(await loadFile(path.join(Global.Path.config, "codecoder.jsonc"))),
    )

    // Load modular configuration files from ~/.codecoder/
    const configDir = Global.Path.config

    // Load secrets.json
    const secrets = await loadJsonFile(path.join(configDir, "secrets.json"))
    if (secrets) {
      result.secrets = mergeDeep(result.secrets ?? {}, secrets) as Secrets
      log.debug("loaded secrets.json")
    }

    // Load trading.json
    const trading = await loadJsonFile(path.join(configDir, "trading.json"))
    if (trading) {
      result.trading = mergeDeep(result.trading ?? {}, trading) as Info["trading"]
      log.debug("loaded trading.json")
    }

    // Load channels.json → zerobot.channels
    const channels = await loadJsonFile(path.join(configDir, "channels.json"))
    if (channels) {
      result.zerobot = result.zerobot ?? {}
      result.zerobot.channels = mergeDeep(result.zerobot.channels ?? {}, channels) as ZeroBot["channels"]
      log.debug("loaded channels.json")
    }

    // Load providers.json → provider
    const providers = await loadJsonFile(path.join(configDir, "providers.json"))
    if (providers) {
      result.provider = mergeDeep(result.provider ?? {}, providers) as Info["provider"]
      log.debug("loaded providers.json")
    }

    // Apply environment variable overrides for API keys
    applyEnvOverrides(result)

    return result
  })

  // Helper to load a plain JSON file (no schema validation, returns raw object)
  async function loadJsonFile<T>(filepath: string): Promise<T | null> {
    try {
      const text = await Bun.file(filepath).text()
      const errors: JsoncParseError[] = []
      const data = parseJsonc(text, errors, { allowTrailingComma: true })
      if (errors.length) {
        log.warn("JSONC parse errors", { path: filepath, errors: errors.map((e) => printParseErrorCode(e.error)).join(", ") })
        return null
      }
      return data as T
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        return null
      }
      log.error("failed to load config file", { path: filepath, error })
      return null
    }
  }

  // Apply environment variable overrides to config
  function applyEnvOverrides(config: Info): void {
    const env = process.env

    // LLM API keys
    const llmKeyMap: Record<string, string> = {
      ANTHROPIC_API_KEY: "anthropic",
      OPENAI_API_KEY: "openai",
      DEEPSEEK_API_KEY: "deepseek",
      GOOGLE_API_KEY: "google",
      OPENROUTER_API_KEY: "openrouter",
      GROQ_API_KEY: "groq",
      MISTRAL_API_KEY: "mistral",
      XAI_API_KEY: "xai",
      TOGETHER_API_KEY: "together",
      FIREWORKS_API_KEY: "fireworks",
      PERPLEXITY_API_KEY: "perplexity",
    }

    for (const [envVar, provider] of Object.entries(llmKeyMap)) {
      if (env[envVar]) {
        config.secrets = config.secrets ?? {}
        config.secrets.llm = config.secrets.llm ?? {}
        config.secrets.llm[provider] = env[envVar]!
      }
    }

    // External API keys
    if (env.LIXIN_API_KEY) {
      config.secrets = config.secrets ?? {}
      config.secrets.external = config.secrets.external ?? {}
      config.secrets.external.lixin = env.LIXIN_API_KEY
    }
    if (env.ITICK_API_KEY) {
      config.secrets = config.secrets ?? {}
      config.secrets.external = config.secrets.external ?? {}
      config.secrets.external.itick = env.ITICK_API_KEY
    }
  }

  async function loadFile(filepath: string): Promise<Info> {
    log.info("loading", { path: filepath })
    let text = await Bun.file(filepath)
      .text()
      .catch((err) => {
        if (err.code === "ENOENT") return
        throw new JsonError({ path: filepath }, { cause: err })
      })
    if (!text) return {}
    return load(text, filepath)
  }

  async function load(text: string, configFilepath: string) {
    const original = text
    text = text.replace(/\{env:([^}]+)\}/g, (_, varName) => {
      return process.env[varName] || ""
    })

    const fileMatches = text.match(/\{file:[^}]+\}/g)
    if (fileMatches) {
      const configDir = path.dirname(configFilepath)
      const lines = text.split("\n")

      for (const match of fileMatches) {
        const lineIndex = lines.findIndex((line) => line.includes(match))
        if (lineIndex !== -1 && lines[lineIndex].trim().startsWith("//")) {
          continue // Skip if line is commented
        }
        let filePath = match.replace(/^\{file:/, "").replace(/\}$/, "")
        if (filePath.startsWith("~/")) {
          filePath = path.join(os.homedir(), filePath.slice(2))
        }
        const resolvedPath = path.isAbsolute(filePath) ? filePath : path.resolve(configDir, filePath)
        const fileContent = (
          await Bun.file(resolvedPath)
            .text()
            .catch((error) => {
              const errMsg = `bad file reference: "${match}"`
              if (error.code === "ENOENT") {
                throw new InvalidError(
                  {
                    path: configFilepath,
                    message: errMsg + ` ${resolvedPath} does not exist`,
                  },
                  { cause: error },
                )
              }
              throw new InvalidError({ path: configFilepath, message: errMsg }, { cause: error })
            })
        ).trim()
        // escape newlines/quotes, strip outer quotes
        text = text.replace(match, JSON.stringify(fileContent).slice(1, -1))
      }
    }

    const errors: JsoncParseError[] = []
    const data = parseJsonc(text, errors, { allowTrailingComma: true })
    if (errors.length) {
      const lines = text.split("\n")
      const errorDetails = errors
        .map((e) => {
          const beforeOffset = text.substring(0, e.offset).split("\n")
          const line = beforeOffset.length
          const column = beforeOffset[beforeOffset.length - 1].length + 1
          const problemLine = lines[line - 1]

          const error = `${printParseErrorCode(e.error)} at line ${line}, column ${column}`
          if (!problemLine) return error

          return `${error}\n   Line ${line}: ${problemLine}\n${"".padStart(column + 9)}^`
        })
        .join("\n")

      throw new JsonError({
        path: configFilepath,
        message: `\n--- JSONC Input ---\n${text}\n--- Errors ---\n${errorDetails}\n--- End ---`,
      })
    }

    const parsed = Info.safeParse(data)
    if (parsed.success) {
      if (!parsed.data.$schema) {
        parsed.data.$schema = "https://code-coder.com/config.json"
        // Write the $schema to the original text to preserve variables like {env:VAR}
        const updated = original.replace(/^\s*\{/, '{\n  "$schema": "https://code-coder.com/config.json",')
        await Bun.write(configFilepath, updated).catch(() => {})
      }
      return parsed.data
    }

    throw new InvalidError({
      path: configFilepath,
      issues: parsed.error.issues,
    })
  }
  export const JsonError = NamedError.create(
    "ConfigJsonError",
    z.object({
      path: z.string(),
      message: z.string().optional(),
    }),
  )

  export const ConfigDirectoryTypoError = NamedError.create(
    "ConfigDirectoryTypoError",
    z.object({
      path: z.string(),
      dir: z.string(),
      suggestion: z.string(),
    }),
  )

  export const InvalidError = NamedError.create(
    "ConfigInvalidError",
    z.object({
      path: z.string(),
      issues: z.custom<z.core.$ZodIssue[]>().optional(),
      message: z.string().optional(),
    }),
  )

  export async function get() {
    return state().then((x) => x.config)
  }

  export async function getGlobal() {
    return global()
  }

  export async function update(config: Info) {
    const filepath = path.join(Instance.directory, "config.json")
    const existing = await loadFile(filepath)
    await Bun.write(filepath, JSON.stringify(mergeDeep(existing, config), null, 2))
    await Instance.dispose()
  }

  function globalConfigFile() {
    const candidates = ["codecoder.jsonc", "codecoder.json", "config.json"].map((file) =>
      path.join(Global.Path.config, file),
    )
    for (const file of candidates) {
      if (existsSync(file)) return file
    }
    return candidates[0]
  }

  function isRecord(value: unknown): value is Record<string, unknown> {
    return !!value && typeof value === "object" && !Array.isArray(value)
  }

  function patchJsonc(input: string, patch: unknown, path: string[] = []): string {
    if (!isRecord(patch)) {
      const edits = modify(input, path, patch, {
        formattingOptions: {
          insertSpaces: true,
          tabSize: 2,
        },
      })
      return applyEdits(input, edits)
    }

    return Object.entries(patch).reduce((result, [key, value]) => {
      if (value === undefined) return result
      return patchJsonc(result, value, [...path, key])
    }, input)
  }

  function parseConfig(text: string, filepath: string): Info {
    const errors: JsoncParseError[] = []
    const data = parseJsonc(text, errors, { allowTrailingComma: true })
    if (errors.length) {
      const lines = text.split("\n")
      const errorDetails = errors
        .map((e) => {
          const beforeOffset = text.substring(0, e.offset).split("\n")
          const line = beforeOffset.length
          const column = beforeOffset[beforeOffset.length - 1].length + 1
          const problemLine = lines[line - 1]

          const error = `${printParseErrorCode(e.error)} at line ${line}, column ${column}`
          if (!problemLine) return error

          return `${error}\n   Line ${line}: ${problemLine}\n${"".padStart(column + 9)}^`
        })
        .join("\n")

      throw new JsonError({
        path: filepath,
        message: `\n--- JSONC Input ---\n${text}\n--- Errors ---\n${errorDetails}\n--- End ---`,
      })
    }

    const parsed = Info.safeParse(data)
    if (parsed.success) return parsed.data

    throw new InvalidError({
      path: filepath,
      issues: parsed.error.issues,
    })
  }

  export async function updateGlobal(config: Info) {
    const filepath = globalConfigFile()
    const before = await Bun.file(filepath)
      .text()
      .catch((err) => {
        if (err.code === "ENOENT") return "{}"
        throw new JsonError({ path: filepath }, { cause: err })
      })

    if (!filepath.endsWith(".jsonc")) {
      const existing = parseConfig(before, filepath)
      await Bun.write(filepath, JSON.stringify(mergeDeep(existing, config), null, 2))
    } else {
      const next = patchJsonc(before, config)
      parseConfig(next, filepath)
      await Bun.write(filepath, next)
    }

    global.reset()
    await Instance.disposeAll()
  }

  export async function directories() {
    return state().then((x) => x.directories)
  }
}
