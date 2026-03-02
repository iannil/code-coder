# CodeCoder Hands 示例文件

本目录包含 CodeCoder Hands 系统的示例文件，展示各种配置和使用场景。

## 快速开始

### 1. 复制示例到 Hands 目录

```bash
# 复制单个示例
cp -r docs/examples/hands/clip-video ~/.codecoder/hands/

# 复制所有示例
cp -r docs/examples/hands/* ~/.codecoder/hands/
```

### 2. 启用/禁用 Hand

编辑 `HAND.md` 中的 `enabled` 字段：

```yaml
enabled: true  # 启用
# enabled: false  # 禁用
```

### 3. 重启 Workflow 服务

```bash
./ops.sh restart workflow
```

## 示例索引

| Hand | Agent(s) | 触发方式 | 自治级别 | 用途 |
|------|----------|----------|----------|------|
| [Clip](./clip-video/) | explore → writer → proofreader | Cron (6h) | wild | 视频内容创作 |
| [Lead](./lead-generator/) | picker | Cron (工作日9点) | crazy | 销售线索生成 |
| [Collector](./market-collector/) | macro → observer (条件) | Cron (4h) | wild | 市场情报收集 |
| [Researcher](./researcher/) | explore → writer → proofreader | Webhook | insane | 深度研究报告 |
| [Predictor](./trend-predictor/) | trader | Cron (工作日8点) | timid | 趋势预测 |
| [Social Media](./social-media/) | writer | Cron (3h) | wild | 社交媒体管理 |
| [Browser](./browser-automation/) | general | Webhook | lunatic | 浏览器自动化 |

## 配置说明

### HAND.md 文件结构

```yaml
---
# 必填字段
id: "unique-id"
name: "Display Name"
schedule: "cron-expression"
agent: "agent-name"  # 或 pipeline 配置

# 可选字段
description: "Description"
autonomy: { level: "wild", ... }
decision: { use_close: true, ... }
risk_control: { max_tokens: 1000, ... }
memory_path: "hands/{id}/{date}.md"
enabled: true
---

# Markdown 内容（Hand 说明文档）
```

### 自治级别 (Autonomy Levels)

| 级别 | 分数范围 | 批准阈值 | 说明 |
|------|----------|----------|------|
| `lunatic` | 90+ | 5.0 | 完全自主，仅关键决策通知 |
| `insane` | 75-89 | 5.5 | 高度自主，执行前通知 |
| `crazy` | 60-74 | 6.0 | 显著自主，半自动执行 |
| `wild` | 40-59 | 6.5 | 部分自主，仅简单任务 |
| `bold` | 20-39 | 7.0 | 谨慎自主，仅定义步骤 |
| `timid` | <20 | 8.0 | 基本不自主，仅收集信息 |

### 触发方式

#### Cron 调度

```yaml
schedule: "0 */6 * * *"  # 每6小时
```

#### Webhook 触发

```yaml
trigger:
  type: "webhook"
  endpoint: "/webhook/my-hand"
  method: "POST"
  auth_required: true
```

### Pipeline 模式

#### 顺序执行 (Sequential)

```yaml
pipeline:
  mode: "sequential"
  agents:
    - name: "explore"
    - name: "writer"
    - name: "proofreader"
```

#### 条件执行 (Conditional)

```yaml
pipeline:
  mode: "conditional"
  agents:
    - name: "macro"
      decision_point: true
      condition:
        metric: "volatility_score"
        threshold: 0.7
    - name: "observer"
      depends_on: "macro"
```

### CLOSE 框架集成

```yaml
decision:
  use_close: true
  close_dimensions:
    - convergence
    - leverage
    - optionality
    - surplus
    - evolution
```

### 自动审批配置

```yaml
approval:
  timeout: 1800  # 30分钟
  auto_approve: true
  auto_approve_conditions:
    - field: "confidence"
      operator: "gte"
      value: 0.8
```

## 与 OpenFang 对照

| OpenFang | CodeCoder | 说明 |
|----------|-----------|------|
| Clip | clip-video | 视频内容创作 |
| Lead | lead-generator | 销售线索生成 |
| Collector | market-collector | 市场情报收集 |
| Researcher | researcher | 深度研究 |
| Predictor | trend-predictor | 趋势预测 |
| Twitter | social-media | 社交媒体管理 |
| Browser | browser-automation | 浏览器自动化 |

## 技术参考

- **解析器**: `services/zero-workflow/src/hands/manifest.rs`
- **类型定义**: `packages/ccode/src/autonomous/hands/bridge.ts`
- **配置 Schema**: `schemas/hand.schema.json`

## 风险控制建议

| 场景 | 推荐级别 | 理由 |
|------|----------|------|
| 内容生成 | wild/insane | 无破坏性操作 |
| 销售线索 | crazy | 需要大量收集 |
| 市场监控 | wild | 持续监控 |
| 深度研究 | insane | 需要多工具 |
| 趋势预测 | timid | 涉及资金决策 |
| 社交媒体 | wild + 审批 | 发布需确认 |
| 浏览器自动化 | lunatic + HITL | 有审批保护 |

## 故障排查

### Hand 未执行

1. 检查 `enabled: true`
2. 验证 cron 表达式
3. 查看 workflow 服务日志: `./ops.sh logs zero-workflow`
4. 确认 agent 配置正确

### 权限错误

1. 检查 `~/.codecoder/hands/` 目录权限
2. 验证 Webhook token 配置
3. 确认 zero-* 服务运行状态

### 资源超限

1. 调整 `risk_control` 中的限制
2. 检查账户余额 (API 配额)
3. 优化 prompt 减少 token 使用
