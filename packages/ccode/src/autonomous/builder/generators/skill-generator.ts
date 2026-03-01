/**
 * Skill Generator
 *
 * Generates SKILL concepts - user-invocable capabilities with structured prompts.
 * Creates SKILL.md files following the established format.
 *
 * Risk Level: LOW
 * Auto-approvable: YES (when content < 1000 chars)
 *
 * @package autonomous/builder/generators
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import path from "path"
import { nanoid } from "nanoid"

import type {
  ConceptType,
  ConceptGenerator,
  GeneratorInput,
  GeneratedConcept,
} from "../types"
import { getLLMSolver } from "../../execution/llm-solver"

const log = Log.create({ service: "autonomous.builder.generators.skill" })

// ============================================================================
// Skill Generator
// ============================================================================

export class SkillGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "SKILL"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating SKILL concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate unique identifier
    const identifier = this.generateIdentifier(input)

    // Generate skill content
    const skillContent = await this.generateSkillContent(identifier, input)

    // Determine target path (project-level .codecoder/skills/)
    const targetDir = path.join(Instance.worktree, ".codecoder", "skills", identifier)
    const targetPath = path.join(targetDir, "SKILL.md")

    return {
      type: "SKILL",
      identifier,
      displayName: this.toDisplayName(identifier),
      description: this.generateDescription(input),
      content: skillContent,
      targetPath,
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "SkillGenerator",
        version: "1.0.0",
        tags: this.extractTags(input),
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 10) {
      issues.push("Gap description too short for skill generation")
    }

    if (input.existingConcepts.includes(input.gap.suggestedName ?? "")) {
      issues.push(`Skill with identifier "${input.gap.suggestedName}" already exists`)
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async generateSkillContent(identifier: string, input: GeneratorInput): Promise<string> {
    const displayName = this.toDisplayName(identifier)
    const description = this.generateDescription(input)

    // Generate the instruction body using LLM
    const instructionBody = await this.generateInstructionBody(input)

    // Build SKILL.md content with frontmatter
    return `---
name: ${identifier}
description: ${description}
---

<command-name>${identifier}</command-name>

# ${displayName}

${description}

## Instructions

${instructionBody}

## Output

Provide clear, actionable output that directly addresses the user's request.

## Examples

### Example 1
\`\`\`
/${identifier}
\`\`\`

Executes the skill with default parameters.
`
  }

  private async generateInstructionBody(input: GeneratorInput): Promise<string> {
    const llmSolver = getLLMSolver()

    const result = await llmSolver.generateCode({
      problem: `Generate clear instructions for an AI skill that: ${input.gap.description}

The instructions should:
1. Clearly explain what the skill does
2. Describe the step-by-step process
3. Include any important constraints or guidelines
4. Be written in imperative form (e.g., "Analyze the code", "Generate a report")

Output just the instruction text, no code formatting.`,
      technology: "text",
      webSources: [],
      previousAttempts: [],
    })

    if (result) {
      return result.code
        .replace(/^```\w*\n?/gm, "")
        .replace(/```$/gm, "")
        .trim()
    }

    // Fallback instructions
    return `When this skill is invoked:

1. Understand the user's intent based on their request
2. ${input.gap.description}
3. Provide clear, structured output
4. Offer to clarify or expand on any aspect if needed

Follow best practices and maintain consistency with project conventions.`
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

    const base = words.join("-") || "skill"
    return `${base}-${nanoid(6)}`
  }

  private generateDescription(input: GeneratorInput): string {
    const baseDesc = input.gap.description
      .replace(/^need to /i, "")
      .replace(/^enable /i, "")

    // Ensure it's a reasonable description length
    if (baseDesc.length > 200) {
      return baseDesc.slice(0, 197) + "..."
    }

    return baseDesc.charAt(0).toUpperCase() + baseDesc.slice(1)
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("-")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private extractTags(input: GeneratorInput): string[] {
    const tags: string[] = ["skill"]

    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    // Extract action tags
    const descLower = input.gap.description.toLowerCase()
    const actionTags = ["analyze", "generate", "review", "create", "update", "delete"]

    for (const tag of actionTags) {
      if (descLower.includes(tag)) {
        tags.push(tag)
      }
    }

    return [...new Set(tags)]
  }
}
