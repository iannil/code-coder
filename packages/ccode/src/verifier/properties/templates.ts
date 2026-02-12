/**
 * Property Templates
 *
 * Standard property-based testing templates for common algebraic properties.
 * Each template includes formal specification and test generation.
 */

import { z } from "zod"
import type { Property } from "../schema/functional-goal"

/**
 * Property category configuration
 */
export interface PropertyConfig {
  fn: string
  args?: string[]
  additionalFns?: Record<string, string>
}

/**
 * Generate all standard properties for a function
 */
export function generateStandardProperties(config: PropertyConfig): Property[] {
  const { fn, args = ["x", "y", "z"] } = config
  const properties: Property[] = []

  // Idempotency: f(f(x)) == f(x)
  properties.push({
    id: `PROP-IDEM-${fn}`,
    name: "idempotency",
    category: "algebraic",
    formal: `forall ${args[0]}. ${fn}(${fn}(${args[0]})) == ${fn}(${args[0]})`,
    verification: "property_test",
    priority: "standard",
  })

  // Associativity: f(f(x,y),z) == f(x,f(y,z))
  if (args.length >= 2) {
    properties.push({
      id: `PROP-ASSOC-${fn}`,
      name: "associativity",
      category: "algebraic",
      formal: `forall ${args.join(",")}. ${fn}(${fn}(${args[0]},${args[1]}),${args[2]}) == ${fn}(${args[0]},${fn}(${args[1]},${args[2]}))`,
      verification: "property_test",
      priority: "standard",
    })

    // Commutativity: f(x,y) == f(y,x)
    properties.push({
      id: `PROP-COMM-${fn}`,
      name: "commutativity",
      category: "algebraic",
      formal: `forall ${args[0]},${args[1]}. ${fn}(${args[0]},${args[1]}) == ${fn}(${args[1]},${args[0]})`,
      verification: "property_test",
      priority: "standard",
    })
  }

  return properties
}

/**
 * Round-trip property for encode/decode pairs
 */
export function roundTripProperty(
  encode: string,
  decode: string,
  inputType = "any",
): Property {
  return {
    id: `PROP-RT-${encode}-${decode}`,
    name: "round_trip",
    category: "relational",
    formal: `forall x: ${inputType}. ${decode}(${encode}(x)) == x`,
    verification: "property_test",
    priority: "critical",
  }
}

/**
 * Monotonicity property for ordered functions
 */
export function monotonicityProperty(fn: string, inputType = "number"): Property {
  return {
    id: `PROP-MONO-${fn}`,
    name: "monotonicity",
    category: "relational",
    formal: `forall x,y: ${inputType}. x <= y implies ${fn}(x) <= ${fn}(y)`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Identity element property
 */
export function identityProperty(
  fn: string,
  identityElement: string,
  inputType = "any",
): Property {
  return {
    id: `PROP-ID-${fn}`,
    name: "identity",
    category: "algebraic",
    formal: `forall x: ${inputType}. ${fn}(x,${identityElement}) == x && ${fn}(${identityElement},x) == x`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Closure property
 */
export function closureProperty(
  fn: string,
  setType: string,
): Property {
  return {
    id: `PROP-CLOSURE-${fn}`,
    name: "closure",
    category: "relational",
    formal: `forall x,y: ${setType}. ${fn}(x,y) in ${setType}`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Distributivity property
 */
export function distributivityProperty(
  outer: string,
  inner1: string,
  inner2: string,
): Property {
  return {
    id: `PROP-DIST-${outer}`,
    name: "distributivity",
    category: "algebraic",
    formal: `forall a,b,c. ${outer}(a,${inner1}(b,c)) == ${inner2}(${outer}(a,b),${outer}(a,c))`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Absorption property
 */
export function absorptionProperty(
  fn1: string,
  fn2: string,
): Property {
  return {
    id: `PROP-ABS-${fn1}`,
    name: "absorption",
    category: "algebraic",
    formal: `forall x,y. ${fn1}(x,${fn2}(x,y)) == x && ${fn2}(x,${fn1}(x,y)) == x`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Injective property (one-to-one)
 */
export function injectiveProperty(fn: string, inputType = "any"): Property {
  return {
    id: `PROP-INJ-${fn}`,
    name: "injective",
    category: "relational",
    formal: `forall x,y: ${inputType}. ${fn}(x) == ${fn}(y) implies x == y`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Surjective property (onto)
 */
export function surjectiveProperty(
  fn: string,
  inputType = "any",
  outputType = "any",
): Property {
  return {
    id: `PROP-SURJ-${fn}`,
    name: "surjective",
    category: "relational",
    formal: `forall y: ${outputType}. exists x: ${inputType}. ${fn}(x) == y`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Bijection property (both injective and surjective)
 */
export function bijectionProperty(fn: string, inputType = "any", outputType = "any"): Property {
  return {
    id: `PROP-BIJ-${fn}`,
    name: "bijection",
    category: "relational",
    formal: `forall x,y: ${inputType}. ${fn}(x) == ${fn}(y) iff x == y`,
    verification: "property_test",
    priority: "critical",
  }
}

/**
 * Property test generators
 */
export const PropertyGenerators = {
  /**
   * Generate test code for idempotency
   */
  idempotency: (fnName: string, examples: string[] = ["[]", '[1,2,3]', '["a","b","c"]']) => ({
    name: "idempotency",
    template: `describe("${fnName} idempotency", () => {
  ${examples.map(ex => `  it("should return same result when called twice on ${ex}", () => {
    const input = ${ex};
    const r1 = ${fnName}(input);
    const r2 = ${fnName}(r1);
    expect(r1).toEqual(r2);
  })`).join("\n\n")}
});`,
  }),

  /**
   * Generate test code for associativity
   */
  associativity: (fnName: string, args = ["a", "b", "c"]) => ({
    name: "associativity",
    template: `describe("${fnName} associativity", () => {
  it("should satisfy associativity: f(f(a,b),c) == f(a,f(b,c))", () => {
    const testCases = [
      { ${args.join(": ")}},
      { ${args.map((a, i) => `${a}: ${i + 1}`).join(", ")}},
      // Add more test cases
    ];

    for (const { ${args.join(", ")} } of testCases) {
      const left = ${fnName}(${fnName}(${args[0]}, ${args[1]}), ${args[2]});
      const right = ${fnName}(${args[0]}, ${fnName}(${args[1]}, ${args[2]}));
      expect(left).toEqual(right);
    }
  });
});`,
  }),

  /**
   * Generate test code for commutativity
   */
  commutative: (fnName: string, args = ["a", "b"]) => ({
    name: "commutativity",
    template: `describe("${fnName} commutativity", () => {
  it("should satisfy commutativity: f(a,b) == f(b,a)", () => {
    const testCases = [
      { ${args.join(": ")}},
      { ${args.map((a, i) => `${a}: ${i + 1}`).join(", ")}},
      // Add more test cases
    ];

    for (const { ${args.join(", ")} } of testCases) {
      const left = ${fnName}(${args[0]}, ${args[1]});
      const right = ${fnName}(${args[1]}, ${args[0]});
      expect(left).toEqual(right);
    }
  });
});`,
  }),

  /**
   * Generate test code for round-trip (encode/decode)
   */
  roundTrip: (encodeFn: string, decodeFn: string, varName = "input") => ({
    name: "round_trip",
    template: `describe("${encodeFn}/${decodeFn} round-trip", () => {
  it("should recover original input: decode(encode(x)) == x", () => {
    const testCases = [
      ${varName},
      // Add more test cases
    ];

    for (const ${varName} of testCases) {
      const encoded = ${encodeFn}(${varName});
      const decoded = ${decodeFn}(encoded);
      expect(decoded).toEqual(${varName});
    }
  });
});`,
  }),

  /**
   * Generate test code for monotonicity
   */
  monotonic: (fnName: string) => ({
    name: "monotonicity",
    template: `describe("${fnName} monotonicity", () => {
  it("should preserve order: x <= y implies f(x) <= f(y)", () => {
    const pairs = [
      [1, 2],
      [5, 10],
      [0, 100],
      // Add more test pairs
    ];

    for (const [x, y] of pairs) {
      if (x <= y) {
        const fx = ${fnName}(x);
        const fy = ${fnName}(y);
        expect(fx).toBeLessThanOrEqual(fy);
      }
    }
  });
});`,
  }),

  /**
   * Generate test code for identity element
   */
  identity: (fnName: string, identityVal: string, args = ["x", "id"]) => ({
    name: "identity",
    template: `describe("${fnName} identity", () => {
  const identity = ${identityVal};

  it("should satisfy identity: f(x,identity) == x", () => {
    const testCases = [
      ${args[0]},
      // Add more test cases
    ];

    for (const ${args[0]} of testCases) {
      const result1 = ${fnName}(${args[0]}, identity);
      const result2 = ${fnName}(identity, ${args[0]});
      expect(result1).toEqual(${args[0]});
      expect(result2).toEqual(${args[0]});
    }
  });
});`,
  }),
} as const

/**
 * Common property patterns for typical operations
 */
export const CommonPropertyPatterns = {
  // Array operations
  sort: ["idempotency", "monotonicity"],
  filter: ["idempotency"],
  map: ["functor_law"], // map(f) compose map(g) = map(f compose g)
  reverse: ["idempotency", "involution"], // reverse(reverse(x)) == x

  // Arithmetic operations
  add: ["associativity", "commutativity", "identity"],
  multiply: ["associativity", "commutativity", "identity"],
  subtract: ["identity"], // Not commutative, not associative
  divide: ["identity"], // Not commutative, not associative

  // Boolean operations
  and: ["associativity", "commutativity", "idempotency", "identity"],
  or: ["associativity", "commutativity", "idempotency", "identity"],
  xor: ["associativity", "commutativity", "identity"],
  not: ["involution"],

  // String operations
  trim: ["idempotency"],
  toUpperCase: ["idempotency"],
  toLowerCase: ["idempotency"],

  // Data structures
  stackPush: ["lifo"],
  queueEnqueue: ["fifo"],
} as const

/**
 * Functor law: map(id) == id
 */
export function functorLawIdentity(mapFn: string): Property {
  return {
    id: `PROP-FUNCTOR-ID-${mapFn}`,
    name: "functor_identity",
    category: "algebraic",
    formal: `${mapFn}(identity) == identity`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Functor composition law: map(f) compose map(g) = map(f compose g)
 */
export function functorLawComposition(mapFn: string): Property {
  return {
    id: `PROP-FUNCTOR-COMP-${mapFn}`,
    name: "functor_composition",
    category: "algebraic",
    formal: `${mapFn}(f) compose ${mapFn}(g) = ${mapFn}(f compose g)`,
    verification: "property_test",
    priority: "standard",
  }
}

/**
 * Monoid law: associativity and identity
 */
export function monoidLaws(op: string, identityElement: string): Property[] {
  return [
    {
      id: `PROP-MONOID-ASSOC-${op}`,
      name: "monoid_associativity",
      category: "algebraic",
      formal: `forall a,b,c. ${op}(a,${op}(b,c)) == ${op}(${op}(a,b),c)`,
      verification: "property_test",
      priority: "standard",
    },
    {
      id: `PROP-MONOID-ID-${op}`,
      name: "monoid_identity",
      category: "algebraic",
      formal: `forall a. ${op}(a,${identityElement}) == a && ${op}(${identityElement},a) == a`,
      verification: "property_test",
      priority: "standard",
    },
  ]
}
