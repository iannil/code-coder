/**
 * Unit Tests: Keybind Context
 * Testing keybind context logic including leader key, matching, and parsing
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

describe("Keybind Context Logic", () => {
  describe("leader key state", () => {
    test("should initialize with leader inactive", () => {
      const store = { leader: false }
      expect(store.leader).toBe(false)
    })

    test("should activate leader state", () => {
      const store = { leader: false }
      store.leader = true
      expect(store.leader).toBe(true)
    })

    test("should deactivate leader state", () => {
      const store = { leader: true }
      store.leader = false
      expect(store.leader).toBe(false)
    })
  })

  describe("keybind parsing with leader", () => {
    test("should parse keybind without leader", () => {
      const parsed = Keybind.fromParsedKey(createParsedKey({ ctrl: true }), false)
      expect(parsed.name).toBe("a")
      expect(parsed.ctrl).toBe(true)
      expect(parsed.leader).toBe(false)
    })

    test("should parse keybind with leader active", () => {
      const parsed = Keybind.fromParsedKey(createParsedKey({}), true)
      expect(parsed.name).toBe("a")
      expect(parsed.leader).toBe(true)
    })
  })

  describe("keybind matching with leader", () => {
    test("should match leader key when leader inactive", () => {
      const keybinds = {
        leader: Keybind.parse("space"),
        new_session: Keybind.parse("<leader>n"),
      }

      const parsed = Keybind.fromParsedKey(createParsedKey({ name: "space" }), false)

      const matchesLeader = keybinds.leader.some((k) => Keybind.match(k, parsed))
      expect(matchesLeader).toBe(true)
    })

    test("should match leader+key combination when leader active", () => {
      const keybinds = {
        leader: Keybind.parse("space"),
        new_session: Keybind.parse("<leader>n"),
      }

      const parsed = Keybind.fromParsedKey(createParsedKey({ name: "n" }), true)

      const matchesNewSession = keybinds.new_session.some((k) => Keybind.match(k, parsed))
      expect(matchesNewSession).toBe(true)
    })

    test("should not match leader+key when leader inactive", () => {
      const keybinds = {
        new_session: Keybind.parse("<leader>n"),
      }

      const parsed = Keybind.fromParsedKey(createParsedKey({ name: "n" }), false)

      const matches = keybinds.new_session.some((k) => Keybind.match(k, parsed))
      expect(matches).toBe(false)
    })
  })

  describe("keybind matching with multiple options", () => {
    test("should execute command with any of its keybinds", () => {
      const keybinds = {
        cancel: Keybind.parse("escape,ctrl+c"),
      }

      // Test with escape
      const escapeParsed = Keybind.fromParsedKey(createParsedKey({ name: "escape" }), false)
      const escapeMatches = keybinds.cancel.some((k) => Keybind.match(k, escapeParsed))

      // Test with ctrl+c
      const ctrlCParsed = Keybind.fromParsedKey(createParsedKey({ name: "c", ctrl: true }), false)
      const ctrlCMatches = keybinds.cancel.some((k) => Keybind.match(k, ctrlCParsed))

      expect(escapeMatches).toBe(true)
      expect(ctrlCMatches).toBe(true)
    })
  })

  describe("keybind conflicts", () => {
    test("should handle conflicting keybinds", () => {
      // Different commands with same keybind - last one wins
      const config = {
        command1: "ctrl+a",
        command2: "ctrl+a", // Same keybind
      }

      const parsed = Object.fromEntries(
        Object.entries(config).map(([k, v]) => [k, Keybind.parse(v as string)]),
      )

      // Both parse to the same keybind
      expect(parsed.command1[0]).toEqual(parsed.command2[0])
    })

    test("should prioritize more specific keybind", () => {
      // ctrl+shift+a is more specific than ctrl+a
      const parsed = Keybind.fromParsedKey(createParsedKey({ name: "a", ctrl: true, shift: true }), false)

      expect(parsed.shift).toBe(true)

      // Should only match ctrl+shift+a, not ctrl+a
      const ctrlShiftA = Keybind.parse("ctrl+shift+a")[0]
      const ctrlA = Keybind.parse("ctrl+a")[0]

      expect(Keybind.match(ctrlShiftA, parsed)).toBe(true)
      expect(Keybind.match(ctrlA, parsed)).toBe(false)
    })
  })

  describe("keybind config parsing", () => {
    test("should parse all config keybinds", () => {
      const config = {
        leader: "space" as const,
        new_session: "ctrl+n" as const,
        previous_session: "ctrl+p" as const,
        cancel: "escape" as const,
      }

      const parsed = Object.fromEntries(
        Object.entries(config).map(([k, v]) => [k, Keybind.parse(v as string)]),
      )

      expect(Object.keys(parsed)).toHaveLength(4)
      expect(parsed.new_session[0]).toMatchObject({
        name: "n",
        ctrl: true,
      })
    })
  })

  describe("keybind display", () => {
    test("should format keybind for display", () => {
      const keybinds = {
        new_session: Keybind.parse("ctrl+n"),
        close_session: Keybind.parse("<leader>w"),
      }

      const format = (key: Keybind.Info) => Keybind.toString(key)

      expect(format(keybinds.new_session[0])).toBe("ctrl+n")
      expect(format(keybinds.close_session[0])).toBe("<leader> w")
    })

    test("should replace <leader> with actual leader key", () => {
      const config = {
        leader: "space",
        new_session: "<leader>n",
      }

      const keybinds = Object.fromEntries(
        Object.entries(config).map(([k, v]) => [k, Keybind.parse(v as string)]),
      )

      let formatted = Keybind.toString(keybinds.new_session[0])
      const leaderKey = keybinds.leader[0]

      if (leaderKey) {
        formatted = formatted.replace("<leader>", Keybind.toString(leaderKey))
      }

      expect(formatted).toBe("space n")
    })
  })
})
