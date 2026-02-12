/**
 * Contract Schema
 *
 * Defines Design by Contract (DbC) specifications for functions and modules.
 * Includes preconditions, postconditions, and invariants.
 */

import { z } from "zod"

/**
 * Variable reference in a contract
 */
export const VariableRefSchema = z.object({
  name: z.string(),
  type: z.string().optional().describe("Type annotation if available"),
  description: z.string().optional(),
})

export type VariableRef = z.infer<typeof VariableRefSchema>

/**
 * Logical expression
 */
export const LogicalExprSchema = z.object({
  expr: z.string().describe("The logical expression"),
  language: z.enum(["typescript", "python", "pseudo", "formal"]).default("pseudo"),
  explanation: z.string().optional().describe("Natural language explanation"),
})

export type LogicalExpr = z.infer<typeof LogicalExprSchema>

/**
 * Contract specification
 */
export const ContractSchema = z.object({
  id: z.string(),
  subject: z.string().describe("Function or module being specified"),

  // Inputs
  inputs: z.array(VariableRefSchema),

  // Outputs
  outputs: z.array(VariableRefSchema),

  // Preconditions: must be true before calling
  requires: z.array(z.object({
    id: z.string(),
    expression: LogicalExprSchema,
    violation: z.string().describe("What happens if precondition is violated"),
  })),

  // Postconditions: guaranteed to be true after return
  ensures: z.array(z.object({
    id: z.string(),
    expression: LogicalExprSchema,
    dependencies: z.array(z.string()).describe("IDs of preconditions this depends on"),
  })),

  // Invariants: must always hold for the module
  maintains: z.array(z.object({
    id: z.string(),
    expression: LogicalExprSchema,
    scope: z.enum(["function", "module", "system"]).default("function"),
  })),
})

export type Contract = z.infer<typeof ContractSchema>

/**
 * Function contract with signature
 */
export const FunctionContractSchema = ContractSchema.extend({
  kind: z.literal("function"),
  signature: z.string().describe("Function signature, e.g., (x: number) => number"),
})

export type FunctionContract = z.infer<typeof FunctionContractSchema>

/**
 * Module contract with exported interface
 */
export const ModuleContractSchema = ContractSchema.extend({
  kind: z.literal("module"),
  exports: z.array(z.object({
    name: z.string(),
    type: z.enum(["function", "class", "constant", "type"]),
  })),
})

export type ModuleContract = z.infer<typeof ModuleContractSchema>

/**
 * Assertion - runtime check of a contract condition
 */
export const AssertionSchema = z.object({
  contractId: z.string(),
  conditionId: z.string(),
  condition: z.enum(["pre", "post", "invariant"]),
  expression: z.string(),
  enabled: z.boolean().default(true),
  action: z.enum(["throw", "log", "ignore"]).default("throw"),
})

export type Assertion = z.infer<typeof AssertionSchema>

/**
 * Create a function contract
 */
export function createFunctionContract(config: {
  id: string
  function: string
  signature: string
  inputs?: Array<{ name: string; type?: string; description?: string }>
  outputs?: Array<{ name: string; type?: string; description?: string }>
}): FunctionContract {
  return {
    id: config.id,
    kind: "function",
    subject: config.function,
    signature: config.signature,
    inputs: config.inputs ?? [],
    outputs: config.outputs ?? [],
    requires: [],
    ensures: [],
    maintains: [],
  }
}

/**
 * Create a module contract
 */
export function createModuleContract(config: {
  id: string
  module: string
  exports?: Array<{ name: string; type: "function" | "class" | "constant" | "type" }>
}): ModuleContract {
  return {
    id: config.id,
    kind: "module",
    subject: config.module,
    exports: config.exports ?? [],
    inputs: [],
    outputs: [],
    requires: [],
    ensures: [],
    maintains: [],
  }
}

/**
 * Common contract templates
 */
export const ContractTemplates = {
  arrayNotEmpty: (arrVar = "arr"): LogicalExpr => ({
    expr: `${arrVar}.length > 0`,
    language: "typescript",
    explanation: "Array must contain at least one element",
  }),

  positiveNumber: (numVar = "n"): LogicalExpr => ({
    expr: `${numVar} > 0`,
    language: "typescript",
    explanation: "Number must be positive",
  }),

  nonNull: (varName = "x"): LogicalExpr => ({
    expr: `${varName} !== null && ${varName} !== undefined`,
    language: "typescript",
    explanation: "Value must not be null or undefined",
  }),

  range: (varName = "x", min = 0, max = 100): LogicalExpr => ({
    expr: `${varName} >= ${min} && ${varName} <= ${max}`,
    language: "typescript",
    explanation: `Value must be between ${min} and ${max} (inclusive)`,
  }),

  validEmail: (emailVar = "email"): LogicalExpr => ({
    expr: `/^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/.test(${emailVar})`,
    language: "typescript",
    explanation: "String must be a valid email address",
  }),

  returnsNonnull: (retVar = "result"): LogicalExpr => ({
    expr: `${retVar} !== null && ${retVar} !== undefined`,
    language: "typescript",
    explanation: "Function must return a non-null value",
  }),

  preservesLength: (arrVar = "arr", retVar = "result"): LogicalExpr => ({
    expr: `${retVar}.length === ${arrVar}.length`,
    language: "typescript",
    explanation: "Output array has same length as input array",
  }),

  sorted: (arrVar = "arr"): LogicalExpr => ({
    expr: `forall i in [0, ${arrVar}.length - 2]: ${arrVar}[i] <= ${arrVar}[i + 1]`,
    language: "pseudo",
    explanation: "Array is sorted in non-decreasing order",
  }),
} as const
