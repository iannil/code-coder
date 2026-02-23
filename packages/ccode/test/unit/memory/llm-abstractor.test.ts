/**
 * LLM Abstractor Tests
 *
 * Tests for the LLM-Enhanced Tool Abstractor module (Phase 17).
 * These tests focus on schema validation, utility functions,
 * and mock LLM response handling.
 *
 * Note: Integration tests with actual LLM calls should be run separately
 * as they require API keys and are slower.
 */

import { describe, test, expect } from "bun:test"
import { ToolTypes } from "../../../src/memory/tools/types"

// ============================================================================
// Schema Tests for Phase 17 Types
// ============================================================================

describe("Phase 17 - LLM Abstraction Types", () => {
  describe("LLMExtractedParameter schema", () => {
    test("should validate valid parameter", () => {
      const param = {
        name: "file_path",
        type: "string" as const,
        description: "Path to the input file",
        required: true,
        extractedFrom: "line 5",
      }

      const result = ToolTypes.LLMExtractedParameter.safeParse(param)
      expect(result.success).toBe(true)
    })

    test("should validate parameter with default value", () => {
      const param = {
        name: "timeout",
        type: "number" as const,
        description: "Request timeout in seconds",
        required: false,
        defaultValue: 30,
        extractedFrom: "hardcoded",
      }

      const result = ToolTypes.LLMExtractedParameter.safeParse(param)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.defaultValue).toBe(30)
      }
    })

    test("should accept all valid types", () => {
      const types = ["string", "number", "boolean", "array", "object"] as const
      for (const type of types) {
        const param = {
          name: "test",
          type,
          description: "Test parameter",
          required: true,
          extractedFrom: "line 1",
        }
        const result = ToolTypes.LLMExtractedParameter.safeParse(param)
        expect(result.success).toBe(true)
      }
    })

    test("should reject invalid type", () => {
      const param = {
        name: "test",
        type: "invalid",
        description: "Test",
        required: true,
        extractedFrom: "line 1",
      }

      const result = ToolTypes.LLMExtractedParameter.safeParse(param)
      expect(result.success).toBe(false)
    })
  })

  describe("HardcodedValue schema", () => {
    test("should validate valid hardcoded value", () => {
      const value = {
        value: "https://api.example.com",
        line: 10,
        shouldParameterize: true,
        suggestedParamName: "api_url",
      }

      const result = ToolTypes.HardcodedValue.safeParse(value)
      expect(result.success).toBe(true)
    })

    test("should validate hardcoded value that should not be parameterized", () => {
      const value = {
        value: "utf-8",
        line: 5,
        shouldParameterize: false,
        suggestedParamName: "",
      }

      const result = ToolTypes.HardcodedValue.safeParse(value)
      expect(result.success).toBe(true)
    })

    test("should reject missing line number", () => {
      const value = {
        value: "test",
        shouldParameterize: true,
        suggestedParamName: "test_param",
      }

      const result = ToolTypes.HardcodedValue.safeParse(value)
      expect(result.success).toBe(false)
    })
  })

  describe("LLMGeneratedExample schema", () => {
    test("should validate valid example", () => {
      const example = {
        description: "Parse a JSON file",
        input: { filePath: "data.json" },
        expectedOutput: '{"key": "value"}',
      }

      const result = ToolTypes.LLMGeneratedExample.safeParse(example)
      expect(result.success).toBe(true)
    })

    test("should validate example with complex input", () => {
      const example = {
        description: "Process data with multiple parameters",
        input: {
          source: "input.csv",
          columns: ["name", "age"],
          limit: 100,
        },
        expectedOutput: "Processed 100 rows",
      }

      const result = ToolTypes.LLMGeneratedExample.safeParse(example)
      expect(result.success).toBe(true)
    })

    test("should validate example with empty input", () => {
      const example = {
        description: "Run without parameters",
        input: {},
        expectedOutput: "Done",
      }

      const result = ToolTypes.LLMGeneratedExample.safeParse(example)
      expect(result.success).toBe(true)
    })
  })

  describe("LLMAnalysisResult schema", () => {
    test("should validate complete analysis result", () => {
      const analysis = {
        purpose: "Parse a CSV file and convert to JSON",
        toolName: "parse_csv_to_json",
        parameters: [
          {
            name: "file_path",
            type: "string" as const,
            description: "Path to the CSV file",
            required: true,
            extractedFrom: "line 3",
          },
          {
            name: "delimiter",
            type: "string" as const,
            description: "CSV delimiter character",
            required: false,
            defaultValue: ",",
            extractedFrom: "hardcoded",
          },
        ],
        hardcodedValues: [
          {
            value: ",",
            line: 5,
            shouldParameterize: true,
            suggestedParamName: "delimiter",
          },
        ],
        examples: [
          {
            description: "Parse comma-separated file",
            input: { file_path: "data.csv" },
            expectedOutput: '[{"name": "Alice"}]',
          },
        ],
      }

      const result = ToolTypes.LLMAnalysisResult.safeParse(analysis)
      expect(result.success).toBe(true)
    })

    test("should validate minimal analysis result", () => {
      const analysis = {
        purpose: "Simple tool",
        toolName: "simple_tool",
        parameters: [],
        hardcodedValues: [],
        examples: [],
      }

      const result = ToolTypes.LLMAnalysisResult.safeParse(analysis)
      expect(result.success).toBe(true)
    })

    test("should validate analysis with generalized code", () => {
      const analysis = {
        purpose: "Fetch data from API",
        toolName: "fetch_api_data",
        parameters: [
          {
            name: "api_url",
            type: "string" as const,
            description: "API endpoint URL",
            required: true,
            extractedFrom: "hardcoded",
          },
        ],
        hardcodedValues: [
          {
            value: "https://api.example.com/data",
            line: 2,
            shouldParameterize: true,
            suggestedParamName: "api_url",
          },
        ],
        examples: [
          {
            description: "Fetch from default API",
            input: { api_url: "https://api.example.com/data" },
            expectedOutput: '{"data": [...]}',
          },
        ],
        generalizedCode: 'fetch("${api_url}").then(r => r.json())',
      }

      const result = ToolTypes.LLMAnalysisResult.safeParse(analysis)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.generalizedCode).toBeDefined()
      }
    })

    test("should reject missing required fields", () => {
      const analysis = {
        purpose: "Test",
        // missing toolName, parameters, hardcodedValues, examples
      }

      const result = ToolTypes.LLMAnalysisResult.safeParse(analysis)
      expect(result.success).toBe(false)
    })
  })
})

// ============================================================================
// Utility Function Tests
// ============================================================================

describe("LLM Abstractor Utilities", () => {
  describe("Tool name sanitization", () => {
    function sanitizeToolName(name: string): string {
      return name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "_")
        .replace(/^_|_$/g, "")
        .slice(0, 50)
    }

    test("should convert to lowercase", () => {
      expect(sanitizeToolName("ParseCSV")).toBe("parsecsv")
    })

    test("should replace spaces with underscores", () => {
      expect(sanitizeToolName("parse csv file")).toBe("parse_csv_file")
    })

    test("should replace special characters with underscores", () => {
      expect(sanitizeToolName("parse-csv/file")).toBe("parse_csv_file")
    })

    test("should remove leading/trailing underscores", () => {
      expect(sanitizeToolName("_parse_csv_")).toBe("parse_csv")
    })

    test("should truncate to 50 characters", () => {
      const longName = "a".repeat(60)
      expect(sanitizeToolName(longName).length).toBeLessThanOrEqual(50)
    })

    test("should handle empty string", () => {
      expect(sanitizeToolName("")).toBe("")
    })
  })

  describe("Type normalization", () => {
    function normalizeType(type: unknown): "string" | "number" | "boolean" | "array" | "object" {
      const normalized = String(type).toLowerCase()
      const validTypes = ["string", "number", "boolean", "array", "object"] as const

      for (const valid of validTypes) {
        if (normalized.includes(valid)) {
          return valid
        }
      }
      return "string"
    }

    test("should recognize string type", () => {
      expect(normalizeType("string")).toBe("string")
      expect(normalizeType("String")).toBe("string")
    })

    test("should recognize number type", () => {
      expect(normalizeType("number")).toBe("number")
      expect(normalizeType("integer")).toBe("string") // doesn't include "number"
    })

    test("should recognize boolean type", () => {
      expect(normalizeType("boolean")).toBe("boolean")
      expect(normalizeType("BOOLEAN")).toBe("boolean")
    })

    test("should recognize array type", () => {
      expect(normalizeType("array")).toBe("array")
      expect(normalizeType("Array")).toBe("array")
    })

    test("should recognize object type", () => {
      expect(normalizeType("object")).toBe("object")
    })

    test("should default to string for unknown types", () => {
      expect(normalizeType("unknown")).toBe("string")
      expect(normalizeType(undefined)).toBe("string")
      expect(normalizeType(null)).toBe("string")
    })
  })

  describe("Placeholder formatting", () => {
    function formatPlaceholder(paramName: string, language: string): string {
      switch (language) {
        case "python":
          return `{${paramName}}`
        case "nodejs":
          return `\${${paramName}}`
        case "bash":
          return `$${paramName}`
        default:
          return `{{${paramName}}}`
      }
    }

    test("should format Python placeholder", () => {
      expect(formatPlaceholder("file_path", "python")).toBe("{file_path}")
    })

    test("should format JavaScript placeholder", () => {
      expect(formatPlaceholder("file_path", "nodejs")).toBe("${file_path}")
    })

    test("should format Bash placeholder", () => {
      expect(formatPlaceholder("file_path", "bash")).toBe("$file_path")
    })

    test("should format default placeholder for unknown languages", () => {
      expect(formatPlaceholder("file_path", "ruby")).toBe("{{file_path}}")
    })
  })

  describe("Regex escaping", () => {
    function escapeRegex(str: string): string {
      return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
    }

    test("should escape special regex characters", () => {
      expect(escapeRegex("test.*")).toBe("test\\.\\*")
      expect(escapeRegex("a+b?c")).toBe("a\\+b\\?c")
    })

    test("should escape brackets", () => {
      expect(escapeRegex("[test]")).toBe("\\[test\\]")
      expect(escapeRegex("(test)")).toBe("\\(test\\)")
    })

    test("should escape backslashes", () => {
      expect(escapeRegex("test\\path")).toBe("test\\\\path")
    })

    test("should handle strings without special characters", () => {
      expect(escapeRegex("test")).toBe("test")
    })
  })
})

// ============================================================================
// Code Generalization Tests
// ============================================================================

describe("Code Generalization", () => {
  function generalizeCode(
    code: string,
    hardcodedValues: ToolTypes.HardcodedValue[],
    language: string,
  ): string {
    let result = code

    const sortedValues = [...hardcodedValues]
      .filter((h) => h.shouldParameterize && h.suggestedParamName)
      .sort((a, b) => b.line - a.line)

    for (const hv of sortedValues) {
      let placeholder: string
      switch (language) {
        case "python":
          placeholder = `{${hv.suggestedParamName}}`
          break
        case "nodejs":
          placeholder = `\${${hv.suggestedParamName}}`
          break
        case "bash":
          placeholder = `$${hv.suggestedParamName}`
          break
        default:
          placeholder = `{{${hv.suggestedParamName}}}`
      }

      const escapedValue = hv.value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
      result = result.replace(new RegExp(escapedValue, "g"), placeholder)
    }

    return result
  }

  test("should replace hardcoded URL in Python", () => {
    const code = 'response = requests.get("https://api.example.com/data")'
    const hardcoded: ToolTypes.HardcodedValue[] = [
      {
        value: "https://api.example.com/data",
        line: 1,
        shouldParameterize: true,
        suggestedParamName: "api_url",
      },
    ]

    const result = generalizeCode(code, hardcoded, "python")
    expect(result).toBe('response = requests.get("{api_url}")')
  })

  test("should replace hardcoded value in JavaScript", () => {
    const code = 'const data = fetch("https://api.example.com/data")'
    const hardcoded: ToolTypes.HardcodedValue[] = [
      {
        value: "https://api.example.com/data",
        line: 1,
        shouldParameterize: true,
        suggestedParamName: "api_url",
      },
    ]

    const result = generalizeCode(code, hardcoded, "nodejs")
    expect(result).toBe('const data = fetch("${api_url}")')
  })

  test("should replace hardcoded value in Bash", () => {
    const code = 'curl "https://api.example.com/data"'
    const hardcoded: ToolTypes.HardcodedValue[] = [
      {
        value: "https://api.example.com/data",
        line: 1,
        shouldParameterize: true,
        suggestedParamName: "api_url",
      },
    ]

    const result = generalizeCode(code, hardcoded, "bash")
    expect(result).toBe('curl "$api_url"')
  })

  test("should skip values not marked for parameterization", () => {
    const code = 'data = json.loads(content, encoding="utf-8")'
    const hardcoded: ToolTypes.HardcodedValue[] = [
      {
        value: "utf-8",
        line: 1,
        shouldParameterize: false,
        suggestedParamName: "",
      },
    ]

    const result = generalizeCode(code, hardcoded, "python")
    expect(result).toBe(code) // unchanged
  })

  test("should replace multiple hardcoded values", () => {
    const code = `
url = "https://api.example.com"
timeout = 30
response = requests.get(url, timeout=timeout)
`
    const hardcoded: ToolTypes.HardcodedValue[] = [
      {
        value: "https://api.example.com",
        line: 2,
        shouldParameterize: true,
        suggestedParamName: "api_url",
      },
      {
        value: "30",
        line: 3,
        shouldParameterize: true,
        suggestedParamName: "timeout",
      },
    ]

    const result = generalizeCode(code, hardcoded, "python")
    expect(result).toContain("{api_url}")
    expect(result).toContain("{timeout}")
  })
})

// ============================================================================
// Mock LLM Response Parsing Tests
// ============================================================================

describe("LLM Response Parsing", () => {
  function parseAnalysisResponse(text: string): ToolTypes.LLMAnalysisResult | null {
    try {
      const jsonMatch = text.match(/\{[\s\S]*\}/)
      if (!jsonMatch) {
        return null
      }

      const raw = JSON.parse(jsonMatch[0])

      const result = ToolTypes.LLMAnalysisResult.safeParse({
        purpose: raw.purpose ?? "",
        toolName: (raw.toolName ?? "")
          .toLowerCase()
          .replace(/[^a-z0-9]+/g, "_")
          .replace(/^_|_$/g, "")
          .slice(0, 50),
        parameters: (raw.parameters ?? []).map((p: Record<string, unknown>) => ({
          name: String(p.name ?? ""),
          type: (() => {
            const t = String(p.type ?? "string").toLowerCase()
            const validTypes = ["string", "number", "boolean", "array", "object"] as const
            for (const valid of validTypes) {
              if (t.includes(valid)) return valid
            }
            return "string"
          })(),
          description: String(p.description ?? ""),
          required: Boolean(p.required ?? true),
          defaultValue: p.defaultValue,
          extractedFrom: String(p.extractedFrom ?? "unknown"),
        })),
        hardcodedValues: (raw.hardcodedValues ?? []).map((h: Record<string, unknown>) => ({
          value: String(h.value ?? ""),
          line: Number(h.line ?? 0),
          shouldParameterize: Boolean(h.shouldParameterize ?? false),
          suggestedParamName: String(h.suggestedParamName ?? ""),
        })),
        examples: (raw.examples ?? []).map((e: Record<string, unknown>) => ({
          description: String(e.description ?? ""),
          input: (e.input as Record<string, unknown>) ?? {},
          expectedOutput: String(e.expectedOutput ?? ""),
        })),
      })

      return result.success ? result.data : null
    } catch {
      return null
    }
  }

  test("should parse valid JSON response", () => {
    const response = `{
      "purpose": "Parse CSV file to JSON",
      "toolName": "parse_csv",
      "parameters": [
        {
          "name": "file_path",
          "type": "string",
          "description": "Input file path",
          "required": true,
          "extractedFrom": "line 1"
        }
      ],
      "hardcodedValues": [],
      "examples": [
        {
          "description": "Basic usage",
          "input": {"file_path": "test.csv"},
          "expectedOutput": "[{...}]"
        }
      ]
    }`

    const result = parseAnalysisResponse(response)
    expect(result).not.toBeNull()
    expect(result?.toolName).toBe("parse_csv")
    expect(result?.parameters.length).toBe(1)
  })

  test("should extract JSON from markdown code block", () => {
    const response = `Here's my analysis:
\`\`\`json
{
  "purpose": "Test tool",
  "toolName": "test_tool",
  "parameters": [],
  "hardcodedValues": [],
  "examples": []
}
\`\`\`
That's my response.`

    const result = parseAnalysisResponse(response)
    expect(result).not.toBeNull()
    expect(result?.toolName).toBe("test_tool")
  })

  test("should handle malformed JSON", () => {
    const response = "This is not valid JSON { broken }"
    const result = parseAnalysisResponse(response)
    expect(result).toBeNull()
  })

  test("should handle empty response", () => {
    const response = ""
    const result = parseAnalysisResponse(response)
    expect(result).toBeNull()
  })

  test("should handle response with no JSON", () => {
    const response = "I couldn't analyze this code"
    const result = parseAnalysisResponse(response)
    expect(result).toBeNull()
  })

  test("should sanitize tool names", () => {
    const response = `{
      "purpose": "Test",
      "toolName": "Parse CSV-File!",
      "parameters": [],
      "hardcodedValues": [],
      "examples": []
    }`

    const result = parseAnalysisResponse(response)
    expect(result).not.toBeNull()
    expect(result?.toolName).toBe("parse_csv_file")
  })

  test("should normalize invalid parameter types to string", () => {
    const response = `{
      "purpose": "Test",
      "toolName": "test",
      "parameters": [
        {
          "name": "param",
          "type": "unknown_type",
          "description": "Test",
          "required": true,
          "extractedFrom": "line 1"
        }
      ],
      "hardcodedValues": [],
      "examples": []
    }`

    const result = parseAnalysisResponse(response)
    expect(result).not.toBeNull()
    expect(result?.parameters[0].type).toBe("string")
  })
})

// ============================================================================
// Conversion Tests
// ============================================================================

describe("Type Conversions", () => {
  describe("LLMExtractedParameter to ToolParameter", () => {
    function toToolParameter(param: ToolTypes.LLMExtractedParameter): ToolTypes.ToolParameter {
      return {
        name: param.name,
        type: param.type,
        description: param.description,
        required: param.required,
        default: param.defaultValue,
      }
    }

    test("should convert basic parameter", () => {
      const llmParam: ToolTypes.LLMExtractedParameter = {
        name: "file_path",
        type: "string",
        description: "Path to file",
        required: true,
        extractedFrom: "line 1",
      }

      const toolParam = toToolParameter(llmParam)
      expect(toolParam.name).toBe("file_path")
      expect(toolParam.type).toBe("string")
      expect(toolParam.required).toBe(true)
      expect(toolParam.default).toBeUndefined()
    })

    test("should preserve default value", () => {
      const llmParam: ToolTypes.LLMExtractedParameter = {
        name: "timeout",
        type: "number",
        description: "Timeout in seconds",
        required: false,
        defaultValue: 30,
        extractedFrom: "hardcoded",
      }

      const toolParam = toToolParameter(llmParam)
      expect(toolParam.default).toBe(30)
    })

    test("should drop extractedFrom field", () => {
      const llmParam: ToolTypes.LLMExtractedParameter = {
        name: "test",
        type: "string",
        description: "Test",
        required: true,
        extractedFrom: "line 5",
      }

      const toolParam = toToolParameter(llmParam)
      expect("extractedFrom" in toolParam).toBe(false)
    })
  })

  describe("LLMGeneratedExample to ToolExample", () => {
    function toToolExample(example: ToolTypes.LLMGeneratedExample): ToolTypes.ToolExample {
      return {
        description: example.description,
        input: example.input as Record<string, unknown>,
        output: example.expectedOutput,
      }
    }

    test("should convert example", () => {
      const llmExample: ToolTypes.LLMGeneratedExample = {
        description: "Basic usage",
        input: { file: "test.csv" },
        expectedOutput: "[...]",
      }

      const toolExample = toToolExample(llmExample)
      expect(toolExample.description).toBe("Basic usage")
      expect(toolExample.input).toEqual({ file: "test.csv" })
      expect(toolExample.output).toBe("[...]")
    })

    test("should rename expectedOutput to output", () => {
      const llmExample: ToolTypes.LLMGeneratedExample = {
        description: "Test",
        input: {},
        expectedOutput: "Result",
      }

      const toolExample = toToolExample(llmExample)
      expect("expectedOutput" in toolExample).toBe(false)
      expect(toolExample.output).toBe("Result")
    })
  })
})
