/**
 * Knowledge Module for BookWriter
 *
 * This module provides knowledge management capabilities for systematic book writing,
 * including knowledge graphs, argument chains, story arcs, and world frameworks.
 */

// ============================================================================
// Knowledge Module Schemas
// ============================================================================
// Export all types from schema
export * from "./schema"


// Re-export knowledge node operations
export * from "./node"

// Re-export framework operations
export * from "./framework"

// Re-export argument chain operations
export * from "./argument"

// Re-export story elements operations
export * from "./story"

// Convenience exports
import { KnowledgeNode } from "./node"
import { Framework } from "./framework"
import { ArgumentChain } from "./argument"
import { StoryElements } from "./story"

export const Knowledge = {
  Node: KnowledgeNode,
  Framework,
  ArgumentChain,
  StoryElements,
}
