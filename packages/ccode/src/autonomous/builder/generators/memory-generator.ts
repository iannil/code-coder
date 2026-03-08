/**
 * Memory Generator
 *
 * Generates MEMORY concepts - structured knowledge storage schemas.
 * Creates JSON Schema files for memory organization.
 *
 * Risk Level: MEDIUM
 * Auto-approvable: YES (schemas are declarative)
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

const log = Log.create({ service: "autonomous.builder.generators.memory" })

// ============================================================================
// Memory Generator
// ============================================================================

export class MemoryGenerator implements ConceptGenerator {
  readonly conceptType: ConceptType = "MEMORY"

  async generate(input: GeneratorInput): Promise<GeneratedConcept> {
    log.info("Generating MEMORY concept", {
      gapId: input.gap.id,
      description: input.gap.description.slice(0, 100),
    })

    // Generate unique identifier
    const identifier = this.generateIdentifier(input)

    // Generate JSON Schema for memory structure
    const schemaContent = await this.generateSchema(identifier, input)

    // Determine target path
    const targetPath = path.join(
      Global.Path.data,
      "memory",
      "schemas",
      `${identifier}.schema.json`,
    )

    return {
      type: "MEMORY",
      identifier,
      displayName: this.toDisplayName(identifier),
      description: `Memory schema for: ${input.gap.description.slice(0, 100)}`,
      content: schemaContent,
      targetPath,
      metadata: {
        generatedAt: Date.now(),
        generatedBy: "MemoryGenerator",
        version: "1.0.0",
        tags: this.extractTags(input),
      },
    }
  }

  async validateInput(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }> {
    const issues: string[] = []

    if (!input.gap.description || input.gap.description.length < 10) {
      issues.push("Gap description too short for memory schema generation")
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async generateSchema(identifier: string, input: GeneratorInput): Promise<string> {
    const displayName = this.toDisplayName(identifier)

    try {
      // Try to generate a more specific schema using LLM
      const schemaProperties = await this.inferSchemaProperties(input)

      const schema = {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        $id: identifier,
        title: displayName,
        description: input.gap.description,
        type: "object",
        properties: schemaProperties,
        required: this.inferRequiredFields(schemaProperties),
        additionalProperties: false,
      }

      return JSON.stringify(schema, null, 2)
    } catch (error) {
      log.warn("Schema inference failed, using fallback", { error })
      return this.generateFallbackSchema(identifier, input)
    }
  }

  private async inferSchemaProperties(input: GeneratorInput): Promise<Record<string, unknown>> {
    const llmSolver = getLLMSolver()

    // Ask LLM to suggest schema properties
    const result = await llmSolver.generateCode({
      problem: `Generate JSON Schema properties for a memory system that stores: ${input.gap.description}

Return ONLY a valid JSON object with property definitions.
Each property should have "type", "description", and optionally "format", "enum", or "items".

Example format:
{
  "id": { "type": "string", "description": "Unique identifier" },
  "name": { "type": "string", "description": "Human-readable name" },
  "createdAt": { "type": "number", "description": "Unix timestamp" }
}`,
      technology: "json",
      webSources: [],
      previousAttempts: [],
    })

    if (result) {
      try {
        // Extract JSON from response
        const jsonMatch = result.code.match(/\{[\s\S]*\}/)
        if (jsonMatch) {
          const parsed = JSON.parse(jsonMatch[0])
          // Validate it looks like schema properties
          if (typeof parsed === "object" && Object.keys(parsed).length > 0) {
            return parsed
          }
        }
      } catch {
        // Fall through to default
      }
    }

    // Default properties
    return this.getDefaultProperties(input)
  }

  private getDefaultProperties(input: GeneratorInput): Record<string, unknown> {
    const descLower = input.gap.description.toLowerCase()

    const properties: Record<string, unknown> = {
      id: {
        type: "string",
        description: "Unique identifier",
      },
      createdAt: {
        type: "number",
        description: "Creation timestamp (Unix milliseconds)",
      },
      updatedAt: {
        type: "number",
        description: "Last update timestamp (Unix milliseconds)",
      },
    }

    // Add contextual properties based on description
    if (descLower.includes("user") || descLower.includes("person")) {
      properties.userId = { type: "string", description: "Associated user ID" }
    }

    if (descLower.includes("session")) {
      properties.sessionId = { type: "string", description: "Session identifier" }
    }

    if (descLower.includes("tag") || descLower.includes("categor")) {
      properties.tags = {
        type: "array",
        items: { type: "string" },
        description: "Classification tags",
      }
    }

    if (descLower.includes("content") || descLower.includes("text") || descLower.includes("message")) {
      properties.content = { type: "string", description: "Main content" }
    }

    if (descLower.includes("score") || descLower.includes("rating")) {
      properties.score = { type: "number", description: "Numeric score" }
    }

    if (descLower.includes("status") || descLower.includes("state")) {
      properties.status = {
        type: "string",
        enum: ["pending", "active", "completed", "archived"],
        description: "Current status",
      }
    }

    // Add a data field for custom content
    properties.data = {
      type: "object",
      description: "Custom data payload",
      additionalProperties: true,
    }

    return properties
  }

  private inferRequiredFields(properties: Record<string, unknown>): string[] {
    const required: string[] = []

    // Always require id if present
    if ("id" in properties) required.push("id")
    if ("createdAt" in properties) required.push("createdAt")

    return required
  }

  private generateFallbackSchema(identifier: string, input: GeneratorInput): string {
    const displayName = this.toDisplayName(identifier)

    const schema = {
      $schema: "https://json-schema.org/draft/2020-12/schema",
      $id: identifier,
      title: displayName,
      description: input.gap.description,
      type: "object",
      properties: {
        id: {
          type: "string",
          description: "Unique identifier",
        },
        data: {
          type: "object",
          description: "Main data payload",
          additionalProperties: true,
        },
        metadata: {
          type: "object",
          description: "Additional metadata",
          properties: {
            createdAt: { type: "number" },
            updatedAt: { type: "number" },
            version: { type: "number" },
          },
        },
      },
      required: ["id"],
      additionalProperties: false,
    }

    return JSON.stringify(schema, null, 2)
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
      .filter((w) => w.length > 2 && !["the", "and", "for", "with"].includes(w))
      .slice(0, 3)

    const base = words.join("_") || "memory"
    return `${base}_${nanoid(6)}`
  }

  private toDisplayName(identifier: string): string {
    return identifier
      .split("_")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  }

  private extractTags(input: GeneratorInput): string[] {
    const tags: string[] = ["memory", "schema"]

    if (input.gap.technology) {
      tags.push(input.gap.technology.toLowerCase())
    }

    return [...new Set(tags)]
  }
}
