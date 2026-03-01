import { describe, expect, it } from "bun:test"
import { Slug } from "../src/slug"

describe("Slug", () => {
  describe("create", () => {
    it("should create a slug with format adjective-noun", () => {
      const slug = Slug.create()
      expect(slug).toMatch(/^[a-z]+-[a-z]+$/)
    })

    it("should create different slugs on subsequent calls", () => {
      const slugs = new Set<string>()
      // Generate 50 slugs - with 30 adjectives and 31 nouns (930 combinations)
      // there's a high probability of uniqueness
      for (let i = 0; i < 50; i++) {
        slugs.add(Slug.create())
      }
      // Most should be unique (allowing some collisions due to randomness)
      expect(slugs.size).toBeGreaterThan(40)
    })

    it("should only contain lowercase letters and hyphens", () => {
      for (let i = 0; i < 20; i++) {
        const slug = Slug.create()
        expect(slug).toMatch(/^[a-z-]+$/)
      }
    })

    it("should have exactly one hyphen", () => {
      for (let i = 0; i < 20; i++) {
        const slug = Slug.create()
        const hyphens = slug.split("-").length - 1
        expect(hyphens).toBe(1)
      }
    })
  })
})
