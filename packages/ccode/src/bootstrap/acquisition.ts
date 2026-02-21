import { Log } from "@/util/log"
import { Config } from "@/config/config"
import { Skill } from "@/skill/skill"
import { Provider } from "@/provider/provider"
import { generateText } from "ai"
import { BootstrapTypes } from "./types"
import { SelfAwareness } from "./awareness"

const log = Log.create({ service: "bootstrap.acquisition" })

/**
 * ResourceAcquisition handles discovering and acquiring new capabilities
 * when the agent encounters tasks beyond its current abilities.
 */
export namespace ResourceAcquisition {
  /**
   * Resource types that can be acquired
   */
  export type ResourceType = "mcp" | "skill" | "api" | "tool"

  /**
   * A discovered resource
   */
  export interface DiscoveredResource {
    type: ResourceType
    name: string
    description: string
    source: string
    installHint?: string
    confidence: number
  }

  /**
   * Result of resource discovery
   */
  export interface DiscoveryResult {
    mcpServers: DiscoveredResource[]
    skills: DiscoveredResource[]
    externalAPIs: DiscoveredResource[]
  }

  /**
   * Discover resources needed for a task
   */
  export async function discoverNeeded(task: string): Promise<DiscoveryResult> {
    log.info("discovering needed resources", { task: task.slice(0, 100) })

    // First check what we're missing
    const canHandle = await SelfAwareness.canHandle(task)

    if (canHandle.confident) {
      return { mcpServers: [], skills: [], externalAPIs: [] }
    }

    const missingCapabilities = canHandle.missingCapabilities ?? []
    const suggestions = canHandle.suggestedResources ?? []

    const result: DiscoveryResult = {
      mcpServers: [],
      skills: [],
      externalAPIs: [],
    }

    // Categorize missing capabilities
    for (const missing of missingCapabilities) {
      const [type, name] = missing.split(":")

      switch (type) {
        case "mcp":
          result.mcpServers.push(await discoverMcpServer(name, task))
          break
        case "skill":
          result.skills.push(await discoverSkill(name, task))
          break
        case "tool":
          // Tools might be provided by MCP
          result.mcpServers.push(await discoverMcpServer(name, task))
          break
        default:
          result.externalAPIs.push(await discoverApi(name, task))
      }
    }

    // Use LLM to suggest additional resources
    const llmSuggestions = await suggestResourcesWithLLM(task, missingCapabilities)
    result.mcpServers.push(...llmSuggestions.mcpServers)
    result.skills.push(...llmSuggestions.skills)
    result.externalAPIs.push(...llmSuggestions.externalAPIs)

    // Deduplicate
    result.mcpServers = deduplicateResources(result.mcpServers)
    result.skills = deduplicateResources(result.skills)
    result.externalAPIs = deduplicateResources(result.externalAPIs)

    log.info("discovered resources", {
      mcpCount: result.mcpServers.length,
      skillCount: result.skills.length,
      apiCount: result.externalAPIs.length,
    })

    return result
  }

  /**
   * Discover an MCP server that might provide needed capability
   */
  async function discoverMcpServer(name: string, context: string): Promise<DiscoveredResource> {
    // Known MCP server mappings
    const knownServers: Record<string, { name: string; source: string; installHint: string }> = {
      github: {
        name: "github",
        source: "npx -y @modelcontextprotocol/server-github",
        installHint: 'Add to config.mcp: {"github": {"command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"]}}',
      },
      filesystem: {
        name: "filesystem",
        source: "npx -y @modelcontextprotocol/server-filesystem",
        installHint: 'Add to config.mcp: {"filesystem": {"command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]}}',
      },
      slack: {
        name: "slack",
        source: "npx -y @modelcontextprotocol/server-slack",
        installHint: 'Add to config.mcp with SLACK_BOT_TOKEN env var',
      },
      browser: {
        name: "playwright",
        source: "npx -y @anthropic/mcp-playwright",
        installHint: 'Add to config.mcp: {"playwright": {"command": "npx", "args": ["-y", "@anthropic/mcp-playwright"]}}',
      },
      memory: {
        name: "memory",
        source: "npx -y @modelcontextprotocol/server-memory",
        installHint: 'Add to config.mcp for persistent memory',
      },
    }

    const nameLower = name.toLowerCase()
    for (const [key, server] of Object.entries(knownServers)) {
      if (nameLower.includes(key)) {
        return {
          type: "mcp",
          name: server.name,
          description: `MCP server for ${key} integration`,
          source: server.source,
          installHint: server.installHint,
          confidence: 0.8,
        }
      }
    }

    return {
      type: "mcp",
      name,
      description: `MCP server for ${name}`,
      source: "unknown",
      confidence: 0.3,
    }
  }

  /**
   * Discover a skill that might help
   */
  async function discoverSkill(name: string, context: string): Promise<DiscoveredResource> {
    // Check existing skills
    const existingSkills = await Skill.all()
    const matching = existingSkills.find((s) =>
      s.name.toLowerCase().includes(name.toLowerCase()) ||
      s.description.toLowerCase().includes(name.toLowerCase()),
    )

    if (matching) {
      return {
        type: "skill",
        name: matching.name,
        description: matching.description,
        source: matching.location,
        confidence: 0.9,
      }
    }

    return {
      type: "skill",
      name,
      description: `Skill for ${name}`,
      source: "not found - consider creating with /crystallize",
      confidence: 0.2,
    }
  }

  /**
   * Discover an external API
   */
  async function discoverApi(name: string, context: string): Promise<DiscoveredResource> {
    // Common API patterns
    const knownApis: Record<string, { name: string; description: string }> = {
      openai: { name: "OpenAI API", description: "AI/ML capabilities via OpenAI" },
      anthropic: { name: "Anthropic API", description: "Claude AI capabilities" },
      google: { name: "Google APIs", description: "Various Google services" },
      stripe: { name: "Stripe API", description: "Payment processing" },
      twilio: { name: "Twilio API", description: "SMS and communications" },
    }

    const nameLower = name.toLowerCase()
    for (const [key, api] of Object.entries(knownApis)) {
      if (nameLower.includes(key)) {
        return {
          type: "api",
          name: api.name,
          description: api.description,
          source: `https://${key}.com`,
          confidence: 0.6,
        }
      }
    }

    return {
      type: "api",
      name,
      description: `External API for ${name}`,
      source: "unknown",
      confidence: 0.2,
    }
  }

  /**
   * Use LLM to suggest additional resources
   */
  async function suggestResourcesWithLLM(
    task: string,
    missingCapabilities: string[],
  ): Promise<DiscoveryResult> {
    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const prompt = `Given this task and missing capabilities, suggest resources that could help.

Task: ${task.slice(0, 500)}

Missing Capabilities:
${missingCapabilities.join("\n")}

Suggest MCP servers, skills, or APIs that could help. Format as JSON:
{
  "mcpServers": [{"name": "...", "description": "...", "installHint": "..."}],
  "skills": [{"name": "...", "description": "..."}],
  "externalAPIs": [{"name": "...", "description": "...", "url": "..."}]
}

Only suggest well-known, established tools. Return ONLY the JSON.`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: 500,
        temperature: 0.3,
      })

      const jsonMatch = result.text.match(/\{[\s\S]*\}/)
      if (!jsonMatch) {
        return { mcpServers: [], skills: [], externalAPIs: [] }
      }

      const suggestions = JSON.parse(jsonMatch[0])

      return {
        mcpServers: (suggestions.mcpServers ?? []).map((s: any) => ({
          type: "mcp" as const,
          name: s.name,
          description: s.description,
          source: s.installHint ?? "unknown",
          installHint: s.installHint,
          confidence: 0.5,
        })),
        skills: (suggestions.skills ?? []).map((s: any) => ({
          type: "skill" as const,
          name: s.name,
          description: s.description,
          source: "suggested",
          confidence: 0.4,
        })),
        externalAPIs: (suggestions.externalAPIs ?? []).map((s: any) => ({
          type: "api" as const,
          name: s.name,
          description: s.description,
          source: s.url ?? "unknown",
          confidence: 0.4,
        })),
      }
    } catch {
      return { mcpServers: [], skills: [], externalAPIs: [] }
    }
  }

  /**
   * Deduplicate resources by name
   */
  function deduplicateResources(resources: DiscoveredResource[]): DiscoveredResource[] {
    const seen = new Set<string>()
    return resources.filter((r) => {
      if (seen.has(r.name)) return false
      seen.add(r.name)
      return true
    })
  }

  /**
   * Attempt to acquire a resource
   */
  export async function acquire(resource: DiscoveredResource): Promise<boolean> {
    log.info("attempting to acquire resource", {
      type: resource.type,
      name: resource.name,
    })

    switch (resource.type) {
      case "mcp":
        return acquireMcpServer(resource)
      case "skill":
        return acquireSkill(resource)
      case "api":
        return acquireApi(resource)
      default:
        return false
    }
  }

  /**
   * Acquire an MCP server (returns install instructions)
   */
  async function acquireMcpServer(resource: DiscoveredResource): Promise<boolean> {
    // We can't automatically install MCP servers, but we can provide instructions
    log.info("MCP server acquisition", {
      name: resource.name,
      installHint: resource.installHint,
    })

    // In future, could automatically update config
    return false
  }

  /**
   * Acquire a skill (check if already exists)
   */
  async function acquireSkill(resource: DiscoveredResource): Promise<boolean> {
    const existing = await Skill.get(resource.name)
    return !!existing
  }

  /**
   * Acquire an API (check if configured)
   */
  async function acquireApi(resource: DiscoveredResource): Promise<boolean> {
    // Check if API key is configured
    const config = await Config.get()
    // This would need more specific logic per API
    return false
  }

  /**
   * Get acquisition instructions for a resource
   */
  export function getAcquisitionInstructions(resource: DiscoveredResource): string {
    switch (resource.type) {
      case "mcp":
        return resource.installHint ?? `Install MCP server: ${resource.source}`
      case "skill":
        return `Create skill "${resource.name}" using /crystallize or install from skill repository`
      case "api":
        return `Configure API credentials for ${resource.name} in environment variables`
      default:
        return `Acquire resource: ${resource.name}`
    }
  }
}
