import z from "zod"
import path from "path"
import { Log } from "@/util/log"
import { Config } from "../config/config"
import { Instance } from "../project/instance"
import { Global } from "../global"
import { BusEvent } from "../bus/bus-event"

export namespace Hook {
  const log = Log.create({ service: "hook" })

  export const Lifecycle = z.enum(["PreToolUse", "PostToolUse", "PreResponse", "Stop"])
  export type Lifecycle = z.infer<typeof Lifecycle>

  export const ActionType = z.enum([
    "scan",
    "check_env",
    "check_style",
    "notify_only",
    "scan_content",
    "run_command",
    "analyze_changes",
    "scan_files",
  ])
  export type ActionType = z.infer<typeof ActionType>

  export const Action = z.object({
    type: ActionType,
    patterns: z.array(z.string()).optional(),
    message: z.string().optional(),
    block: z.boolean().optional(),
    command: z.string().optional(),
    async: z.boolean().optional(),
    variable: z.string().optional(),
    command_pattern: z.string().optional(),
    file_pattern: z.string().optional(),
    on_output: z.record(z.string(), z.string()).optional(),
    on_vulnerabilities: z.string().optional(),
  })
  export type Action = z.infer<typeof Action>

  export const HookDefinition = z.object({
    pattern: z.string().optional(),
    description: z.string().optional(),
    command_pattern: z.string().optional(),
    file_pattern: z.string().optional(),
    actions: z.array(Action),
  })
  export type HookDefinition = z.infer<typeof HookDefinition>

  export const HooksConfig = z.object({
    hooks: z.object({
      PreToolUse: z.record(z.string(), HookDefinition).optional(),
      PostToolUse: z.record(z.string(), HookDefinition).optional(),
      PreResponse: z.record(z.string(), HookDefinition).optional(),
      Stop: z.record(z.string(), HookDefinition).optional(),
    }),
    settings: z
      .object({
        enabled: z.boolean().optional(),
        blocking_mode: z.enum(["interactive", "silent", "strict"]).optional(),
        log_level: z.enum(["debug", "info", "warn", "error"]).optional(),
      })
      .optional(),
  })
  export type HooksConfig = z.infer<typeof HooksConfig>

  export interface Context {
    tool?: string
    input?: Record<string, unknown>
    output?: string
    filePath?: string
    fileContent?: string
    command?: string
    sessionID?: string
    diff?: string
  }

  export interface RunResult {
    blocked: boolean
    message?: string
    hookName?: string
    actionType?: ActionType
  }

  const state = Instance.state(async () => {
    const configs: HooksConfig[] = []

    const directories = [Global.Path.config, ...(await Config.directories())]

    // Also check the instance directory directly for .ccode/hooks
    const instanceDir = Instance.directory
    const instanceCcodeHooks = path.join(instanceDir, ".ccode", "hooks", "hooks.json")
    const instanceHookFile = Bun.file(instanceCcodeHooks)
    if (await instanceHookFile.exists()) {
      const content = await instanceHookFile.json().catch((err) => {
        log.error("Failed to parse hooks.json", { path: instanceCcodeHooks, error: err })
        return null
      })
      if (content) {
        const parsed = HooksConfig.safeParse(content)
        if (parsed.success) {
          configs.push(parsed.data)
        } else {
          log.error("Invalid hooks.json schema", { path: instanceCcodeHooks, issues: parsed.error.issues })
        }
      }
    }

    for (const dir of directories) {
      const hooksFile = path.join(dir, "hooks", "hooks.json")
      const file = Bun.file(hooksFile)
      if (await file.exists()) {
        const content = await file.json().catch((err) => {
          log.error("Failed to parse hooks.json", { path: hooksFile, error: err })
          return null
        })
        if (content) {
          const parsed = HooksConfig.safeParse(content)
          if (parsed.success) {
            configs.push(parsed.data)
          } else {
            log.error("Invalid hooks.json schema", { path: hooksFile, issues: parsed.error.issues })
          }
        }
      }
    }

    return { configs }
  })

  export async function load(): Promise<HooksConfig[]> {
    const { configs } = await state()
    return configs
  }

  function matchesPattern(pattern: string, value: string): boolean {
    if (!pattern) return true
    const regex = new RegExp(`^${pattern}$`)
    return regex.test(value)
  }

  function matchesCommandPattern(pattern: string | undefined, command: string | undefined): boolean {
    if (!pattern || !command) return !pattern
    const regex = new RegExp(pattern)
    return regex.test(command)
  }

  function matchesFilePattern(pattern: string | undefined, filePath: string | undefined): boolean {
    if (!pattern || !filePath) return !pattern
    const regex = new RegExp(pattern)
    return regex.test(filePath)
  }

  async function scanForPatterns(content: string, patterns: string[]): Promise<{ found: boolean; matches: string[] }> {
    const matches: string[] = []
    for (const pattern of patterns) {
      const regex = new RegExp(pattern, "gm")
      const match = content.match(regex)
      if (match) {
        matches.push(...match)
      }
    }
    return { found: matches.length > 0, matches }
  }

  async function executeAction(
    action: Action,
    ctx: Context,
    hookName: string,
  ): Promise<{ blocked: boolean; message?: string }> {
    switch (action.type) {
      case "scan": {
        if (!action.patterns) return { blocked: false }

        // Convert input object to string by extracting actual string values
        // to avoid JSON escaping issues (e.g., " becomes \")
        const stringifyValues = (obj: unknown, depth = 0): string => {
          if (depth > 5) return ""
          if (obj === null || obj === undefined) return ""
          if (typeof obj === "string") return obj
          if (typeof obj === "number" || typeof obj === "boolean") return String(obj)
          if (Array.isArray(obj)) {
            return obj.map((v) => stringifyValues(v, depth + 1)).join(" ")
          }
          if (typeof obj === "object") {
            return Object.values(obj).map((v) => stringifyValues(v, depth + 1)).join(" ")
          }
          return ""
        }

        const inputContent = ctx.input ? stringifyValues(ctx.input) : ""
        const outputContent = ctx.output ?? ""

        // Scan both input and output (output is relevant for PostToolUse)
        const combinedContent = [inputContent, outputContent].join(" ")
        const { found, matches } = await scanForPatterns(combinedContent, action.patterns)
        if (found && action.block) {
          const msg = action.message?.replace("{match}", matches.join(", ")) ?? "Sensitive pattern detected"
          return { blocked: true, message: msg }
        }
        if (found && action.message) {
          log.warn(hookName, { message: action.message.replace("{match}", matches.join(", ")) })
        }
        return { blocked: false }
      }

      case "scan_content": {
        if (!action.patterns || !ctx.fileContent) return { blocked: false }
        const { found, matches } = await scanForPatterns(ctx.fileContent, action.patterns)
        if (found) {
          const lines: string[] = []
          const contentLines = ctx.fileContent.split("\n")
          for (const pattern of action.patterns) {
            const regex = new RegExp(pattern, "gm")
            for (let i = 0; i < contentLines.length; i++) {
              if (regex.test(contentLines[i])) {
                lines.push(`Line ${i + 1}`)
              }
            }
          }
          const msg =
            action.message
              ?.replace("{file}", ctx.filePath ?? "unknown")
              .replace("{line}", lines.join(", "))
              .replace("{match}", matches.join(", ")) ?? "Pattern detected in content"
          if (action.block) {
            return { blocked: true, message: msg }
          }
          log.warn(hookName, { message: msg })
        }
        return { blocked: false }
      }

      case "check_env": {
        if (!action.variable) return { blocked: false }
        const envValue = process.env[action.variable]
        const status = envValue ? "active" : "not set"
        if (action.command_pattern && ctx.command) {
          const regex = new RegExp(action.command_pattern)
          if (regex.test(ctx.command)) {
            const msg =
              action.message?.replace("{status}", status) ?? `Environment check: ${action.variable} is ${status}`
            if (action.block && !envValue) {
              return { blocked: true, message: msg }
            }
            log.info(hookName, { message: msg })
          }
        }
        return { blocked: false }
      }

      case "check_style": {
        const msg = action.message ?? "Style check reminder"
        log.info(hookName, { message: msg })
        return { blocked: false }
      }

      case "notify_only": {
        const msg = action.message ?? "Notification"
        if (action.block) {
          return { blocked: true, message: msg }
        }
        log.info(hookName, { message: msg })
        return { blocked: false }
      }

      case "run_command": {
        if (!action.command) return { blocked: false }
        const cmd = action.command.replace("{file}", ctx.filePath ?? "")
        if (action.async) {
          Bun.spawn(["sh", "-c", cmd], {
            cwd: Instance.directory,
            stdout: "inherit",
            stderr: "inherit",
          })
          return { blocked: false }
        }
        const proc = Bun.spawn(["sh", "-c", cmd], {
          cwd: Instance.directory,
          stdout: "pipe",
          stderr: "pipe",
        })
        const output = await new Response(proc.stdout).text()
        const exitCode = await proc.exited
        if (exitCode !== 0 && action.block) {
          return { blocked: true, message: `Command failed: ${cmd}\n${output}` }
        }
        if (output.trim() && action.on_output?.non_empty) {
          const msg = action.on_output.non_empty.replace("{output}", output.trim())
          log.warn(hookName, { message: msg })
        }
        return { blocked: false }
      }

      case "analyze_changes": {
        return { blocked: false }
      }

      case "scan_files": {
        return { blocked: false }
      }

      default:
        return { blocked: false }
    }
  }

  export async function run(lifecycle: Lifecycle, ctx: Context): Promise<RunResult> {
    const configs = await load()

    for (const config of configs) {
      if (config.settings?.enabled === false) continue

      const lifecycleHooks = config.hooks[lifecycle]
      if (!lifecycleHooks) continue

      for (const [hookName, hookDef] of Object.entries(lifecycleHooks)) {
        if (hookDef.pattern && ctx.tool && !matchesPattern(hookDef.pattern, ctx.tool)) {
          continue
        }

        if (hookDef.command_pattern && !matchesCommandPattern(hookDef.command_pattern, ctx.command)) {
          continue
        }

        if (hookDef.file_pattern && !matchesFilePattern(hookDef.file_pattern, ctx.filePath)) {
          continue
        }

        for (const action of hookDef.actions) {
          if (action.command_pattern && !matchesCommandPattern(action.command_pattern, ctx.command)) {
            continue
          }

          if (action.file_pattern && !matchesFilePattern(action.file_pattern, ctx.filePath)) {
            continue
          }

          const result = await executeAction(action, ctx, hookName)
          if (result.blocked) {
            return {
              blocked: true,
              message: result.message,
              hookName,
              actionType: action.type,
            }
          }
        }
      }
    }

    return { blocked: false }
  }

  export const Event = {
    Blocked: BusEvent.define(
      "hook.blocked",
      z.object({
        lifecycle: Lifecycle,
        hookName: z.string(),
        message: z.string().optional(),
        tool: z.string().optional(),
      }),
    ),
    Executed: BusEvent.define(
      "hook.executed",
      z.object({
        lifecycle: Lifecycle,
        hookName: z.string(),
        tool: z.string().optional(),
      }),
    ),
  }
}
