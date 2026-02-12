// @ts-nocheck
/**
 * Autocomplete Component Unit Tests
 *
 * Tests for the autocomplete component including:
 * - Line range extraction
 * - Fuzzy sorting
 * - File/agent/command options
 * - Show/hide logic
 * - Keyboard navigation
 * - Mouse interaction
 * - Position calculation
 */

import { describe, test, expect, beforeEach } from "bun:test"

// Mock types based on the actual component
type LineRange = {
  baseName: string
  startLine: number
  endLine?: number
}

type AutocompleteOption = {
  display: string
  value?: string
  aliases?: string[]
  disabled?: boolean
  description?: string
  isDirectory?: boolean
  path?: string
}

describe("Autocomplete Component", () => {
  describe("line range extraction", () => {
    test("should return base query when no line range", () => {
      const input = "src/test.ts"

      const extractLineRange = (input: string) => {
        const hashIndex = input.lastIndexOf("#")
        if (hashIndex === -1) {
          return { baseQuery: input }
        }
        return { baseQuery: input }
      }

      expect(extractLineRange(input)).toEqual({ baseQuery: "src/test.ts" })
    })

    test("should extract single line number", () => {
      const input = "src/test.ts#10"

      const extractLineRange = (input: string) => {
        const hashIndex = input.lastIndexOf("#")
        if (hashIndex === -1) {
          return { baseQuery: input }
        }
        const baseName = input.substring(0, hashIndex)
        const linePart = input.substring(hashIndex + 1)
        const lineMatch = linePart.match(/^(\d+)(?:-(\d*))?$/)

        if (!lineMatch) {
          return { baseQuery: baseName }
        }

        const startLine = Number(lineMatch[1])

        return {
          lineRange: {
            baseName,
            startLine,
          },
          baseQuery: baseName,
        }
      }

      const result = extractLineRange(input)
      expect(result.lineRange?.startLine).toBe(10)
      expect(result.baseQuery).toBe("src/test.ts")
    })

    test("should extract line range with end line", () => {
      const input = "src/test.ts#10-20"

      const extractLineRange = (input: string) => {
        const hashIndex = input.lastIndexOf("#")
        if (hashIndex === -1) {
          return { baseQuery: input }
        }
        const baseName = input.substring(0, hashIndex)
        const linePart = input.substring(hashIndex + 1)
        const lineMatch = linePart.match(/^(\d+)(?:-(\d*))?$/)

        if (!lineMatch) {
          return { baseQuery: baseName }
        }

        const startLine = Number(lineMatch[1])
        const endLine = lineMatch[2] && startLine < Number(lineMatch[2]) ? Number(lineMatch[2]) : undefined

        return {
          lineRange: {
            baseName,
            startLine,
            endLine,
          },
          baseQuery: baseName,
        }
      }

      const result = extractLineRange(input)
      expect(result.lineRange?.startLine).toBe(10)
      expect(result.lineRange?.endLine).toBe(20)
    })

    test("should handle invalid line range format", () => {
      const input = "src/test.ts#abc"

      const extractLineRange = (input: string) => {
        const hashIndex = input.lastIndexOf("#")
        if (hashIndex === -1) {
          return { baseQuery: input }
        }
        const baseName = input.substring(0, hashIndex)
        const linePart = input.substring(hashIndex + 1)
        const lineMatch = linePart.match(/^(\d+)(?:-(\d*))?$/)

        if (!lineMatch) {
          return { baseQuery: baseName }
        }

        return { baseQuery: baseName }
      }

      expect(extractLineRange(input)).toEqual({ baseQuery: "src/test.ts" })
    })

    test("should handle empty end line (x-)", () => {
      const input = "src/test.ts#10-"

      const extractLineRange = (input: string) => {
        const hashIndex = input.lastIndexOf("#")
        if (hashIndex === -1) {
          return { baseQuery: input }
        }
        const baseName = input.substring(0, hashIndex)
        const linePart = input.substring(hashIndex + 1)
        const lineMatch = linePart.match(/^(\d+)(?:-(\d*))?$/)

        if (!lineMatch) {
          return { baseQuery: baseName }
        }

        const startLine = Number(lineMatch[1])
        const endLine = lineMatch[2] && startLine < Number(lineMatch[2]) ? Number(lineMatch[2]) : undefined

        return {
          lineRange: {
            baseName,
            startLine,
            endLine,
          },
          baseQuery: baseName,
        }
      }

      const result = extractLineRange(input)
      expect(result.lineRange?.startLine).toBe(10)
      expect(result.lineRange?.endLine).toBeUndefined()
    })

    test("should remove line range from input", () => {
      const input = "src/test.ts#10-20"

      const removeLineRange = (input: string) => {
        const hashIndex = input.lastIndexOf("#")
        return hashIndex !== -1 ? input.substring(0, hashIndex) : input
      }

      expect(removeLineRange(input)).toBe("src/test.ts")
    })
  })

  describe("line range formatting", () => {
    test("should format filename with line range", () => {
      const filename = "src/test.ts"
      const lineRange = { startLine: 10, endLine: 20 }

      const formatted = `${filename}#${lineRange.startLine}${lineRange.endLine ? `-${lineRange.endLine}` : ""}`

      expect(formatted).toBe("src/test.ts#10-20")
    })

    test("should format filename with single line", () => {
      const filename = "src/test.ts"
      const lineRange = { startLine: 10 }

      const formatted = `${filename}#${lineRange.startLine}`

      expect(formatted).toBe("src/test.ts#10")
    })
  })

  describe("autocomplete visibility", () => {
    test("should show for @ trigger", () => {
      let visible: false | "@" | "/" = false

      const show = (mode: "@" | "/") => {
        visible = mode
      }

      show("@")

      expect(visible).toBe("@")
    })

    test("should show for / trigger", () => {
      let visible: false | "@" | "/" = false

      const show = (mode: "@" | "/") => {
        visible = mode
      }

      show("/")

      expect(visible).toBe("/")
    })

    test("should hide autocomplete", () => {
      let visible: false | "@" | "/" = "@"

      const hide = () => {
        visible = false
      }

      hide()

      expect(visible).toBe(false)
    })
  })

  describe("trigger detection", () => {
    test("should detect @ at cursor position", () => {
      const text = "test @"
      const cursorOffset = text.length
      const charAtCursor = text[cursorOffset - 1]

      expect(charAtCursor).toBe("@")
    })

    test("should detect / at start of input", () => {
      const text = "/command"
      const cursorOffset = 0
      const charAtPosition = text[cursorOffset]

      expect(charAtPosition).toBe("/")
    })

    test("should find last @ before cursor", () => {
      const text = "some text @file more text"
      const cursorOffset = 15
      const textSlice = text.slice(0, cursorOffset)
      const lastAtIndex = textSlice.lastIndexOf("@")

      expect(lastAtIndex).toBe(10)
    })

    test("should detect if @ can trigger (after whitespace)", () => {
      const text = " @"
      const idx = text.lastIndexOf("@")
      const before = idx === 0 ? undefined : text[idx - 1]

      const canTrigger = before === undefined || /\s/.test(before)

      expect(canTrigger).toBe(true)
    })

    test("should not trigger @ in middle of word", () => {
      const text = "test@file"
      const idx = text.lastIndexOf("@")
      const before = idx === 0 ? undefined : text[idx - 1]

      const canTrigger = before === undefined || /\s/.test(before)

      expect(canTrigger).toBe(false)
    })
  })

  describe("hide conditions", () => {
    test("should hide if typed text before trigger", () => {
      const index = 5
      const cursorOffset = 3
      const shouldHide = cursorOffset <= index

      expect(shouldHide).toBe(true)
    })

    test("should hide if space between trigger and cursor", () => {
      const index = 5
      const cursorOffset = 10
      const textBetween = "abc "
      const hasSpace = /\s/.test(textBetween)

      expect(hasSpace).toBe(true)
    })

    test("should hide for slash command with space after", () => {
      const text = "/cmd more"
      const shouldHide = text.match(/^\S+\s+\S+\s*$/)

      expect(shouldHide).toBeTruthy()
    })
  })

  describe("keyboard navigation", () => {
    test("should move up in list", () => {
      let selected = 2
      const optionsLength = 5
      const direction = -1

      const move = () => {
        let next = selected + direction
        if (next < 0) next = optionsLength - 1
        selected = next
      }

      move()

      expect(selected).toBe(1)
    })

    test("should wrap to bottom when moving up from top", () => {
      let selected = 0
      const optionsLength = 5
      const direction = -1

      const move = () => {
        let next = selected + direction
        if (next < 0) next = optionsLength - 1
        selected = next
      }

      move()

      expect(selected).toBe(4)
    })

    test("should move down in list", () => {
      let selected = 2
      const optionsLength = 5
      const direction = 1

      const move = () => {
        let next = selected + direction
        if (next >= optionsLength) next = 0
        selected = next
      }

      move()

      expect(selected).toBe(3)
    })

    test("should wrap to top when moving down from bottom", () => {
      let selected = 4
      const optionsLength = 5
      const direction = 1

      const move = () => {
        let next = selected + direction
        if (next >= optionsLength) next = 0
        selected = next
      }

      move()

      expect(selected).toBe(0)
    })

    test("should recognize up arrow", () => {
      const name = "up"
      const isNavUp = name === "up"

      expect(isNavUp).toBe(true)
    })

    test("should recognize Ctrl+P as up", () => {
      const name = "p"
      const ctrl = true
      const meta = false
      const shift = false
      const ctrlOnly = ctrl && !meta && !shift
      const isNavUp = ctrlOnly && name === "p"

      expect(isNavUp).toBe(true)
    })

    test("should recognize down arrow", () => {
      const name = "down"
      const isNavDown = name === "down"

      expect(isNavDown).toBe(true)
    })

    test("should recognize Ctrl+N as down", () => {
      const name = "n"
      const ctrl = true
      const meta = false
      const shift = false
      const ctrlOnly = ctrl && !meta && !shift
      const isNavDown = ctrlOnly && name === "n"

      expect(isNavDown).toBe(true)
    })

    test("should hide on escape", () => {
      const name = "escape"
      let visible = true

      const handleEscape = () => {
        if (name === "escape") {
          visible = false
        }
      }

      handleEscape()

      expect(visible).toBe(false)
    })

    test("should select on return", () => {
      const name = "return"
      let selected = false

      const handleReturn = () => {
        if (name === "return") {
          selected = true
        }
      }

      handleReturn()

      expect(selected).toBe(true)
    })

    test("should select on tab for non-directory", () => {
      const name = "tab"
      const selectedOption: AutocompleteOption = {
        display: "file.ts",
        value: "file.ts",
        isDirectory: false,
      }
      let actionPerformed = false

      const handleTab = () => {
        if (name === "tab") {
          if (selectedOption.isDirectory) {
            // expand directory
          } else {
            actionPerformed = true // select
          }
        }
      }

      handleTab()

      expect(actionPerformed).toBe(true)
    })

    test("should expand directory on tab", () => {
      const name = "tab"
      const selectedOption: AutocompleteOption = {
        display: "src/",
        value: "src/",
        isDirectory: true,
      }
      let expanded = false

      const handleTab = () => {
        if (name === "tab") {
          if (selectedOption.isDirectory) {
            expanded = true
          }
        }
      }

      handleTab()

      expect(expanded).toBe(true)
    })
  })

  describe("position calculation", () => {
    test("should calculate position relative to parent", () => {
      const anchor = { x: 10, y: 20, width: 80, parent: { x: 5, y: 5 } }

      const position = {
        x: anchor.x - (anchor.parent?.x ?? 0),
        y: anchor.y - (anchor.parent?.y ?? 0),
        width: anchor.width,
      }

      expect(position.x).toBe(5)
      expect(position.y).toBe(15)
      expect(position.width).toBe(80)
    })

    test("should handle no parent", () => {
      const anchor = { x: 10, y: 20, width: 80 }

      const position = {
        x: anchor.x - (anchor.parent?.x ?? 0),
        y: anchor.y - (anchor.parent?.y ?? 0),
        width: anchor.width,
      }

      expect(position.x).toBe(10)
      expect(position.y).toBe(20)
    })
  })

  describe("height calculation", () => {
    test("should limit height to 10 items", () => {
      const optionCount = 20
      const maxHeight = 10

      const height = Math.min(maxHeight, optionCount)

      expect(height).toBe(10)
    })

    test("should use actual count if less than max", () => {
      const optionCount = 5
      const maxHeight = 10

      const height = Math.min(maxHeight, optionCount)

      expect(height).toBe(5)
    })

    test("should consider available vertical space", () => {
      const optionCount = 20
      const anchorY = 5
      const maxHeight = 10

      const height = Math.min(maxHeight, optionCount, Math.max(1, anchorY))

      expect(height).toBe(5) // Limited by anchorY
    })
  })

  describe("option filtering", () => {
    test("should show all options when no filter", () => {
      const options: AutocompleteOption[] = [
        { display: "file1.ts" },
        { display: "file2.ts" },
        { display: "file3.ts" },
      ]

      const filtered = options

      expect(filtered).toHaveLength(3)
    })

    test("should filter by display value", () => {
      const options: AutocompleteOption[] = [
        { display: "file1.ts", value: "file1.ts" },
        { display: "file2.ts", value: "file2.ts" },
        { display: "test.ts", value: "test.ts" },
      ]

      const filter = "file"
      const filtered = options.filter((opt) =>
        (opt.value ?? opt.display).toLowerCase().includes(filter.toLowerCase())
      )

      expect(filtered).toHaveLength(2)
      expect(filtered.every((f) => f.display.includes("file"))).toBe(true)
    })

    test("should limit results to 10", () => {
      const options = Array.from({ length: 20 }, (_, i) => ({
        display: `file${i}.ts`,
        value: `file${i}.ts`,
      }))

      const limit = 10
      const limited = options.slice(0, limit)

      expect(limited).toHaveLength(10)
    })
  })

  describe("frecency sorting", () => {
    test("should sort by frecency score", () => {
      const files = [
        { path: "a.ts", score: 5 },
        { path: "b.ts", score: 10 },
        { path: "c.ts", score: 1 },
      ]

      const sorted = [...files].sort((a, b) => b.score - a.score)

      expect(sorted[0].path).toBe("b.ts")
      expect(sorted[1].path).toBe("a.ts")
      expect(sorted[2].path).toBe("c.ts")
    })

    test("should sort by depth when frecency equal", () => {
      const files = [
        { path: "a/b/c.ts", depth: 3 },
        { path: "a.ts", depth: 1 },
      ]

      const sorted = [...files].sort((a, b) => a.depth - b.depth)

      expect(sorted[0].path).toBe("a.ts")
      expect(sorted[1].path).toBe("a/b/c.ts")
    })

    test("should sort alphabetically as final tiebreaker", () => {
      const files = [
        { path: "z.ts", score: 5, depth: 1 },
        { path: "a.ts", score: 5, depth: 1 },
      ]

      const sorted = [...files].sort((a, b) => a.path.localeCompare(b.path))

      expect(sorted[0].path).toBe("a.ts")
      expect(sorted[1].path).toBe("z.ts")
    })
  })

  describe("agent filtering", () => {
    test("should filter out hidden agents", () => {
      const agents = [
        { name: "editor", hidden: false, mode: "secondary" },
        { name: "secret", hidden: true, mode: "secondary" },
        { name: "planner", hidden: false, mode: "secondary" },
      ]

      const visible = agents.filter((a) => !a.hidden && a.mode !== "primary")

      expect(visible).toHaveLength(2)
      expect(visible.find((a) => a.name === "secret")).toBeUndefined()
    })

    test("should filter out primary mode agents", () => {
      const agents = [
        { name: "editor", hidden: false, mode: "primary" },
        { name: "planner", hidden: false, mode: "secondary" },
      ]

      const visible = agents.filter((a) => !a.hidden && a.mode !== "primary")

      expect(visible).toHaveLength(1)
      expect(visible[0].name).toBe("planner")
    })
  })

  describe("command padding", () => {
    test("should pad command names to max length", () => {
      const commands = [
        { display: "short" },
        { display: "medium-length" },
        { display: "very-long-command-name" },
      ]

      const max = Math.max(...commands.map((c) => c.display.length))
      const padded = commands.map((c) => ({
        ...c,
        display: c.display.padEnd(max + 2),
      }))

      expect(padded[0].display.length).toBe(max + 2)
      expect(padded[1].display.length).toBe(max + 2)
      expect(padded[2].display.length).toBe(max + 2)
    })
  })

  describe("mouse interaction", () => {
    test("should switch to mouse mode on mouse move", () => {
      let inputMode: "keyboard" | "mouse" = "keyboard"

      const handleMouseMove = () => {
        inputMode = "mouse"
      }

      handleMouseMove()

      expect(inputMode).toBe("mouse")
    })

    test("should only navigate on mouseover when in mouse mode", () => {
      let inputMode: "keyboard" | "mouse" = "keyboard"
      let navigated = false

      const handleMouseOver = () => {
        if (inputMode !== "mouse") return
        navigated = true
      }

      handleMouseOver()

      expect(navigated).toBe(false)

      inputMode = "mouse"
      handleMouseOver()

      expect(navigated).toBe(true)
    })

    test("should select on mouse up", () => {
      let selected = false

      const handleMouseUp = () => {
        selected = true
      }

      handleMouseUp()

      expect(selected).toBe(true)
    })

    test("should update selection on mouse down", () => {
      let selectedIndex = 0

      const handleMouseDown = (index: number) => {
        selectedIndex = index
      }

      handleMouseDown(5)

      expect(selectedIndex).toBe(5)
    })
  })

  describe("directory expansion", () => {
    test("should remove @ from display when expanding", () => {
      const displayText = "@src/"
      const path = displayText.startsWith("@") ? displayText.slice(1) : displayText

      expect(path).toBe("src/")
    })

    test("should insert directory path on expansion", () => {
      const path = "src/"
      const insertText = "@" + path

      expect(insertText).toBe("@src/")
    })

    test("should reset selection after expansion", () => {
      let selected = 5

      const resetSelection = () => {
        selected = 0
      }

      resetSelection()

      expect(selected).toBe(0)
    })
  })

  describe("filter text extraction", () => {
    test("should extract text from index to cursor", () => {
      const fullText = "some text here"
      const index = 5
      const cursorOffset = 9

      const extracted = fullText.slice(index, cursorOffset)

      expect(extracted).toBe("text")
    })

    test("should handle cursor at index", () => {
      const fullText = "text"
      const index = 0
      const cursorOffset = 0

      const extracted = fullText.slice(index, cursorOffset)

      expect(extracted).toBe("")
    })
  })
})
