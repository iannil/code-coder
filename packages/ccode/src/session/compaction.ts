import { BusEvent } from "@/bus/bus-event"
import { Bus } from "@/bus"
import { Session } from "."
import { Identifier } from "../id/id"
import { Instance } from "../project/instance"
import { Provider } from "../provider/provider"
import { MessageV2 } from "./message-v2"
import z from "zod"
import { Log } from "@/util/log"
import { SessionProcessor } from "./processor"
import { fn } from "@/util/fn"
import { Agent } from "@/agent/agent"
import { Config } from "@/config/config"
import {
  isOverflow as nativeIsOverflow,
  computePrunePlanWithTurns as nativeComputePrunePlanWithTurns,
  createDefaultPruneConfig,
  type NapiMessageInfo,
  type NapiToolPartInfo,
  type NapiPruneConfig,
} from "@codecoder-ai/core"

export namespace SessionCompaction {
  const log = Log.create({ service: "session.compaction" })

  export const Event = {
    Compacted: BusEvent.define(
      "session.compacted",
      z.object({
        sessionID: z.string(),
      }),
    ),
  }

  /**
   * Check if token usage overflows the model's context limit.
   *
   * Uses native Rust implementation for consistent calculation.
   */
  export async function isOverflow(input: { tokens: MessageV2.Assistant["tokens"]; model: Provider.Model }) {
    const config = await Config.get()
    if (config.compaction?.auto === false) return false

    if (!nativeIsOverflow) {
      throw new Error("Native isOverflow binding is unavailable")
    }

    return nativeIsOverflow(
      {
        input: input.tokens.input,
        output: input.tokens.output,
        cacheRead: input.tokens.cache.read,
        cacheWrite: input.tokens.cache.write,
      },
      {
        context: input.model.limit.context,
        output: input.model.limit.output,
        input: input.model.limit.input || undefined,
      },
    )
  }

  export const PRUNE_MINIMUM = 20_000
  export const PRUNE_PROTECT = 40_000

  const PRUNE_PROTECTED_TOOLS = ["skill"]

  /**
   * Convert messages to native format for prune computation.
   */
  function toNativeMessageInfo(msgs: MessageV2.WithParts[]): NapiMessageInfo[] {
    return msgs.map((msg) => ({
      messageId: msg.info.id,
      role: msg.info.role,
      isSummary: msg.info.role === "assistant" && "summary" in msg.info && msg.info.summary === true,
      toolParts: msg.parts
        .filter((p): p is MessageV2.ToolPart => p.type === "tool")
        .map((part): NapiToolPartInfo => ({
          messageId: part.messageID,
          partId: part.id,
          tool: part.tool,
          status: part.state.status,
          compacted: part.state.status === "completed" && "time" in part.state && !!part.state.time.compacted,
          output: part.state.status === "completed" ? part.state.output : "",
        })),
    }))
  }

  /**
   * Prune old tool outputs to reduce context usage.
   *
   * Goes backwards through parts until there are PRUNE_PROTECT tokens worth of tool
   * calls, then erases output of previous tool calls. Uses native Rust computation
   * for efficiency.
   */
  export async function prune(input: { sessionID: string }) {
    const config = await Config.get()
    if (config.compaction?.prune === false) return
    log.info("pruning")

    if (!nativeComputePrunePlanWithTurns) {
      throw new Error("Native computePrunePlanWithTurns binding is unavailable")
    }

    const msgs = await Session.messages({ sessionID: input.sessionID })

    const nativeMsgs = toNativeMessageInfo(msgs)
    const pruneConfig: NapiPruneConfig = createDefaultPruneConfig?.() ?? {
      minimum: PRUNE_MINIMUM,
      protect: PRUNE_PROTECT,
      protectedTools: PRUNE_PROTECTED_TOOLS,
    }

    const plan = nativeComputePrunePlanWithTurns(nativeMsgs, pruneConfig)

    log.info("found", { pruned: plan.totalTokensToPrune, shouldExecute: plan.shouldExecute })

    if (plan.shouldExecute) {
      // Apply the prune plan - mark parts as compacted
      for (const partRef of plan.partsToPrune) {
        // Find the original part and update it
        for (const msg of msgs) {
          for (const part of msg.parts) {
            if (part.type === "tool" && part.id === partRef.partId && part.state.status === "completed") {
              part.state.time.compacted = Date.now()
              await Session.updatePart(part)
            }
          }
        }
      }
      log.info("pruned", { count: plan.partsToPrune.length })
    }
  }

  export async function process(input: {
    parentID: string
    messages: MessageV2.WithParts[]
    sessionID: string
    abort: AbortSignal
    auto: boolean
  }) {
    const userMessage = input.messages.findLast((m) => m.info.id === input.parentID)!.info as MessageV2.User
    const agent = await Agent.get("compaction")
    const model = agent.model
      ? await Provider.getModel(agent.model.providerID, agent.model.modelID)
      : await Provider.getModel(userMessage.model.providerID, userMessage.model.modelID)
    const msg = (await Session.updateMessage({
      id: Identifier.ascending("message"),
      role: "assistant",
      parentID: input.parentID,
      sessionID: input.sessionID,
      mode: "compaction",
      agent: "compaction",
      summary: true,
      path: {
        cwd: Instance.directory,
        root: Instance.worktree,
      },
      cost: 0,
      tokens: {
        output: 0,
        input: 0,
        reasoning: 0,
        cache: { read: 0, write: 0 },
      },
      modelID: model.id,
      providerID: model.providerID,
      time: {
        created: Date.now(),
      },
    })) as MessageV2.Assistant
    const processor = SessionProcessor.create({
      assistantMessage: msg,
      sessionID: input.sessionID,
      model,
      abort: input.abort,
    })
    const defaultPrompt =
      "Provide a detailed prompt for continuing our conversation above. Focus on information that would be helpful for continuing the conversation, including what we did, what we're doing, which files we're working on, and what we're going to do next considering new session will not have access to our conversation."
    const promptText = defaultPrompt
    const result = await processor.process({
      user: userMessage,
      agent,
      abort: input.abort,
      sessionID: input.sessionID,
      tools: {},
      system: [],
      messages: [
        ...(await MessageV2.toModelMessages(input.messages, model)),
        {
          role: "user",
          content: [
            {
              type: "text",
              text: promptText,
            },
          ],
        },
      ],
      model,
    })

    if (result === "continue" && input.auto) {
      const continueMsg = await Session.updateMessage({
        id: Identifier.ascending("message"),
        role: "user",
        sessionID: input.sessionID,
        time: {
          created: Date.now(),
        },
        agent: userMessage.agent,
        model: userMessage.model,
      })
      await Session.updatePart({
        id: Identifier.ascending("part"),
        messageID: continueMsg.id,
        sessionID: input.sessionID,
        type: "text",
        synthetic: true,
        text: "Continue if you have next steps",
        time: {
          start: Date.now(),
          end: Date.now(),
        },
      })
    }
    if (processor.message.error) return "stop"
    Bus.publish(Event.Compacted, { sessionID: input.sessionID })
    return "continue"
  }

  export const create = fn(
    z.object({
      sessionID: Identifier.schema("session"),
      agent: z.string(),
      model: z.object({
        providerID: z.string(),
        modelID: z.string(),
      }),
      auto: z.boolean(),
    }),
    async (input) => {
      const msg = await Session.updateMessage({
        id: Identifier.ascending("message"),
        role: "user",
        model: input.model,
        sessionID: input.sessionID,
        agent: input.agent,
        time: {
          created: Date.now(),
        },
      })
      await Session.updatePart({
        id: Identifier.ascending("part"),
        messageID: msg.id,
        sessionID: msg.sessionID,
        type: "compaction",
        auto: input.auto,
      })
    },
  )
}
