/**
 * Task Classification Types
 *
 * Defines types for intelligent task classification that routes
 * messages to appropriate execution loops.
 */

import z from "zod"

/** Supported task types */
export type TaskType = "implementation" | "research" | "query" | "acceptance" | "fix" | "other"

/** Classification result schema */
export const ClassificationResultSchema = z.object({
  type: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
  confidence: z.number().min(0).max(1),
  reasoning: z.string(),
  /** Research-specific: identified topic */
  researchTopic: z.string().optional(),
  /** Research-specific: suggested data sources */
  suggestedSources: z.array(z.string()).optional(),
  /** Research-specific: is this a periodic task? */
  isPeriodic: z.boolean().optional(),
})

export type ClassificationResult = z.infer<typeof ClassificationResultSchema>

/** Keywords for quick pre-classification */
export const RESEARCH_KEYWORDS = [
  // Chinese
  "梳理", "分析", "研究", "调研", "走势", "行情",
  "汇总", "总结", "对比", "评估", "趋势", "预测",
  "盘点", "回顾", "展望", "解读", "综述",
  // English
  "analyze", "research", "trend", "summary", "review",
  "compare", "evaluate", "forecast", "outlook",
] as const

export const IMPLEMENTATION_KEYWORDS = [
  // Chinese
  "实现", "创建", "修复", "开发", "构建", "编写", "生成", "执行",
  "部署", "配置", "设置", "安装", "更新", "修改", "重构", "优化",
  "自动", "定时", "调度",
  // English
  "implement", "create", "fix", "build", "write", "generate", "execute",
  "deploy", "configure", "setup", "install", "update", "modify", "refactor",
  "automate", "schedule", "cron",
] as const

export const QUERY_KEYWORDS = [
  // Chinese
  "什么是", "为什么", "怎么", "如何", "哪些", "哪个",
  "是什么", "能否", "可以吗", "有没有",
  // English
  "what is", "why", "how", "which", "can you", "is there",
] as const

export const ACCEPTANCE_KEYWORDS = [
  // Chinese
  "验收", "检查", "审核", "验证", "确认", "质检", "测试通过",
  "符合要求", "达标", "合格", "满足需求",
  // English
  "acceptance", "verify", "validate", "check", "review", "quality check",
  "meets requirements", "passes tests", "confirm", "approve",
] as const

export const FIX_KEYWORDS = [
  // Chinese
  "修复", "修正", "解决", "处理", "调整", "纠正", "修改",
  "解决问题", "修bug", "补丁",
  // English
  "fix", "repair", "resolve", "adjust", "correct", "patch",
  "troubleshoot", "debug", "hotfix",
] as const

/** Classifier configuration */
export interface ClassifierConfig {
  /** Use LLM for uncertain cases (default: true) */
  useLLMFallback?: boolean
  /** Confidence threshold for rule-based classification (default: 0.7) */
  ruleConfidenceThreshold?: number
  /** LLM model to use (default: haiku) */
  llmModel?: "haiku" | "sonnet"
}

export const DEFAULT_CLASSIFIER_CONFIG: Required<ClassifierConfig> = {
  useLLMFallback: true,
  ruleConfidenceThreshold: 0.7,
  llmModel: "haiku",
}
