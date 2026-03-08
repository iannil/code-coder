/**
 * Prompt Generator
 *
 * Generates PROMPT concepts - text templates for LLM interactions.
 * Creates structured prompt files with context and instructions.
 *
 * Risk Level: LOW
 * Auto-approvable: YES
 *
 * @package autonomous/builder/generators
 */

import { Log } from "@/util/log"
import { Global } from "@/util/global"
import path from "path"
import { nanoid } from "nanoid"

import type {
  ConceptType,
  ConceptGenerator,
  GeneratorInput,
  GeneratedConcept,
} from "../types"
import { getLLMSolver } from "../../execution/llm-solver"

const log = Log.create({ service: "autonomous.builder.generators.prompt" })

// ============================================================================
// Prompt Templates
// ============================================================================

const PROMPT_GENERATION_SYSTEM = `You are a prompt engineer creating structured prompts for AI assistants.

Generate a clear, effective prompt that:
1. Clearly states the task/role
2. Provides context and constraints
3. Includes relevant examples if helpful
4. Specifies the expected output format
5. Uses clear, unambiguous language

Output the prompt as plain text, ready to use.`

// ============================================================================
// Prompt Generator
// ============================================================================

export class PromptGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "PROMPT"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating PROMPT concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate prompt content using LLM
    const promptContent = await this.generatePromptContent(input)

    // Generate unique identifier
    const identifier = this.generateIdentifier(input)

    // Determine target path
    const targetPath = path.join(
      Global.Path.data,
      "prompts",
      `${identifier}.txt`,
    )

    return {
      type: "PROMPT",
      identifier,
      displayName: this.toDisplayName(identifier),
      description: `Prompt template for: ${input.gap.description.slice(0, 100)}`,
      content: promptContent,
      targetPath,
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "PromptGenerator",
        version: "1.0.0",
        tags: this.extractTags(input),
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 5) {
      issues.push("Gap description too short for prompt generation")
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async generatePromptContent(input: GeneratorInput): Promise<string> {
    const llmSolver = getLLMSolver()

    // Use the LLM to generate a well-structured prompt
    const result = await llmSolver.generateCode({
      problem: `Generate a prompt template for: ${input.gap.description}

The prompt should help an AI assistant to effectively ${input.gap.description}.

${input.context.taskDescription ? `Context: ${input.context.taskDescription}` : ""}
${input.gap.technology ? `Technology focus: ${input.gap.technology}` : ""}`,
      technology: "text",
      webSources: [],
      previousAttempts: [],
    })

    if (result) {
      // Clean up if it looks like code
      const content = result.code
        .replace(/^```\w*\n?/gm, "")
        .replace(/```$/gm, "")
        .trim()

      return this.formatPrompt(content, input)
    }

    // Fallback: generate a basic prompt template
    return this.generateFallbackPrompt(input)
  }

  private formatPrompt(content: string, input: GeneratorInput): string {
    const header = `# ${this.toDisplayName(this.generateIdentifier(input))}
# Generated: ${new Date().toISOString()}
# Purpose: ${input.gap.description.slice(0, 100)}

`
    return header + content
  }

  private generateFallbackPrompt(input: GeneratorInput): string {
    const description = input.gap.description

    return `# ${this.toDisplayName(this.generateIdentifier(input))}
# Generated: ${new Date().toISOString()}

## Context
You are an AI assistant helping with: ${description}

## Task
${description}

## Instructions
1. Understand the user's request carefully
2. Provide clear, accurate information
3. Follow best practices for the domain
4. Ask for clarification if needed

## Output Format
Provide a clear, structured response that directly addresses the task.

## Constraints
- Be concise but thorough
- Cite sources when applicable
- Highlight any assumptions made
`
  }

  private generateIdentifier(input: GeneratorInput): string {
    if (input.gap.suggestedName) {
      const normalized = input.gap.suggestedName
        .toLowerCase()
        .replace(/[^a-z0-9_]/g, "_")
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

    const base = words.join("_") || "prompt"
    return `${base}_${nanoid(6)}`
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("_")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private extractTags(input: GeneratorInput): string[] {
    const tags: string[] = ["prompt"]

    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    return [...new Set(tags)]
  }
}
