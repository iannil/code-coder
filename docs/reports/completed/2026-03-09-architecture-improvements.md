# 架构改进实施报告

> **任务**: 执行架构评估报告的短期和中期改进建议
> **开始时间**: 2026-03-09
> **完成时间**: 2026-03-09
> **状态**: 已完成 (含 Expander 合并)

## 实施概要

基于 `docs/reports/2026-03-08-architecture-evaluation.md` 的评估结果，成功执行了以下五个阶段的改进任务。

---

## 第一阶段: 修复 Observer 测试 ✅

### 问题分析

测试运行时出现以下初始化错误：
- `TypeError: undefined is not an object (evaluating 'Log.create')`
- `ReferenceError: Cannot access 'Instance' before initialization`

**根本原因**: 模块初始化顺序问题，preload.ts 在 mock 生效前导入真实模块

### 修复内容

| 文件 | 修改类型 | 说明 |
|------|----------|------|
| `test/preload.ts` | 扩展 | 添加 Log mock 和 observer 测试检测 |
| `test/observer/setup.ts` | 扩展 | 添加 Instance、Context、Config、memory-markdown 等 mock |
| `src/observer/integration/memory-client.ts` | Bug 修复 | 修复 `recordBatch` 和 `query` 的目录不一致问题 |
| `src/observer/controller/escalation.ts` | Bug 修复 | 添加 ID 作为次要排序条件确保稳定排序 |
| `src/observer/responders/executor.ts` | Bug 修复 | `stop()` 方法现在正确拒绝所有 pending 执行 |
| `test/observer/consensus/engine.test.ts` | 测试修复 | 将 `stream.push()` 改为 `stream.ingest()` |
| `test/observer/responders/executor.test.ts` | 测试修复 | 添加 `useDialControl: false` 使用模式控制 |

### 验证结果

```
326 pass
0 fail
579 expect() calls
Ran 326 tests across 16 files. [1320.00ms]
```

---

## 第二阶段: 创建 Agent 能力矩阵 ✅

### 交付物

创建 `docs/architecture/AGENT_CAPABILITY_MATRIX.md`，包含：

- **总览**: 29 个 Agent 按模式分类统计 (合并后)
- **完整能力矩阵**: 所有 Agent 的详细配置表
- **Observer Network 集成**: 7 个已集成 Agent 的映射
- **功能重叠分析**: 高/中/低重叠对识别
- **配置扩展指南**: 如何为自定义 Agent 添加 Observer 能力

### Agent 分类统计 (合并后)

| 分类 | 数量 | 代表 Agent |
|------|------|-----------|
| 主模式 | 5 | build, plan, writer, autonomous |
| 工程质量 | 6 | explore, code-reviewer, security-reviewer |
| 逆向工程 | 2 | code-reverse, jar-code-reverse |
| 内容创作 | 3 | expander, proofreader, verifier |
| 祝融说系列 | 8 | observer, decision, macro, trader |
| 产品与可行性 | 2 | prd-generator, feasibility-assess |
| 其他 | 1 | synton-assistant |
| 系统隐藏 | 3 | compaction, title, summary |

---

## 第三阶段: Observer-Agent 深度集成 ✅

### 新增组件

创建 `src/observer/integration/observation-router.ts`，实现：

1. **ObservationRouter 类**: 自动将观察事件路由到相关 Agent
2. **路由规则系统**: 可配置的规则定义
3. **事件订阅**: 自动订阅所有观察事件类型
4. **异常/机会路由**: 特殊事件的专门处理逻辑
5. **队列管理**: 高负载时的容量控制

### 默认路由规则

| 规则 ID | 观察类型 | 目标 Agent | 优先级 |
|---------|----------|-----------|--------|
| code-to-explore | file_change, build_status | explore | 50 |
| world-to-macro | market_data, news_signal | macro, trader | 60 |
| self-to-reviewers | agent_behavior, decision_point | code-reviewer, security-reviewer | 40 |
| meta-to-observer | system_health, quality_report | observer | 70 |

### 导出更新

- `src/observer/integration/index.ts` 添加 ObservationRouter 导出
- `src/observer/index.ts` 添加公开 API

---

## 第四阶段: Agent 合并评估 ✅

### 评估结果

| Agent 组 | 重叠度 | 决策 | 理由 |
|----------|--------|------|------|
| expander 系列 | ~85% | **合并** | 核心逻辑相同，仅 temperature 不同 |
| code-reviewer / security-reviewer | ~55% | 保持独立 | 关注点完全不同 |
| macro / trader | ~45% | 保持独立 | 分析周期和方法论不同 |
| general / explore | ~40% | 保持独立 | 用途和工具权限不同 |

---

## 第五阶段: 实施 Expander 合并 ✅

### 合并策略

采用 **统一 Prompt + 领域标签** 方式合并，而非运行时参数：

- 创建统一的 `expander.txt` prompt，包含三种领域的完整指导
- 使用中间温度 0.7 (平衡创意与精确)
- 通过 prompt 中的领域检测逻辑自动选择合适的写作风格
- 支持显式 `[DOMAIN:fiction]` 或 `[DOMAIN:nonfiction]` 标签覆盖

### 修改文件清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/agent/prompt/expander.txt` | 重写 | 统一 prompt (238行 → 505行) |
| `src/agent/agent.ts` | 修改 | 移除 expander-fiction/nonfiction 定义 |
| `src/agent/registry.ts` | 修改 | 合并 registry 条目 |
| `src/agent/mode.ts` | 修改 | 更新 writer 模式能力列表 |
| `src/tool/task.ts` | 修改 | 简化 expander 检测逻辑 |
| `src/autonomous/expansion/orchestrator.ts` | 修改 | 使用统一 expander + 领域标签 |
| `src/cli/cmd/book-writer.ts` | 修改 | 移除 agent 选项 |
| `src/agent/writer-stats-monitor.ts` | 修改 | 更新注释 |
| `src/agent/prompt/writer.txt` | 修改 | 更新 expander 调用说明 |
| `docs/architecture/AGENT_CAPABILITY_MATRIX.md` | 修改 | 更新统计和合并状态 |
| `CLAUDE.md` | 修改 | 更新 Agent 总数 (31 → 29) |

### 统一 Prompt 结构

```markdown
# Domain Detection
- Fiction indicators: 故事、小说、角色、情节、世界观、冲突
- Non-fiction indicators: 论证、论据、证据、逻辑、分析、研究
- Explicit tag override: [DOMAIN:fiction] 或 [DOMAIN:nonfiction]

# Phase 1-5: [General Framework - 保留原有结构]

# For Fiction: World & Character Framework
[整合自 expander-fiction.txt]

# For Non-Fiction: Knowledge & Argument Framework
[整合自 expander-nonfiction.txt]

# Domain-Specific Writing Guidelines
# Domain-Specific Validation Checks
```

### 验证结果

```
$ bun run typecheck
$ tsgo --noEmit
(无错误)
```

### 收益

- **Agent 数量**: 31 → 29 (-2)
- **简化用户选择**: 无需在三个 expander 中选择
- **统一维护**: 扩展逻辑集中在单一 prompt
- **保留专业能力**: 通过领域检测和标签覆盖保持差异化

---

## 关键文件变更汇总

### 新增文件

| 路径 | 说明 |
|------|------|
| `docs/architecture/AGENT_CAPABILITY_MATRIX.md` | Agent 能力矩阵文档 |
| `src/observer/integration/observation-router.ts` | 观察路由器实现 |

### 修改文件 (Phase 1-4)

| 路径 | 变更类型 |
|------|----------|
| `test/preload.ts` | 扩展 mock 覆盖 |
| `test/observer/setup.ts` | 扩展 mock 覆盖 |
| `src/observer/integration/memory-client.ts` | Bug 修复 |
| `src/observer/controller/escalation.ts` | Bug 修复 |
| `src/observer/responders/executor.ts` | Bug 修复 |
| `test/observer/consensus/engine.test.ts` | 测试修复 |
| `test/observer/responders/executor.test.ts` | 测试修复 |
| `test/observer/integration/agent-client.test.ts` | 测试修复 |
| `src/observer/integration/index.ts` | 导出更新 |
| `src/observer/index.ts` | 导出更新 |

### 修改文件 (Phase 5 - Expander 合并)

| 路径 | 变更类型 |
|------|----------|
| `src/agent/prompt/expander.txt` | 重写为统一 prompt |
| `src/agent/agent.ts` | 移除 2 个 agent 定义 |
| `src/agent/registry.ts` | 合并 registry 条目 |
| `src/agent/mode.ts` | 更新能力列表 |
| `src/tool/task.ts` | 简化检测逻辑 |
| `src/autonomous/expansion/orchestrator.ts` | 使用统一 agent |
| `src/cli/cmd/book-writer.ts` | 移除 agent 选项 |
| `src/agent/writer-stats-monitor.ts` | 更新注释 |
| `src/agent/prompt/writer.txt` | 更新调用说明 |
| `docs/architecture/AGENT_CAPABILITY_MATRIX.md` | 更新统计 |
| `CLAUDE.md` | 更新 Agent 总数 |

---

## 后续建议

1. ~~**实施 expander 合并**: 按 P1 优先级执行~~ ✅ 已完成
2. **添加 ObservationRouter 测试**: 覆盖路由逻辑
3. **监控 Observer 集成**: 收集实际运行数据
4. **评估内部 Agent 调用**: 考虑添加非 HTTP 调用路径以提升性能
5. **清理备份文件**: 确认合并稳定后可删除 `expander-fiction.txt` 和 `expander-nonfiction.txt`
