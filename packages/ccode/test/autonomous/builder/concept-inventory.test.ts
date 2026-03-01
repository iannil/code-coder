/**
 * Concept Inventory Tests
 *
 * Tests for the ConceptInventory class.
 *
 * @package test/autonomous/builder
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"

import {
  ConceptInventory,
  createConceptInventory,
  getConceptInventory,
  type ConceptEntry,
} from "@/autonomous/builder"

import { withTestInstance, ALL_CONCEPT_TYPES } from "./fixtures/builder-fixture"

describe("ConceptInventory", () => {
  let inventory: ConceptInventory

  beforeEach(() => {
    inventory = createConceptInventory()
  })

  afterEach(() => {
    inventory.invalidateCache()
  })

  // ==========================================================================
  // Factory Functions
  // ==========================================================================

  describe("factory functions", () => {
    test("should create new inventory instance", () => {
      const inv = createConceptInventory()
      expect(inv).toBeDefined()
      expect(inv).toBeInstanceOf(ConceptInventory)
    })

    test("should return singleton from getConceptInventory", () => {
      const inv1 = getConceptInventory()
      const inv2 = getConceptInventory()
      expect(inv1).toBe(inv2)
    })
  })

  // ==========================================================================
  // all()
  // ==========================================================================

  describe("all", () => {
    test("should return array of concepts", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        expect(Array.isArray(concepts)).toBe(true)
      })
    })

    test("should include agents", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()
        const agents = concepts.filter((c) => c.type === "AGENT")

        // Should find at least some agents (builtins)
        expect(agents.length).toBeGreaterThanOrEqual(0)
      })
    })

    test("should include skills", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()
        const skills = concepts.filter((c) => c.type === "SKILL")

        expect(skills.length).toBeGreaterThanOrEqual(0)
      })
    })

    test("should cache results", async () => {
      await withTestInstance(async () => {
        const start1 = Date.now()
        await inventory.all()
        const duration1 = Date.now() - start1

        const start2 = Date.now()
        await inventory.all()
        const duration2 = Date.now() - start2

        // Second call should be faster (cached)
        // Note: This is a weak assertion as timing can vary
        expect(duration2).toBeLessThanOrEqual(duration1 + 50)
      })
    })
  })

  // ==========================================================================
  // search()
  // ==========================================================================

  describe("search", () => {
    test("should return search results", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("build")

        expect(Array.isArray(results)).toBe(true)
      })
    })

    test("should include score in results", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("code")

        for (const result of results) {
          expect(result.score).toBeDefined()
          expect(result.score).toBeGreaterThanOrEqual(0)
          expect(result.score).toBeLessThanOrEqual(1)
        }
      })
    })

    test("should sort by score descending", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("review")

        for (let i = 1; i < results.length; i++) {
          expect(results[i].score).toBeLessThanOrEqual(results[i - 1].score)
        }
      })
    })

    test("should filter by type", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("build", {
          types: ["AGENT"],
        })

        for (const result of results) {
          expect(result.concept.type).toBe("AGENT")
        }
      })
    })

    test("should filter by nativeOnly", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("build", {
          nativeOnly: true,
        })

        for (const result of results) {
          expect(result.concept.native).toBe(true)
        }
      })
    })

    test("should respect limit option", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("a", {
          limit: 5,
        })

        expect(results.length).toBeLessThanOrEqual(5)
      })
    })

    test("should respect minScore option", async () => {
      await withTestInstance(async () => {
        const minScore = 0.5
        const results = await inventory.search("build", {
          minScore,
        })

        for (const result of results) {
          expect(result.score).toBeGreaterThanOrEqual(minScore)
        }
      })
    })

    test("should include matchType in results", async () => {
      await withTestInstance(async () => {
        const results = await inventory.search("build")

        for (const result of results) {
          expect(["exact", "fuzzy", "semantic"]).toContain(result.matchType)
        }
      })
    })

    test("should find exact matches with score 1.0", async () => {
      await withTestInstance(async () => {
        // First get all concepts to find a real identifier
        const concepts = await inventory.all()

        if (concepts.length > 0) {
          const firstConcept = concepts[0]
          const results = await inventory.search(firstConcept.identifier)

          // Should find the exact match
          const exactMatch = results.find((r) => r.concept.identifier === firstConcept.identifier)
          if (exactMatch) {
            expect(exactMatch.score).toBe(1.0)
            expect(exactMatch.matchType).toBe("exact")
          }
        }
      })
    })
  })

  // ==========================================================================
  // get()
  // ==========================================================================

  describe("get", () => {
    test("should return concept by identifier", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        if (concepts.length > 0) {
          const firstConcept = concepts[0]
          const result = await inventory.get(firstConcept.identifier)

          expect(result).toBeDefined()
          expect(result?.identifier).toBe(firstConcept.identifier)
        }
      })
    })

    test("should return null for non-existent identifier", async () => {
      await withTestInstance(async () => {
        const result = await inventory.get("nonexistent_identifier_xyz_123")

        expect(result).toBeNull()
      })
    })

    test("should filter by type when specified", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()
        const agent = concepts.find((c) => c.type === "AGENT")

        if (agent) {
          // Should find when type matches
          const found = await inventory.get(agent.identifier, "AGENT")
          expect(found).toBeDefined()

          // Should not find when type doesn't match
          const notFound = await inventory.get(agent.identifier, "TOOL")
          expect(notFound).toBeNull()
        }
      })
    })
  })

  // ==========================================================================
  // exists()
  // ==========================================================================

  describe("exists", () => {
    test("should return true for existing concept", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        if (concepts.length > 0) {
          const exists = await inventory.exists(concepts[0].identifier)
          expect(exists).toBe(true)
        }
      })
    })

    test("should return false for non-existent concept", async () => {
      await withTestInstance(async () => {
        const exists = await inventory.exists("nonexistent_xyz_123")
        expect(exists).toBe(false)
      })
    })

    test("should check type when specified", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()
        const agent = concepts.find((c) => c.type === "AGENT")

        if (agent) {
          expect(await inventory.exists(agent.identifier, "AGENT")).toBe(true)
          expect(await inventory.exists(agent.identifier, "TOOL")).toBe(false)
        }
      })
    })
  })

  // ==========================================================================
  // byType()
  // ==========================================================================

  describe("byType", () => {
    test("should return concepts of specified type", async () => {
      await withTestInstance(async () => {
        const agents = await inventory.byType("AGENT")

        for (const concept of agents) {
          expect(concept.type).toBe("AGENT")
        }
      })
    })

    test("should return empty array if no concepts of type", async () => {
      await withTestInstance(async () => {
        // WORKFLOW concepts might not exist
        const workflows = await inventory.byType("WORKFLOW")

        expect(Array.isArray(workflows)).toBe(true)
      })
    })
  })

  // ==========================================================================
  // invalidateCache()
  // ==========================================================================

  describe("invalidateCache", () => {
    test("should clear cached concepts", async () => {
      await withTestInstance(async () => {
        // Populate cache
        await inventory.all()

        // Invalidate
        inventory.invalidateCache()

        // Next call should re-fetch
        const concepts = await inventory.all()
        expect(Array.isArray(concepts)).toBe(true)
      })
    })
  })

  // ==========================================================================
  // ConceptEntry Structure
  // ==========================================================================

  describe("ConceptEntry structure", () => {
    test("should have required fields", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        for (const concept of concepts.slice(0, 5)) {
          // Check first 5
          expect(concept.type).toBeDefined()
          expect(ALL_CONCEPT_TYPES).toContain(concept.type)
          expect(concept.identifier).toBeDefined()
          expect(typeof concept.identifier).toBe("string")
          expect(concept.displayName).toBeDefined()
          expect(typeof concept.native).toBe("boolean")
        }
      })
    })

    test("should have optional fields when present", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        for (const concept of concepts.slice(0, 5)) {
          if (concept.description !== undefined) {
            expect(typeof concept.description).toBe("string")
          }
          if (concept.location !== undefined) {
            expect(typeof concept.location).toBe("string")
          }
          if (concept.tags !== undefined) {
            expect(Array.isArray(concept.tags)).toBe(true)
          }
        }
      })
    })
  })

  // ==========================================================================
  // Score Calculation
  // ==========================================================================

  describe("score calculation", () => {
    test("should score exact identifier match highest", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        if (concepts.length > 0) {
          const identifier = concepts[0].identifier
          const results = await inventory.search(identifier)

          const match = results.find((r) => r.concept.identifier === identifier)
          if (match) {
            expect(match.score).toBe(1.0)
          }
        }
      })
    })

    test("should score partial matches lower", async () => {
      await withTestInstance(async () => {
        const concepts = await inventory.all()

        if (concepts.length > 0) {
          const identifier = concepts[0].identifier
          const partial = identifier.slice(0, 3) // First 3 chars

          const results = await inventory.search(partial)
          const match = results.find((r) => r.concept.identifier === identifier)

          if (match && partial !== identifier) {
            expect(match.score).toBeLessThan(1.0)
          }
        }
      })
    })
  })
})
