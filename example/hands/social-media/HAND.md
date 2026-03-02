---
# Twitter - 社交媒体管理 Hand
# 带 auto-approve 配置的超时自动批准机制

id: "social-media-manager"
name: "Social Media Manager"
description: "自动化社交媒体内容生成和发布管理"
version: "1.0.0"
author: "CodeCoder"

# 调度配置 - 每3小时生成一次内容
schedule: "0 */3 * * *"

# 单 Agent - writer
agent: "writer"

# 自治级别
autonomy:
  level: "wild"
  score_threshold: 50
  approval_threshold: 6.5

# 内容生成参数
params:
  platforms:
    - name: "twitter"
      max_length: 280
      hashtags: true
      emojis: "moderate"
    - name: "linkedin"
      max_length: 3000
      hashtags: true
      emojis: "minimal"
    - name: "mastodon"
      max_length: 500
      hashtags: true
      emojis: "moderate"
  content_types:
    - "insight"
    - "tip"
    - "thread"
    - "question"
  topics:
    - "AI 工程"
    - "Rust 开发"
    - "系统设计"
    - "技术哲学"
  tone: "professional_friendly"

# CLOSE 框架集成
decision:
  use_close: true
  auto_continue: true
  web_search: true
  evolution: true

# 风险控制
risk_control:
  max_tokens: 3000
  max_cost_usd: 0.15
  max_duration_sec: 90

# 自动审批配置
approval:
  # 超时自动批准机制
  timeout: 1800  # 30分钟无人工响应则自动批准
  auto_approve: true
  auto_approve_conditions:
    - field: "confidence"
      operator: "gte"
      value: 0.8
    - field: "risk_score"
      operator: "lte"
      value: 0.2
  # 自动拒绝条件
  auto_reject_conditions:
    - field: "contains_sensitive"
      operator: "eq"
      value: true
  # 通知配置
  notification:
    - type: "webhook"
      url: "${ZERO_WEBHOOK_URL}/social-approval"
    - type: "console"
      level: "info"

# 发布配置 (与 zero-channels 集成)
publishing:
  enabled: true
  service: "zero-channels"
  endpoint: "http://localhost:4431"
  channels:
    - platform: "twitter"
      enabled: false  # 需要人工审核后发布
    - platform: "linkedin"
      enabled: false
  dry_run: true  # 默认只生成不发布

# 记忆存储路径
memory_path: "hands/social/{date}/{hour}.md"

# 输出配置
output:
  format: "markdown"
  include_all_platforms: true
  include_scheduling: true

# 启用状态
enabled: true
---

# Social Media Manager Hand

## 概述

此 Hand 自动化社交媒体内容生成，具有独特的超时自动批准机制，确保内容发布节奏不被人工延迟影响。

## 工作流程

```
┌────────────────────────────────────────────────────────────┐
│                    每3小时触发                              │
└────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│              Writer Agent 生成内容                          │
│  - 选择话题                                                 │
│  - 生成多平台版本                                           │
│  - 计算置信度和风险分数                                     │
└────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│                    审批决策                                 │
├────────────────────────────────────────────────────────────┤
│  立即拒绝？          ← 包含敏感内容                          │
│       │                                                    │
│       ↓ No                                                 │
│  立即批准？          ← 置信度≥80% 且风险≤20%                │
│       │                                                    │
│       ↓ No                                                 │
│  等待人工 (30分钟)                                         │
│       │                                                    │
│       ├─ 人工批准 → 发布                                   │
│       ├─ 人工拒绝 → 取消                                   │
│       └─ 超时 → 自动批准                                   │
└────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│               发布到 zero-channels                          │
│  (dry_run=true 时只模拟)                                    │
└────────────────────────────────────────────────────────────┘
```

## 输出格式

```markdown
## 社交媒体内容 - {date} {hour}:00

### 🐦 Twitter
{内容}

**分析**:
- 置信度: 85%
- 风险分数: 0.1
- 预估互动: 中等
- 状态: [自动批准/待审批/已拒绝]

### 💼 LinkedIn
{内容}

**分析**:
- 置信度: 72%
- 风险分数: 0.15
- 预估互动: 较高
- 状态: 待审批

### 📋 发布时间建议
- Twitter: {datetime}
- LinkedIn: {datetime}

---
**CLOSE 评估**: [简要说明]
```

## 自动批准逻辑

### 自动批准 (无需等待)
- 置信度 ≥ 80%
- 风险分数 ≤ 0.2
- 不包含敏感内容关键词

### 自动拒绝 (保护机制)
- 包含敏感内容 (政治、争议话题等)
- 风险分数 > 0.5

### 超时处理 (默认30分钟)
- 无人工响应时自动批准
- 避免内容积压
- 保持发布节奏

## 集成配置

此 Hand 可与 `zero-channels` 服务集成实现自动发布：

```bash
# 启用自动发布
# 在 HAND.md 中设置:
publishing.dry_run = false
publishing.channels[0].enabled = true
```

## 使用建议

1. **初期**: 保持 `dry_run: true`，观察内容质量
2. **信任建立**: 逐步启用低风险平台
3. **敏感操作**: 永远保持人工审批
