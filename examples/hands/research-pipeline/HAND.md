---
id: "research-pipeline"
name: "研究管道"
version: "1.0.0"
schedule: "0 0 8 * * 1"
enabled: true
memory_path: "hands/research-pipeline/{date}.md"
agents:
  - explore
  - general
  - writer
pipeline: "sequential"
autonomy:
  level: "wild"
  unattended: true
  max_iterations: 3
decision:
  use_close: true
  web_search: true
resources:
  max_tokens: 150000
  max_cost_usd: 8.0
  max_duration_sec: 2400
params:
  topic: "AI Agent 架构"
  depth: "comprehensive"
  output_format: "markdown_report"
---

# 研究管道

多 Agent 协作研究管道，使用顺序执行模式：

1. **explore** - 探索代码库或目标领域
2. **general** - 深度分析和研究
3. **writer** - 生成结构化研究报告

## 工作流程

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   explore   │ ───▶ │   general   │ ───▶ │   writer    │
│  (探索发现)  │      │  (深度研究)  │      │  (报告生成)  │
└─────────────┘      └─────────────┘      └─────────────┘
```

## Pipeline 模式说明

### Sequential（顺序）

当前使用的模式。每个 Agent 依次执行，前一个 Agent 的输出会作为下一个 Agent 的上下文输入。

适用场景：
- 研究流程：探索 → 分析 → 报告
- 内容创作：大纲 → 草稿 → 润色
- 代码审查：安全扫描 → 质量检查 → 性能分析

### Parallel（并行）

所有 Agent 同时执行，最后合并输出。

```yaml
pipeline: "parallel"
```

适用场景：
- 多角度分析：同时从安全、性能、可维护性角度审查
- 多源数据：同时获取不同来源的信息

### Conditional（条件）

根据 CLOSE 框架评估决定下一个 Agent。

```yaml
pipeline: "conditional"
```

适用场景：
- 决策树：根据分析结果选择不同路径
- 异常处理：正常流程 vs 问题处理

## 参数说明

| 参数 | 值 | 说明 |
|------|-----|------|
| topic | "AI Agent 架构" | 研究主题 |
| depth | "comprehensive" | 研究深度 |
| output_format | "markdown_report" | 输出格式 |

## 输出示例

```markdown
# 研究报告 - AI Agent 架构

## 摘要
{由 writer Agent 生成的摘要}

## 发现
{由 explore Agent 发现的关键信息}

## 分析
{由 general Agent 进行的深度分析}

## 结论与建议
{综合分析后的结论}
```

## 使用方法

1. 复制到 `~/.codecoder/hands/research-pipeline/`
2. 修改 `params.topic` 为你要研究的主题
3. 启动 zero-workflow 服务
4. 等待定时执行或手动触发：

```bash
curl -X POST http://localhost:4432/api/v1/hands/research-pipeline/trigger
```
