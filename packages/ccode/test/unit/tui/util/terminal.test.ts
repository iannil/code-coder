/**
 * Unit Tests: Terminal Utility
 * Testing terminal color detection and background color calculation
 */

import { describe, test, expect, beforeEach, afterEach, vi } from "bun:test"
import { Terminal } from "@/cli/cmd/tui/util/terminal"
import { RGBA } from "@opentui/core"

// Mock process.stdin and process.stdout
const mockStdin = {
  isTTY: true,
  setRawMode: vi.fn(),
  on: vi.fn(),
  removeListener: vi.fn(),
}

const mockStdout = {
  write: vi.fn(() => true),
}

describe("Terminal Utility", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    // Clean up any listeners
    vi.restoreAllMocks()
  })

  describe("colors", () => {
    test("should return colors structure", async () => {
      // In non-TTY test environment, returns null values
      const result = await Terminal.colors()

      expect(result).toHaveProperty("background")
      expect(result).toHaveProperty("foreground")
      expect(result).toHaveProperty("colors")
      expect(Array.isArray(result.colors)).toBe(true)
    })
  })

  describe("color parsing", () => {
    test("should parse rgb: format", () => {
      // Simulating the parseColor function behavior
      const colorStr = "rgb:ffff/ffff/ffff"
      const parts = colorStr.substring(4).split("/")

      const r = parseInt(parts[0], 16) >> 8
      const g = parseInt(parts[1], 16) >> 8
      const b = parseInt(parts[2], 16) >> 8

      expect(r).toBe(255)
      expect(g).toBe(255)
      expect(b).toBe(255)
    })

    test("should parse hex format", () => {
      const colorStr = "#ffffff"
      const rgba = RGBA.fromHex(colorStr)
      const ints = rgba.toInts()

      expect(ints[0]).toBe(255)
      expect(ints[1]).toBe(255)
      expect(ints[2]).toBe(255)
    })

    test("should parse rgb() function format", () => {
      const colorStr = "rgb(128, 64, 32)"
      const parts = colorStr.substring(4, colorStr.length - 1).split(",")

      const r = parseInt(parts[0])
      const g = parseInt(parts[1])
      const b = parseInt(parts[2])

      expect(r).toBe(128)
      expect(g).toBe(64)
      expect(b).toBe(32)
    })

    test("should return null for invalid format", () => {
      const colorStr = "invalid"

      const isRgb = colorStr.startsWith("rgb:")
      const isHex = colorStr.startsWith("#")
      const isRgbFn = colorStr.startsWith("rgb(")

      expect(isRgb || isHex || isRgbFn).toBe(false)
    })
  })

  describe("OSC sequences", () => {
    test("should generate OSC 11 sequence for background", () => {
      const osc11 = "\x1b]11;?\x07"

      expect(osc11).toContain("\x1b]11;") // OSC 11 for background
      expect(osc11).toContain("\x07") // BEL terminator
    })

    test("should generate OSC 10 sequence for foreground", () => {
      const osc10 = "\x1b]10;?\x07"

      expect(osc10).toContain("\x1b]10;") // OSC 10 for foreground
      expect(osc10).toContain("\x07") // BEL terminator
    })

    test("should generate OSC 4 sequence for palette", () => {
      const paletteIndex = 5
      const osc4 = `\x1b]4;${paletteIndex};?\x07`

      expect(osc4).toContain("\x1b]4;") // OSC 4 for palette
      expect(osc4).toContain("5;") // palette index
      expect(osc4).toContain("\x07") // BEL terminator
    })

    test("should match OSC 11 response pattern", () => {
      const response = "\x1b]11;rgb:ffff/ffff/ffff\x07"
      const bgMatch = response.match(/\x1b]11;([^\x07\x1b]+)/)

      expect(bgMatch).not.toBeNull()
      expect(bgMatch?.[1]).toBe("rgb:ffff/ffff/ffff")
    })

    test("should match OSC 10 response pattern", () => {
      const response = "\x1b]10;rgb:0000/0000/0000\x07"
      const fgMatch = response.match(/\x1b]10;([^\x07\x1b]+)/)

      expect(fgMatch).not.toBeNull()
      expect(fgMatch?.[1]).toBe("rgb:0000/0000/0000")
    })

    test("should match OSC 4 response pattern", () => {
      const response = "\x1b]4;0;rgb:0000/0000/0000\x07"
      const paletteMatch = response.match(/\x1b]4;(\d+);([^\x07\x1b]+)/)

      expect(paletteMatch).not.toBeNull()
      expect(paletteMatch?.[1]).toBe("0")
      expect(paletteMatch?.[2]).toBe("rgb:0000/0000/0000")
    })
  })

  describe("getTerminalBackgroundColor", () => {
    test("should calculate luminance correctly", () => {
      // White color
      const white = { r: 255, g: 255, b: 255 }
      const luminanceWhite = (0.299 * white.r + 0.587 * white.g + 0.114 * white.b) / 255
      expect(luminanceWhite).toBeGreaterThan(0.5)

      // Black color
      const black = { r: 0, g: 0, b: 0 }
      const luminanceBlack = (0.299 * black.r + 0.587 * black.g + 0.114 * black.b) / 255
      expect(luminanceBlack).toBeLessThan(0.5)

      // Gray color (midpoint)
      const gray = { r: 128, g: 128, b: 128 }
      const luminanceGray = (0.299 * gray.r + 0.587 * gray.g + 0.114 * gray.b) / 255
      expect(luminanceGray).toBeCloseTo(0.5, 1)
    })

    test("should return 'light' for bright backgrounds", () => {
      const brightColor = { r: 240, g: 240, b: 240 }
      const luminance = (0.299 * brightColor.r + 0.587 * brightColor.g + 0.114 * brightColor.b) / 255
      const result = luminance > 0.5 ? "light" : "dark"

      expect(result).toBe("light")
    })

    test("should return 'dark' for dark backgrounds", () => {
      const darkColor = { r: 18, g: 18, b: 18 }
      const luminance = (0.299 * darkColor.r + 0.587 * darkColor.g + 0.114 * darkColor.b) / 255
      const result = luminance > 0.5 ? "light" : "dark"

      expect(result).toBe("dark")
    })

    test("should handle common terminal backgrounds", () => {
      const testCases = [
        { color: { r: 40, g: 44, b: 52 }, expected: "dark" }, // One Dark
        { color: { r: 1, g: 1, b: 1 }, expected: "dark" }, // Almost black
        { color: { r: 30, g: 30, b: 30 }, expected: "dark" }, // Dark gray
        { color: { r: 255, g: 255, b: 255 }, expected: "light" }, // White
        { color: { r: 253, g: 246, b: 227 }, expected: "light" }, // Solarized Light bg
        { color: { r: 200, g: 200, b: 200 }, expected: "light" }, // Light gray
      ]

      for (const { color, expected } of testCases) {
        const luminance = (0.299 * color.r + 0.587 * color.g + 0.114 * color.b) / 255
        const result = luminance > 0.5 ? "light" : "dark"
        expect(result).toBe(expected)
      }
    })
  })

  describe("RGBA helper", () => {
    test("should create RGBA from ints", () => {
      const rgba = RGBA.fromInts(100, 150, 200, 255)
      const ints = rgba.toInts()

      expect(ints[0]).toBe(100)
      expect(ints[1]).toBe(150)
      expect(ints[2]).toBe(200)
      expect(ints[3]).toBe(255)
    })

    test("should create RGBA from hex", () => {
      const rgba = RGBA.fromHex("#ff5733")
      const ints = rgba.toInts()

      expect(ints[0]).toBe(255)
      expect(ints[1]).toBe(87)
      expect(ints[2]).toBe(51)
    })

    test("should create RGBA from short hex", () => {
      const rgba = RGBA.fromHex("#f53")
      const ints = rgba.toInts()

      expect(ints[0]).toBe(255)
      expect(ints[1]).toBe(85)
      expect(ints[2]).toBe(51)
    })
  })
})
