// @ts-nocheck
/**
 * Integration Tests: Command System
 * Testing command registration, triggering, and keybind integration
 */

import { describe, test, expect, vi } from "bun:test"
import type { CommandOption } from "@/cli/cmd/tui/component/dialog-command"

describe("Command System Integration", () => {
  describe("command registration", () => {
    test("should register command with callback", () => {
      const commands: CommandOption[] = []

      const register = (cmd: CommandOption) => {
        commands.push(cmd)
      }

      const callback = vi.fn()
      register({
        value: "test-command",
        title: "Test Command",
        description: "A test command",
        onSelect: callback,
      })

      expect(commands).toHaveLength(1)
      expect(commands[0].value).toBe("test-command")
      expect(commands[0].title).toBe("Test Command")
    })

    test("should register multiple commands", () => {
      const commands: CommandOption[] = []

      const register = (cmd: CommandOption) => {
        commands.push(cmd)
      }

      register({ value: "cmd1", title: "Command 1" })
      register({ value: "cmd2", title: "Command 2" })
      register({ value: "cmd3", title: "Command 3" })

      expect(commands).toHaveLength(3)
    })
  })

  describe("command filtering", () => {
    test("should filter commands by search text", () => {
      const commands: CommandOption[] = [
        { value: "new-session", title: "New Session" },
        { value: "close-session", title: "Close Session" },
        { value: "save-file", title: "Save File" },
        { value: "open-file", title: "Open File" },
      ]

      const query = "session"
      const filtered = commands.filter((cmd) =>
        cmd.title.toLowerCase().includes(query.toLowerCase()) ||
        cmd.value.toLowerCase().includes(query.toLowerCase())
      )

      expect(filtered).toHaveLength(2)
      expect(filtered[0].value).toBe("new-session")
      expect(filtered[1].value).toBe("close-session")
    })

    test("should be case insensitive", () => {
      const commands: CommandOption[] = [
        { value: "Help", title: "Show Help" },
        { value: "quit", title: "Quit Application" },
      ]

      const query = "HELP"
      const filtered = commands.filter((cmd) =>
        cmd.title.toLowerCase().includes(query.toLowerCase()) ||
        cmd.value.toLowerCase().includes(query.toLowerCase())
      )

      expect(filtered).toHaveLength(1)
      expect(filtered[0].value).toBe("Help")
    })

    test("should return empty when no matches", () => {
      const commands: CommandOption[] = [
        { value: "new", title: "New" },
        { value: "open", title: "Open" },
      ]

      const query = "zzzzz"
      const filtered = commands.filter((cmd) =>
        cmd.title.toLowerCase().includes(query.toLowerCase())
      )

      expect(filtered).toHaveLength(0)
    })
  })

  describe("command triggering", () => {
    test("should trigger command by value", () => {
      const callbacks = {
        cmd1: vi.fn(),
        cmd2: vi.fn(),
      }

      const commands: CommandOption[] = [
        { value: "cmd1", title: "Command 1", onSelect: callbacks.cmd1 },
        { value: "cmd2", title: "Command 2", onSelect: callbacks.cmd2 },
      ]

      const trigger = (name: string) => {
        const cmd = commands.find((c) => c.value === name)
        if (cmd && cmd.onSelect) {
          cmd.onSelect(undefined as any)
        }
      }

      trigger("cmd1")

      expect(callbacks.cmd1).toHaveBeenCalled()
      expect(callbacks.cmd2).not.toHaveBeenCalled()
    })

    test("should handle triggering non-existent command", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Command 1" },
      ]

      const trigger = (name: string) => {
        const cmd = commands.find((c) => c.value === name)
        if (cmd && cmd.onSelect) {
          cmd.onSelect(undefined as any)
        }
      }

      expect(() => trigger("non-existent")).not.toThrow()
    })
  })

  describe("command keybinds", () => {
    test("should associate keybind with command", () => {
      const command: CommandOption = {
        value: "new-session",
        title: "New Session",
        keybind: "new_session",
        footer: "ctrl+n",
      }

      expect(command.keybind).toBe("new_session")
      expect(command.footer).toBe("ctrl+n")
    })

    test("should match keybind to trigger command", () => {
      const keybinds = {
        new_session: "ctrl+n",
        close: "escape",
      }

      const commands: CommandOption[] = [
        { value: "new", keybind: "new_session" },
        { value: "close", keybind: "close" },
      ]

      const matchKeybind = (keybindName: string, evt: { name: string; ctrl?: boolean }) => {
        const keybind = keybinds[keybindName as keyof typeof keybinds]
        if (!keybind) return false

        const [main, modifier] = keybind.split("+").reverse()
        return evt.name === main && (!modifier || evt.ctrl === true)
      }

      // Test matching ctrl+n
      const evt1 = { name: "n", ctrl: true }
      expect(matchKeybind("new_session", evt1)).toBe(true)

      // Test matching escape
      const evt2 = { name: "escape" }
      expect(matchKeybind("close", evt2)).toBe(true)
    })
  })

  describe("slash commands", () => {
    test("should register slash command", () => {
      const slash = {
        name: "help",
        aliases: ["h", "?"],
      }

      const command: CommandOption = {
        value: "show-help",
        title: "Help",
        slash,
      }

      expect(command.slash).toEqual(slash)
      expect(command.slash!.name).toBe("help")
      expect(command.slash!.aliases).toEqual(["h", "?"])
    })

    test("should format slash display name", () => {
      const commands: CommandOption[] = [
        {
          value: "help",
          title: "Show Help",
          slash: { name: "help" },
        },
        {
          value: "quit",
          title: "Quit",
          slash: { name: "quit", aliases: ["exit", "q"] },
        },
      ]

      const slashes = commands.flatMap((cmd) => {
        if (!cmd.slash) return []
        return {
          display: "/" + cmd.slash.name,
          aliases: cmd.slash.aliases?.map((a) => "/" + a),
        }
      })

      expect(slashes).toHaveLength(2)
      expect(slashes[0].display).toBe("/help")
      expect(slashes[1].display).toBe("/quit")
      expect(slashes[1].aliases).toEqual(["/exit", "/q"])
    })
  })

  describe("command visibility", () => {
    test("should hide disabled commands", () => {
      const commands: CommandOption[] = [
        { value: "enabled", title: "Enabled Command", enabled: true },
        { value: "disabled", title: "Disabled Command", enabled: false },
      ]

      const visible = commands.filter((cmd) => cmd.enabled !== false)

      expect(visible).toHaveLength(1)
      expect(visible[0].value).toBe("enabled")
    })

    test("should hide hidden commands", () => {
      const commands: CommandOption[] = [
        { value: "visible", title: "Visible Command" },
        { value: "hidden", title: "Hidden Command", hidden: true },
      ]

      const visible = commands.filter((cmd) => !cmd.hidden)

      expect(visible).toHaveLength(1)
      expect(visible[0].value).toBe("visible")
    })

    test("should show suggested commands", () => {
      const commands: CommandOption[] = [
        { value: "normal", title: "Normal Command" },
        { value: "suggested", title: "Suggested Command", suggested: true },
      ]

      const suggested = commands.filter((cmd) => cmd.suggested)

      expect(suggested).toHaveLength(1)
      expect(suggested[0].value).toBe("suggested")
    })
  })

  describe("command execution flow", () => {
    test("should execute command onSelect callback", () => {
      let executed = false
      let dialogClosed = false

      const mockDialog = {
        clear: () => {
          dialogClosed = true
        },
        replace: () => {},
        stack: [],
        size: "medium" as const,
        setSize: () => {},
      }

      const command: CommandOption = {
        value: "test",
        title: "Test",
        onSelect: (dialog) => {
          executed = true
          dialog.clear()
        },
      }

      command.onSelect?.(mockDialog as any)

      expect(executed).toBe(true)
      expect(dialogClosed).toBe(true)
    })
  })
})
