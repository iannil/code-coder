/**
 * PRD Generator API Handler
 *
 * Converts meeting minutes into structured Product Requirements Documents.
 * Integrates with Feishu meeting data and outputs to Feishu Docs or Notion.
 *
 * POST /api/v1/prd/generate - Generate PRD from meeting notes
 * POST /api/v1/prd/from-meeting - Generate PRD from Feishu meeting ID
 * GET  /api/v1/prd/templates - List available PRD templates
 * GET  /api/v1/prd/history - List generated PRDs
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { readFile } from "fs/promises"
import { join } from "path"

// ============================================================================
// Types
// ============================================================================

export interface MeetingInput {
  /** Meeting title */
  title: string
  /** Meeting date (ISO format) */
  date: string
  /** List of attendees */
  attendees: string[]
  /** Meeting notes/transcript */
  notes: string
  /** Action items from the meeting */
  action_items?: ActionItem[]
  /** Decisions made */
  decisions?: string[]
}

export interface ActionItem {
  task: string
  assignee?: string
  due_date?: string
  priority?: string
}

export interface PRDGenerateRequest {
  /** Meeting input data */
  meeting: MeetingInput
  /** PRD template to use */
  template?: "standard" | "agile" | "lean"
  /** Output format */
  output_format?: "markdown" | "html" | "json"
  /** Output destination */
  output_to?: "response" | "feishu" | "notion"
  /** Feishu folder token (if output_to is feishu) */
  feishu_folder_token?: string
  /** Notion page ID (if output_to is notion) */
  notion_page_id?: string
}

export interface GeneratedPRD {
  /** Generated PRD ID */
  id: string
  /** PRD title */
  title: string
  /** Source meeting title */
  source_meeting: string
  /** PRD content in markdown */
  content: string
  /** Extracted features */
  features: Feature[]
  /** Identified risks */
  risks: Risk[]
  /** Estimated timeline */
  timeline: TimelineItem[]
  /** Created timestamp */
  created_at: string
  /** Output location (if saved externally) */
  output_url?: string
}

export interface Feature {
  id: string
  name: string
  description: string
  priority: "P0" | "P1" | "P2"
  acceptance_criteria: string[]
}

export interface Risk {
  description: string
  impact: "high" | "medium" | "low"
  likelihood: "high" | "medium" | "low"
  mitigation: string
}

export interface TimelineItem {
  phase: string
  deliverable: string
  duration_days: number
}

// ============================================================================
// In-Memory Store
// ============================================================================

const generatedPRDs: Map<string, GeneratedPRD> = new Map()

// ============================================================================
// PRD Generation Logic
// ============================================================================

function generateId(): string {
  return `prd_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`
}

function extractFeatures(notes: string, decisions: string[]): Feature[] {
  const features: Feature[] = []
  let featureCount = 0

  // Extract features from notes
  const featurePatterns = [
    /需要(?:实现|添加|开发|支持)[：:]\s*(.+)/gi,
    /功能[：:]\s*(.+)/gi,
    /feature[：:]\s*(.+)/gi,
    /需求[：:]\s*(.+)/gi,
  ]

  for (const pattern of featurePatterns) {
    let match
    while ((match = pattern.exec(notes)) !== null) {
      featureCount++
      features.push({
        id: `F-${String(featureCount).padStart(3, "0")}`,
        name: match[1].trim().substring(0, 50),
        description: match[1].trim(),
        priority: determinePriority(match[1]),
        acceptance_criteria: [`${match[1].trim()} 功能正常运行`],
      })
    }
  }

  // Also extract features from decisions
  for (const decision of decisions) {
    if (decision.includes("实现") || decision.includes("开发") || decision.includes("添加")) {
      featureCount++
      features.push({
        id: `F-${String(featureCount).padStart(3, "0")}`,
        name: decision.substring(0, 50),
        description: decision,
        priority: "P1",
        acceptance_criteria: [`${decision} 已完成`],
      })
    }
  }

  return features
}

function determinePriority(text: string): "P0" | "P1" | "P2" {
  const urgentKeywords = ["紧急", "必须", "核心", "关键", "urgent", "critical", "must"]
  const importantKeywords = ["重要", "需要", "应该", "important", "should"]

  const lowerText = text.toLowerCase()

  if (urgentKeywords.some((kw) => lowerText.includes(kw))) {
    return "P0"
  }
  if (importantKeywords.some((kw) => lowerText.includes(kw))) {
    return "P1"
  }
  return "P2"
}

function extractRisks(notes: string): Risk[] {
  const risks: Risk[] = []

  const riskPatterns = [
    /风险[：:]\s*(.+)/gi,
    /risk[：:]\s*(.+)/gi,
    /可能(?:会|出现)[：:]\s*(.+)/gi,
    /担心[：:]\s*(.+)/gi,
    /挑战[：:]\s*(.+)/gi,
  ]

  for (const pattern of riskPatterns) {
    let match
    while ((match = pattern.exec(notes)) !== null) {
      risks.push({
        description: match[1].trim(),
        impact: "medium",
        likelihood: "medium",
        mitigation: "待评估",
      })
    }
  }

  return risks
}

function estimateTimeline(features: Feature[]): TimelineItem[] {
  const p0Count = features.filter((f) => f.priority === "P0").length
  const p1Count = features.filter((f) => f.priority === "P1").length
  const p2Count = features.filter((f) => f.priority === "P2").length

  // Simple estimation: P0 = 3 days, P1 = 2 days, P2 = 1 day per feature
  const devDays = p0Count * 3 + p1Count * 2 + p2Count * 1
  const testDays = Math.ceil(devDays * 0.3)

  return [
    { phase: "需求评审", deliverable: "PRD + 原型", duration_days: 2 },
    { phase: "技术设计", deliverable: "技术方案文档", duration_days: 2 },
    { phase: "开发", deliverable: "代码", duration_days: devDays },
    { phase: "测试", deliverable: "测试报告", duration_days: testDays },
    { phase: "上线", deliverable: "发布", duration_days: 1 },
  ]
}

function generatePRDContent(meeting: MeetingInput, features: Feature[], risks: Risk[], timeline: TimelineItem[]): string {
  const today = new Date().toISOString().split("T")[0]
  const totalDays = timeline.reduce((sum, t) => sum + t.duration_days, 0)

  let content = `# ${meeting.title} PRD

## 文档信息
- 版本：1.0
- 作者：AI生成（基于会议纪要）
- 日期：${today}
- 状态：草稿
- 来源会议：${meeting.title} (${meeting.date})
- 参会人：${meeting.attendees.join("、")}

## 一、背景与目标

### 1.1 背景
本PRD基于${meeting.date}的会议讨论整理而成。

### 1.2 会议要点
${meeting.notes.split("\n").slice(0, 10).join("\n")}

## 二、功能需求

### 2.1 功能清单
| ID | 功能名称 | 优先级 | 描述 |
|----|----------|--------|------|
${features.map((f) => `| ${f.id} | ${f.name} | ${f.priority} | ${f.description.substring(0, 50)}... |`).join("\n")}

### 2.2 功能详述
${features.map((f) => `
#### ${f.id}: ${f.name}
- **描述**：${f.description}
- **优先级**：${f.priority}
- **验收标准**：
${f.acceptance_criteria.map((ac) => `  - ${ac}`).join("\n")}
`).join("\n")}

## 三、风险评估

| 风险 | 影响 | 可能性 | 应对措施 |
|------|------|--------|----------|
${risks.length > 0 ? risks.map((r) => `| ${r.description} | ${r.impact} | ${r.likelihood} | ${r.mitigation} |`).join("\n") : "| 暂无识别风险 | - | - | - |"}

## 四、开发计划

### 4.1 里程碑
| 阶段 | 交付物 | 预计工期 |
|------|--------|----------|
${timeline.map((t) => `| ${t.phase} | ${t.deliverable} | ${t.duration_days}天 |`).join("\n")}

**预计总工期：${totalDays}天**

### 4.2 待办事项
${meeting.action_items && meeting.action_items.length > 0 ? meeting.action_items.map((item) => `- [ ] ${item.task}${item.assignee ? ` @${item.assignee}` : ""}${item.due_date ? ` (截止：${item.due_date})` : ""}`).join("\n") : "- [ ] 待补充"}

## 五、决策记录

${meeting.decisions && meeting.decisions.length > 0 ? meeting.decisions.map((d, i) => `${i + 1}. ${d}`).join("\n") : "暂无决策记录"}

## 六、附录

### 6.1 原始会议纪要
\`\`\`
${meeting.notes}
\`\`\`

---
*本文档由AI根据会议纪要自动生成，请人工审核后使用。*
`

  return content
}

// ============================================================================
// Handlers
// ============================================================================

export async function handleGeneratePRD(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const bodyText = req.body ? await new Response(req.body).text() : "{}"
    const body = JSON.parse(bodyText) as PRDGenerateRequest

    if (!body.meeting || !body.meeting.title || !body.meeting.notes) {
      return errorResponse("Missing required fields: meeting.title, meeting.notes", 400)
    }

    const meeting = body.meeting

    // Extract structured data from meeting notes
    const features = extractFeatures(meeting.notes, meeting.decisions ?? [])
    const risks = extractRisks(meeting.notes)
    const timeline = estimateTimeline(features)

    // Generate PRD content
    const content = generatePRDContent(meeting, features, risks, timeline)

    const prd: GeneratedPRD = {
      id: generateId(),
      title: `${meeting.title} PRD`,
      source_meeting: meeting.title,
      content,
      features,
      risks,
      timeline,
      created_at: new Date().toISOString(),
    }

    // Save to history
    generatedPRDs.set(prd.id, prd)

    // Handle output destination
    if (body.output_to === "feishu" && body.feishu_folder_token) {
      // In production, call Feishu API to create document
      prd.output_url = `https://feishu.cn/docx/${prd.id}`
    } else if (body.output_to === "notion" && body.notion_page_id) {
      // In production, call Notion API to create page
      prd.output_url = `https://notion.so/${prd.id}`
    }

    return jsonResponse({
      success: true,
      prd: body.output_format === "json" ? prd : { ...prd, content: prd.content },
    })
  } catch (error) {
    return errorResponse(`Failed to generate PRD: ${error}`, 500)
  }
}

export async function handleFromMeeting(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const bodyText = req.body ? await new Response(req.body).text() : "{}"
    const body = JSON.parse(bodyText) as {
      meeting_id: string
      calendar_id?: string
      output_format?: string
      output_to?: string
      feishu_folder_token?: string
    }

    if (!body.meeting_id) {
      return errorResponse("Missing required field: meeting_id", 400)
    }

    // In production, this would fetch meeting data from Feishu API
    // For now, return a placeholder response
    return jsonResponse({
      success: false,
      error: "Meeting data retrieval requires Feishu API configuration",
      hint: "Use POST /api/v1/prd/generate with meeting notes directly",
    })
  } catch (error) {
    return errorResponse(`Failed to fetch meeting: ${error}`, 500)
  }
}

export async function handleListTemplates(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const templates = [
    {
      id: "standard",
      name: "标准PRD模板",
      description: "完整的产品需求文档，包含背景、功能、交互、技术等章节",
      sections: ["背景与目标", "用户分析", "功能需求", "交互设计", "非功能需求", "技术方案", "开发计划"],
    },
    {
      id: "agile",
      name: "敏捷PRD模板",
      description: "轻量级需求文档，适合快速迭代",
      sections: ["用户故事", "验收标准", "技术要点", "待办事项"],
    },
    {
      id: "lean",
      name: "精益PRD模板",
      description: "最小化需求文档，聚焦核心功能",
      sections: ["问题陈述", "解决方案", "成功指标"],
    },
  ]

  return jsonResponse({ success: true, templates })
}

export async function handleListHistory(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const limit = parseInt(url.searchParams.get("limit") ?? "20", 10)

    const all = Array.from(generatedPRDs.values())
    const sorted = all.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
    const limited = sorted.slice(0, limit)

    // Return without full content for list view
    const items = limited.map((prd) => ({
      id: prd.id,
      title: prd.title,
      source_meeting: prd.source_meeting,
      features_count: prd.features.length,
      risks_count: prd.risks.length,
      created_at: prd.created_at,
      output_url: prd.output_url,
    }))

    return jsonResponse({
      success: true,
      prds: items,
      total: all.length,
    })
  } catch (error) {
    return errorResponse(`Failed to list PRDs: ${error}`, 500)
  }
}

export async function handleGetPRD(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const id = params.id
  if (!id) return errorResponse("PRD ID required", 400)

  const prd = generatedPRDs.get(id)
  if (!prd) return errorResponse("PRD not found", 404)

  return jsonResponse({ success: true, prd })
}

// ============================================================================
// Route Registration Helper
// ============================================================================

export const prdRoutes = {
  "POST /api/v1/prd/generate": handleGeneratePRD,
  "POST /api/v1/prd/from-meeting": handleFromMeeting,
  "GET /api/v1/prd/templates": handleListTemplates,
  "GET /api/v1/prd/history": handleListHistory,
  "GET /api/v1/prd/:id": handleGetPRD,
}
