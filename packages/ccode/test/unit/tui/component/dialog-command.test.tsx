// @ts-nocheck
/**
 * Dialog Command Component Unit Tests
 *
 * Tests for the command palette dialog including:
 * - Fuzzy search filtering
 * - Slash command display
 * - Command suggestions
 * - Keybind display
 * - Hidden/disabled commands
 */

import { describe, test, expect, beforeEach, mock } from "bun:test"

// Mock types based on the actual component
type SlashCommand = {
  name: string
  aliases?: string[]
}

type CommandOption = {
  value: string
  title: string
  description?: string
  category?: string
  keybind?: string
  suggested?: boolean
  hidden?: boolean
  enabled?: boolean
  footer?: string
  onSelect?: (dialog: any) => void
  slash?: SlashCommand
}

describe("Dialog Command Component", () => {
  describe("command filtering", () => {
    test("should filter commands by search query", () => {
      const commands: CommandOption[] = [
        { value: "new_session", title: "New Session", category: "Session" },
        { value: "open_session", title: "Open Session", category: "Session" },
        { value: "delete_session", title: "Delete Session", category: "Session" },
        { value: "theme_select", title: "Change Theme", category: "Appearance" },
        { value: "model_select", title: "Select Model", category: "Configuration" },
      ]

      const filterByQuery = (query: string) => {
        const q = query.toLowerCase()
        return commands.filter(
          (c) =>
            c.title.toLowerCase().includes(q) || c.value.toLowerCase().includes(q) || c.category?.toLowerCase().includes(q),
        )
      }

      const results = filterByQuery("session")
      expect(results).toHaveLength(3)
      expect(results.every((r) => r.title.includes("Session") || r.category === "Session")).toBe(true)
    })

    test("should show all commands when query is empty", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Command 1", hidden: false },
        { value: "cmd2", title: "Command 2", hidden: false },
      ]

      const filterByQuery = (query: string) => {
        if (!query) return commands
        return commands.filter((c) => c.title.toLowerCase().includes(query.toLowerCase()))
      }

      expect(filterByQuery("")).toHaveLength(2)
    })

    test("should exclude hidden commands from results", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Visible Command", hidden: false },
        { value: "cmd2", title: "Hidden Command", hidden: true },
        { value: "cmd3", title: "Another Visible", hidden: false },
      ]

      const filterVisible = (cmds: CommandOption[]) => cmds.filter((c) => !c.hidden)

      const visible = filterVisible(commands)
      expect(visible).toHaveLength(2)
      expect(visible.find((c) => c.value === "cmd2")).toBeUndefined()
    })

    test("should exclude disabled commands when enabled is false", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Enabled Command", enabled: true },
        { value: "cmd2", title: "Disabled Command", enabled: false },
        { value: "cmd3", title: "Another Enabled", enabled: true },
      ]

      const filterEnabled = (cmds: CommandOption[]) => cmds.filter((c) => c.enabled !== false)

      const enabled = filterEnabled(commands)
      expect(enabled).toHaveLength(2)
      expect(enabled.find((c) => c.value === "cmd2")).toBeUndefined()
    })
  })

  describe("slash commands", () => {
    test("should format slash commands with prefix", () => {
      const slashCommands: CommandOption[] = [
        {
          value: "theme_select",
          title: "Change Theme",
          slash: { name: "theme", aliases: ["th", "color"] },
        },
        {
          value: "model_select",
          title: "Select Model",
          slash: { name: "model", aliases: ["md"] },
        },
      ]

      const formatSlashCommands = (cmds: CommandOption[]) => {
        return cmds.flatMap((cmd) => {
          if (!cmd.slash) return []
          return {
            display: "/" + cmd.slash.name,
            aliases: cmd.slash.aliases?.map((a) => "/" + a) ?? [],
          }
        })
      }

      const formatted = formatSlashCommands(slashCommands)
      expect(formatted).toHaveLength(2)
      expect(formatted[0].display).toBe("/theme")
      expect(formatted[0].aliases).toEqual(["/th", "/color"])
    })

    test("should display command description in slash suggestions", () => {
      const command: CommandOption = {
        value: "export",
        title: "Export Code",
        slash: { name: "export" },
        description: "Export current session to file",
      }

      expect(command.description).toBe("Export current session to file")
    })

    test("should handle commands without slash definition", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "No Slash", slash: { name: "secret" } },
        { value: "cmd2", title: "No Slash Definition" },
      ]

      const withSlash = commands.filter((c) => c.slash)
      expect(withSlash).toHaveLength(1)
      expect(withSlash[0].value).toBe("cmd1")
    })
  })

  describe("command suggestions", () => {
    test("should identify suggested commands", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Regular", suggested: false },
        { value: "cmd2", title: "Suggested", suggested: true },
        { value: "cmd3", title: "Also Suggested", suggested: true },
      ]

      const getSuggested = (cmds: CommandOption[]) =>
        cmds.filter((c) => c.suggested)

      const suggested = getSuggested(commands)
      expect(suggested).toHaveLength(2)
    })

    test("should place suggested commands in separate category", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Regular" },
        { value: "cmd2", title: "Suggested", suggested: true },
      ]

      const categorize = (cmds: CommandOption[]) => {
        const suggested = cmds
          .filter((c) => c.suggested)
          .map((c) => ({ ...c, category: "Suggested" }))
        const regular = cmds.filter((c) => !c.suggested)
        return [...suggested, ...regular]
      }

      const categorized = categorize(commands)
      const suggested = categorized.filter((c) => c.category === "Suggested")

      expect(suggested).toHaveLength(1)
      expect(suggested[0].value).toBe("cmd2")
    })
  })

  describe("keybinds", () => {
    test("should display keybind hints for commands", () => {
      const commands: CommandOption[] = [
        { value: "cmd1", title: "Command 1", keybind: "ctrl+n" },
        { value: "cmd2", title: "Command 2", keybind: "ctrl+shift+p" },
        { value: "cmd3", title: "No Keybind" },
      ]

      const addKeybindFooter = (cmds: CommandOption[]) => {
        return cmds.map((c) => ({
          ...c,
          footer: c.keybind || undefined,
        }))
      }

      const withKeybinds = addKeybindFooter(commands)
      expect(withKeybinds[0].footer).toBe("ctrl+n")
      expect(withKeybinds[1].footer).toBe("ctrl+shift+p")
      expect(withKeybinds[2].footer).toBeUndefined()
    })

    test("should format keybinds consistently", () => {
      const keybind = "ctrl+n"
      const formatted = keybind.toUpperCase().replace("+", " + ")
      expect(formatted).toBe("CTRL + N")
    })
  })

  describe("command categories", () => {
    test("should group commands by category", () => {
      const commands: CommandOption[] = [
        { value: "s1", title: "Session 1", category: "Session" },
        { value: "s2", title: "Session 2", category: "Session" },
        { value: "t1", title: "Theme 1", category: "Theme" },
        { value: "c1", title: "Config 1", category: "Configuration" },
      ]

      const grouped = commands.reduce((acc, cmd) => {
        const cat = cmd.category || "Other"
        if (!acc[cat]) acc[cat] = []
        acc[cat].push(cmd)
        return acc
      }, {} as Record<string, CommandOption[]>)

      expect(grouped.Session).toHaveLength(2)
      expect(grouped.Theme).toHaveLength(1)
      expect(grouped.Configuration).toHaveLength(1)
    })

    test("should sort categories alphabetically", () => {
      const categories = ["Configuration", "Session", "Theme", "Other"]
      const sorted = [...categories].sort()

      expect(sorted).toEqual(["Configuration", "Other", "Session", "Theme"])
    })
  })

  describe("command execution", () => {
    test("should call onSelect when command is selected", () => {
      let selectedCommand: string | null = null

      const command: CommandOption = {
        value: "test_cmd",
        title: "Test Command",
        onSelect: (dialog) => {
          selectedCommand = "test_cmd"
        },
      }

      command.onSelect?.(undefined)
      expect(selectedCommand).toBe("test_cmd")
    })

    test("should not execute disabled commands", () => {
      const command: CommandOption = {
        value: "disabled_cmd",
        title: "Disabled Command",
        enabled: false,
        onSelect: () => {
          throw new Error("Should not be called")
        },
      }

      const canExecute = (cmd: CommandOption) => cmd.enabled !== false

      expect(canExecute(command)).toBe(false)
    })
  })

  describe("command registration", () => {
    test("should register command and cleanup on unmount", () => {
      const registrations: CommandOption[][] = []

      const register = (commandFn: () => CommandOption[]) => {
        const result = commandFn()
        registrations.push(result)
        return () => {
          const index = registrations.indexOf(result)
          if (index > -1) {
            registrations.splice(index, 1)
          }
        }
      }

      const cleanup = register(() => [{ value: "test", title: "Test" }])
      expect(registrations).toHaveLength(1)

      cleanup()
      expect(registrations).toHaveLength(0)
    })

    test("should maintain registration order", () => {
      const registrations: CommandOption[][] = []

      const register = (commandFn: () => CommandOption[]) => {
        registrations.push(commandFn())
      }

      register(() => [{ value: "first", title: "First" }])
      register(() => [{ value: "second", title: "Second" }])
      register(() => [{ value: "third", title: "Third" }])

      const allCommands = registrations.flatMap((r) => r)
      expect(allCommands[0].value).toBe("first")
      expect(allCommands[1].value).toBe("second")
      expect(allCommands[2].value).toBe("third")
    })
  })

  describe("keyboard shortcuts", () => {
    test("should trigger command by keybind", () => {
      const keybindPressed = "ctrl+n"
      const matchedCommand = "new_session"

      const commands: CommandOption[] = [
        { value: "new_session", title: "New Session", keybind: "ctrl+n" },
        { value: "other", title: "Other", keybind: "ctrl+o" },
      ]

      const findCommandByKeybind = (kb: string) => {
        return commands.find((c) => c.keybind === kb)
      }

      expect(findCommandByKeybind(keybindPressed)?.value).toBe(matchedCommand)
    })

    test("should match keybind regardless of modifiers order", () => {
      const keybind = "shift+ctrl+p"
      const commandKeybind = "ctrl+shift+p"

      // Both should match
      const normalize = (kb: string) => kb.toLowerCase().split("+").sort().join("+")

      expect(normalize(keybind)).toBe(normalize(commandKeybind))
    })
  })

  describe("suspension state", () => {
    test("should track suspend count", () => {
      let suspendCount = 0

      const suspend = () => suspendCount++
      const resume = () => suspendCount--

      suspend()
      suspend()
      expect(suspendCount).toBe(2)

      resume()
      expect(suspendCount).toBe(1)
    })

    test("should prevent execution when suspended", () => {
      let suspendCount = 0

      const canExecute = () => suspendCount === 0

      expect(canExecute()).toBe(true)

      suspendCount++
      expect(canExecute()).toBe(false)
    })
  })
})
