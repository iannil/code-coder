/**
 * Bootstrap Flywheel System
 *
 * The Agent Self-Bootstrap system enables agents to:
 * 1. **Awareness** - Introspect capabilities and limitations
 * 2. **Expansion** - Acquire new resources when needed
 * 3. **Creation** - Generate new skills from solutions
 * 4. **Verification** - Validate and evolve skill confidence
 *
 * This module provides the core infrastructure for the bootstrap flywheel.
 */

export { BootstrapTypes } from "./types"
export { CandidateStore } from "./candidate-store"
export { SelfAwareness } from "./awareness"
export { SkillGeneration } from "./generation"
export { ConfidenceSystem } from "./confidence"
export { ExecutionLoop } from "./verification"
export { PromptCompression } from "./compression"
export { CostTracker } from "./cost-tracker"
export { Triggers } from "./triggers"
export { BootstrapHooks } from "./hooks"
export { ResourceAcquisition } from "./acquisition"
