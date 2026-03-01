/**
 * Concept Generators Registry
 *
 * Central registry for all concept generators.
 * Each generator implements the ConceptGenerator interface
 * and handles a specific concept type.
 *
 * @package autonomous/builder/generators
 */

import type { ConceptType, ConceptGenerator, GeneratorInput, GeneratedConcept } from "../types"
import { ToolGenerator } from "./tool-generator"
import { PromptGenerator } from "./prompt-generator"
import { SkillGenerator } from "./skill-generator"
import { AgentGenerator } from "./agent-generator"
import { MemoryGenerator } from "./memory-generator"
import { HandGenerator } from "./hand-generator"
import { WorkflowGenerator } from "./workflow-generator"

// ============================================================================
// Generator Registry
// ============================================================================

/**
 * Map of concept types to their generators
 */
const generators = new Map<ConceptType, ConceptGenerator>()

// Initialize generators
generators.set("TOOL", new ToolGenerator())
generators.set("PROMPT", new PromptGenerator())
generators.set("SKILL", new SkillGenerator())
generators.set("AGENT", new AgentGenerator())
generators.set("MEMORY", new MemoryGenerator())
generators.set("HAND", new HandGenerator())
generators.set("WORKFLOW", new WorkflowGenerator())

/**
 * Get the generator for a concept type
 */
export function getGenerator(type: ConceptType): ConceptGenerator {
  const generator = generators.get(type)
  if (!generator) {
    throw new Error(`No generator registered for concept type: ${type}`)
  }
  return generator
}

/**
 * Generate a concept using the appropriate generator
 */
export async function generateConcept(
  type: ConceptType,
  input: GeneratorInput,
): Promise<GeneratedConcept> {
  const generator = getGenerator(type)
  return generator.generate(input)
}

/**
 * Register a custom generator
 */
export function registerGenerator(type: ConceptType, generator: ConceptGenerator): void {
  generators.set(type, generator)
}

/**
 * Get all registered generator types
 */
export function getRegisteredTypes(): ConceptType[] {
  return Array.from(generators.keys())
}

// ============================================================================
// Re-exports
// ============================================================================

export { ToolGenerator } from "./tool-generator"
export { PromptGenerator } from "./prompt-generator"
export { SkillGenerator } from "./skill-generator"
export { AgentGenerator } from "./agent-generator"
export { MemoryGenerator } from "./memory-generator"
export { HandGenerator } from "./hand-generator"
export { WorkflowGenerator } from "./workflow-generator"
