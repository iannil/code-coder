# CodeCoder 系统能力验证报告

**日期**: 2026-03-09
**验证者**: Claude Code (触发验证，不参与实际分析)
**验证目标**: 验证 CodeCoder 能否解决复杂开放性问题

---

## 执行摘要

| 验证项 | 状态 | 说明 |
|--------|------|------|
| **自主模式启动** | ✅ 通过 | CLI 命令正常执行 |
| **原生绑定加载** | ✅ 通过 | @codecoder-ai/core NAPI 模块正常 |
| **任务分类** | ❌ 失败 | 研究任务被误判为代码任务 |
| **研究循环触发** | ❌ 未触发 | research-loop 未被调用 |
| **Web 搜索** | ⚠️ 部分 | 仅在错误恢复时触发 |
| **CLOSE 评估** | ❌ 未执行 | 未进入决策评估流程 |

**结论**: 系统具备所有必要组件，但 **任务路由逻辑缺失** 导致研究类问题无法正确处理。

---

## 详细分析

### 1. 组件可用性验证 ✅

| 组件 | 文件路径 | 状态 |
|------|----------|------|
| Research Loop | `autonomous/execution/research-loop.ts` | ✅ 存在 |
| Web Search | `autonomous/execution/web-search.ts` | ✅ 存在 |
| PDCA Research Strategy | `autonomous/pdca/strategies/research.ts` | ✅ 存在 |
| Macro Agent Prompt | `agent/prompt/macro.txt` | ✅ 存在 |
| Decision Agent Prompt | `agent/prompt/decision.txt` | ✅ 存在 |
| Observer Agent Prompt | `agent/prompt/observer.txt` | ✅ 存在 |

### 2. 执行流程分析

**预期流程 (研究任务)**:
```
用户输入 → 任务分类 → 研究循环 → Web搜索 → LLM分析 → CLOSE评估 → 报告生成
```

**实际流程 (观察到的)**:
```
用户输入 → 直接进入TDD循环 → 生成测试代码 → 测试失败 → 错误恢复 → 暂停
```

**关键发现**:
- `Orchestrator` 类 (`orchestration/orchestrator.ts`) 没有任务类型判断逻辑
- 所有输入都被当作代码实现需求处理
- `createResearchLoop()` 从未被调用

### 3. Web 搜索行为分析

日志片段:
```
INFO  service=autonomous.execution.web-search shouldSearch=false isResearchQuery=false
```

**问题**: `isResearchQuery` 判断逻辑 (`web-search.ts:233`):
```typescript
const isResearchQuery = !errorMessage && !technology
```

这个逻辑在错误恢复流程中被调用时，`errorMessage` 已经存在，所以返回 `false`。

原始研究问题从未被直接传递给 WebSearcher。

### 4. 测试执行日志

```
ERROR  agent=tdd-guide error=No object generated: response did not match schema
ERROR  service=autonomous.execution.test-runner error=JSON Parse error: Unexpected identifier "Bun"
```

系统错误地尝试为地缘政治分析问题生成测试代码。

---

## 能力差距分析

### 现有能力 (组件层)

| 能力 | 实现状态 | 入口点 |
|------|----------|--------|
| Web 搜索 (Exa API) | ✅ 完整 | `createWebSearcher()` |
| 研究循环 (6阶段) | ✅ 完整 | `createResearchLoop()` |
| CLOSE 评估 | ✅ 完整 | `decision` agent |
| 报告生成 | ✅ 完整 | `renderReport()` |

### 缺失能力 (集成层)

| 缺失点 | 位置 | 建议修复 |
|--------|------|----------|
| 任务类型分类器 | `orchestrator.ts` | 添加 `classifyTaskType()` 函数 |
| 研究任务路由 | `orchestrator.ts` | 当 `taskType === 'research'` 时调用 `researchLoop.research()` |
| 直接研究触发 | `autonomous.ts` CLI | 添加 `--mode research` 选项 |

---

## 建议修复方案

### 方案 A: 添加任务分类器 (推荐)

```typescript
// orchestrator.ts
async classifyTaskType(request: string): Promise<'code' | 'research' | 'decision'> {
  const researchKeywords = ['分析', '研究', '调研', '评估', '预测', 'analyze', 'research']
  const codeKeywords = ['实现', '修复', '添加', '创建', 'implement', 'fix', 'add', 'create']

  // ... LLM-based classification or keyword matching
}
```

### 方案 B: 添加显式模式参数

```bash
# CLI 新增参数
ccode autonomous --mode research "分析美以和伊朗战争发展"
ccode autonomous --mode code "实现用户认证功能"
```

### 方案 C: 使用现有 Agent 系统

```bash
# 直接调用 decision 或 macro agent
bun dev -m decision
> 分析美以和伊朗的战争未来发展，使用CLOSE框架评估
```

---

## 验证方法论

1. **触发方式**: `bun run --cwd packages/ccode src/index.ts autonomous`
2. **参数设置**: `--autonomy-level timid --max-tokens 30000 --unattended`
3. **日志级别**: `--print-logs --log-level DEBUG`
4. **输出捕获**: `tee` 到日志文件

---

## 附录: 关键代码路径

### 研究循环未被调用的原因

```typescript
// orchestrator.ts - 没有研究任务分支
async process(request: string) {
  // 直接进入 TDD 流程
  await this.executor.execute(...)  // ← 始终调用代码执行器

  // research-loop 从未在此处被调用
}
```

### 修复建议代码位置

- `packages/ccode/src/autonomous/orchestration/orchestrator.ts:132` - `start()` 方法
- `packages/ccode/src/autonomous/orchestration/orchestrator.ts` - 添加 `processResearch()` 方法
- `packages/ccode/src/cli/cmd/autonomous.ts:71` - 添加 `--mode` 参数

---

## 结论

CodeCoder 系统 **具备解决复杂研究问题的所有底层能力**（Web 搜索、研究循环、CLOSE 评估、报告生成），但 **自主模式的任务路由逻辑** 需要增强，以正确区分代码任务和研究任务。

**推荐下一步**:
1. 在 `Orchestrator.start()` 中添加任务分类逻辑
2. 当检测到研究任务时，调用 `createResearchLoop().research()`
3. 添加 CLI 参数 `--mode research|code|auto` 以支持显式指定

---

*报告由 Claude Code 自动生成 @ 2026-03-09*
