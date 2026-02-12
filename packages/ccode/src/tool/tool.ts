import z from "zod"
import type { MessageV2 } from "../session/message-v2"
import type { Agent } from "../agent/agent"
import type { PermissionNext } from "../permission/next"
import { Truncate } from "./truncation"
import { Hook } from "../hook"
import { NamedError } from "@codecoder-ai/util/error"
import { AutonomousModeHook } from "../autonomous"

export namespace Tool {
  interface Metadata {
    [key: string]: any
  }

  export interface InitContext {
    agent?: Agent.Info
  }

  export const HookBlockedError = NamedError.create(
    "HookBlockedError",
    z.object({
      hookName: z.string(),
      message: z.string(),
      tool: z.string(),
      lifecycle: z.string(),
    }),
  )

  export type Context<M extends Metadata = Metadata> = {
    sessionID: string
    messageID: string
    agent: string
    abort: AbortSignal
    callID?: string
    extra?: { [key: string]: any }
    metadata(input: { title?: string; metadata?: M }): void
    ask(input: Omit<PermissionNext.Request, "id" | "sessionID" | "tool">): Promise<void>
  }
  export interface Info<Parameters extends z.ZodType = z.ZodType, M extends Metadata = Metadata> {
    id: string
    init: (ctx?: InitContext) => Promise<{
      description: string
      parameters: Parameters
      execute(
        args: z.infer<Parameters>,
        ctx: Context,
      ): Promise<{
        title: string
        metadata: M
        output: string
        attachments?: MessageV2.FilePart[]
      }>
      formatValidationError?(error: z.ZodError): string
    }>
  }

  export type InferParameters<T extends Info> = T extends Info<infer P> ? z.infer<P> : never
  export type InferMetadata<T extends Info> = T extends Info<any, infer M> ? M : never

  export function define<Parameters extends z.ZodType, Result extends Metadata>(
    id: string,
    init: Info<Parameters, Result>["init"] | Awaited<ReturnType<Info<Parameters, Result>["init"]>>,
  ): Info<Parameters, Result> {
    return {
      id,
      init: async (initCtx) => {
        const toolInfo = init instanceof Function ? await init(initCtx) : init
        const execute = toolInfo.execute
        toolInfo.execute = async (args, ctx) => {
          try {
            toolInfo.parameters.parse(args)
          } catch (error) {
            if (error instanceof z.ZodError && toolInfo.formatValidationError) {
              throw new Error(toolInfo.formatValidationError(error), { cause: error })
            }
            throw new Error(
              `The ${id} tool was called with invalid arguments: ${error}.\nPlease rewrite the input so it satisfies the expected schema.`,
              { cause: error },
            )
          }

          // Run PreToolUse hooks
          const preHookCtx: Hook.Context = {
            tool: id,
            input: args as Record<string, unknown>,
            sessionID: ctx.sessionID,
            filePath: (args as any).filePath ?? (args as any).file_path,
            command: (args as any).command,
          }
          const preResult = await Hook.run("PreToolUse", preHookCtx)
          if (preResult.blocked) {
            throw new HookBlockedError({
              hookName: preResult.hookName ?? "unknown",
              message: preResult.message ?? "Operation blocked by hook",
              tool: id,
              lifecycle: "PreToolUse",
            })
          }

          // Autonomous Mode: Run CLOSE decision evaluation
          let autonomousDecision: Awaited<ReturnType<typeof AutonomousModeHook.evaluateToolCall>> | undefined
          if (ctx.agent === "autonomous") {
            autonomousDecision = await AutonomousModeHook.evaluateToolCall({
              sessionId: ctx.sessionID,
              toolName: id,
              toolInput: args as Record<string, unknown>,
            })

            if (!autonomousDecision.allowed) {
              throw new HookBlockedError({
                hookName: "AutonomousMode",
                message: autonomousDecision.decision?.reasoning ?? "Blocked by CLOSE decision framework",
                tool: id,
                lifecycle: "PreToolUse",
              })
            }
          }

          const result = await execute(args, ctx)

          // Add CLOSE decision info to result metadata for autonomous agent
          if (autonomousDecision?.decision && ctx.agent === "autonomous") {
            result.metadata = {
              ...result.metadata,
              closeDecision: {
                action: autonomousDecision.decision.action,
                score: autonomousDecision.decision.score,
                reasoning: autonomousDecision.decision.reasoning,
                tool: id,
              },
            }
          }

          // Run PostToolUse hooks
          const postHookCtx: Hook.Context = {
            tool: id,
            input: args as Record<string, unknown>,
            output: result.output,
            sessionID: ctx.sessionID,
            filePath: (args as any).filePath ?? (args as any).file_path,
            command: (args as any).command,
            fileContent: result.metadata?.filediff?.after,
            diff: result.metadata?.diff,
          }
          const postResult = await Hook.run("PostToolUse", postHookCtx)
          if (postResult.blocked) {
            throw new HookBlockedError({
              hookName: postResult.hookName ?? "unknown",
              message: postResult.message ?? "Operation blocked by post-execution hook",
              tool: id,
              lifecycle: "PostToolUse",
            })
          }

          // skip truncation for tools that handle it themselves
          if (result.metadata.truncated !== undefined) {
            return result
          }
          const truncated = await Truncate.output(result.output, {}, initCtx?.agent)
          return {
            ...result,
            output: truncated.content,
            metadata: {
              ...result.metadata,
              truncated: truncated.truncated,
              ...(truncated.truncated && { outputPath: truncated.outputPath }),
            },
          }
        }
        return toolInfo
      },
    }
  }
}
