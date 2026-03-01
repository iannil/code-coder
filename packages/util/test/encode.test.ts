import { describe, expect, it } from "bun:test"
import { base64Encode, base64Decode, hash, checksum } from "../src/encode"

describe("base64Encode", () => {
  it("should encode a simple string", () => {
    const result = base64Encode("hello")
    expect(result).toBe("aGVsbG8")
  })

  it("should produce URL-safe base64", () => {
    const result = base64Encode("hello+world/test")
    expect(result).not.toContain("+")
    expect(result).not.toContain("/")
    expect(result).not.toContain("=")
  })

  it("should encode unicode characters", () => {
    const result = base64Encode("你好")
    expect(result).toBe("5L2g5aW9")
  })

  it("should handle empty string", () => {
    const result = base64Encode("")
    expect(result).toBe("")
  })
})

describe("base64Decode", () => {
  it("should decode a simple string", () => {
    const result = base64Decode("aGVsbG8")
    expect(result).toBe("hello")
  })

  it("should decode URL-safe base64", () => {
    const encoded = base64Encode("hello+world/test")
    const decoded = base64Decode(encoded)
    expect(decoded).toBe("hello+world/test")
  })

  it("should decode unicode characters", () => {
    const result = base64Decode("5L2g5aW9")
    expect(result).toBe("你好")
  })

  it("should be inverse of encode", () => {
    const original = "Test string with special chars: @#$%"
    const encoded = base64Encode(original)
    const decoded = base64Decode(encoded)
    expect(decoded).toBe(original)
  })
})

describe("hash", () => {
  it("should produce SHA-256 hash by default", async () => {
    const result = await hash("hello")
    expect(result).toBe("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
  })

  it("should produce consistent hashes", async () => {
    const hash1 = await hash("test")
    const hash2 = await hash("test")
    expect(hash1).toBe(hash2)
  })

  it("should produce different hashes for different inputs", async () => {
    const hash1 = await hash("hello")
    const hash2 = await hash("world")
    expect(hash1).not.toBe(hash2)
  })

  it("should handle empty string", async () => {
    const result = await hash("")
    expect(result).toBe("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
  })
})

describe("checksum", () => {
  it("should produce a consistent checksum", () => {
    const result1 = checksum("hello")
    const result2 = checksum("hello")
    expect(result1).toBe(result2)
  })

  it("should produce different checksums for different inputs", () => {
    const check1 = checksum("hello")
    const check2 = checksum("world")
    expect(check1).not.toBe(check2)
  })

  it("should return undefined for empty string", () => {
    const result = checksum("")
    expect(result).toBeUndefined()
  })

  it("should return a base36 string", () => {
    const result = checksum("test")!
    expect(result).toMatch(/^[0-9a-z]+$/)
  })
})
