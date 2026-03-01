/**
 * Autonomous Builder Module
 *
 * Enables CodeCoder to detect capability gaps and autonomously build
 * new concepts: AGENT, PROMPT, SKILL, TOOL, HAND, MEMORY, WORKFLOW.
 *
 * This is the "meta-capability" that allows the system to extend itself.
 *
 * @package autonomous/builder
 */

// Types
export {
  ConceptTypeSchema,
  ConceptRiskLevelSchema,
  GapEvidenceSchema,
  GapDetectionResultSchema,
  BuildContextSchema,
  BuildConstraintsSchema,
  BuildRequestSchema,
  GeneratorInputSchema,
  GeneratedConceptSchema,
  ValidationResultSchema,
  RegistrationResultSchema,
  BuildPhaseStatusSchema,
  BuildPhaseResultSchema,
  BuildResultSchema,
  CONCEPT_METADATA,
  AUTONOMY_CONCEPT_GATES,
  isConceptAllowed,
  getMinimumAutonomyLevel,
  createSelfBuildingCriteria,
} from "./types"

export type {
  ConceptType,
  ConceptRiskLevel,
  GapEvidence,
  GapDetectionResult,
  BuildContext,
  BuildConstraints,
  BuildRequest,
  GeneratorInput,
  GeneratedConcept,
  ConceptGenerator,
  ValidationResult,
  ConceptValidator,
  RegistrationResult,
  ConceptRegistrar,
  BuildPhaseStatus,
  BuildPhaseResult,
  BuildResult,
} from "./types"

// Concept Inventory
export {
  ConceptInventory,
  getConceptInventory,
  createConceptInventory,
  ConceptEntrySchema,
} from "./concept-inventory"
export type { ConceptEntry, SearchOptions, SearchResult } from "./concept-inventory"

// Gap Detection
export {
  GapDetector,
  getGapDetector,
  createGapDetector,
} from "./gap-detector"
export type {
  TaskFailure,
  FailurePattern,
  GapDetectorConfig,
} from "./gap-detector"

// Generators
export {
  getGenerator,
  generateConcept,
  registerGenerator,
  getRegisteredTypes,
  ToolGenerator,
  PromptGenerator,
  SkillGenerator,
  AgentGenerator,
  MemoryGenerator,
  HandGenerator,
  WorkflowGenerator,
} from "./generators"

// Validation
export {
  getValidator,
  validateConcept,
  ToolValidator,
  PromptValidator,
  SkillValidator,
  AgentValidator,
  MemoryValidator,
  HandValidator,
  WorkflowValidator,
} from "./validation"

// Registration
export {
  getRegistrar,
  registerConcept,
  unregisterConcept,
  ToolRegistrar,
  PromptRegistrar,
  SkillRegistrar,
  AgentRegistrar,
  MemoryRegistrar,
  HandRegistrar,
  WorkflowRegistrar,
} from "./registration"

// Meta Builder (Orchestrator)
export {
  MetaBuilder,
  getMetaBuilder,
  createMetaBuilder,
} from "./meta-builder"
export type { MetaBuilderConfig } from "./meta-builder"
