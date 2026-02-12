/**
 * Unit Tests: Keybind Utility
 * Testing the Keybind namespace functions for parsing, matching, and formatting keybindings
 */

import { describe, test, expect } from "bun:test"
import { Keybind } from "@/util/keybind"
import type { ParsedKey } from "@opentui/core"

// Helper to create a valid ParsedKey for testing
function createParsedKey(partial: Partial<ParsedKey> = {}): ParsedKey {
  return {
    name: "a",
    ctrl: false,
    meta: false,
    shift: false,
    option: false,
    sequence: "",
    number: false,
    raw: "",
    eventType: "press",
    source: "raw",
    ...partial,
  }
}

// Helper to create expected info (super may be undefined)
function expectInfo(info: Keybind.Info, expected: Partial<Keybind.Info>) {
  expect(info.name).toBe(expected.name ?? "")
  expect(info.ctrl).toBe(expected.ctrl ?? false)
  expect(info.meta).toBe(expected.meta ?? false)
  expect(info.shift).toBe(expected.shift ?? false)
  expect(info.leader).toBe(expected.leader ?? false)
  // super is optional in parsed results
  if (expected.super !== undefined) {
    expect(info.super).toBe(expected.super)
  }
}

describe("Keybind Utility", () => {
  describe("parse", () => {
    test("should parse simple key", () => {
      const result = Keybind.parse("a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a" })
    })

    test("should parse ctrl+key combination", () => {
      const result = Keybind.parse("ctrl+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", ctrl: true })
    })

    test("should parse multiple modifiers", () => {
      const result = Keybind.parse("ctrl+shift+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", ctrl: true, shift: true })
    })

    test("should parse ctrl+shift+meta combination", () => {
      const result = Keybind.parse("ctrl+shift+meta+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", ctrl: true, shift: true, meta: true })
    })

    test("should parse alt as meta", () => {
      const result = Keybind.parse("alt+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", meta: true })
    })

    test("should parse option as meta", () => {
      const result = Keybind.parse("option+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", meta: true })
    })

    test("should parse super key", () => {
      const result = Keybind.parse("super+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", super: true })
    })

    test("should parse leader key", () => {
      const result = Keybind.parse("leader+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", leader: true })
    })

    test("should parse leader with modifiers", () => {
      const result = Keybind.parse("leader+ctrl+a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", ctrl: true, leader: true })
    })

    test("should parse <leader> syntax", () => {
      const result = Keybind.parse("<leader>a")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "a", leader: true })
    })

    test("should parse delete key", () => {
      const result = Keybind.parse("delete")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "delete" })
    })

    test("should parse escape as escape", () => {
      const result = Keybind.parse("esc")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "escape" })
    })

    test("should parse comma-separated keybindings", () => {
      const result = Keybind.parse("ctrl+a,ctrl+b")
      expect(result).toHaveLength(2)
      expect(result[0].name).toBe("a")
      expect(result[0].ctrl).toBe(true)
      expect(result[1].name).toBe("b")
      expect(result[1].ctrl).toBe(true)
    })

    test("should parse return key", () => {
      const result = Keybind.parse("return")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "return" })
    })

    test("should parse space key", () => {
      const result = Keybind.parse("space")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "space" })
    })

    test("should parse tab key", () => {
      const result = Keybind.parse("tab")
      expect(result).toHaveLength(1)
      expectInfo(result[0], { name: "tab" })
    })

    test("should return empty array for 'none'", () => {
      const result = Keybind.parse("none")
      expect(result).toHaveLength(0)
    })

    test("should be case-insensitive", () => {
      const result = Keybind.parse("CTRL+SHIFT+A")
      expect(result).toHaveLength(1)
      expect(result[0].name).toBe("a")
      expect(result[0].ctrl).toBe(true)
      expect(result[0].shift).toBe(true)
    })
  })

  describe("match", () => {
    test("should match identical keybinds", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      const parsed: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.match(info, parsed)).toBe(true)
    })

    test("should not match different keybinds", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      const parsed: Keybind.Info = {
        name: "b",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.match(info, parsed)).toBe(false)
    })

    test("should not match when modifiers differ", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      const parsed: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: true,
        shift: false,
        leader: false,
      }
      expect(Keybind.match(info, parsed)).toBe(false)
    })

    test("should match when leader differs", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: true,
      }
      const parsed: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.match(info, parsed)).toBe(false)
    })

    test("should handle undefined super field", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      const parsed: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.match(info, parsed)).toBe(true)
    })

    test("should return false for undefined keybind", () => {
      const parsed: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.match(undefined, parsed)).toBe(false)
    })
  })

  describe("toString", () => {
    test("should format simple key", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("a")
    })

    test("should format ctrl+key", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("ctrl+a")
    })

    test("should format ctrl+shift+key", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: true,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("ctrl+shift+a")
    })

    test("should format with alt modifier", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: true,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("alt+a")
    })

    test("should format with super modifier", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: false,
        shift: false,
        super: true,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("super+a")
    })

    test("should format all modifiers", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: true,
        shift: true,
        super: true,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("ctrl+alt+super+shift+a")
    })

    test("should format leader prefix", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: false,
        meta: false,
        shift: false,
        leader: true,
      }
      expect(Keybind.toString(info)).toBe("<leader> a")
    })

    test("should format leader with modifiers", () => {
      const info: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: true,
      }
      expect(Keybind.toString(info)).toBe("<leader> ctrl+a")
    })

    test("should format delete as del", () => {
      const info: Keybind.Info = {
        name: "delete",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("del")
    })

    test("should return empty string for undefined", () => {
      expect(Keybind.toString(undefined)).toBe("")
    })

    test("should format escape", () => {
      const info: Keybind.Info = {
        name: "escape",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("escape")
    })

    test("should format return", () => {
      const info: Keybind.Info = {
        name: "return",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("return")
    })

    test("should format space", () => {
      const info: Keybind.Info = {
        name: "space",
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
      }
      expect(Keybind.toString(info)).toBe("space")
    })
  })

  describe("fromParsedKey", () => {
    test("should convert ParsedKey to Keybind.Info", () => {
      const parsed = createParsedKey({ ctrl: true })
      const result = Keybind.fromParsedKey(parsed)
      expect(result.name).toBe("a")
      expect(result.ctrl).toBe(true)
      expect(result.meta).toBe(false)
      expect(result.shift).toBe(false)
      expect(result.super).toBe(false)
      expect(result.leader).toBe(false)
    })

    test("should set leader to true when specified", () => {
      const parsed = createParsedKey({})
      const result = Keybind.fromParsedKey(parsed, true)
      expect(result.leader).toBe(true)
    })

    test("should preserve super when true", () => {
      const parsed = createParsedKey({ super: true })
      const result = Keybind.fromParsedKey(parsed)
      expect(result.super).toBe(true)
    })
  })

  describe("integration tests", () => {
    test("should parse and format consistently", () => {
      const original = "ctrl+shift+a"
      const parsed = Keybind.parse(original)
      expect(parsed).toHaveLength(1)
      const formatted = Keybind.toString(parsed[0])
      expect(formatted).toBe(original)
    })

    test("should handle leader key sequence", () => {
      const withLeader = Keybind.parse("<leader>a")
      const withoutLeader = Keybind.parse("a")

      expect(Keybind.match(withLeader[0], withoutLeader[0])).toBe(false)

      const withLeaderParsed = Keybind.fromParsedKey(createParsedKey({}), true)
      expect(Keybind.match(withLeader[0], withLeaderParsed)).toBe(true)
    })

    test("should handle multiple keybindings for same action", () => {
      const keybinds = Keybind.parse("ctrl+a,ctrl+b")
      const eventA: Keybind.Info = {
        name: "a",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      const eventB: Keybind.Info = {
        name: "b",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }
      const eventC: Keybind.Info = {
        name: "c",
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
      }

      const matchesA = keybinds.some((k) => Keybind.match(k, eventA))
      const matchesB = keybinds.some((k) => Keybind.match(k, eventB))
      const matchesC = keybinds.some((k) => Keybind.match(k, eventC))

      expect(matchesA).toBe(true)
      expect(matchesB).toBe(true)
      expect(matchesC).toBe(false)
    })
  })
})
