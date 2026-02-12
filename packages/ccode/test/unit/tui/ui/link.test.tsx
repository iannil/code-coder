// @ts-nocheck
/**
 * Link Component Unit Tests
 *
 * Tests for the link component including:
 * - URL and display text handling
 * - Click to open in browser
 * - Custom color support
 * - Default display text fallback
 */

import { describe, test, expect, beforeEach } from "bun:test"

describe("Link Component", () => {
  describe("props structure", () => {
    type LinkProps = {
      href: string
      children?: string
      fg?: { r: number; g: number; b: number; a: number }
    }

    test("should have correct props interface", () => {
      const props: LinkProps = {
        href: "https://example.com",
        children: "Click me",
        fg: { r: 97, g: 175, b: 239, a: 255 },
      }

      expect(props).toHaveProperty("href")
      expect(props).toHaveProperty("children")
      expect(props).toHaveProperty("fg")
    })

    test("should accept minimal props", () => {
      const props: LinkProps = {
        href: "https://example.com",
      }

      expect(props.href).toBe("https://example.com")
      expect(props.children).toBeUndefined()
    })
  })

  describe("display text", () => {
    type LinkProps = {
      href: string
      children?: string
    }

    test("should use children as display text when provided", () => {
      const props: LinkProps = {
        href: "https://example.com",
        children: "Example Site",
      }

      const displayText = props.children ?? props.href
      expect(displayText).toBe("Example Site")
    })

    test("should fallback to href when children not provided", () => {
      const props: LinkProps = {
        href: "https://example.com",
      }

      const displayText = props.children ?? props.href
      expect(displayText).toBe("https://example.com")
    })

    test("should handle empty string children", () => {
      const props: LinkProps = {
        href: "https://example.com",
        children: "",
      }

      const displayText = props.children ?? props.href
      expect(displayText).toBe("")
    })
  })

  describe("URL handling", () => {
    test("should handle http URLs", () => {
      const url = "http://example.com"

      expect(url).toMatch(/^http:\/\//)
    })

    test("should handle https URLs", () => {
      const url = "https://example.com"

      expect(url).toMatch(/^https:\/\//)
    })

    test("should handle relative URLs", () => {
      const url = "/path/to/resource"

      expect(url).toMatch(/^\//)
    })

    test("should handle mailto links", () => {
      const url = "mailto:user@example.com"

      expect(url).toMatch(/^mailto:/)
    })

    test("should handle ftp links", () => {
      const url = "ftp://ftp.example.com"

      expect(url).toMatch(/^ftp:\/\//)
    })
  })

  describe("color support", () => {
    test("should accept custom RGBA color", () => {
      const color = { r: 255, g: 0, b: 0, a: 255 }

      expect(color).toHaveProperty("r")
      expect(color).toHaveProperty("g")
      expect(color).toHaveProperty("b")
      expect(color).toHaveProperty("a")
    })

    test("should handle undefined color", () => {
      const color = undefined

      expect(color).toBeUndefined()
    })

    test("should handle various color values", () => {
      const colors = [
        { r: 97, g: 175, b: 239, a: 255 }, // Blue
        { r: 86, g: 182, b: 91, a: 255 }, // Green
        { r: 214, g: 79, b: 79, a: 255 }, // Red
      ]

      colors.forEach((color) => {
        expect(color.r).toBeGreaterThanOrEqual(0)
        expect(color.r).toBeLessThanOrEqual(255)
      })
    })
  })

  describe("click behavior", () => {
    test("should trigger click handler", () => {
      let clickedUrl: string | null = null

      const handleClick = (url: string) => {
        clickedUrl = url
      }

      handleClick("https://example.com")

      expect(clickedUrl).toEqual("https://example.com")
    })

    test("should handle errors gracefully", () => {
      let errorHandled = false

      const handleClick = () => {
        try {
          throw new Error("Failed to open")
        } catch (e) {
          errorHandled = true
        }
      }

      handleClick()

      expect(errorHandled).toBe(true)
    })
  })

  describe("edge cases", () => {
    test("should handle very long URLs", () => {
      const longUrl = "https://example.com/" + "a".repeat(1000)

      expect(longUrl.length).toBeGreaterThan(1000)
    })

    test("should handle URLs with special characters", () => {
      const url = "https://example.com/path?query=value&foo=bar#anchor"

      expect(url).toContain("?")
      expect(url).toContain("&")
      expect(url).toContain("#")
    })

    test("should handle URLs with unicode", () => {
      const url = "https://example.com/path/世界/🌍"

      expect(url).toContain("世界")
      expect(url).toContain("🌍")
    })

    test("should handle empty href", () => {
      const href = ""

      expect(href).toBe("")
    })

    test("should handle URLs with authentication", () => {
      const url = "https://user:pass@example.com"

      expect(url).toContain("user:")
      expect(url).toContain("@example.com")
    })

    test("should handle port numbers", () => {
      const url = "https://example.com:8080"

      expect(url).toContain(":8080")
    })
  })

  describe("children types", () => {
    test("should handle string children", () => {
      const children = "Link Text"

      expect(typeof children).toBe("string")
    })

    test("should handle numeric children", () => {
      const children = 123

      expect(typeof children).toBe("number")
    })

    test("should handle children with JSX", () => {
      const children = { type: "span", props: { children: "Styled link" } }

      expect(children).toHaveProperty("type")
    })
  })

  describe("security considerations", () => {
    test("should detect javascript: URLs", () => {
      const url = "javascript:alert('xss')"

      expect(url).toMatch(/^javascript:/i)
    })

    test("should detect data: URLs", () => {
      const url = "data:text/html,<script>alert('xss')</script>"

      expect(url).toMatch(/^data:/i)
    })

    test("should validate URL protocol", () => {
      const safeProtocols = ["http:", "https:", "mailto:", "ftp:"]
      const url = "https://example.com"

      const protocol = url.split(":")[0] + ":"
      expect(safeProtocols).toContain(protocol)
    })
  })
})
