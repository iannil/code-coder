import { Config } from "../config/config"
import z from "zod"
import { Provider } from "../provider/provider"
import { generateObject, streamObject, type ModelMessage } from "ai"
import { SystemPrompt } from "../session/system"
import { Instance } from "../project/instance"
import { Truncate } from "../tool/truncation"
import { Auth } from "../auth"
import { ProviderTransform } from "../provider/transform"

import PROMPT_BUILD from "./prompt/build.txt"
import PROMPT_GENERATE from "./generate.txt"
import PROMPT_COMPACTION from "./prompt/compaction.txt"
import PROMPT_EXPLORE from "./prompt/explore.txt"
import PROMPT_SUMMARY from "./prompt/summary.txt"
import PROMPT_TITLE from "./prompt/title.txt"
import PROMPT_CODE_REVIEWER from "./prompt/code-reviewer.txt"
import PROMPT_SECURITY_REVIEWER from "./prompt/security-reviewer.txt"
import PROMPT_TDD_GUIDE from "./prompt/tdd-guide.txt"
import PROMPT_ARCHITECT from "./prompt/architect.txt"
import PROMPT_WRITER from "./prompt/writer.txt"
import PROMPT_PROOFREADER from "./prompt/proofreader.txt"
import PROMPT_CODE_REVERSE from "./prompt/code-reverse.txt"
import PROMPT_JAR_CODE_REVERSE from "./prompt/jar-code-reverse.txt"
import PROMPT_OBSERVER from "./prompt/observer.txt"
import PROMPT_DECISION from "./prompt/decision.txt"
import PROMPT_MACRO from "./prompt/macro.txt"
import PROMPT_TRADER from "./prompt/trader.txt"
import PROMPT_PICKER from "./prompt/picker.txt"
import PROMPT_MINIPRODUCT from "./prompt/miniproduct.txt"
import PROMPT_SYNTON_ASSISTANT from "./prompt/synton-assistant.txt"
import PROMPT_AI_ENGINEER from "./prompt/ai-engineer.txt"
import PROMPT_AUTONOMOUS from "./prompt/autonomous.txt"
import PROMPT_VERIFIER from "./prompt/verifier.txt"
import PROMPT_EXPANDER from "./prompt/expander.txt"
import PROMPT_EXPANDER_FICTION from "./prompt/expander-fiction.txt"
import PROMPT_EXPANDER_NONFICTION from "./prompt/expander-nonfiction.txt"
import * as WriterService from "./writer-service"
import { PermissionNext } from "@/permission/next"
import { mergeDeep, pipe, sortBy, values } from "remeda"
import { Global } from "@/global"
import path from "path"

export namespace Agent {
  export const Info = z
    .object({
      name: z.string(),
      description: z.string().optional(),
      mode: z.enum(["subagent", "primary", "all"]),
      native: z.boolean().optional(),
      hidden: z.boolean().optional(),
      topP: z.number().optional(),
      temperature: z.number().optional(),
      color: z.string().optional(),
      permission: PermissionNext.Ruleset,
      model: z
        .object({
          modelID: z.string(),
          providerID: z.string(),
        })
        .optional(),
      prompt: z.string().optional(),
      options: z.record(z.string(), z.any()),
      steps: z.number().int().positive().optional(),
    })
    .meta({
      ref: "Agent",
    })
  export type Info = z.infer<typeof Info>

  const state = Instance.state(async () => {
    const cfg = await Config.get()

    const defaults = PermissionNext.fromConfig({
      "*": "allow",
      doom_loop: "ask",
      external_directory: {
        "*": "ask",
        [Truncate.DIR]: "allow",
        [Truncate.GLOB]: "allow",
      },
      question: "deny",
      plan_enter: "deny",
      plan_exit: "deny",
      // mirrors github.com/github/gitignore Node.gitignore pattern for .env files
      read: {
        "*": "allow",
        "*.env": "ask",
        "*.env.*": "ask",
        "*.env.example": "allow",
      },
    })
    const user = PermissionNext.fromConfig(cfg.permission ?? {})

    const result: Record<string, Info> = {
      build: {
        name: "build",
        prompt: PROMPT_BUILD,
        options: {},
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            question: "allow",
            plan_enter: "allow",
          }),
          user,
        ),
        mode: "primary",
        native: true,
      },
      plan: {
        name: "plan",
        options: { maxOutputTokens: 128_000 },
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            question: "allow",
            plan_exit: "allow",
            external_directory: {
              [path.join(Global.Path.data, "plans", "*")]: "allow",
            },
            edit: {
              "*": "deny",
              [path.join(".ccode", "plans", "*.md")]: "allow",
              [path.join(".codecoder", "plans", "*.md")]: "allow",
              [path.relative(Instance.worktree, path.join(Global.Path.data, path.join("plans", "*.md")))]: "allow",
            },
          }),
          user,
        ),
        mode: "primary",
        native: true,
      },
      general: {
        name: "general",
        description: `General-purpose agent for researching complex questions and executing multi-step tasks. Use this agent to execute multiple units of work in parallel.`,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            todoread: "deny",
            todowrite: "deny",
          }),
          user,
        ),
        options: {},
        mode: "subagent",
        native: true,
      },
      explore: {
        name: "explore",
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            "*": "deny",
            grep: "allow",
            glob: "allow",
            list: "allow",
            bash: "allow",
            webfetch: "allow",
            websearch: "allow",
            codesearch: "allow",
            read: "allow",
            external_directory: {
              [Truncate.DIR]: "allow",
              [Truncate.GLOB]: "allow",
            },
          }),
          user,
        ),
        description: `Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns (eg. "src/components/**/*.tsx"), search code for keywords (eg. "API endpoints"), or answer questions about the codebase (eg. "how do API endpoints work?"). When calling this agent, specify the desired thoroughness level: "quick" for basic searches, "medium" for moderate exploration, or "very thorough" for comprehensive analysis across multiple locations and naming conventions.`,
        prompt: PROMPT_EXPLORE,
        options: {},
        mode: "subagent",
        native: true,
      },
      compaction: {
        name: "compaction",
        mode: "primary",
        native: true,
        hidden: true,
        prompt: PROMPT_COMPACTION,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            "*": "deny",
          }),
          user,
        ),
        options: {},
      },
      title: {
        name: "title",
        mode: "primary",
        options: {},
        native: true,
        hidden: true,
        temperature: 0.5,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            "*": "deny",
          }),
          user,
        ),
        prompt: PROMPT_TITLE,
      },
      summary: {
        name: "summary",
        mode: "primary",
        options: {},
        native: true,
        hidden: true,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            "*": "deny",
          }),
          user,
        ),
        prompt: PROMPT_SUMMARY,
      },
      "code-reviewer": {
        name: "code-reviewer",
        description: "Performs comprehensive code quality reviews with specific, actionable feedback",
        mode: "subagent",
        native: true,
        prompt: PROMPT_CODE_REVIEWER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
      },
      "security-reviewer": {
        name: "security-reviewer",
        description: "Analyzes code for security vulnerabilities and best practices",
        mode: "subagent",
        native: true,
        prompt: PROMPT_SECURITY_REVIEWER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
      },
      "tdd-guide": {
        name: "tdd-guide",
        description: "Enforces test-driven development methodology throughout the implementation",
        mode: "subagent",
        native: true,
        prompt: PROMPT_TDD_GUIDE,
        permission: PermissionNext.merge(defaults, user),
        options: {},
      },
      architect: {
        name: "architect",
        description: "Designs system architecture, defines interfaces, and establishes patterns",
        mode: "subagent",
        native: true,
        prompt: PROMPT_ARCHITECT,
        permission: PermissionNext.merge(defaults, user),
        options: {},
      },
      writer: {
        name: "writer",
        description:
          "Specialized agent for writing long-form content (20k+ words). Handles outline generation, chapter-by-chapter writing, and style consistency.",
        mode: "primary",
        native: true,
        prompt: PROMPT_WRITER,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            question: "allow",
            plan_enter: "allow",
          }),
          user,
        ),
        options: {
          maxOutputTokens: 128_000,
          // Disable thinking mode to prevent truncation when using large maxOutputTokens
          // Thinking mode reduces available output tokens by budgetTokens amount
          thinking: { type: "disabled" },
        },
        temperature: 0.7,
      },
      expander: {
        name: "expander",
        description:
          "Content expansion specialist for developing ideas into full-length books through systematic framework building, knowledge-aware writing, and consistency validation.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_EXPANDER,
        permission: PermissionNext.merge(defaults, user),
        options: {
          maxOutputTokens: 128_000,
          thinking: { type: "disabled" },
        },
        temperature: 0.7,
      },
      "expander-fiction": {
        name: "expander-fiction",
        description:
          "Fiction expansion specialist for transforming story ideas into coherent novels with consistent worldbuilding, character arcs, and narrative structure.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_EXPANDER_FICTION,
        permission: PermissionNext.merge(defaults, user),
        options: {
          maxOutputTokens: 128_000,
          thinking: { type: "disabled" },
        },
        temperature: 0.8,
      },
      "expander-nonfiction": {
        name: "expander-nonfiction",
        description:
          "Non-fiction expansion specialist for transforming ideas into comprehensive books through logical argumentation, evidence frameworks, and systematic reasoning.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_EXPANDER_NONFICTION,
        permission: PermissionNext.merge(defaults, user),
        options: {
          maxOutputTokens: 128_000,
          thinking: { type: "disabled" },
        },
        temperature: 0.6,
      },
      proofreader: {
        name: "proofreader",
        description:
          "Specialized agent for proofreading long-form text content. Checks grammar, spelling, punctuation, style, terminology, flow, readability, and structure using the PROOF framework.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_PROOFREADER,
        permission: PermissionNext.merge(defaults, user),
        options: {
          maxOutputTokens: 128_000,
          thinking: { type: "disabled" },
        },
        temperature: 0.3,
      },
      "code-reverse": {
        name: "code-reverse",
        description:
          "Website reverse engineering agent for pixel-perfect recreation planning. Analyzes websites, identifies technology stacks, extracts design systems, and generates comprehensive development plans.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_CODE_REVERSE,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            question: "allow",
            plan_enter: "allow",
            plan_exit: "allow",
          }),
          user,
        ),
        options: {},
        temperature: 0.3,
        color: "cyan",
      },
      "jar-code-reverse": {
        name: "jar-code-reverse",
        description:
          "JAR reverse engineering agent for Java source code reconstruction. Analyzes JAR files, identifies Java frameworks and libraries, extracts class structure, and generates comprehensive development plans.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_JAR_CODE_REVERSE,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            question: "allow",
            plan_enter: "allow",
            plan_exit: "allow",
            read: {
              "*.jar": "allow",
              "META-INF/*": "allow",
            },
          }),
          user,
        ),
        options: {},
        temperature: 0.3,
        color: "magenta",
      },
      observer: {
        name: "observer",
        description:
          "基于'祝融说'观察者理论分析问题，用可能性基底、观察收敛、观察共识等核心概念重新诠释现象，揭示隐藏的可能性空间",
        mode: "subagent",
        native: true,
        prompt: PROMPT_OBSERVER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.7,
      },
      decision: {
        name: "decision",
        description: "基于可持续决策理论的决策智慧师，使用CLOSE五维评估框架分析选择，帮助保持选择权和可用余量",
        mode: "subagent",
        native: true,
        prompt: PROMPT_DECISION,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.6,
      },
      macro: {
        name: "macro",
        description: "宏观经济分析师，基于18章课程体系解读GDP、工业、投资、消费、贸易、货币政策等数据，构建分析框架",
        mode: "subagent",
        native: true,
        prompt: PROMPT_MACRO,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.5,
      },
      trader: {
        name: "trader",
        description: "超短线交易指南，提供情绪周期、模式识别、仓位管理等技术分析框架（仅供教育参考，不构成投资建议）",
        mode: "subagent",
        native: true,
        prompt: PROMPT_TRADER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.5,
      },
      picker: {
        name: "picker",
        description: "选品专家，基于'爆品之眼'方法论，使用七宗罪选品法和数据验证框架识别市场机会和爆款潜力",
        mode: "subagent",
        native: true,
        prompt: PROMPT_PICKER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.6,
      },
      miniproduct: {
        name: "miniproduct",
        description: "极小产品教练，指导独立开发者从0到1构建可盈利软件产品，涵盖需求验证、AI辅助开发、变现和退出策略",
        mode: "subagent",
        native: true,
        prompt: PROMPT_MINIPRODUCT,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.6,
      },
      "synton-assistant": {
        name: "synton-assistant",
        description: "SYNTON-DB助手，帮助理解和使用专为LLM设计的记忆数据库，包括张量图存储、PaQL查询、Graph-RAG检索",
        mode: "subagent",
        native: true,
        prompt: PROMPT_SYNTON_ASSISTANT,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.5,
      },
      "ai-engineer": {
        name: "ai-engineer",
        description: "AI工程师导师，基于实战课程体系，从Python基础到LLM应用开发、RAG系统构建、微调和性能优化",
        mode: "subagent",
        native: true,
        prompt: PROMPT_AI_ENGINEER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.5,
      },
      verifier: {
        name: "verifier",
        description:
          "Verification agent for comprehensive validation. Performs build check, type check, lint check, test suite execution, console.log audit, git status analysis, formal methods, property-based testing, contract verification, and coverage analysis.",
        mode: "subagent",
        native: true,
        prompt: PROMPT_VERIFIER,
        permission: PermissionNext.merge(defaults, user),
        options: {},
        temperature: 0.1,
      },
      autonomous: {
        name: "autonomous",
        description:
          "自主模式 - 完全自主的执行代理，使用CLOSE决策框架，遵循祝融说哲学，可自主规划、决策、执行TDD循环、自我纠错",
        mode: "primary",
        native: true,
        prompt: PROMPT_AUTONOMOUS,
        permission: PermissionNext.merge(
          defaults,
          PermissionNext.fromConfig({
            question: "allow",
            plan_enter: "allow",
            plan_exit: "allow",
            doom_loop: "deny",
          }),
          user,
        ),
        options: {
          maxOutputTokens: 128_000,
          // Disable thinking mode to prevent truncation when using large maxOutputTokens
          // Thinking mode reduces available output tokens by budgetTokens amount
          // Set to environment variable CCODE_AUTONOMOUS_THINKING=true to enable if needed
          thinking: { type: "disabled" },
        },
        temperature: 0.6,
        color: "magenta",
      },
    }

    for (const [key, value] of Object.entries(cfg.agent ?? {})) {
      if (value.disable) {
        delete result[key]
        continue
      }
      let item = result[key]
      if (!item)
        item = result[key] = {
          name: key,
          mode: "all",
          permission: PermissionNext.merge(defaults, user),
          options: {},
          native: false,
        }
      if (value.model) item.model = Provider.parseModel(value.model)
      item.prompt = value.prompt ?? item.prompt
      item.description = value.description ?? item.description
      item.temperature = value.temperature ?? item.temperature
      item.topP = value.top_p ?? item.topP
      item.mode = value.mode ?? item.mode
      item.color = value.color ?? item.color
      item.hidden = value.hidden ?? item.hidden
      item.name = value.name ?? item.name
      item.steps = value.steps ?? item.steps
      item.options = mergeDeep(item.options, value.options ?? {})
      item.permission = PermissionNext.merge(item.permission, PermissionNext.fromConfig(value.permission ?? {}))
    }

    // Ensure Truncate.DIR is allowed unless explicitly configured
    for (const name in result) {
      const agent = result[name]
      const explicit = agent.permission.some((r) => {
        if (r.permission !== "external_directory") return false
        if (r.action !== "deny") return false
        return r.pattern === Truncate.DIR || r.pattern === Truncate.GLOB
      })
      if (explicit) continue

      result[name].permission = PermissionNext.merge(
        result[name].permission,
        PermissionNext.fromConfig({ external_directory: { [Truncate.DIR]: "allow", [Truncate.GLOB]: "allow" } }),
      )
    }

    return result
  })

  export async function get(agent: string) {
    return state().then((x) => x[agent])
  }

  export async function list() {
    const cfg = await Config.get()
    return pipe(
      await state(),
      values(),
      sortBy([(x) => (cfg.default_agent ? x.name === cfg.default_agent : x.name === "build"), "desc"]),
    )
  }

  export async function defaultAgent() {
    const cfg = await Config.get()
    const agents = await state()

    // Try configured default agent first
    if (cfg.default_agent) {
      const agent = agents[cfg.default_agent]

      // If agent doesn't exist in the list, it might be disabled
      // In this case, fall through to auto-detection
      if (!agent) {
        // Check if this is a known native agent that was disabled
        const nativeAgents = ["build", "plan", "code-reverse", "jar-code-reverse", "autonomous"]
        if (nativeAgents.includes(cfg.default_agent)) {
          // Agent was disabled, fall through to auto-detection
        } else {
          // Agent doesn't exist at all (non-existent agent name)
          throw new Error(`default agent "${cfg.default_agent}" not found`)
        }
      } else {
        // Agent exists, validate it
        if (agent.mode === "subagent") throw new Error(`default agent "${cfg.default_agent}" is a subagent`)
        if (agent.hidden === true) throw new Error(`default agent "${cfg.default_agent}" is hidden`)
        return agent.name
      }
    }

    // Fall back to first available primary visible agent
    const primaryVisible = Object.values(agents).find((a) => a.mode !== "subagent" && a.hidden !== true)
    if (!primaryVisible) throw new Error("no primary visible agent found")
    return primaryVisible.name
  }

  export async function generate(input: { description: string; model?: { providerID: string; modelID: string } }) {
    const cfg = await Config.get()
    const defaultModel = input.model ?? (await Provider.defaultModel())
    const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
    const language = await Provider.getLanguage(model)

    const system = SystemPrompt.header(defaultModel.providerID)
    system.push(PROMPT_GENERATE)
    const existing = await list()

    const params = {
      experimental_telemetry: {
        isEnabled: cfg.experimental?.openTelemetry,
        metadata: {
          userId: cfg.username ?? "unknown",
        },
      },
      temperature: 0.3,
      messages: [
        ...system.map(
          (item): ModelMessage => ({
            role: "system",
            content: item,
          }),
        ),
        {
          role: "user",
          content: `Create an agent configuration based on this request: \"${input.description}\".\n\nIMPORTANT: The following identifiers already exist and must NOT be used: ${existing.map((i) => i.name).join(", ")}\n  Return ONLY the JSON object, no other text, do not wrap in backticks`,
        },
      ],
      model: language,
      schema: z.object({
        identifier: z.string(),
        whenToUse: z.string(),
        systemPrompt: z.string(),
      }),
    } satisfies Parameters<typeof generateObject>[0]

    if (defaultModel.providerID === "openai" && (await Auth.get(defaultModel.providerID))?.type === "oauth") {
      const result = streamObject({
        ...params,
        providerOptions: ProviderTransform.providerOptions(model, {
          instructions: SystemPrompt.instructions(),
          store: false,
        }),
        onError: () => {},
      })
      for await (const part of result.fullStream) {
        if (part.type === "error") throw part.error
      }
      return result.object
    }

    const result = await generateObject(params)
    return result.object
  }
}
