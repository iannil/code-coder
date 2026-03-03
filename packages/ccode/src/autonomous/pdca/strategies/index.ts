/**
 * PDCA Strategies Index
 *
 * Exports all acceptance strategies and provides a factory
 * for creating the appropriate strategy based on task type.
 */

import type { TaskType } from "../../classification/types"
import type { AcceptanceStrategy } from "./base"

// Re-export base types
export { BaseAcceptanceStrategy } from "./base"
export type { AcceptanceStrategy, StrategyContext } from "./base"

// Re-export strategy implementations
export { ImplementationStrategy, createImplementationStrategy } from "./implementation"
export { ResearchStrategy, createResearchStrategy } from "./research"
export { QueryStrategy, createQueryStrategy } from "./query"
export { GenericStrategy, createGenericStrategy } from "./generic"

// ============================================================================
// Strategy Factory
// ============================================================================

/**
 * Create the appropriate acceptance strategy for a task type.
 *
 * @param taskType - The type of task to create a strategy for
 * @returns An acceptance strategy instance
 */
export function createStrategy(taskType: TaskType): AcceptanceStrategy {
  switch (taskType) {
    case "implementation":
      // Lazy import to avoid circular dependencies
      const { createImplementationStrategy } = require("./implementation")
      return createImplementationStrategy()

    case "research":
      const { createResearchStrategy } = require("./research")
      return createResearchStrategy()

    case "query":
      const { createQueryStrategy } = require("./query")
      return createQueryStrategy()

    case "acceptance":
    case "fix":
    case "other":
    default:
      const { createGenericStrategy } = require("./generic")
      return createGenericStrategy(taskType)
  }
}

/**
 * Strategy Factory class for more control over strategy creation.
 */
export class StrategyFactory {
  private static cache = new Map<TaskType, AcceptanceStrategy>()

  /**
   * Create a strategy for the given task type.
   * Uses caching by default to reuse strategy instances.
   *
   * @param taskType - The task type
   * @param useCache - Whether to use cached instances (default: true)
   * @returns An acceptance strategy
   */
  static create(taskType: TaskType, useCache = true): AcceptanceStrategy {
    if (useCache && this.cache.has(taskType)) {
      return this.cache.get(taskType)!
    }

    const strategy = createStrategy(taskType)

    if (useCache) {
      this.cache.set(taskType, strategy)
    }

    return strategy
  }

  /**
   * Clear the strategy cache.
   */
  static clearCache(): void {
    this.cache.clear()
  }

  /**
   * Check if a specific strategy type is supported.
   */
  static isSupported(taskType: TaskType): boolean {
    return ["implementation", "research", "query", "acceptance", "fix", "other"].includes(taskType)
  }

  /**
   * Get all supported task types.
   */
  static getSupportedTypes(): TaskType[] {
    return ["implementation", "research", "query", "acceptance", "fix", "other"]
  }
}
