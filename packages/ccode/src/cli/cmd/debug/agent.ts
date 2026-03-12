// @ts-nocheck
// Debug agent command - needs permission type handling
import { EOL } from "os"
import { basename } from "path"
import { getAgentBridge, toAgentInfo, type AgentInfoType } from "@/sdk/agent-bridge"
import { getDefaultModelWithFallback } from "@/sdk/provider-bridge"
import { Session } from "../../../session"
import type { MessageV2 } from "../../../session/message-v2"
import { Identifier } from "@/util/id/id"
import { ToolRegistry } from "../../../tool/registry"
import { Instance } from "../../../project/instance"
import { PermissionNext } from "@/security/permission/next"
import { iife } from "@/util/iife"
import { bootstrap } from "../../bootstrap"
import { cmd } from "../cmd"

export const AgentCommand = cmd({
  command: "agent <name>",
  describe: "show agent configuration details",
  builder: (yargs) =>
    yargs
      .positional("name", {
        type: "string",
        demandOption: true,
        description: "Agent name",
      })
      .option("tool", {
        type: "string",
        description: "Tool id to execute",
      })
      .option("params", {
        type: "string",
        description: "Tool params as JSON or a JS object literal",
      }),
  async handler(args) {
    await bootstrap(process.cwd(), async () => {
      const agentName = args.name as string
      const bridge = await getAgentBridge()
      const agentData = await bridge.get(agentName)
      if (!agentData) {
        process.stderr.write(
          `Agent ${agentName} not found, run '${basename(process.execPath)} agent list' to get an agent list` + EOL,
        )
        process.exit(1)
      }
      const agent = toAgentInfo(agentData)
      const availableTools = await getAvailableTools(agent)
      const resolvedTools = await resolveTools(agent, availableTools)
      const toolID = args.tool as string | undefined
      if (toolID) {
        const tool = availableTools.find((item) => item.id === toolID)
        if (!tool) {
          process.stderr.write(`Tool ${toolID} not found for agent ${agentName}` + EOL)
          process.exit(1)
        }
        if (resolvedTools[toolID] === false) {
          process.stderr.write(`Tool ${toolID} is disabled for agent ${agentName}` + EOL)
          process.exit(1)
        }
        const params = parseToolParams(args.params as string | undefined)
        const ctx = await createToolContext(agent)
        const result = await tool.execute(params, ctx)
        process.stdout.write(JSON.stringify({ tool: toolID, input: params, result }, null, 2) + EOL)
        return
      }

      const output = {
        ...agent,
        tools: resolvedTools,
      }
      process.stdout.write(JSON.stringify(output, null, 2) + EOL)
    })
  },
})

async function getAvailableTools(agent: AgentInfoType) {
  const model = agent.model ?? (await getDefaultModelWithFallback())
  return ToolRegistry.tools(model, agent)
}

async function resolveTools(agent: AgentInfoType, availableTools: Awaited<ReturnType<typeof getAvailableTools>>) {
  const disabled = PermissionNext.disabled(
    availableTools.map((tool) => tool.id),
    agent.permission,
  )
  const resolved: Record<string, boolean> = {}
  for (const tool of availableTools) {
    resolved[tool.id] = !disabled.has(tool.id)
  }
  return resolved
}

function parseToolParams(input?: string) {
  if (!input) return {}
  const trimmed = input.trim()
  if (trimmed.length === 0) return {}

  const parsed = iife(() => {
    try {
      return JSON.parse(trimmed)
    } catch (jsonError) {
      try {
        return new Function(`return (${trimmed})`)()
      } catch (evalError) {
        throw new Error(
          `Failed to parse --params. Use JSON or a JS object literal. JSON error: ${jsonError}. Eval error: ${evalError}.`,
        )
      }
    }
  })

  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Tool params must be an object.")
  }
  return parsed as Record<string, unknown>
}

async function createToolContext(agent: AgentInfoType) {
  const session = await Session.create({ title: `Debug tool run (${agent.name})` })
  const messageID = Identifier.ascending("message")
  const model = agent.model ?? (await getDefaultModelWithFallback())
  const now = Date.now()
  const message: MessageV2.Assistant = {
    id: messageID,
    sessionID: session.id,
    role: "assistant",
    time: {
      created: now,
    },
    parentID: messageID,
    modelID: model.modelID,
    providerID: model.providerID,
    mode: "debug",
    agent: agent.name,
    path: {
      cwd: Instance.directory,
      root: Instance.worktree,
    },
    cost: 0,
    tokens: {
      input: 0,
      output: 0,
      reasoning: 0,
      cache: {
        read: 0,
        write: 0,
      },
    },
  }
  await Session.updateMessage(message)

  const ruleset = PermissionNext.merge(agent.permission, session.permission ?? [])

  return {
    sessionID: session.id,
    messageID,
    callID: Identifier.ascending("part"),
    agent: agent.name,
    abort: new AbortController().signal,
    metadata: () => {},
    async ask(req: Omit<PermissionNext.Request, "id" | "sessionID" | "tool">) {
      for (const pattern of req.patterns) {
        const rule = PermissionNext.evaluate(req.permission, pattern, ruleset)
        if (rule.action === "deny") {
          throw new PermissionNext.DeniedError(ruleset)
        }
      }
    },
  }
}
