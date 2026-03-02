---
# Browser - 浏览器自动化 Hand
# 与 zero-browser 服务集成，HITL 审批保护

id: "browser-automation"
name: "Browser Automation"
description: "浏览器自动化任务，与 zero-browser 微服务集成"
version: "1.0.0"
author: "CodeCoder"

# Webhook 触发 - 按需执行
trigger:
  type: "webhook"
  endpoint: "/webhook/browser"
  method: "POST"
  auth_required: true

# 单 Agent - general (可执行浏览器操作)
agent: "general"

# 最高自治级别 - 但有 HITL 保护
autonomy:
  level: "lunatic"
  score_threshold: 90
  approval_threshold: 5.0
  human_in_loop: true

# zero-browser 集成配置
integration:
  service: "zero-browser"
  endpoint: "http://localhost:4433"
  timeout: 300

# 支持的浏览器操作
params:
  operations:
    - name: "navigate"
      description: "导航到 URL"
      params: ["url"]
      risk: "low"
    - name: "screenshot"
      description: "截图"
      params: ["selector", "filename"]
      risk: "low"
    - name: "click"
      description: "点击元素"
      params: ["selector"]
      risk: "medium"
    - name: "fill"
      description: "填写表单"
      params: ["selector", "value"]
      risk: "medium"
    - name: "extract"
      description: "提取数据"
      params: ["selector"]
      risk: "low"
    - name: "wait_for"
      description: "等待条件"
      params: ["condition", "timeout"]
      risk: "low"
  # 禁止的高风险操作
  forbidden:
    - "file_upload"
    - "download"
    - "execute_script"

# CLOSE 框架集成
decision:
  use_close: true
  auto_continue: false  # 每步需要确认
  web_search: false
  evolution: false

# 严格风险控制
risk_control:
  max_tokens: 5000
  max_cost_usd: 0.20
  max_duration_sec: 300
  # 浏览器特定限制
  max_pages: 10
  max_clicks: 20
  allowed_domains: []  # 空表示不限制

# HITL 审批流程
approval:
  mode: "step_by_step"  # 每步都需要人工确认
  timeout: 300  # 5分钟无响应则中止
  auto_approve: false
  preview_before_action: true  # 执行前预览操作
  notification:
    - type: "console"
      level: "warn"
      show_preview: true

# 记忆存储路径
memory_path: "hands/browser/{date}/{task_id}.md"

# 输出配置
output:
  format: "markdown"
  include_screenshots: true
  include_execution_trace: true

# 启用状态
enabled: true
---

# Browser Automation Hand

## 概述

此 Hand 实现浏览器自动化任务，通过 HITL (Human-In-The-Loop) 审批保护，确保自动化操作的安全性。

## 架构集成

```
┌─────────────────────────────────────────────────────────────┐
│                     Webhook 触发                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   General Agent                              │
│  - 理解任务需求                                              │
│  - 规划操作序列                                              │
│  - 生成浏览器指令                                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   HITL 审批层                                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Step 1: navigate → https://example.com                  ││
│  │ 预览: 将访问新页面                                       ││
│  │ [批准] [拒绝] [修改]                                    ││
│  └─────────────────────────────────────────────────────────┘│
│  ↓ (批准)                                                    │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ Step 2: screenshot → header.png                         ││
│  │ 预览: 将保存页面截图                                     ││
│  │ [批准] [拒绝] [修改]                                    ││
│  └─────────────────────────────────────────────────────────┘│
│  ↓ (批准)                                                    │
│  ...                                                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   zero-browser (4433)                        │
│  - 执行 Playwright 操作                                      │
│  - 返回结果和截图                                            │
└─────────────────────────────────────────────────────────────┘
```

## 触发方式

### Webhook 请求

```bash
curl -X POST http://localhost:4432/webhook/browser \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZERO_WEBHOOK_TOKEN" \
  -d '{
    "task": "访问 https://example.com 并截图",
    "steps": [
      {"action": "navigate", "url": "https://example.com"},
      {"action": "screenshot", "filename": "homepage.png"},
      {"action": "extract", "selector": "h1"}
    ]
  }'
```

### 简化任务描述

```bash
curl -X POST http://localhost:4432/webhook/browser \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZERO_WEBHOOK_TOKEN" \
  -d '{
    "task": "打开 GitHub 首页，告诉我今天的热门仓库是什么"
  }'
```

## 安全措施

### 1. HITL 逐步审批
- 每个操作前需要人工确认
- 显示操作预览
- 可修改或跳过特定步骤

### 2. 禁止操作
- 文件上传 (防止数据泄露)
- 文件下载 (防止恶意软件)
- 脚本执行 (防止代码注入)

### 3. 域名白名单
```yaml
risk_control.allowed_domains:
  - "github.com"
  - "example.com"
```

### 4. 操作限制
- 最多访问 10 个页面
- 最多 20 次点击
- 5 分钟超时

## 输出格式

```markdown
## 浏览器自动化报告 - {date}

### 任务
{原始任务描述}

### 执行步骤
| Step | Action | Target | Status | Screenshot |
|------|--------|--------|--------|------------|
| 1 | navigate | https://... | ✅ | step1.png |
| 2 | screenshot | - | ✅ | step2.png |
| 3 | extract | h1 | ✅ | - |

### 提取的数据
```
{提取的内容}
```

### 截图
[附上截图路径或嵌入]

### 执行统计
- 总步骤: 3
- 成功: 3
- 失败: 0
- 总耗时: 15.3s
```

## 使用场景

1. **数据采集**: 定期截图网页变化
2. **简单测试**: 表单提交验证
3. **内容监控**: 检测页面更新
4. **自动化操作**: 重复性浏览任务

## 零信任原则

此 Hand 设计遵循零信任原则：
- 默认需要人工审批
- 每步操作可被中止
- 所有操作有记录
- 敏感操作被禁止
