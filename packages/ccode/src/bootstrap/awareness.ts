import { Log } from "@/util/log"
import { Agent } from "@/agent/agent"
import { ToolRegistry } from "@/tool/registry"
import { Skill } from "@/skill/skill"
import { Config } from "@/config/config"
import { BootstrapTypes } from "./types"

const log = Log.create({ service: "bootstrap.awareness" })

/**
 * SelfAwareness module enables agents to introspect their capabilities
 * and understand what they can and cannot do.
 */
export namespace SelfAwareness {
  /**
   * Introspect the current agent's capabilities
   */
  export async function introspect(agentName?: string): Promise<BootstrapTypes.AgentCapabilities> {
    const agent = agentName ? await Agent.get(agentName) : await Agent.get("build")

    if (!agent) {
      log.warn("agent not found, using defaults", { agentName })
      return {
        name: agentName ?? "unknown",
        tools: [],
        skills: [],
        mcpServers: [],
        permissions: {},
      }
    }

    // Get available tools
    const toolIds = await ToolRegistry.ids()

    // Get available skills
    const skills = await Skill.all()
    const skillNames = skills.map((s) => s.name)

    // Get MCP servers from config
    const config = await Config.get()
    const mcpServers = Object.keys(config.mcp ?? {})

    // Extract permission summary
    const permissions: Record<string, boolean> = {}
    for (const rule of agent.permission) {
      if (rule.pattern === "*") {
        permissions[rule.permission] = rule.action !== "deny"
      }
    }

    const result: BootstrapTypes.AgentCapabilities = {
      name: agent.name,
      description: agent.description,
      tools: toolIds,
      skills: skillNames,
      mcpServers,
      permissions,
      model: agent.model,
    }

    log.info("introspected capabilities", {
      agent: agent.name,
      toolCount: toolIds.length,
      skillCount: skillNames.length,
      mcpCount: mcpServers.length,
    })

    return result
  }

  /**
   * Check if the agent can handle a specific task
   * Returns confidence level and any missing capabilities
   */
  export async function canHandle(
    task: string,
    agentName?: string,
  ): Promise<BootstrapTypes.CanHandleResult> {
    const capabilities = await introspect(agentName)

    // Analyze task requirements (simple heuristic-based approach)
    const analysis = analyzeTaskRequirements(task)

    const missingCapabilities: string[] = []
    const suggestedResources: string[] = []
    let confidence = 0.8 // Start with reasonable confidence

    // Check for tool requirements
    for (const requiredTool of analysis.requiredTools) {
      if (!capabilities.tools.includes(requiredTool)) {
        missingCapabilities.push(`tool:${requiredTool}`)
        confidence -= 0.15
      }
    }

    // Check for skill requirements
    for (const requiredSkill of analysis.requiredSkills) {
      if (!capabilities.skills.some((s) => s.toLowerCase().includes(requiredSkill.toLowerCase()))) {
        missingCapabilities.push(`skill:${requiredSkill}`)
        suggestedResources.push(`skill:${requiredSkill}`)
        confidence -= 0.1
      }
    }

    // Check for MCP requirements
    for (const requiredMcp of analysis.requiredMcp) {
      if (!capabilities.mcpServers.some((m) => m.toLowerCase().includes(requiredMcp.toLowerCase()))) {
        missingCapabilities.push(`mcp:${requiredMcp}`)
        suggestedResources.push(`mcp:${requiredMcp}`)
        confidence -= 0.1
      }
    }

    // Check for domain expertise indicators
    if (analysis.domainIndicators.length > 0) {
      // Reduce confidence if dealing with specialized domains
      confidence -= 0.05 * analysis.domainIndicators.length
    }

    confidence = Math.max(0, Math.min(1, confidence))
    const confident = confidence >= 0.6

    log.info("assessed task capability", {
      task: task.slice(0, 100),
      confident,
      confidence,
      missingCount: missingCapabilities.length,
    })

    return {
      confident,
      confidence,
      missingCapabilities: missingCapabilities.length > 0 ? missingCapabilities : undefined,
      suggestedResources: suggestedResources.length > 0 ? suggestedResources : undefined,
    }
  }

  /**
   * Analyze a task string to identify required capabilities
   */
  function analyzeTaskRequirements(task: string): {
    requiredTools: string[]
    requiredSkills: string[]
    requiredMcp: string[]
    domainIndicators: string[]
  } {
    const taskLower = task.toLowerCase()
    const requiredTools: string[] = []
    const requiredSkills: string[] = []
    const requiredMcp: string[] = []
    const domainIndicators: string[] = []

    // Tool indicators
    const toolPatterns: Record<string, string[]> = {
      bash: ["run", "execute", "command", "shell", "terminal", "script"],
      read: ["read", "view", "show", "display", "content"],
      edit: ["edit", "modify", "change", "update", "fix"],
      write: ["write", "create", "generate", "make"],
      grep: ["search", "find", "look for", "grep"],
      glob: ["files", "pattern", "glob", "list"],
      websearch: ["search web", "google", "look up", "research"],
      webfetch: ["fetch", "download", "url", "website"],
    }

    for (const [tool, patterns] of Object.entries(toolPatterns)) {
      if (patterns.some((p) => taskLower.includes(p))) {
        requiredTools.push(tool)
      }
    }

    // Skill/domain indicators
    const skillPatterns: Record<string, string[]> = {
      tdd: ["test", "tdd", "testing", "unit test", "coverage"],
      security: ["security", "vulnerability", "auth", "secure"],
      review: ["review", "code review", "check"],
      architect: ["architecture", "design", "system", "structure"],
      debugging: ["debug", "fix bug", "error", "issue"],
      database: ["database", "sql", "query", "postgres", "mysql"],
      api: ["api", "rest", "graphql", "endpoint"],
      frontend: ["frontend", "react", "vue", "ui", "component"],
      backend: ["backend", "server", "node", "express"],
    }

    for (const [skill, patterns] of Object.entries(skillPatterns)) {
      if (patterns.some((p) => taskLower.includes(p))) {
        requiredSkills.push(skill)
      }
    }

    // MCP indicators
    const mcpPatterns: Record<string, string[]> = {
      github: ["github", "gh", "pull request", "pr", "issue"],
      slack: ["slack", "message", "channel"],
      jira: ["jira", "ticket", "story"],
      filesystem: ["file system", "directory", "folder"],
      browser: ["browser", "playwright", "selenium", "web page"],
    }

    for (const [mcp, patterns] of Object.entries(mcpPatterns)) {
      if (patterns.some((p) => taskLower.includes(p))) {
        requiredMcp.push(mcp)
      }
    }

    // Domain expertise indicators
    const domainPatterns: string[] = [
      "machine learning",
      "ai",
      "neural",
      "blockchain",
      "crypto",
      "kubernetes",
      "k8s",
      "devops",
      "aws",
      "gcp",
      "azure",
      "terraform",
      "mobile",
      "ios",
      "android",
    ]

    for (const domain of domainPatterns) {
      if (taskLower.includes(domain)) {
        domainIndicators.push(domain)
      }
    }

    return {
      requiredTools,
      requiredSkills,
      requiredMcp,
      domainIndicators,
    }
  }

  /**
   * Get a summary of what the agent knows and doesn't know
   */
  export async function getSummary(agentName?: string): Promise<string> {
    const capabilities = await introspect(agentName)

    const lines = [
      `## Agent: ${capabilities.name}`,
      "",
      capabilities.description ? `${capabilities.description}` : "",
      "",
      `### Tools (${capabilities.tools.length})`,
      capabilities.tools.slice(0, 10).join(", ") + (capabilities.tools.length > 10 ? "..." : ""),
      "",
      `### Skills (${capabilities.skills.length})`,
      capabilities.skills.slice(0, 10).join(", ") + (capabilities.skills.length > 10 ? "..." : ""),
      "",
      `### MCP Servers (${capabilities.mcpServers.length})`,
      capabilities.mcpServers.join(", ") || "None configured",
    ]

    return lines.filter(Boolean).join("\n")
  }
}
