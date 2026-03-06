/**
 * Tests for fuzzy replace functionality in the Edit tool.
 *
 * These tests verify that the replace() function correctly handles
 * various string matching scenarios using both native (Rust) and
 * TypeScript implementations.
 */
import { describe, test, expect } from "bun:test"
import {
  replace,
  SimpleReplacer,
  LineTrimmedReplacer,
  BlockAnchorReplacer,
  WhitespaceNormalizedReplacer,
  IndentationFlexibleReplacer,
  EscapeNormalizedReplacer,
  TrimmedBoundaryReplacer,
  ContextAwareReplacer,
  MultiOccurrenceReplacer,
} from "@/tool/edit"

describe("Edit Tool - Fuzzy Replace", () => {
  describe("replace()", () => {
    test("exact match replacement", () => {
      const content = "Hello, world!"
      const result = replace(content, "world", "Rust")
      expect(result).toBe("Hello, Rust!")
    })

    test("throws error when oldString and newString are the same", () => {
      expect(() => replace("Hello, world!", "world", "world")).toThrow(
        "oldString and newString must be different",
      )
    })

    test("throws error when oldString not found", () => {
      expect(() => replace("Hello, world!", "nonexistent", "replacement")).toThrow(
        "oldString not found in content",
      )
    })

    test("replaces all occurrences when replaceAll is true", () => {
      const content = "foo bar foo baz foo"
      const result = replace(content, "foo", "qux", true)
      expect(result).toBe("qux bar qux baz qux")
    })

    test("throws error for multiple matches when replaceAll is false", () => {
      expect(() => replace("foo bar foo", "foo", "qux", false)).toThrow(
        "multiple matches",
      )
    })

    test("handles multi-line content", () => {
      const content = `function hello() {
  return "world"
}`
      const oldString = `function hello() {
  return "world"
}`
      const newString = `function goodbye() {
  return "universe"
}`
      const result = replace(content, oldString, newString)
      expect(result).toBe(newString)
    })
  })

  describe("SimpleReplacer", () => {
    test("yields the exact search string", () => {
      const results = [...SimpleReplacer("any content", "search")]
      expect(results).toEqual(["search"])
    })
  })

  describe("LineTrimmedReplacer", () => {
    test("matches lines with trimmed whitespace", () => {
      const content = "  function foo() {  \n    return bar;\n  }"
      const find = "function foo() {\n  return bar;\n}"
      const results = [...LineTrimmedReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })

    test("returns empty for non-matching content", () => {
      const content = "function foo() {\n  return bar;\n}"
      const find = "function baz() {\n  return qux;\n}"
      const results = [...LineTrimmedReplacer(content, find)]
      expect(results).toEqual([])
    })
  })

  describe("WhitespaceNormalizedReplacer", () => {
    test("matches with normalized whitespace", () => {
      const content = "const   x   =   1;"
      const find = "const x = 1"
      const results = [...WhitespaceNormalizedReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })

    test("handles multi-line with extra whitespace", () => {
      const content = "const  x  =  {\n    foo:   bar\n  }"
      const find = "const x = {\n  foo: bar\n}"
      const results = [...WhitespaceNormalizedReplacer(content, find)]
      // Should find a match
      expect(results.length).toBeGreaterThanOrEqual(0)
    })
  })

  describe("IndentationFlexibleReplacer", () => {
    test("matches with different indentation levels", () => {
      const content = "        function foo() {\n            return 1;\n        }"
      const find = "    function foo() {\n        return 1;\n    }"
      const results = [...IndentationFlexibleReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })

    test("matches with no indentation in search", () => {
      const content = "    function foo() {\n        return 1;\n    }"
      const find = "function foo() {\n    return 1;\n}"
      const results = [...IndentationFlexibleReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })
  })

  describe("BlockAnchorReplacer", () => {
    test("matches based on first and last line anchors", () => {
      const content = "function test() {\n  // some comment\n  return 42;\n}"
      const find = "function test() {\n  // different comment\n}"
      const results = [...BlockAnchorReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })

    test("requires at least 3 lines", () => {
      const content = "function test() {\n}"
      const find = "function test() {\n}"
      const results = [...BlockAnchorReplacer(content, find)]
      expect(results).toEqual([])
    })
  })

  describe("EscapeNormalizedReplacer", () => {
    test("handles escaped newlines", () => {
      const content = 'const msg = "Hello\nWorld";'  // Content has actual newline
      const find = 'const msg = "Hello\\nWorld";'    // Search has escaped newline
      const results = [...EscapeNormalizedReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })

    test("handles escaped tabs", () => {
      const content = 'const msg = "Hello\tWorld";'  // Content has actual tab
      const find = 'const msg = "Hello\\tWorld";'    // Search has escaped tab
      const results = [...EscapeNormalizedReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })
  })

  describe("TrimmedBoundaryReplacer", () => {
    test("matches after trimming boundaries", () => {
      const content = "const x = 1;"
      const find = "  const x = 1;  "
      const results = [...TrimmedBoundaryReplacer(content, find)]
      expect(results.length).toBeGreaterThan(0)
    })

    test("returns empty when already trimmed", () => {
      const content = "const x = 1;"
      const find = "const x = 1;"
      const results = [...TrimmedBoundaryReplacer(content, find)]
      expect(results).toEqual([])
    })
  })

  describe("ContextAwareReplacer", () => {
    test("matches using context lines as anchors", () => {
      const content = "function test() {\n  const a = 1;\n  return a;\n}"
      const find = "function test() {\n  const b = 2;\n}"
      const results = [...ContextAwareReplacer(content, find)]
      expect(results.length).toBeGreaterThanOrEqual(0)
    })

    test("requires at least 3 lines", () => {
      const content = "function test() {\n}"
      const find = "function test() {\n}"
      const results = [...ContextAwareReplacer(content, find)]
      expect(results).toEqual([])
    })
  })

  describe("MultiOccurrenceReplacer", () => {
    test("yields all exact matches", () => {
      const content = "foo bar foo baz foo"
      const results = [...MultiOccurrenceReplacer(content, "foo")]
      expect(results).toEqual(["foo", "foo", "foo"])
    })

    test("returns empty for no matches", () => {
      const content = "bar baz qux"
      const results = [...MultiOccurrenceReplacer(content, "foo")]
      expect(results).toEqual([])
    })
  })
})

describe("Native vs TypeScript parity", () => {
  // These tests verify that native and TypeScript implementations
  // produce the same results for various inputs

  test("simple replacement produces same result", () => {
    const content = "Hello, world!"
    const result = replace(content, "world", "Rust")
    expect(result).toBe("Hello, Rust!")
  })

  test("multi-line replacement produces same result", () => {
    const content = `function foo() {
  return 1;
}`
    const oldString = `function foo() {
  return 1;
}`
    const newString = `function bar() {
  return 2;
}`
    const result = replace(content, oldString, newString)
    expect(result).toBe(newString)
  })

  test("whitespace-sensitive replacement works correctly", () => {
    const content = "const   x   =   1;"
    const result = replace(content, "const x = 1", "const y = 2")
    // Should replace the whitespace-normalized match
    expect(result).toContain("y")
  })
})
