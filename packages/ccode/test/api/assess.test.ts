import { describe, expect, test, beforeAll, mock } from "bun:test"
import z from "zod"

/**
 * Tests for the Technical Feasibility Assessment API
 *
 * This API enables PM/non-developer workflows:
 * - IM channel asks "Is adding WeChat login complex?"
 * - CodeCoder scans codebase and returns structured assessment
 */

// Mock the formatForChannel function for unit testing
const formatForChannel = (analysis: FeasibilityAnalysis): string => {
  const complexityEmoji: Record<string, string> = {
    low: "🟢",
    medium: "🟡",
    high: "🟠",
    critical: "🔴",
  }

  const complexityLabel: Record<string, string> = {
    low: "低",
    medium: "中等",
    high: "较高",
    critical: "关键",
  }

  const lines: string[] = [
    "📊 **技术可行性评估**",
    "",
    `**需求**: ${analysis.summary}`,
    `**复杂度**: ${complexityEmoji[analysis.complexity]} ${complexityLabel[analysis.complexity]}`,
    "",
  ]

  if (analysis.existing_capabilities.length > 0) {
    lines.push("✅ **现有能力**")
    for (const cap of analysis.existing_capabilities.slice(0, 5)) {
      lines.push(`• ${cap.name} (${cap.path})`)
    }
    lines.push("")
  }

  if (analysis.required_changes.length > 0) {
    lines.push("📝 **需要修改**")
    for (const change of analysis.required_changes.slice(0, 8)) {
      const actionLabel = change.action === "create" ? "[新建]" : change.action === "modify" ? "[修改]" : "[删除]"
      lines.push(`${actionLabel} ${change.file}`)
    }
    lines.push("")
  }

  if (analysis.dependencies.length > 0) {
    lines.push("📦 **新增依赖**")
    for (const dep of analysis.dependencies.slice(0, 5)) {
      lines.push(`• ${dep.name} (${dep.type})`)
    }
    lines.push("")
  }

  if (analysis.risks.length > 0) {
    lines.push("⚠️ **风险提示**")
    for (const risk of analysis.risks.slice(0, 3)) {
      lines.push(`• ${risk}`)
    }
    lines.push("")
  }

  lines.push(`置信度: ${Math.round(analysis.confidence * 100)}%`)

  return lines.join("\n")
}

// Type definitions matching the API
interface ExistingCapability {
  name: string
  path: string
  relevance: string
}

interface RequiredChange {
  file: string
  action: "create" | "modify" | "delete"
  description: string
}

interface Dependency {
  name: string
  type: "npm" | "pip" | "cargo" | "other"
  reason: string
}

interface FeasibilityAnalysis {
  complexity: "low" | "medium" | "high" | "critical"
  summary: string
  existing_capabilities: ExistingCapability[]
  required_changes: RequiredChange[]
  dependencies: Dependency[]
  risks: string[]
  confidence: number
}

// Zod schemas for validation
const FeasibilityRequest = z.object({
  query: z.string().min(1),
  project_path: z.string().optional(),
  options: z
    .object({
      depth: z.enum(["quick", "standard", "deep"]).default("standard"),
      include_code_refs: z.boolean().default(true),
      language: z.enum(["zh-CN", "en-US"]).default("zh-CN"),
    })
    .optional(),
})

const FeasibilityAnalysisSchema = z.object({
  complexity: z.enum(["low", "medium", "high", "critical"]),
  summary: z.string(),
  existing_capabilities: z.array(
    z.object({
      name: z.string(),
      path: z.string(),
      relevance: z.string(),
    }),
  ),
  required_changes: z.array(
    z.object({
      file: z.string(),
      action: z.enum(["create", "modify", "delete"]),
      description: z.string(),
    }),
  ),
  dependencies: z.array(
    z.object({
      name: z.string(),
      type: z.enum(["npm", "pip", "cargo", "other"]),
      reason: z.string(),
    }),
  ),
  risks: z.array(z.string()),
  confidence: z.number().min(0).max(1),
})

describe("Feasibility Assessment API", () => {
  describe("Request Validation", () => {
    test("should validate a minimal request", () => {
      const request = {
        query: "增加微信扫码登录功能，复杂度高吗？",
      }

      const result = FeasibilityRequest.safeParse(request)
      expect(result.success).toBe(true)
    })

    test("should validate a full request with options", () => {
      const request = {
        query: "增加微信扫码登录功能，复杂度高吗？",
        project_path: "/path/to/project",
        options: {
          depth: "deep",
          include_code_refs: true,
          language: "zh-CN",
        },
      }

      const result = FeasibilityRequest.safeParse(request)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.options?.depth).toBe("deep")
      }
    })

    test("should reject empty query", () => {
      const request = {
        query: "",
      }

      const result = FeasibilityRequest.safeParse(request)
      expect(result.success).toBe(false)
    })

    test("should apply default options", () => {
      const request = {
        query: "test query",
        options: {},
      }

      const result = FeasibilityRequest.safeParse(request)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.options?.depth).toBe("standard")
        expect(result.data.options?.include_code_refs).toBe(true)
        expect(result.data.options?.language).toBe("zh-CN")
      }
    })
  })

  describe("Response Schema Validation", () => {
    test("should validate a complete analysis response", () => {
      const analysis = {
        complexity: "low",
        summary: "低风险，预计改动 3 个文件",
        existing_capabilities: [
          {
            name: "Auth 模块",
            path: "src/auth/",
            relevance: "OAuth2.0 基础框架已存在",
          },
        ],
        required_changes: [
          {
            file: "src/auth/providers/wechat.ts",
            action: "create",
            description: "新建微信 OAuth 提供商",
          },
          {
            file: "src/auth/config.ts",
            action: "modify",
            description: "添加微信配置项",
          },
        ],
        dependencies: [
          {
            name: "wechat-oauth",
            type: "npm",
            reason: "微信 OAuth SDK",
          },
        ],
        risks: ["需要申请微信开放平台应用"],
        confidence: 0.85,
      }

      const result = FeasibilityAnalysisSchema.safeParse(analysis)
      expect(result.success).toBe(true)
    })

    test("should validate empty arrays", () => {
      const analysis = {
        complexity: "low",
        summary: "简单变更",
        existing_capabilities: [],
        required_changes: [],
        dependencies: [],
        risks: [],
        confidence: 0.95,
      }

      const result = FeasibilityAnalysisSchema.safeParse(analysis)
      expect(result.success).toBe(true)
    })

    test("should reject invalid complexity", () => {
      const analysis = {
        complexity: "extreme", // invalid
        summary: "test",
        existing_capabilities: [],
        required_changes: [],
        dependencies: [],
        risks: [],
        confidence: 0.5,
      }

      const result = FeasibilityAnalysisSchema.safeParse(analysis)
      expect(result.success).toBe(false)
    })

    test("should reject confidence out of range", () => {
      const analysis = {
        complexity: "low",
        summary: "test",
        existing_capabilities: [],
        required_changes: [],
        dependencies: [],
        risks: [],
        confidence: 1.5, // invalid
      }

      const result = FeasibilityAnalysisSchema.safeParse(analysis)
      expect(result.success).toBe(false)
    })
  })

  describe("Channel Formatting", () => {
    test("should format low complexity response", () => {
      const analysis: FeasibilityAnalysis = {
        complexity: "low",
        summary: "增加微信登录功能",
        existing_capabilities: [
          { name: "Auth模块", path: "src/auth/", relevance: "OAuth基础设施" },
        ],
        required_changes: [
          { file: "src/auth/wechat.ts", action: "create", description: "新建微信OAuth" },
        ],
        dependencies: [{ name: "wechat-oauth", type: "npm", reason: "微信SDK" }],
        risks: ["需要申请微信开放平台"],
        confidence: 0.85,
      }

      const formatted = formatForChannel(analysis)

      expect(formatted).toContain("📊 **技术可行性评估**")
      expect(formatted).toContain("🟢 低")
      expect(formatted).toContain("Auth模块")
      expect(formatted).toContain("[新建]")
      expect(formatted).toContain("wechat-oauth")
      expect(formatted).toContain("需要申请微信开放平台")
      expect(formatted).toContain("85%")
    })

    test("should format high complexity response", () => {
      const analysis: FeasibilityAnalysis = {
        complexity: "high",
        summary: "重构整个认证系统",
        existing_capabilities: [],
        required_changes: [
          { file: "src/auth/index.ts", action: "modify", description: "重构入口" },
          { file: "src/auth/legacy.ts", action: "delete", description: "删除旧代码" },
        ],
        dependencies: [],
        risks: ["需要全面回归测试", "可能影响现有用户"],
        confidence: 0.6,
      }

      const formatted = formatForChannel(analysis)

      expect(formatted).toContain("🟠 较高")
      expect(formatted).toContain("[修改]")
      expect(formatted).toContain("[删除]")
      expect(formatted).toContain("60%")
    })

    test("should handle empty sections gracefully", () => {
      const analysis: FeasibilityAnalysis = {
        complexity: "low",
        summary: "简单变更",
        existing_capabilities: [],
        required_changes: [],
        dependencies: [],
        risks: [],
        confidence: 0.95,
      }

      const formatted = formatForChannel(analysis)

      expect(formatted).toContain("📊 **技术可行性评估**")
      expect(formatted).toContain("🟢 低")
      expect(formatted).not.toContain("✅ **现有能力**")
      expect(formatted).not.toContain("📝 **需要修改**")
      expect(formatted).not.toContain("📦 **新增依赖**")
      expect(formatted).not.toContain("⚠️ **风险提示**")
      expect(formatted).toContain("95%")
    })

    test("should truncate long lists", () => {
      const analysis: FeasibilityAnalysis = {
        complexity: "medium",
        summary: "大规模重构",
        existing_capabilities: Array.from({ length: 10 }, (_, i) => ({
          name: `模块${i}`,
          path: `src/mod${i}/`,
          relevance: `相关${i}`,
        })),
        required_changes: Array.from({ length: 15 }, (_, i) => ({
          file: `src/file${i}.ts`,
          action: "modify" as const,
          description: `修改${i}`,
        })),
        dependencies: Array.from({ length: 8 }, (_, i) => ({
          name: `pkg${i}`,
          type: "npm" as const,
          reason: `原因${i}`,
        })),
        risks: Array.from({ length: 5 }, (_, i) => `风险项${i}`),
        confidence: 0.7,
      }

      const formatted = formatForChannel(analysis)

      // Should truncate to 5 capabilities
      expect(formatted.match(/模块/g)?.length).toBe(5)

      // Should truncate to 8 changes
      expect(formatted.match(/\[修改\]/g)?.length).toBe(8)

      // Should truncate to 5 dependencies
      expect(formatted.match(/pkg/g)?.length).toBe(5)

      // Should truncate to 3 risks (risk items only, excluding header)
      expect(formatted.match(/风险项/g)?.length).toBe(3)
    })
  })

  describe("Complexity Labels", () => {
    test.each([
      ["low", "🟢", "低"],
      ["medium", "🟡", "中等"],
      ["high", "🟠", "较高"],
      ["critical", "🔴", "关键"],
    ])("should format %s complexity correctly", (complexity, emoji, label) => {
      const analysis: FeasibilityAnalysis = {
        complexity: complexity as FeasibilityAnalysis["complexity"],
        summary: "测试",
        existing_capabilities: [],
        required_changes: [],
        dependencies: [],
        risks: [],
        confidence: 0.5,
      }

      const formatted = formatForChannel(analysis)
      expect(formatted).toContain(emoji)
      expect(formatted).toContain(label)
    })
  })
})
