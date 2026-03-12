import { QuestionTool } from "./question"
import { BashTool } from "./bash"
import { EditTool } from "./edit"
import { GlobTool } from "./glob"
import { GrepTool } from "./grep"
import { BatchTool } from "./batch"
import { ReadTool } from "./read"
import { TaskTool } from "./task"
import { TodoWriteTool, TodoReadTool } from "./todo"
import { WebFetchTool } from "./webfetch"
import { WriteTool } from "./write"
import { InvalidTool } from "./invalid"
import { SkillTool } from "./skill"
import type { AgentInfoType } from "@/sdk/agent-bridge"
import { Tool } from "./tool"
import { Instance } from "@/project/instance"
import { Config } from "@/config/config"
import path from "path"
import z from "zod"
import { WebSearchTool } from "./websearch"
import { CodeSearchTool } from "./codesearch"
import { Flag } from "@/util/flag/flag"
import { Log } from "@/util/log"
import { LspTool } from "./lsp"
import { Truncate } from "./truncation"
import { PlanExitTool, PlanEnterTool } from "./plan"
import { ApplyPatchTool } from "./apply_patch"
import { NetworkAnalyzerTool } from "./network-analyzer"
import { GetCredentialTool } from "./credential"
import { ReachTools } from "./reach"
import { ProjectTools } from "./project"
import {
  SchedulerCreateTaskTool,
  SchedulerListTasksTool,
  SchedulerDeleteTaskTool,
  SchedulerRunTaskTool,
  SchedulerDelayTaskTool,
} from "./scheduler"
import {
  MacroSystem,
  type ToolMacro,
  type MacroExecutionContext,
  BuiltinMacros,
} from "./macro"

export namespace ToolRegistry {
  const log = Log.create({ service: "tool.registry" })

  // Store for registered macros
  const macroStore = new Map<string, ToolMacro>()

  export const state = Instance.state(async () => {
    const custom = [] as Tool.Info[]
    const glob = new Bun.Glob("{tool,tools}/*.{js,ts}")

    for (const dir of await Config.directories()) {
      for await (const match of glob.scan({
        cwd: dir,
        absolute: true,
        followSymlinks: true,
        dot: true,
      })) {
        const namespace = path.basename(match, path.extname(match))
        const mod = await import(match)
        for (const [id, def] of Object.entries(mod)) {
          if (typeof def === "function" && "tool" in def && (def as Function & { tool?: unknown }).tool) {
            const { id: _toolId, ...toolInfo } = (def as Function & { tool: Tool.Info }).tool
            custom.push({
              ...toolInfo,
              id: id === "default" ? namespace : `${namespace}_${id}`,
            })
          }
        }
      }
    }

    // Load built-in macros
    for (const macro of Object.values(BuiltinMacros)) {
      macroStore.set(macro.id, macro)
    }

    return { custom }
  })

  export async function register(tool: Tool.Info) {
    const { custom } = await state()
    const idx = custom.findIndex((t) => t.id === tool.id)
    if (idx >= 0) {
      custom.splice(idx, 1, tool)
      return
    }
    custom.push(tool)
  }

  /**
   * Register a macro as a tool
   */
  export function registerMacro(macro: ToolMacro): void {
    const validation = MacroSystem.validate(macro)
    if (!validation.valid) {
      log.error("Invalid macro", { id: macro.id, errors: validation.errors })
      throw new Error(`Invalid macro: ${validation.errors.map((e) => e.message).join(", ")}`)
    }
    macroStore.set(macro.id, macro)
    log.debug("Registered macro", { id: macro.id, name: macro.name })
  }

  /**
   * Get all registered macros
   */
  export function getMacros(): ToolMacro[] {
    return Array.from(macroStore.values())
  }

  /**
   * Get a specific macro by ID
   */
  export function getMacro(id: string): ToolMacro | undefined {
    return macroStore.get(id)
  }

  /**
   * Create a Tool.Info from a macro definition
   */
  export function macroToTool(macro: ToolMacro): Tool.Info {
    // Build parameter schema from macro parameters
    const paramSchema: Record<string, z.ZodTypeAny> = {}
    for (const param of macro.parameters) {
      let schema: z.ZodTypeAny
      switch (param.type) {
        case "string":
          schema = z.string()
          break
        case "number":
          schema = z.number()
          break
        case "boolean":
          schema = z.boolean()
          break
        case "array":
          schema = z.array(z.unknown())
          break
        case "object":
          schema = z.record(z.string(), z.unknown())
          break
        default:
          schema = z.unknown()
      }

      if (!param.required && param.default !== undefined) {
        schema = schema.optional().default(param.default as any)
      } else if (!param.required) {
        schema = schema.optional()
      }

      paramSchema[param.name] = schema
    }

    return Tool.define(`macro_${macro.id}`, {
      description: `[Macro] ${macro.description}\n\nSteps: ${macro.steps.map((s) => s.tool).join(" → ")}`,
      parameters: z.object(paramSchema),
      async execute(args, ctx) {
        const context: MacroExecutionContext = {
          sessionId: ctx.sessionID,
          workingDirectory: process.cwd(),
          agent: ctx.agent,
          timestamp: Date.now(),
          abort: ctx.abort,
        }

        const result = await MacroSystem.execute(macro, args, context)

        const statusEmoji = result.status === "success" ? "✓" : result.status === "partial" ? "⚠" : "✗"
        const stepSummary = result.stepResults
          .map((s) => `  ${s.status === "success" || s.status === "retried" ? "✓" : s.status === "skipped" ? "○" : "✗"} ${s.tool}`)
          .join("\n")

        return {
          title: `Macro: ${macro.name}`,
          metadata: {
            macroId: macro.id,
            status: result.status,
            totalDurationMs: result.totalDurationMs,
            outputs: result.outputs,
          },
          output: `${statusEmoji} Macro "${macro.name}" ${result.status}\n\nSteps:\n${stepSummary}${result.error ? `\n\nError: ${result.error}` : ""}`,
        }
      },
    })
  }

  async function all(): Promise<Tool.Info[]> {
    const custom = await state().then((x) => x.custom)
    const config = await Config.get()

    // Convert registered macros to tools
    const macroTools = Array.from(macroStore.values()).map(macroToTool)

    return [
      InvalidTool,
      ...(["app", "cli", "desktop"].includes(Flag.CCODE_CLIENT) ? [QuestionTool] : []),
      BashTool,
      ReadTool,
      GlobTool,
      GrepTool,
      EditTool,
      WriteTool,
      TaskTool,
      WebFetchTool,
      TodoWriteTool,
      TodoReadTool,
      WebSearchTool,
      CodeSearchTool,
      SkillTool,
      ApplyPatchTool,
      NetworkAnalyzerTool,
      GetCredentialTool,
      ...(Flag.CCODE_EXPERIMENTAL_LSP_TOOL ? [LspTool] : []),
      ...(config.experimental?.batch_tool === true ? [BatchTool] : []),
      ...(Flag.CCODE_EXPERIMENTAL_PLAN_MODE && Flag.CCODE_CLIENT === "cli"
        ? [PlanExitTool, PlanEnterTool]
        : []),
      ...ReachTools,
      // Scheduler tools for autonomous task management
      SchedulerCreateTaskTool,
      SchedulerListTasksTool,
      SchedulerDeleteTaskTool,
      SchedulerRunTaskTool,
      SchedulerDelayTaskTool,
      // Project tools for autonomous project creation
      ...ProjectTools,
      // Macro tools
      ...macroTools,
      ...custom,
    ]
  }

  export async function ids() {
    return all().then((x) => x.map((t) => t.id))
  }

  export async function tools(
    model: {
      providerID: string
      modelID: string
    },
    agent?: AgentInfoType,
  ) {
    const tools = await all()
    const result = await Promise.all(
      tools
        .filter((t) => {
          // Enable websearch/codesearch for zen users, via enable flag, or for autonomous agent
          if (t.id === "codesearch" || t.id === "websearch") {
            // Always enable for autonomous agent (needs real-time data)
            if (agent?.name === "autonomous") {
              return true
            }
            return model.providerID === "ccode" || Flag.CCODE_ENABLE_EXA
          }

          // use apply tool in same format as codex
          const usePatch =
            model.modelID.includes("gpt-") && !model.modelID.includes("oss") && !model.modelID.includes("gpt-4")
          if (t.id === "apply_patch") return usePatch
          if (t.id === "edit" || t.id === "write") return !usePatch

          return true
        })
        .map(async (t) => {
          using _ = log.time(t.id)
          return {
            id: t.id,
            ...(await t.init({ agent })),
          }
        }),
    )
    return result
  }
}
