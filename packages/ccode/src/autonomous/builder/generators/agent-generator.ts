/**
 * Agent Generator
 *
 * Generates AGENT concepts - specialized AI personas with unique behaviors.
 * Extends the existing Agent.generate() pattern.
 *
 * Risk Level: MEDIUM
 * Auto-approvable: NO (always requires human approval)
 *
 * @package autonomous/builder/generators
 */

import { Log } from "@/util/log"
import { Global } from "@/global"
import { Agent } from "@/agent/agent"
import { Provider } from "@/provider/provider"
import { SystemPrompt } from "@/session/system"
import { generateObject, type ModelMessage } from "ai"
import path from "path"
import { nanoid } from "nanoid"
import z from "zod"

import type {
  ConceptType,
  ConceptGenerator,
  GeneratorInput,
  GeneratedConcept,
} from "../types"

const log = Log.create({ service: "autonomous.builder.generators.agent" })

// ============================================================================
// Agent Generator
// ============================================================================

export class AgentGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "AGENT"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating AGENT concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate agent configuration using LLM
    const agentConfig = await this.generateAgentConfig(input)

    // Generate unique identifier
    const identifier = this.sanitizeIdentifier(agentConfig.identifier)

    // Build agent JSON configuration
    const agentJson = this.buildAgentJson(identifier, agentConfig, input)

    // Determine target paths
    const agentJsonPath = path.join(Global.Path.data, "agents", `${identifier}.json`)
    const promptPath = path.join(Global.Path.data, "agents", "prompts", `${identifier}.txt`)

    return {
      type: "AGENT",
      identifier,
      displayName: this.toDisplayName(identifier),
      description: agentConfig.whenToUse,
      content: JSON.stringify(agentJson, null, 2),
      targetPath: agentJsonPath,
      additionalFiles: [
        {
          path: promptPath,
          content: agentConfig.systemPrompt,
        },
      ],
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "AgentGenerator",
        version: "1.0.0",
        tags: this.extractTags(input, agentConfig),
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 20) {
      issues.push("Gap description too short for agent generation (need at least 20 chars)")
    }

    // Check for existing agents
    const existingAgents = await Agent.list()
    const suggestedName = input.gap.suggestedName?.toLowerCase()

    if (suggestedName && existingAgents.some((a) => a.name.toLowerCase() === suggestedName)) {
      issues.push(`Agent with name "${input.gap.suggestedName}" already exists`)
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async generateAgentConfig(input: GeneratorInput): Promise<{
    identifier: string
    whenToUse: string
    systemPrompt: string
  }> {
    try {
      // Try using the existing Agent.generate pattern
      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const existingAgents = await Agent.list()
      const existingNames = existingAgents.map((a) => a.name).join(", ")

      const system = SystemPrompt.header(defaultModel.providerID)
      system.push(`You are creating a new AI agent configuration.

An agent is a specialized AI persona with specific capabilities and behaviors.

Generate an agent that:
1. Has a clear, focused purpose
2. Has a descriptive system prompt that guides its behavior
3. Has a unique identifier (lowercase, hyphenated)
4. Describes when it should be used

The agent should be designed to: ${input.gap.description}

${input.gap.technology ? `Technology focus: ${input.gap.technology}` : ""}
${input.context.taskDescription ? `Additional context: ${input.context.taskDescription}` : ""}`)

      const result = await generateObject({
        temperature: 0.3,
        messages: [
          ...system.map((item): ModelMessage => ({
            role: "system",
            content: item,
          })),
          {
            role: "user",
            content: `Create an agent configuration for: "${input.gap.description}"

IMPORTANT: These identifiers already exist and must NOT be used: ${existingNames}

Return ONLY the JSON object.`,
          },
        ],
        model: language,
        schema: z.object({
          identifier: z.string().describe("Unique identifier (lowercase-hyphenated)"),
          whenToUse: z.string().describe("Description of when to use this agent"),
          systemPrompt: z.string().describe("System prompt that defines agent behavior"),
        }),
      })

      return result.object
    } catch (error) {
      log.warn("LLM generation failed, using fallback", { error })
      return this.generateFallbackConfig(input)
    }
  }

  private generateFallbackConfig(input: GeneratorInput): {
    identifier: string
    whenToUse: string
    systemPrompt: string
  } {
    const identifier = this.generateIdentifier(input)
    const displayName = this.toDisplayName(identifier)

    return {
      identifier,
      whenToUse: `Use this agent for ${input.gap.description}`,
      systemPrompt: `You are ${displayName}, an AI assistant specialized in:

${input.gap.description}

## Your Role

You are an expert in this domain. Your responsibilities include:
1. Understanding user requests related to your specialty
2. Providing accurate, helpful guidance
3. Following best practices for the domain
4. Being thorough but concise

## Guidelines

- Always clarify requirements when they are ambiguous
- Provide examples when helpful
- Cite sources or reasoning when making recommendations
- Ask for additional context if needed

## Constraints

- Stay focused on your area of expertise
- Be honest about limitations
- Do not make up information
`,
    }
  }

  private buildAgentJson(
    identifier: string,
    config: { identifier: string; whenToUse: string; systemPrompt: string },
    input: GeneratorInput,
  ): Record<string, unknown> {
    return {
      name: identifier,
      description: config.whenToUse,
      mode: "subagent",
      native: false,
      hidden: false,
      promptFile: `prompts/${identifier}.txt`,
      permission: {
        "*": "allow",
      },
      options: {},
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "AgentGenerator",
        gapId: input.gap.id,
        technology: input.gap.technology,
      },
    }
  }

  private generateIdentifier(input: GeneratorInput): string {
    if (input.gap.suggestedName) {
      const normalized = input.gap.suggestedName
        .toLowerCase()
        .replace(/[^a-z0-9-]/g, "-")
        .replace(/--+/g, "-")
        .slice(0, 40)

      if (!input.existingConcepts.includes(normalized)) {
        return normalized
      }
    }

    const words = input.gap.description
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 2)
      .slice(0, 3)

    const base = words.join("-") || "agent"
    return `${base}-${nanoid(6)}`
  }

  private sanitizeIdentifier(identifier: string): string {
    return identifier
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, "-")
      .replace(/--+/g, "-")
      .replace(/^-|-$/g, "")
      .slice(0, 40)
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("-")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private extractTags(
    input: GeneratorInput,
    config: { identifier: string; whenToUse: string; systemPrompt: string },
  ): string[] {
    const tags: string[] = ["agent", "subagent"]

    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    // Extract domain tags from description
    const domainKeywords = [
      "code", "review", "security", "test", "analyze", "generate",
      "document", "debug", "refactor", "design", "architect",
    ]
    const descLower = (config.whenToUse + " " + input.gap.description).toLowerCase()

    for (const keyword of domainKeywords) {
      if (descLower.includes(keyword)) {
        tags.push(keyword)
      }
    }

    return [...new Set(tags)]
  }
}
