---
# Researcher - 深度研究员 Hand
# 3-Agent Pipeline 完整研究报告，Webhook 触发

id: "deep-researcher"
name: "Deep Researcher"
description: "按需触发的深度研究，生成完整的多维度研究报告"
version: "1.0.0"
author: "CodeCoder"

# Webhook 触发 - 按需执行
trigger:
  type: "webhook"
  endpoint: "/webhook/research"
  method: "POST"
  auth_required: true

# 3-Agent Pipeline - 顺序执行
pipeline:
  mode: "sequential"
  agents:
    - name: "explore"
      role: "information_gatherer"
      params:
        depth: "comprehensive"
        sources: ["academic", "industry", "news"]
        max_results: 50
    - name: "writer"
      role: "report_synthesizer"
      params:
        format: "research_report"
        sections:
          - "executive_summary"
          - "background"
          - "analysis"
          - "findings"
          - "recommendations"
          - "references"
        tone: "professional"
    - name: "proofreader"
      role: "quality_assurer"
      params:
        checks:
          - "grammar"
          - "citations"
          - "consistency"
          - "clarity"

# 高自治级别 - 深度研究需要多工具能力
autonomy:
  level: "insane"
  score_threshold: 80
  approval_threshold: 5.5

# CLOSE 框架集成
decision:
  use_close: true
  auto_continue: true
  web_search: true
  evolution: true
  close_dimensions:
    - convergence
    - leverage
    - optionality
    - surplus
    - evolution

# 请求参数模板
params:
  template: "research_request"
  required_fields:
    - "topic"
    - "purpose"
  optional_fields:
    - "focus_areas"
    - "time_range"
    - "geography"

# 风险控制
risk_control:
  max_tokens: 25000
  max_cost_usd: 2.00
  max_duration_sec: 600

# 记忆存储路径
memory_path: "hands/research/{topic_slug}/{date}.md"

# 输出配置
output:
  format: "markdown"
  include_sources: true
  include_confidence: true
  include_next_steps: true

# 启用状态
enabled: true
---

# Deep Researcher Hand

## 概述

此 Hand 实现完整的三阶段研究流程，通过 Webhook 按需触发，适合需要深度分析的复杂主题。

## 触发方式

### Webhook 请求格式

```bash
curl -X POST http://localhost:4432/webhook/research \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZERO_WEBHOOK_TOKEN" \
  -d '{
    "topic": "AI Agent 在金融科技中的应用",
    "purpose": "投资决策支持",
    "focus_areas": ["风险管理", "客户服务", "反欺诈"],
    "time_range": "2023-2025",
    "geography": ["US", "CN", "EU"]
  }'
```

## 工作流程

```
┌─────────────────────────────────────────────────────────────────┐
│                        Webhook 触发                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  阶段 1: Explore (信息收集)                                       │
│  - 多源搜索 (学术/行业/新闻)                                      │
│  - 去重和排序                                                     │
│  - 初步筛选                                                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  阶段 2: Writer (报告合成)                                       │
│  - 结构化撰写                                                     │
│  - CLOSE 框架分析                                                 │
│  - 多维度观点                                                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  阶段 3: Proofreader (质量保证)                                  │
│  - 语法和拼写检查                                                 │
│  - 引用验证                                                       │
│  - 一致性检查                                                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                        输出完整报告
```

## CLOSE 框架应用

| 维度 | 研究问题 |
|------|----------|
| **Convergence** | 不同信息源的共识点在哪里？ |
| **Leverage** | 哪些信息具有高价值/低获取成本？ |
| **Optionality** | 哪些发现保留未来探索空间？ |
| **Surplus** | 资源预算如何分配最优？ |
| **Evolution** | 与历史研究相比有什么演变？ |

## 输出结构

```markdown
# {主题} 深度研究报告

## 执行摘要
[300字以内的核心结论]

## 背景
[主题背景和重要性]

## 分析
[多维度分析，按 focus_areas 分组]

## 发现
[关键发现，按重要性排序]

## 建议
[可执行的建议，标注置信度]

## 参考文献
[所有来源的完整引用]

## 附录
- CLOSE 分析详情
- 信息源质量评分
- 后续研究建议
```

## 使用场景

1. **投资决策前研究**: 公司/行业深度分析
2. **技术选型**: 新技术栈评估
3. **市场进入**: 新市场可行性研究
4. **竞品分析**: 竞争对手全面扫描
