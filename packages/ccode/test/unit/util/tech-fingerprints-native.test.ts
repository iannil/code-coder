import { describe, it, expect, beforeAll } from "bun:test"
import {
  isNativeAvailable,
  isUsingNative,
  detectWebTechnologiesNative,
  getWebFingerprintsNative,
  getWebCategoriesNative,
  detectWebTechnologies,
  getWebCategories,
  type WebTechDetection,
} from "../../../src/util/tech-fingerprints-native"

describe("tech-fingerprints-native", () => {
  describe("Native availability", () => {
    it("should check if native is available", async () => {
      const available = await isNativeAvailable()
      // May or may not be available depending on build
      expect(typeof available).toBe("boolean")
    })

    it("should have sync isUsingNative after async check", async () => {
      await isNativeAvailable()
      const using = isUsingNative()
      expect(typeof using).toBe("boolean")
    })
  })

  describe("Hybrid API", () => {
    it("should detect React in content", async () => {
      const results = await detectWebTechnologies({
        content: "data-reactroot __react__ React.createElement",
      })

      expect(results.size).toBeGreaterThan(0)
      expect(results.has("React")).toBe(true)
    })

    it("should detect Next.js in content", async () => {
      const results = await detectWebTechnologies({
        content: '<script id="__NEXT_DATA__" type="application/json">',
      })

      expect(results.has("Next.js")).toBe(true)
    })

    it("should detect multiple technologies", async () => {
      const results = await detectWebTechnologies({
        content: `
          data-reactroot
          __NEXT_DATA__
          tailwindcss
          @tanstack/react-query
          gtag(
          googletagmanager.com
        `,
      })

      expect(results.has("React")).toBe(true)
      expect(results.has("Next.js")).toBe(true)
      expect(results.has("Tailwind CSS")).toBe(true)
      expect(results.has("TanStack Query")).toBe(true)
      expect(results.has("Google Analytics")).toBe(true)
    })

    it("should return all categories", async () => {
      const categories = await getWebCategories()

      expect(categories.length).toBeGreaterThan(0)
      expect(categories).toContain("frontend")
      expect(categories).toContain("ui")
      expect(categories).toContain("state")
      expect(categories).toContain("build")
      expect(categories).toContain("analytics")
      expect(categories).toContain("auth")
      expect(categories).toContain("payment")
    })
  })

  describe("Native API (if available)", () => {
    let nativeAvailable: boolean

    beforeAll(async () => {
      nativeAvailable = await isNativeAvailable()
    })

    it("should detect technologies natively", async () => {
      if (!nativeAvailable) return

      const results = await detectWebTechnologiesNative({
        content: "data-reactroot",
      })

      expect(results).not.toBeNull()
      if (results) {
        expect(results.length).toBeGreaterThan(0)
        expect(results.some((d: WebTechDetection) => d.name === "React")).toBe(true)
      }
    })

    it("should get fingerprints natively", async () => {
      if (!nativeAvailable) return

      const fps = await getWebFingerprintsNative()

      expect(fps).not.toBeNull()
      if (fps) {
        expect(fps.length).toBeGreaterThan(0)
        expect(fps.some((fp) => fp.name === "React")).toBe(true)
      }
    })

    it("should get categories natively", async () => {
      if (!nativeAvailable) return

      const cats = await getWebCategoriesNative()

      expect(cats).not.toBeNull()
      if (cats) {
        expect(cats.length).toBeGreaterThan(0)
        expect(cats).toContain("frontend")
      }
    })
  })

  describe("Edge cases", () => {
    it("should handle empty content", async () => {
      const results = await detectWebTechnologies({
        content: "",
      })

      expect(results.size).toBe(0)
    })

    it("should handle undefined input fields", async () => {
      const results = await detectWebTechnologies({})

      expect(results.size).toBe(0)
    })

    it("should handle case insensitivity", async () => {
      const results = await detectWebTechnologies({
        content: "DATA-REACTROOT __REACT__",
      })

      expect(results.has("React")).toBe(true)
    })
  })
})
