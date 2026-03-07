import z from "zod"
import { Tool } from "./tool"
import DESCRIPTION from "./batch.txt"
import type { MessageV2 } from "../session/message-v2"
import type { ToolRegistryHandleType } from "@codecoder-ai/core"

const DISALLOWED = new Set(["batch"])
const FILTERED_FROM_SUGGESTIONS = new Set(["invalid", "patch", ...DISALLOWED])

// Native tools that can be batched in Rust
const NATIVE_TOOLS = new Set(["grep", "glob", "read", "edit", "write", "ls", "apply_patch", "multiedit"])

export const BatchTool = Tool.define("batch", async () => {
  // Try to get native tool registry
  let nativeRegistry: ToolRegistryHandleType | null = null
  try {
    const { isNative, createToolRegistry } = await import("@codecoder-ai/core")
    if (isNative && createToolRegistry) {
      nativeRegistry = createToolRegistry()
    }
  } catch {
    // Native bindings not available
  }

  return {
    description: DESCRIPTION,
    parameters: z.object({
      tool_calls: z
        .array(
          z.object({
            tool: z.string().describe("The name of the tool to execute"),
            parameters: z.object({}).loose().describe("Parameters for the tool"),
          }),
        )
        .min(1, "Provide at least one tool call")
        .describe("Array of tool calls to execute in parallel"),
    }),
    formatValidationError(error) {
      const formattedErrors = error.issues
        .map((issue) => {
          const path = issue.path.length > 0 ? issue.path.join(".") : "root"
          return `  - ${path}: ${issue.message}`
        })
        .join("\n")

      return `Invalid parameters for tool 'batch':\n${formattedErrors}\n\nExpected payload format:\n  [{"tool": "tool_name", "parameters": {...}}, {...}]`
    },
    async execute(params, ctx) {
      const { Session } = await import("../session")
      const { Identifier } = await import("../id/id")

      const toolCalls = params.tool_calls.slice(0, 25)
      const discardedCalls = params.tool_calls.slice(25)

      const { ToolRegistry } = await import("./registry")
      const availableTools = await ToolRegistry.tools({ modelID: "", providerID: "" })
      const toolMap = new Map(availableTools.map((t) => [t.id, t]))

      // Separate native and non-native calls
      const nativeCalls: { call: (typeof toolCalls)[0]; index: number; partID: string }[] = []
      const jsCalls: { call: (typeof toolCalls)[0]; index: number; partID: string }[] = []

      for (let i = 0; i < toolCalls.length; i++) {
        const call = toolCalls[i]
        const partID = Identifier.ascending("part")

        if (nativeRegistry && NATIVE_TOOLS.has(call.tool)) {
          nativeCalls.push({ call, index: i, partID })
        } else {
          jsCalls.push({ call, index: i, partID })
        }
      }

      type CallResult = {
        success: true
        tool: string
        result: { output: string; title?: string; metadata?: unknown; attachments?: MessageV2.FilePart[] }
        partID: string
        index: number
      } | {
        success: false
        tool: string
        error: Error
        partID: string
        index: number
      }

      const allResults: CallResult[] = []

      // Execute native calls in a single batch NAPI call
      if (nativeRegistry && nativeCalls.length > 0) {
        const callStartTime = Date.now()

        // Update session parts to "running" for native calls
        await Promise.all(
          nativeCalls.map(({ call, partID }) =>
            Session.updatePart({
              id: partID,
              messageID: ctx.messageID,
              sessionID: ctx.sessionID,
              type: "tool",
              tool: call.tool,
              callID: partID,
              state: {
                status: "running",
                input: call.parameters,
                time: { start: callStartTime },
              },
            }),
          ),
        )

        // Prepare batch calls
        const batchCalls = nativeCalls.map(({ call, partID }) => ({
          tool: call.tool,
          argsJson: JSON.stringify(call.parameters),
          callId: partID,
        }))

        try {
          // Execute all native tools in parallel via single NAPI call
          const batchResult = await nativeRegistry.executeBatch(batchCalls)

          // Process results
          for (let i = 0; i < nativeCalls.length; i++) {
            const { call, index, partID } = nativeCalls[i]
            const result = batchResult.results[i]

            const endTime = Date.now()

            if (result.success) {
              await Session.updatePart({
                id: partID,
                messageID: ctx.messageID,
                sessionID: ctx.sessionID,
                type: "tool",
                tool: call.tool,
                callID: partID,
                state: {
                  status: "completed",
                  input: call.parameters,
                  output: result.output,
                  title: `${call.tool} (native batch)`,
                  metadata: { native: true, durationMs: result.durationMs },
                  time: { start: callStartTime, end: endTime },
                },
              })

              allResults.push({
                success: true,
                tool: call.tool,
                result: {
                  output: result.output,
                  title: `${call.tool} (native batch)`,
                  metadata: { native: true, durationMs: result.durationMs },
                },
                partID,
                index,
              })
            } else {
              await Session.updatePart({
                id: partID,
                messageID: ctx.messageID,
                sessionID: ctx.sessionID,
                type: "tool",
                tool: call.tool,
                callID: partID,
                state: {
                  status: "error",
                  input: call.parameters,
                  error: result.error ?? "Unknown error",
                  time: { start: callStartTime, end: endTime },
                },
              })

              allResults.push({
                success: false,
                tool: call.tool,
                error: new Error(result.error ?? "Unknown error"),
                partID,
                index,
              })
            }
          }
        } catch (error) {
          // Batch execution failed entirely, mark all as error
          const endTime = Date.now()
          for (const { call, index, partID } of nativeCalls) {
            await Session.updatePart({
              id: partID,
              messageID: ctx.messageID,
              sessionID: ctx.sessionID,
              type: "tool",
              tool: call.tool,
              callID: partID,
              state: {
                status: "error",
                input: call.parameters,
                error: error instanceof Error ? error.message : String(error),
                time: { start: callStartTime, end: endTime },
              },
            })

            allResults.push({
              success: false,
              tool: call.tool,
              error: error instanceof Error ? error : new Error(String(error)),
              partID,
              index,
            })
          }
        }
      }

      // Execute non-native calls in parallel (as before)
      const executeCall = async ({ call, index, partID }: (typeof jsCalls)[0]): Promise<CallResult> => {
        const callStartTime = Date.now()

        try {
          if (DISALLOWED.has(call.tool)) {
            throw new Error(
              `Tool '${call.tool}' is not allowed in batch. Disallowed tools: ${Array.from(DISALLOWED).join(", ")}`,
            )
          }

          const tool = toolMap.get(call.tool)
          if (!tool) {
            const availableToolsList = Array.from(toolMap.keys()).filter((name) => !FILTERED_FROM_SUGGESTIONS.has(name))
            throw new Error(
              `Tool '${call.tool}' not in registry. External tools (MCP, environment) cannot be batched - call them directly. Available tools: ${availableToolsList.join(", ")}`,
            )
          }
          const validatedParams = tool.parameters.parse(call.parameters)

          await Session.updatePart({
            id: partID,
            messageID: ctx.messageID,
            sessionID: ctx.sessionID,
            type: "tool",
            tool: call.tool,
            callID: partID,
            state: {
              status: "running",
              input: call.parameters,
              time: { start: callStartTime },
            },
          })

          const result = await tool.execute(validatedParams, { ...ctx, callID: partID })

          await Session.updatePart({
            id: partID,
            messageID: ctx.messageID,
            sessionID: ctx.sessionID,
            type: "tool",
            tool: call.tool,
            callID: partID,
            state: {
              status: "completed",
              input: call.parameters,
              output: result.output,
              title: result.title,
              metadata: result.metadata,
              attachments: result.attachments,
              time: { start: callStartTime, end: Date.now() },
            },
          })

          return { success: true, tool: call.tool, result, partID, index }
        } catch (error) {
          await Session.updatePart({
            id: partID,
            messageID: ctx.messageID,
            sessionID: ctx.sessionID,
            type: "tool",
            tool: call.tool,
            callID: partID,
            state: {
              status: "error",
              input: call.parameters,
              error: error instanceof Error ? error.message : String(error),
              time: { start: callStartTime, end: Date.now() },
            },
          })

          return { success: false, tool: call.tool, error: error instanceof Error ? error : new Error(String(error)), partID, index }
        }
      }

      const jsResults = await Promise.all(jsCalls.map(executeCall))
      allResults.push(...jsResults)

      // Add discarded calls as errors
      const now = Date.now()
      for (const call of discardedCalls) {
        const partID = Identifier.ascending("part")
        await Session.updatePart({
          id: partID,
          messageID: ctx.messageID,
          sessionID: ctx.sessionID,
          type: "tool",
          tool: call.tool,
          callID: partID,
          state: {
            status: "error",
            input: call.parameters,
            error: "Maximum of 25 tools allowed in batch",
            time: { start: now, end: now },
          },
        })
        allResults.push({
          success: false,
          tool: call.tool,
          error: new Error("Maximum of 25 tools allowed in batch"),
          partID,
          index: allResults.length,
        })
      }

      // Sort results by original index to maintain order
      allResults.sort((a, b) => a.index - b.index)

      const successfulCalls = allResults.filter((r) => r.success).length
      const failedCalls = allResults.length - successfulCalls
      const nativeCount = nativeCalls.length

      const outputMessage =
        failedCalls > 0
          ? `Executed ${successfulCalls}/${allResults.length} tools successfully (${nativeCount} native batch). ${failedCalls} failed.`
          : `All ${successfulCalls} tools executed successfully (${nativeCount} native batch).\n\nKeep using the batch tool for optimal performance in your next response!`

      return {
        title: `Batch execution (${successfulCalls}/${allResults.length} successful, ${nativeCount} native)`,
        output: outputMessage,
        attachments: allResults
          .filter((result): result is Extract<typeof result, { success: true }> => result.success)
          .flatMap((r) => r.result.attachments ?? []),
        metadata: {
          totalCalls: allResults.length,
          successful: successfulCalls,
          failed: failedCalls,
          nativeCount,
          tools: params.tool_calls.map((c) => c.tool),
          details: allResults.map((r) => ({ tool: r.tool, success: r.success })),
        },
      }
    },
  }
})
