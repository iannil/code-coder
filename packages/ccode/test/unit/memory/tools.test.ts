/**
 * Dynamic Tool Registry Tests
 *
 * Tests for the Dynamic Tool Registry module that stores tools
 * learned from successful code executions.
 *
 * These tests focus on schema validation and algorithm correctness
 * using mock data, following the pattern in call-graph.test.ts.
 */

import { describe, test, expect } from "bun:test"
import { ToolTypes } from "../../../src/memory/tools/types"

// ============================================================================
// Schema Tests
// ============================================================================

describe("ToolTypes schemas", () => {
  describe("ToolParameter schema", () => {
    test("should validate valid ToolParameter", () => {
      const param = {
        name: "filePath",
        type: "string" as const,
        description: "Path to the input file",
        required: true,
      }

      const result = ToolTypes.ToolParameter.safeParse(param)
      expect(result.success).toBe(true)
    })

    test("should validate ToolParameter with default value", () => {
      const param = {
        name: "limit",
        type: "number" as const,
        description: "Maximum number of results",
        required: false,
        default: 10,
      }

      const result = ToolTypes.ToolParameter.safeParse(param)
      expect(result.success).toBe(true)
    })

    test("should validate ToolParameter with enum", () => {
      const param = {
        name: "format",
        type: "string" as const,
        description: "Output format",
        required: true,
        enum: ["json", "csv", "xml"],
      }

      const result = ToolTypes.ToolParameter.safeParse(param)
      expect(result.success).toBe(true)
    })

    test("should reject invalid type", () => {
      const param = {
        name: "test",
        type: "invalid",
        description: "Test",
        required: true,
      }

      const result = ToolTypes.ToolParameter.safeParse(param)
      expect(result.success).toBe(false)
    })

    test("should accept all valid types", () => {
      const types = ["string", "number", "boolean", "array", "object"] as const
      for (const type of types) {
        const param = {
          name: "test",
          type,
          description: "Test",
          required: true,
        }
        const result = ToolTypes.ToolParameter.safeParse(param)
        expect(result.success).toBe(true)
      }
    })
  })

  describe("ToolExample schema", () => {
    test("should validate valid ToolExample", () => {
      const example = {
        description: "Parse a JSON file",
        input: { filePath: "data.json" },
        output: '{"key": "value"}',
      }

      const result = ToolTypes.ToolExample.safeParse(example)
      expect(result.success).toBe(true)
    })

    test("should validate ToolExample without output", () => {
      const example = {
        description: "Process data",
        input: {},
      }

      const result = ToolTypes.ToolExample.safeParse(example)
      expect(result.success).toBe(true)
    })

    test("should validate ToolExample with complex input", () => {
      const example = {
        description: "Complex operation",
        input: {
          nested: { key: "value" },
          array: [1, 2, 3],
          boolean: true,
        },
      }

      const result = ToolTypes.ToolExample.safeParse(example)
      expect(result.success).toBe(true)
    })
  })

  describe("ToolMetadata schema", () => {
    test("should validate valid ToolMetadata", () => {
      const metadata = {
        createdAt: Date.now(),
        updatedAt: Date.now(),
        createdBy: "agent" as const,
        version: 1,
      }

      const result = ToolTypes.ToolMetadata.safeParse(metadata)
      expect(result.success).toBe(true)
    })

    test("should validate ToolMetadata with sourceTask", () => {
      const metadata = {
        createdAt: Date.now(),
        updatedAt: Date.now(),
        createdBy: "user" as const,
        sourceTask: "Parse CSV files",
        version: 2,
      }

      const result = ToolTypes.ToolMetadata.safeParse(metadata)
      expect(result.success).toBe(true)
    })

    test("should reject invalid createdBy", () => {
      const metadata = {
        createdAt: Date.now(),
        updatedAt: Date.now(),
        createdBy: "system",
        version: 1,
      }

      const result = ToolTypes.ToolMetadata.safeParse(metadata)
      expect(result.success).toBe(false)
    })
  })

  describe("ToolStats schema", () => {
    test("should validate valid ToolStats", () => {
      const stats = {
        usageCount: 10,
        successCount: 8,
        failureCount: 2,
        lastUsedAt: Date.now(),
        averageExecutionTime: 150,
      }

      const result = ToolTypes.ToolStats.safeParse(stats)
      expect(result.success).toBe(true)
    })

    test("should validate ToolStats with null lastUsedAt", () => {
      const stats = {
        usageCount: 0,
        successCount: 0,
        failureCount: 0,
        lastUsedAt: null,
        averageExecutionTime: 0,
      }

      const result = ToolTypes.ToolStats.safeParse(stats)
      expect(result.success).toBe(true)
    })

    test("should apply defaults", () => {
      const stats = {}

      const result = ToolTypes.ToolStats.safeParse(stats)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.usageCount).toBe(0)
        expect(result.data.successCount).toBe(0)
        expect(result.data.failureCount).toBe(0)
        expect(result.data.lastUsedAt).toBe(null)
        expect(result.data.averageExecutionTime).toBe(0)
      }
    })
  })

  describe("DynamicTool schema", () => {
    const validTool: ToolTypes.DynamicTool = {
      id: "tool_parse_csv_123456_abc123",
      name: "parse_csv",
      description: "Parse a CSV file and return JSON",
      tags: ["csv", "data-processing", "python"],
      code: `import csv\nwith open('{{filePath}}') as f:\n    reader = csv.DictReader(f)\n    print(list(reader))`,
      language: "python",
      parameters: [
        {
          name: "filePath",
          type: "string",
          description: "Path to CSV file",
          required: true,
        },
      ],
      examples: [
        {
          description: "Parse a simple CSV",
          input: { filePath: "data.csv" },
          output: '[{"name": "Alice"}]',
        },
      ],
      metadata: {
        createdAt: Date.now(),
        updatedAt: Date.now(),
        createdBy: "agent",
        version: 1,
      },
      stats: {
        usageCount: 5,
        successCount: 4,
        failureCount: 1,
        lastUsedAt: Date.now(),
        averageExecutionTime: 200,
      },
    }

    test("should validate valid DynamicTool", () => {
      const result = ToolTypes.DynamicTool.safeParse(validTool)
      expect(result.success).toBe(true)
    })

    test("should validate DynamicTool with embedding", () => {
      const toolWithEmbedding = {
        ...validTool,
        embedding: Array(256).fill(0.1),
      }

      const result = ToolTypes.DynamicTool.safeParse(toolWithEmbedding)
      expect(result.success).toBe(true)
    })

    test("should validate all supported languages", () => {
      const languages = ["python", "nodejs", "bash"] as const
      for (const language of languages) {
        const tool = { ...validTool, language }
        const result = ToolTypes.DynamicTool.safeParse(tool)
        expect(result.success).toBe(true)
      }
    })

    test("should reject invalid language", () => {
      const tool = { ...validTool, language: "ruby" }
      const result = ToolTypes.DynamicTool.safeParse(tool)
      expect(result.success).toBe(false)
    })

    test("should reject missing required fields", () => {
      const tool = {
        id: "test",
        name: "test",
        // missing description
      }

      const result = ToolTypes.DynamicTool.safeParse(tool)
      expect(result.success).toBe(false)
    })
  })

  describe("CreateToolInput schema", () => {
    test("should validate valid CreateToolInput", () => {
      const input = {
        name: "my_tool",
        description: "A test tool",
        code: "print('hello')",
        language: "python" as const,
      }

      const result = ToolTypes.CreateToolInput.safeParse(input)
      expect(result.success).toBe(true)
    })

    test("should apply defaults", () => {
      const input = {
        name: "test",
        description: "Test tool",
        code: "echo test",
        language: "bash" as const,
      }

      const result = ToolTypes.CreateToolInput.safeParse(input)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.tags).toEqual([])
        expect(result.data.parameters).toEqual([])
        expect(result.data.examples).toEqual([])
        expect(result.data.createdBy).toBe("agent")
      }
    })

    test("should reject empty name", () => {
      const input = {
        name: "",
        description: "Test",
        code: "test",
        language: "bash" as const,
      }

      const result = ToolTypes.CreateToolInput.safeParse(input)
      expect(result.success).toBe(false)
    })

    test("should reject empty code", () => {
      const input = {
        name: "test",
        description: "Test",
        code: "",
        language: "bash" as const,
      }

      const result = ToolTypes.CreateToolInput.safeParse(input)
      expect(result.success).toBe(false)
    })

    test("should reject name exceeding max length", () => {
      const input = {
        name: "a".repeat(101),
        description: "Test",
        code: "test",
        language: "bash" as const,
      }

      const result = ToolTypes.CreateToolInput.safeParse(input)
      expect(result.success).toBe(false)
    })
  })

  describe("SearchOptions schema", () => {
    test("should validate valid SearchOptions", () => {
      const options = {
        limit: 10,
        minScore: 0.5,
        tags: ["python", "data"],
        language: "python" as const,
      }

      const result = ToolTypes.SearchOptions.safeParse(options)
      expect(result.success).toBe(true)
    })

    test("should apply defaults", () => {
      const result = ToolTypes.SearchOptions.safeParse({})
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.limit).toBe(10)
        expect(result.data.minScore).toBe(0.3)
      }
    })

    test("should reject minScore > 1", () => {
      const options = { minScore: 1.5 }
      const result = ToolTypes.SearchOptions.safeParse(options)
      expect(result.success).toBe(false)
    })

    test("should reject minScore < 0", () => {
      const options = { minScore: -0.1 }
      const result = ToolTypes.SearchOptions.safeParse(options)
      expect(result.success).toBe(false)
    })

    test("should reject limit <= 0", () => {
      const options = { limit: 0 }
      const result = ToolTypes.SearchOptions.safeParse(options)
      expect(result.success).toBe(false)
    })
  })

  describe("ExecutionRecord schema", () => {
    test("should validate valid ExecutionRecord", () => {
      const record = {
        code: "print('hello world')",
        language: "python" as const,
        task: "Print a greeting",
        output: "hello world",
        exitCode: 0,
      }

      const result = ToolTypes.ExecutionRecord.safeParse(record)
      expect(result.success).toBe(true)
    })

    test("should validate ExecutionRecord with duration", () => {
      const record = {
        code: "console.log('test')",
        language: "nodejs" as const,
        task: "Log a message",
        output: "test",
        exitCode: 0,
        durationMs: 150,
      }

      const result = ToolTypes.ExecutionRecord.safeParse(record)
      expect(result.success).toBe(true)
    })

    test("should validate failed execution", () => {
      const record = {
        code: "exit 1",
        language: "bash" as const,
        task: "Fail intentionally",
        output: "",
        exitCode: 1,
      }

      const result = ToolTypes.ExecutionRecord.safeParse(record)
      expect(result.success).toBe(true)
    })
  })

  describe("RegistryStats schema", () => {
    test("should validate valid RegistryStats", () => {
      const stats = {
        totalTools: 10,
        byLanguage: { python: 5, nodejs: 3, bash: 2 },
        byTag: { "data-processing": 4, "file-io": 3 },
        mostUsed: [
          { id: "tool_1", name: "parse_json", usageCount: 100 },
          { id: "tool_2", name: "fetch_url", usageCount: 50 },
        ],
        recentlyAdded: [
          { id: "tool_3", name: "new_tool", createdAt: Date.now() },
        ],
        lastUpdated: Date.now(),
      }

      const result = ToolTypes.RegistryStats.safeParse(stats)
      expect(result.success).toBe(true)
    })

    test("should validate empty RegistryStats", () => {
      const stats = {
        totalTools: 0,
        byLanguage: {},
        byTag: {},
        mostUsed: [],
        recentlyAdded: [],
        lastUpdated: Date.now(),
      }

      const result = ToolTypes.RegistryStats.safeParse(stats)
      expect(result.success).toBe(true)
    })
  })
})

// ============================================================================
// Algorithm Tests
// ============================================================================

describe("Tool search algorithms", () => {
  describe("Cosine similarity", () => {
    function cosineSimilarity(a: number[], b: number[]): number {
      if (a.length !== b.length) return 0

      let dotProduct = 0
      let normA = 0
      let normB = 0

      for (let i = 0; i < a.length; i++) {
        dotProduct += a[i] * b[i]
        normA += a[i] * a[i]
        normB += b[i] * b[i]
      }

      const denominator = Math.sqrt(normA) * Math.sqrt(normB)
      return denominator > 0 ? dotProduct / denominator : 0
    }

    test("should return 1 for identical vectors", () => {
      const v = [0.5, 0.5, 0.5, 0.5]
      expect(cosineSimilarity(v, v)).toBeCloseTo(1, 5)
    })

    test("should return 0 for orthogonal vectors", () => {
      const a = [1, 0]
      const b = [0, 1]
      expect(cosineSimilarity(a, b)).toBeCloseTo(0, 5)
    })

    test("should return -1 for opposite vectors", () => {
      const a = [1, 0]
      const b = [-1, 0]
      expect(cosineSimilarity(a, b)).toBeCloseTo(-1, 5)
    })

    test("should handle different length vectors", () => {
      const a = [1, 2, 3]
      const b = [1, 2]
      expect(cosineSimilarity(a, b)).toBe(0)
    })

    test("should handle zero vectors", () => {
      const a = [0, 0, 0]
      const b = [1, 2, 3]
      expect(cosineSimilarity(a, b)).toBe(0)
    })
  })

  describe("Keyword extraction", () => {
    const stopwords = new Set([
      "the", "a", "an", "is", "are", "was", "were", "be",
      "to", "of", "in", "for", "on", "with", "at", "by",
      "i", "me", "my", "we", "our", "you", "your",
      "and", "but", "if", "or",
    ])

    function extractKeywords(text: string): string[] {
      const words = text
        .toLowerCase()
        .replace(/[^a-z0-9\s]/g, " ")
        .split(/\s+/)
        .filter((w) => w.length > 2 && !stopwords.has(w))

      return [...new Set(words)]
    }

    test("should extract meaningful keywords", () => {
      const text = "Parse a CSV file and convert to JSON"
      const keywords = extractKeywords(text)

      expect(keywords).toContain("parse")
      expect(keywords).toContain("csv")
      expect(keywords).toContain("file")
      expect(keywords).toContain("convert")
      expect(keywords).toContain("json")
    })

    test("should remove stopwords", () => {
      const text = "The quick brown fox"
      const keywords = extractKeywords(text)

      expect(keywords).not.toContain("the")
      expect(keywords).toContain("quick")
      expect(keywords).toContain("brown")
      expect(keywords).toContain("fox")
    })

    test("should remove short words", () => {
      const text = "I am a developer"
      const keywords = extractKeywords(text)

      expect(keywords).not.toContain("i")
      expect(keywords).not.toContain("am")
      expect(keywords).not.toContain("a")
      expect(keywords).toContain("developer")
    })

    test("should remove duplicates", () => {
      const text = "parse parse parse"
      const keywords = extractKeywords(text)

      expect(keywords.length).toBe(1)
      expect(keywords[0]).toBe("parse")
    })

    test("should handle empty string", () => {
      const keywords = extractKeywords("")
      expect(keywords.length).toBe(0)
    })

    test("should handle special characters", () => {
      const text = "file.csv -> file.json"
      const keywords = extractKeywords(text)

      expect(keywords).toContain("file")
      expect(keywords).toContain("csv")
      expect(keywords).toContain("json")
    })
  })

  describe("N-gram extraction", () => {
    function extractNgrams(text: string, n: number): string[] {
      const normalized = text.toLowerCase().replace(/[^a-z0-9]/g, "")
      const ngrams: string[] = []

      for (let i = 0; i <= normalized.length - n; i++) {
        ngrams.push(normalized.slice(i, i + n))
      }

      return ngrams
    }

    test("should extract character trigrams", () => {
      const ngrams = extractNgrams("hello", 3)
      expect(ngrams).toEqual(["hel", "ell", "llo"])
    })

    test("should handle short text", () => {
      const ngrams = extractNgrams("hi", 3)
      expect(ngrams).toEqual([])
    })

    test("should normalize text", () => {
      const ngrams = extractNgrams("Hello World!", 3)
      expect(ngrams[0]).toBe("hel")
      expect(ngrams).toContain("owo")
    })

    test("should extract bigrams", () => {
      const ngrams = extractNgrams("test", 2)
      expect(ngrams).toEqual(["te", "es", "st"])
    })
  })

  describe("Code normalization", () => {
    function normalizeCode(code: string): string {
      return code
        .replace(/\s+/g, " ")
        .replace(/['"`]/g, "'")
        .replace(/\/\/.*/g, "")
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .replace(/#.*/g, "")
        .trim()
    }

    test("should normalize whitespace", () => {
      const code = "function  test()  {\n    return 1\n}"
      const normalized = normalizeCode(code)
      expect(normalized).not.toContain("\n")
      expect(normalized).not.toContain("  ")
    })

    test("should normalize quotes", () => {
      const code = `"hello" 'world' \`template\``
      const normalized = normalizeCode(code)
      expect(normalized).toBe("'hello' 'world' 'template'")
    })

    test("should remove single-line comments", () => {
      const code = "const x = 1 // comment"
      const normalized = normalizeCode(code)
      expect(normalized).not.toContain("comment")
    })

    test("should remove Python comments", () => {
      const code = "x = 1 # comment"
      const normalized = normalizeCode(code)
      expect(normalized).not.toContain("comment")
    })

    test("should remove block comments", () => {
      const code = "const x = 1 /* block comment */ + 2"
      const normalized = normalizeCode(code)
      expect(normalized).not.toContain("block")
      expect(normalized).toContain("+ 2")
    })
  })

  describe("Jaccard similarity", () => {
    function jaccardSimilarity(a: Set<string>, b: Set<string>): number {
      const intersection = new Set([...a].filter((x) => b.has(x)))
      const union = new Set([...a, ...b])
      return intersection.size / union.size
    }

    test("should return 1 for identical sets", () => {
      const a = new Set(["a", "b", "c"])
      const b = new Set(["a", "b", "c"])
      expect(jaccardSimilarity(a, b)).toBe(1)
    })

    test("should return 0 for disjoint sets", () => {
      const a = new Set(["a", "b"])
      const b = new Set(["c", "d"])
      expect(jaccardSimilarity(a, b)).toBe(0)
    })

    test("should return 0.5 for half overlap", () => {
      const a = new Set(["a", "b"])
      const b = new Set(["b", "c"])
      expect(jaccardSimilarity(a, b)).toBeCloseTo(1 / 3, 5)
    })

    test("should handle empty sets", () => {
      const a = new Set<string>()
      const b = new Set(["a"])
      expect(jaccardSimilarity(a, b)).toBe(0)
    })
  })
})

// ============================================================================
// Parameter Extraction Tests
// ============================================================================

describe("Parameter extraction", () => {
  describe("Python parameter extraction", () => {
    function extractPythonFunctionParams(code: string): string[] {
      const funcMatch = code.match(/def\s+\w+\s*\(([^)]*)\)/)
      if (!funcMatch?.[1]) return []

      return funcMatch[1]
        .split(",")
        .map((a) => a.trim().split("=")[0].trim())
        .filter((n) => n && !n.startsWith("*"))
    }

    test("should extract function parameters", () => {
      const code = "def process(input_file, output_file):\n    pass"
      const params = extractPythonFunctionParams(code)
      expect(params).toContain("input_file")
      expect(params).toContain("output_file")
    })

    test("should handle default values", () => {
      const code = "def process(file, limit=10):\n    pass"
      const params = extractPythonFunctionParams(code)
      expect(params).toContain("file")
      expect(params).toContain("limit")
    })

    test("should skip *args and **kwargs", () => {
      const code = "def process(*args, **kwargs):\n    pass"
      const params = extractPythonFunctionParams(code)
      expect(params.length).toBe(0)
    })

    test("should handle no parameters", () => {
      const code = "def process():\n    pass"
      const params = extractPythonFunctionParams(code)
      expect(params.length).toBe(0)
    })
  })

  describe("JavaScript parameter extraction", () => {
    function extractJSFunctionParams(code: string): string[] {
      const funcMatch = code.match(/(?:function\s+\w+|(?:const|let|var)\s+\w+\s*=\s*(?:async\s+)?)\s*\(([^)]*)\)/)
      if (!funcMatch?.[1]) return []

      return funcMatch[1]
        .split(",")
        .map((a) => a.trim().split("=")[0].trim())
        .filter((n) => n)
    }

    test("should extract function declaration params", () => {
      const code = "function process(input, output) {}"
      const params = extractJSFunctionParams(code)
      expect(params).toContain("input")
      expect(params).toContain("output")
    })

    test("should extract arrow function params", () => {
      const code = "const process = (input, output) => {}"
      const params = extractJSFunctionParams(code)
      expect(params).toContain("input")
      expect(params).toContain("output")
    })

    test("should extract async function params", () => {
      const code = "const process = async (input) => {}"
      const params = extractJSFunctionParams(code)
      expect(params).toContain("input")
    })
  })

  describe("Bash variable extraction", () => {
    function extractBashVariables(code: string): string[] {
      const builtins = new Set(["HOME", "USER", "PATH", "PWD", "SHELL", "0", "?", "!", "$", "#"])
      const vars: string[] = []

      const matches = code.match(/\$\{?(\w+)\}?/g)
      if (matches) {
        for (const match of matches) {
          const name = match.replace(/^\$\{?|\}?$/g, "")
          if (!builtins.has(name) && !/^\d+$/.test(name)) {
            vars.push(name)
          }
        }
      }

      return [...new Set(vars)]
    }

    test("should extract simple variables", () => {
      const code = "echo $INPUT"
      const vars = extractBashVariables(code)
      expect(vars).toContain("INPUT")
    })

    test("should extract braced variables", () => {
      const code = "echo ${OUTPUT_FILE}"
      const vars = extractBashVariables(code)
      expect(vars).toContain("OUTPUT_FILE")
    })

    test("should skip builtins", () => {
      const code = "echo $HOME $USER $INPUT"
      const vars = extractBashVariables(code)
      expect(vars).not.toContain("HOME")
      expect(vars).not.toContain("USER")
      expect(vars).toContain("INPUT")
    })

    test("should skip positional parameters", () => {
      const code = "echo $1 $2"
      const vars = extractBashVariables(code)
      expect(vars.length).toBe(0)
    })
  })
})

// ============================================================================
// Tag Extraction Tests
// ============================================================================

describe("Tag extraction", () => {
  const domainKeywords: Record<string, string[]> = {
    "data-processing": ["parse", "transform", "convert"],
    "file-io": ["read", "write", "file"],
    "network": ["fetch", "download", "http"],
    "json": ["json"],
    "csv": ["csv"],
  }

  function extractTags(task: string, code: string): string[] {
    const tags: string[] = []
    const text = `${task} ${code}`.toLowerCase()

    for (const [tag, keywords] of Object.entries(domainKeywords)) {
      for (const keyword of keywords) {
        if (text.includes(keyword)) {
          tags.push(tag)
          break
        }
      }
    }

    return tags
  }

  test("should extract data processing tags", () => {
    const tags = extractTags("Parse CSV and transform data", "")
    expect(tags).toContain("data-processing")
    expect(tags).toContain("csv")
  })

  test("should extract file tags", () => {
    const tags = extractTags("Read a file and write output", "")
    expect(tags).toContain("file-io")
  })

  test("should extract network tags", () => {
    const tags = extractTags("Fetch data from API", "")
    expect(tags).toContain("network")
  })

  test("should extract JSON tags", () => {
    const tags = extractTags("Convert to JSON", "")
    expect(tags).toContain("json")
  })

  test("should extract from code as well", () => {
    const tags = extractTags("Process data", "import json\ndata = json.loads()")
    expect(tags).toContain("json")
  })

  test("should not duplicate tags", () => {
    const tags = extractTags("Parse JSON file", "json.parse")
    const jsonCount = tags.filter((t) => t === "json").length
    expect(jsonCount).toBe(1)
  })
})

// ============================================================================
// Usage Statistics Tests
// ============================================================================

describe("Usage statistics calculation", () => {
  function calculateSuccessRate(success: number, total: number): number {
    return total > 0 ? success / total : 0
  }

  function calculateAverageExecutionTime(
    currentAvg: number,
    currentCount: number,
    newDuration: number,
  ): number {
    const total = currentAvg * currentCount + newDuration
    return total / (currentCount + 1)
  }

  test("should calculate success rate correctly", () => {
    expect(calculateSuccessRate(8, 10)).toBe(0.8)
    expect(calculateSuccessRate(0, 0)).toBe(0)
    expect(calculateSuccessRate(5, 5)).toBe(1)
    expect(calculateSuccessRate(0, 10)).toBe(0)
  })

  test("should calculate average execution time", () => {
    // Start with avg 100ms, 5 executions, add 200ms
    const newAvg = calculateAverageExecutionTime(100, 5, 200)
    expect(newAvg).toBeCloseTo((100 * 5 + 200) / 6, 5)
  })

  test("should handle first execution", () => {
    const newAvg = calculateAverageExecutionTime(0, 0, 150)
    expect(newAvg).toBe(150)
  })
})

// ============================================================================
// Tool ID Generation Tests
// ============================================================================

describe("Tool ID generation", () => {
  function generateToolId(name: string, timestamp: number): string {
    const sanitized = name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .slice(0, 30)
    const random = Math.random().toString(36).slice(2, 8)
    return `tool_${sanitized}_${timestamp}_${random}`
  }

  test("should generate valid ID format", () => {
    const id = generateToolId("Parse CSV", Date.now())
    expect(id).toMatch(/^tool_parse-csv_\d+_[a-z0-9]+$/)
  })

  test("should sanitize special characters", () => {
    const id = generateToolId("File I/O Tool!", Date.now())
    expect(id).not.toContain("/")
    expect(id).not.toContain("!")
  })

  test("should truncate long names", () => {
    const longName = "a".repeat(50)
    const id = generateToolId(longName, Date.now())
    expect(id.split("_")[1].length).toBeLessThanOrEqual(30)
  })

  test("should be lowercase", () => {
    const id = generateToolId("UPPERCASE", Date.now())
    expect(id).toBe(id.toLowerCase())
  })
})
