/**
 * Hand Generator
 *
 * Generates HAND concepts - autonomous scheduled agents with execution permissions.
 * Creates HAND.md files following the established format.
 *
 * Risk Level: HIGH
 * Auto-approvable: NO (always requires human approval)
 *
 * @package autonomous/builder/generators
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Agent } from "@/agent/agent"
import path from "path"
import { nanoid } from "nanoid"

import type {
  ConceptType,
  ConceptGenerator,
  GeneratorInput,
  GeneratedConcept,
} from "../types"
import type { AutonomyLevel } from "../../decision/engine"

const log = Log.create({ service: "autonomous.builder.generators.hand" })

// ============================================================================
// Hand Generator
// ============================================================================

export class HandGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "HAND"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating HAND concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate unique identifier
    const identifier = this.generateIdentifier(input)

    // Infer configuration
    const agentName = await this.inferAgent(input)
    const schedule = this.inferSchedule(input)
    const autonomyLevel = this.inferAutonomyLevel(input)

    // Generate HAND.md content
    const handContent = this.generateHandContent({
      identifier,
      displayName: this.toDisplayName(identifier),
      description: input.gap.description,
      agent: agentName,
      schedule,
      autonomyLevel,
      technology: input.gap.technology,
    })

    // Determine target path
    const targetDir = path.join(Instance.worktree, ".codecoder", "hands", identifier)
    const targetPath = path.join(targetDir, "HAND.md")

    return {
      type: "HAND",
      identifier,
      displayName: this.toDisplayName(identifier),
      description: `Autonomous hand: ${input.gap.description.slice(0, 100)}`,
      content: handContent,
      targetPath,
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "HandGenerator",
        version: "1.0.0",
        tags: this.extractTags(input),
        dependencies: [agentName],
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 20) {
      issues.push("Gap description too short for hand generation (need at least 20 chars)")
    }

    // Verify at least one agent exists
    const agents = await Agent.list()
    if (agents.length === 0) {
      issues.push("No agents available - cannot create a hand without an agent")
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async inferAgent(input: GeneratorInput): Promise<string> {
    const agents = await Agent.list()
    const descLower = input.gap.description.toLowerCase()

    // Try to match based on description keywords
    for (const agent of agents) {
      const agentDesc = (agent.description ?? "").toLowerCase()
      const agentName = agent.name.toLowerCase()

      // Check for keyword overlap
      const keywords = descLower.split(/\s+/).filter((w) => w.length > 3)
      for (const keyword of keywords) {
        if (agentDesc.includes(keyword) || agentName.includes(keyword)) {
          return agent.name
        }
      }
    }

    // Default to build agent
    return "build"
  }

  private inferSchedule(input: GeneratorInput): string | undefined {
    const descLower = input.gap.description.toLowerCase()

    // Look for schedule indicators
    if (descLower.includes("hourly")) return "0 * * * *"
    if (descLower.includes("daily")) return "0 9 * * *"
    if (descLower.includes("weekly")) return "0 9 * * 1"
    if (descLower.includes("monthly")) return "0 9 1 * *"
    if (descLower.includes("every minute")) return "* * * * *"
    if (descLower.includes("every hour")) return "0 * * * *"

    // Check for specific time patterns
    const timeMatch = descLower.match(/at (\d{1,2}):?(\d{2})?\s*(am|pm)?/i)
    if (timeMatch) {
      let hour = parseInt(timeMatch[1], 10)
      const minute = timeMatch[2] ? parseInt(timeMatch[2], 10) : 0
      const ampm = timeMatch[3]?.toLowerCase()

      if (ampm === "pm" && hour < 12) hour += 12
      if (ampm === "am" && hour === 12) hour = 0

      return `${minute} ${hour} * * *`
    }

    // Default: no schedule (manual trigger only)
    return undefined
  }

  private inferAutonomyLevel(input: GeneratorInput): AutonomyLevel {
    const descLower = input.gap.description.toLowerCase()

    // Higher autonomy for routine tasks
    if (descLower.includes("routine") || descLower.includes("automatic")) {
      return "wild"
    }

    // Lower autonomy for critical tasks
    if (descLower.includes("critical") || descLower.includes("important")) {
      return "bold"
    }

    // Default: conservative autonomy
    return "timid"
  }

  private generateHandContent(params: {
    identifier: string
    displayName: string
    description: string
    agent: string
    schedule?: string
    autonomyLevel: AutonomyLevel
    technology?: string
  }): string {
    return `---
id: ${params.identifier}
name: ${params.displayName}
version: "1.0.0"
description: ${params.description}
${params.schedule ? `schedule: "${params.schedule}"` : "# schedule: \"0 9 * * *\"  # Uncomment to enable scheduling"}
agent: ${params.agent}
enabled: false  # Set to true to activate
---

# ${params.displayName}

${params.description}

## Purpose

This hand automates: ${params.description}

## Configuration

### Agent
Uses the \`${params.agent}\` agent for execution.

### Schedule
${params.schedule
  ? `Runs on schedule: \`${params.schedule}\` (cron format)`
  : "Manual trigger only. Uncomment the schedule line to enable automatic execution."}

### Autonomy
Level: \`${params.autonomyLevel}\`
- Maximum iterations: 10
- Unattended mode: false

## Parameters

You can customize this hand's behavior by adding parameters in the frontmatter:

\`\`\`yaml
params:
  key: value
\`\`\`

## Resource Limits

Default limits (customize in frontmatter):
- Max tokens: 100,000
- Max cost: $1.00 USD
- Max duration: 300 seconds

## Safety

This hand requires manual approval before activation.
Review the agent's capabilities and ensure the schedule is appropriate
before enabling.

## Example Usage

### Manual Trigger
\`\`\`bash
ccode hands trigger ${params.identifier}
\`\`\`

### Check Status
\`\`\`bash
ccode hands status ${params.identifier}
\`\`\`
`
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
      .filter((w) => w.length > 2 && !["the", "and", "for", "with"].includes(w))
      .slice(0, 3)

    const base = words.join("-") || "hand"
    return `${base}-${nanoid(6)}`
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("-")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private extractTags(input: GeneratorInput): string[] {
    const tags: string[] = ["hand", "autonomous"]

    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    const descLower = input.gap.description.toLowerCase()
    if (descLower.includes("schedule") || descLower.includes("cron")) {
      tags.push("scheduled")
    }
    if (descLower.includes("daily")) tags.push("daily")
    if (descLower.includes("hourly")) tags.push("hourly")

    return [...new Set(tags)]
  }
}
