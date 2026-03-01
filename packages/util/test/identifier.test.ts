import { describe, expect, it } from "bun:test"
import { Identifier } from "../src/identifier"

describe("Identifier", () => {
  describe("create", () => {
    it("should create a 26-character identifier", () => {
      const id = Identifier.create(false)
      expect(id).toHaveLength(26)
    })

    it("should create unique identifiers", () => {
      const ids = new Set<string>()
      for (let i = 0; i < 100; i++) {
        ids.add(Identifier.create(false))
      }
      expect(ids.size).toBe(100)
    })

    it("should create ascending identifiers when descending=false", () => {
      const id1 = Identifier.create(false)
      const id2 = Identifier.create(false)
      expect(id1 < id2).toBe(true)
    })

    it("should create descending identifiers when descending=true", () => {
      const id1 = Identifier.create(true)
      const id2 = Identifier.create(true)
      expect(id1 > id2).toBe(true)
    })

    it("should respect provided timestamp", () => {
      // Note: With the same timestamp, counter increments, so time portion differs slightly
      const timestamp = Date.now()
      const id1 = Identifier.create(false, timestamp)
      const id2 = Identifier.create(false, timestamp)
      // First 8 chars (timestamp portion without counter) should be similar
      expect(id1.slice(0, 8)).toBe(id2.slice(0, 8))
      // But full IDs should differ due to counter and random suffix
      expect(id1).not.toBe(id2)
    })
  })

  describe("ascending", () => {
    it("should create ascending identifiers", () => {
      const id1 = Identifier.ascending()
      const id2 = Identifier.ascending()
      expect(id1 < id2).toBe(true)
    })
  })

  describe("descending", () => {
    it("should create descending identifiers", () => {
      const id1 = Identifier.descending()
      const id2 = Identifier.descending()
      expect(id1 > id2).toBe(true)
    })
  })
})
