/**
 * Types and Schema Validation Tests
 */

import { describe, it, expect } from "bun:test"
import { MemoryEntrySchema, MemoryConfigSchema, DEFAULT_CONFIG } from "../src/types"

describe("MemoryEntrySchema", () => {
  it("should validate a valid entry", () => {
    const entry = {
      id: "123",
      key: "test_key",
      content: "test content",
      category: "preference",
      timestamp: "2024-01-01T00:00:00Z",
    }
    const result = MemoryEntrySchema.safeParse(entry)
    expect(result.success).toBe(true)
  })

  it("should validate entry with optional fields", () => {
    const entry = {
      id: "123",
      key: "test_key",
      content: "test content",
      category: "preference",
      timestamp: "2024-01-01T00:00:00Z",
      score: 0.95,
      source: "sqlite",
    }
    const result = MemoryEntrySchema.safeParse(entry)
    expect(result.success).toBe(true)
  })

  it("should reject entry with missing required fields", () => {
    const entry = {
      id: "123",
      key: "test_key",
      // missing content, category, timestamp
    }
    const result = MemoryEntrySchema.safeParse(entry)
    expect(result.success).toBe(false)
  })

  it("should validate markdown source", () => {
    const entry = {
      id: "123",
      key: "test_key",
      content: "test content",
      category: "preference",
      timestamp: "2024-01-01T00:00:00Z",
      source: "markdown",
    }
    const result = MemoryEntrySchema.safeParse(entry)
    expect(result.success).toBe(true)
  })

  it("should reject invalid source", () => {
    const entry = {
      id: "123",
      key: "test_key",
      content: "test content",
      category: "preference",
      timestamp: "2024-01-01T00:00:00Z",
      source: "invalid",
    }
    const result = MemoryEntrySchema.safeParse(entry)
    expect(result.success).toBe(false)
  })
})

describe("MemoryConfigSchema", () => {
  it("should validate sqlite backend config", () => {
    const config = {
      backend: "sqlite",
      sqlite: {
        dbPath: "/path/to/db.sqlite",
        vectorWeight: 0.7,
        keywordWeight: 0.3,
      },
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(true)
  })

  it("should validate markdown backend config", () => {
    const config = {
      backend: "markdown",
      markdown: {
        basePath: "./memory",
        longTermFile: "MEMORY.md",
        dailyDir: "daily",
      },
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(true)
  })

  it("should validate composite backend config", () => {
    const config = {
      backend: "composite",
      sqlite: { dbPath: "/path/to/db.sqlite" },
      markdown: { basePath: "./memory" },
      composite: {
        primary: "sqlite",
        writeToAll: true,
        conflictStrategy: "primary-wins",
      },
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(true)
  })

  it("should reject invalid backend type", () => {
    const config = {
      backend: "invalid",
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(false)
  })

  it("should reject vectorWeight outside 0-1 range", () => {
    const config = {
      backend: "sqlite",
      sqlite: {
        vectorWeight: 1.5,
      },
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(false)
  })

  it("should reject negative embeddingCacheSize", () => {
    const config = {
      backend: "sqlite",
      sqlite: {
        embeddingCacheSize: -100,
      },
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(false)
  })

  it("should reject invalid conflict strategy", () => {
    const config = {
      backend: "composite",
      composite: {
        conflictStrategy: "invalid-strategy",
      },
    }
    const result = MemoryConfigSchema.safeParse(config)
    expect(result.success).toBe(false)
  })
})

describe("DEFAULT_CONFIG", () => {
  it("should have valid backend", () => {
    expect(DEFAULT_CONFIG.backend).toBe("sqlite")
  })

  it("should have valid sqlite config", () => {
    expect(DEFAULT_CONFIG.sqlite.vectorWeight).toBe(0.7)
    expect(DEFAULT_CONFIG.sqlite.keywordWeight).toBe(0.3)
    expect(DEFAULT_CONFIG.sqlite.embeddingCacheSize).toBe(10000)
    expect(DEFAULT_CONFIG.sqlite.readOnly).toBe(false)
  })

  it("should have valid markdown config", () => {
    expect(DEFAULT_CONFIG.markdown.basePath).toBe("./memory")
    expect(DEFAULT_CONFIG.markdown.longTermFile).toBe("MEMORY.md")
    expect(DEFAULT_CONFIG.markdown.dailyDir).toBe("daily")
  })

  it("should have valid composite config", () => {
    expect(DEFAULT_CONFIG.composite.primary).toBe("sqlite")
    expect(DEFAULT_CONFIG.composite.writeToAll).toBe(true)
    expect(DEFAULT_CONFIG.composite.conflictStrategy).toBe("primary-wins")
  })

  it("should pass schema validation", () => {
    const result = MemoryConfigSchema.safeParse(DEFAULT_CONFIG)
    expect(result.success).toBe(true)
  })
})
