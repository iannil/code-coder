/**
 * Invariant Patterns
 *
 * Common invariant patterns that apply to various types of functions and modules.
 * These serve as templates for generating invariants during verification.
 */

import { z } from "zod"
import type { Invariant } from "../schema/functional-goal"

/**
 * Invariant pattern category
 */
export type InvariantCategory =
  | "data_structure"
  | "algorithm"
  | "state_machine"
  | "resource"
  | "security"
  | "performance"

/**
 * Invariant pattern template
 */
export interface InvariantPattern {
  id: string
  name: string
  category: InvariantCategory
  description: string
  formal: string
  applicableTo: string[]
  generate: (context: InvariantContext) => Invariant
}

/**
 * Context for generating invariants
 */
export interface InvariantContext {
  functionName?: string
  className?: string
  moduleName?: string
  types?: Record<string, string>
  variables?: string[]
}

/**
 * Common invariant patterns
 */
export const InvariantPatterns: InvariantPattern[] = [
  // Data structure invariants
  {
    id: "sorted_order",
    name: "Sorted Order Invariant",
    category: "data_structure",
    description: "Sorted array maintains order between adjacent elements",
    formal: "forall arr: sorted(arr) implies forall i: arr[i] <= arr[i+1]",
    applicableTo: ["sort", "sorted", "insert_sorted", "merge_sorted"],
    generate: (ctx) => ({
      id: `INV-SORTED-${Date.now()}`,
      name: "sorted_order",
      statement: "Array elements are in non-decreasing order",
      formal: "forall i in [0, length-2]: arr[i] <= arr[i+1]",
      scope: "function",
      violation: "throws Error('Array not sorted')",
    }),
  },

  {
    id: "array_bounds",
    name: "Array Bounds Invariant",
    category: "data_structure",
    description: "All array accesses are within bounds",
    formal: "forall arr, i: access(arr, i) implies 0 <= i < length(arr)",
    applicableTo: ["array", "list", "vector"],
    generate: (ctx) => ({
      id: `INV-BOUNDS-${Date.now()}`,
      name: "array_bounds",
      statement: "All array accesses are within valid bounds",
      formal: "forall i: 0 <= i && i < arr.length",
      scope: "function",
      violation: "throws RangeError('Index out of bounds')",
    }),
  },

  {
    id: "unique_elements",
    name: "Unique Elements Invariant",
    category: "data_structure",
    description: "Set contains unique elements only",
    formal: "forall set: size(set) == size(to_list(set))",
    applicableTo: ["set", "unique", "deduplicate"],
    generate: (ctx) => ({
      id: `INV-UNIQUE-${Date.now()}`,
      name: "unique_elements",
      statement: "All elements in the collection are unique",
      formal: "forall i,j: i != j implies arr[i] != arr[j]",
      scope: "function",
      violation: "throws Error('Duplicate element found')",
    }),
  },

  // State machine invariants
  {
    id: "valid_state",
    name: "Valid State Invariant",
    category: "state_machine",
    description: "State machine always in a valid state",
    formal: "forall sm: state(sm) in valid_states(sm)",
    applicableTo: ["state", "fsm", "machine"],
    generate: (ctx) => ({
      id: `INV-STATE-${Date.now()}`,
      name: "valid_state",
      statement: "State machine is always in a valid state",
      formal: "currentState in validStates",
      scope: "module",
      violation: "throws Error('Invalid state')",
    }),
  },

  {
    id: "state_transition",
    name: "State Transition Invariant",
    category: "state_machine",
    description: "State transitions follow defined edges",
    formal: "forall sm, s1, s2: transition(sm, s1, s2) implies (s1, s2) in transitions(sm)",
    applicableTo: ["transition", "next_state", "advance"],
    generate: (ctx) => ({
      id: `INV-TRANS-${Date.now()}`,
      name: "state_transition",
      statement: "All state transitions are defined",
      formal: "transitionTable[currentState][event] in validStates",
      scope: "module",
      violation: "throws Error('Invalid transition')",
    }),
  },

  // Resource invariants
  {
    id: "resource_balance",
    name: "Resource Balance Invariant",
    category: "resource",
    description: "Acquired resources are eventually released",
    formal: "forall r: acquire(r) implies eventually release(r)",
    applicableTo: ["file", "connection", "lock", "mutex"],
    generate: (ctx) => ({
      id: `INV-RES-BAL-${Date.now()}`,
      name: "resource_balance",
      statement: "Every acquire has a matching release",
      formal: "acquiredCount == releasedCount",
      scope: "module",
      violation: "throws Error('Resource leak detected')",
    }),
  },

  {
    id: "no_leak",
    name: "No Leak Invariant",
    category: "resource",
    description: "Memory/resources are not leaked",
    formal: "forall t: allocated(t) - freed(t) <= max_leak",
    applicableTo: ["memory", "buffer", "allocation"],
    generate: (ctx) => ({
      id: `INV-LEAK-${Date.now()}`,
      name: "no_leak",
      statement: "No resource leaks occur",
      formal: "currentUsage <= initialUsage + maxAllowedGrowth",
      scope: "module",
      violation: "throws Error('Resource leak detected')",
    }),
  },

  // Security invariants
  {
    id: "type_safety",
    name: "Type Safety Invariant",
    category: "security",
    description: "Operations maintain type correctness",
    formal: "forall x: typeof(x) == expected_type(x)",
    applicableTo: ["parser", "validator", "input"],
    generate: (ctx) => ({
      id: `INV-TYPE-${Date.now()}`,
      name: "type_safety",
      statement: "All values have expected types",
      formal: "forall x: x instanceof ExpectedType",
      scope: "function",
      violation: "throws TypeError('Type mismatch')",
    }),
  },

  {
    id: "no_null_dereference",
    name: "No Null Dereference Invariant",
    category: "security",
    description: "No null/undefined values are dereferenced",
    formal: "forall x: dereference(x) implies x != null && x != undefined",
    applicableTo: ["access", "property", "method"],
    generate: (ctx) => ({
      id: `INV-NULL-${Date.now()}`,
      name: "no_null_dereference",
      statement: "No null or undefined values are accessed",
      formal: "x !== null && x !== undefined before x.property",
      scope: "function",
      violation: "throws Error('Null dereference')",
    }),
  },

  // Performance invariants
  {
    id: "time_bound",
    name: "Time Bound Invariant",
    category: "performance",
    description: "Operations complete within time limit",
    formal: "forall input: duration(operation(input)) <= max_time",
    applicableTo: ["process", "compute", "execute"],
    generate: (ctx) => ({
      id: `INV-TIME-${Date.now()}`,
      name: "time_bound",
      statement: "Operation completes within time bound",
      formal: "executionTime <= maxAllowedTime",
      scope: "function",
      violation: "throws Error('Timeout exceeded')",
    }),
  },

  {
    id: "memory_bound",
    name: "Memory Bound Invariant",
    category: "performance",
    description: "Memory usage stays within bound",
    formal: "forall t: memory_usage(t) <= max_memory",
    applicableTo: ["process", "buffer", "cache"],
    generate: (ctx) => ({
      id: `INV-MEM-${Date.now()}`,
      name: "memory_bound",
      statement: "Memory usage stays within bound",
      formal: "currentMemoryUsage <= maxMemoryLimit",
      scope: "module",
      violation: "throws Error('Memory limit exceeded')",
    }),
  },

  // Algorithm invariants (for loops)
  {
    id: "loop_invariant",
    name: "Loop Invariant",
    category: "algorithm",
    description: "Property that holds before, during, and after each iteration",
    formal: "forall i: invariant(i) holds before and after iteration i",
    applicableTo: ["while", "for", "loop", "iterate"],
    generate: (ctx) => ({
      id: `INV-LOOP-${Date.now()}`,
      name: "loop_invariant",
      statement: "Loop invariant holds at each iteration",
      formal: "P(0) && forall k: P(k) implies P(k+1)",
      scope: "function",
      violation: "throws Error('Loop invariant violated')",
    }),
  },

  // Cache invariants
  {
    id: "cache_consistency",
    name: "Cache Consistency Invariant",
    category: "data_structure",
    description: "Cache values match source when valid",
    formal: "forall k, v: cache.get(k) == v implies v == source.get(k) || cache.isDirty(k)",
    applicableTo: ["cache", "memo", "store"],
    generate: (ctx) => ({
      id: `INV-CACHE-${Date.now()}`,
      name: "cache_consistency",
      statement: "Cache values are consistent with source",
      formal: "isValid(key) implies cache[key] == source[key]",
      scope: "module",
      violation: "throws Error('Cache inconsistency detected')",
    }),
  },

  // Collection size invariants
  {
    id: "size_preservation",
    name: "Size Preservation Invariant",
    category: "data_structure",
    description: "Non-modifying operations preserve collection size",
    formal: "forall coll, op: isReadOnly(op) implies size(op(coll)) == size(coll)",
    applicableTo: ["filter", "map", "find", "search"],
    generate: (ctx) => ({
      id: `INV-SIZE-${Date.now()}`,
      name: "size_preservation",
      statement: "Read-only operations preserve size",
      formal: "result.length == input.length for non-modifying ops",
      scope: "function",
      violation: "throws Error('Size unexpectedly changed')",
    }),
  },
]

/**
 * Find applicable invariant patterns for a given function/module
 */
export function findApplicablePatterns(
  name: string,
  category?: InvariantCategory,
): InvariantPattern[] {
  const lowerName = name.toLowerCase()

  return InvariantPatterns.filter((pattern) => {
    // Filter by category if specified
    if (category && pattern.category !== category) return false

    // Check if any applicable pattern matches
    return pattern.applicableTo.some((applicable) => lowerName.includes(applicable))
  })
}

/**
 * Generate invariants for a function
 */
export function generateInvariants(
  functionName: string,
  context?: Partial<InvariantContext>,
): Invariant[] {
  const patterns = findApplicablePatterns(functionName)

  const fullContext: InvariantContext = {
    functionName,
    ...context,
  }

  return patterns.map((pattern) => pattern.generate(fullContext))
}

/**
 * Common invariant templates by category
 */
export const InvariantByCategory: Record<
  InvariantCategory,
  InvariantPattern[]
> = {
  data_structure: InvariantPatterns.filter((p) => p.category === "data_structure"),
  algorithm: InvariantPatterns.filter((p) => p.category === "algorithm"),
  state_machine: InvariantPatterns.filter((p) => p.category === "state_machine"),
  resource: InvariantPatterns.filter((p) => p.category === "resource"),
  security: InvariantPatterns.filter((p) => p.category === "security"),
  performance: InvariantPatterns.filter((p) => p.category === "performance"),
}

/**
 * Create a custom invariant
 */
export function createCustomInvariant(config: {
  name: string
  statement: string
  formal: string
  scope: "function" | "module" | "system"
  violation?: string
}): Invariant {
  return {
    id: `INV-CUSTOM-${Date.now()}`,
    name: config.name,
    statement: config.statement,
    formal: config.formal,
    scope: config.scope,
    violation: config.violation ?? "throws Error('Invariant violated')",
  }
}
