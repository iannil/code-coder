# P0/P1 设计债务修复完成报告

> 完成时间: 2026-03-01
> 执行者: Claude Agent
> 相关计划: P0/P1 设计债务修复计划 (见对话上下文)

## 变更摘要

修复了 code-coder 项目中的 P0 和 P1 级别设计债务：

### P0 级别 (架构一致性)

1. **NotificationSink trait 统一** - 将两处重复定义合并到 zero-common
2. **ConfirmationHandler trait 删除** - 移除已废弃的死代码
3. **Channel trait 文档化** - 记录 dyn-compatible 设计决策

### P1 级别 (代码可维护性)

4. **HybridDecisionMaker 实现** - MacroOrchestrator 正式实现该 trait
5. **comparePeriods() 完成** - 完整实现时间段性能对比功能
6. **清理未使用的 TypeScript 导出** - 移除 visualizer.ts 和 query.ts 中的死代码

## 验收结果

### 功能验收

| 验收项 | 预期 | 实际 | 状态 |
|--------|------|------|------|
| NotificationSink 仅在 zero-common 定义 | 1 处定义 | 1 处定义 | PASS |
| ConfirmationHandler 完全移除 | 0 处引用 | 0 处引用 | PASS |
| Channel trait 有设计文档 | 有说明 | 有说明 | PASS |
| HybridDecisionMaker 已实现 | MacroOrchestrator 实现 | 已实现 | PASS |
| comparePeriods() 完成 | 非桩实现 | 完整实现 | PASS |
| 未使用导出已移除 | 4 个函数移除 | 已移除 | PASS |
| 编译无错误 | cargo check 成功 | 成功 | PASS |
| Rust 测试全部通过 | 所有测试通过 | 188 passed | PASS |
| TS 测试无新增失败 | 无新增失败 | 1169 pass | PASS |

### 测试结果

```bash
# P0 编译检查
cargo check --workspace
# ✅ Finished `dev` profile

# P0 测试结果
cargo test -p zero-common -p zero-agent --lib
# ✅ 180 passed; 0 failed

# P0 验证 grep
grep -r "trait NotificationSink" services/*/src --include="*.rs"
# → 仅 zero-common/src/notification.rs

grep -r "ConfirmationHandler" services/*/src --include="*.rs"
# → 无结果

# P1 验证: HybridDecisionMaker
cargo test -p zero-trading macro_agent::orchestrator
# ✅ 8 passed

# P1 验证: TypeScript 测试
bun test test/unit
# ✅ 1169 pass (无新增失败)

# P1 验证: 死代码已移除
grep -E "formatColoredEntry|formatTimeline|formatServiceFlow|findRelatedTraces" \
  packages/ccode/src/trace/*.ts
# → 无结果 (已移除)
```

## 变更清单

### 新建文件

| 文件路径 | 说明 |
|----------|------|
| `services/zero-common/src/notification.rs` | 统一的 NotificationSink trait 定义，含完整文档和测试 |

### 修改文件

| 文件路径 | 变更说明 |
|----------|----------|
| `services/zero-common/src/lib.rs` | 添加 `pub mod notification` 和 `pub use NotificationSink` |
| `services/zero-agent/src/confirmation.rs` | 删除本地 trait 定义，改为 `pub use zero_common::NotificationSink` |
| `services/zero-agent/src/context.rs` | 移除 `confirmation_handler` 字段和 `with_handler()` 方法，简化为纯数据结构 |
| `services/zero-agent/src/lib.rs` | 从 re-export 列表移除 `ConfirmationHandler` |
| `services/zero-cli/src/channels/mod.rs` | 删除本地 trait 定义，改为 `pub use zero_common::NotificationSink` |
| `services/zero-channels/src/traits.rs` | 为 Channel trait 添加详细的 dyn-compatible 设计文档 |
| `services/zero-trading/src/macro_agent/types.rs` | 添加 trait 实现: AnalysisTrigger, AgentResult, HybridDecision |
| `services/zero-trading/src/macro_agent/orchestrator.rs` | 实现 HybridDecisionMaker trait |
| `packages/ccode/src/trace/profiler.ts` | 完成 comparePeriods() 时间段对比功能 |
| `packages/ccode/src/trace/visualizer.ts` | 删除未使用的 formatColoredEntry, formatTimeline, formatServiceFlow |
| `packages/ccode/src/trace/query.ts` | 删除未使用的 findRelatedTraces |
| `CLAUDE.md` | 更新 Agent 数量 (23→31) 和端口配置 (+4433) |
| `services/zero-cli/src/runtime/traits.rs` | 添加 RuntimeAdapter 设计文档 |
| `services/zero-cli/src/channels/mod.rs` | 添加 dead_code 注释说明 |

### 删除文件

无

## 技术要点

### 1. Trait 统一策略

将共享 trait 放到依赖树最底层的 crate (`zero-common`)，上层 crate 通过 `pub use` 重新导出。这样：
- 避免循环依赖
- 单一真实来源 (Single Source of Truth)
- 上层 crate 可以选择性重新导出

### 2. ToolContext 简化

移除 `confirmation_handler` 后，`ToolContext` 变为纯数据结构：
- 可以派生 `Default`
- 无需自定义 `Debug` 实现
- 更易于测试和序列化

### 3. Channel trait 设计决策

保留 `Channel::listen<F>` 的泛型回调设计是有意为之：
- 编译时单态化，避免虚函数调用开销
- 消息处理是热路径，性能敏感
- 需要运行时多态时，使用 `zero-cli/src/channels/traits.rs` 的 `mpsc::Sender` 版本

## 相关文档

- [CLAUDE.md](../../CLAUDE.md) - 项目整体指南
- [docs/architecture/CCODE_VS_ZERO.md](../../architecture/CCODE_VS_ZERO.md) - ccode 与 zero-* 职责划分

## 后续工作 (已完成)

根据原评估报告：

- [x] ~~**P1-1**: 完成 HybridDecisionMaker trait 实现~~ ✅
- [x] ~~**P1-2**: 完成 `comparePeriods()` 桩实现~~ ✅
- [x] ~~**P1-3**: 清理未使用的 TypeScript 导出~~ ✅
- [x] ~~**P2-1**: RuntimeAdapter 文档化~~ ✅ (添加设计说明，NativeRuntime 已覆盖主要场景)
- [x] ~~**P2-2**: 完成 Event Sourcing Phase 3-4~~ ✅ (Task #12 完成：AgentInfo/SkillUse 事件转换)
- [x] ~~**P2-3**: 更新 CLAUDE.md 中的 Agent 列表和端口配置~~ ✅
- [x] ~~**P3**: 审核 dead_code 标记~~ ✅ (70+ 标记已审核，均为有意保留)

---

## P2/P3 技术要点

### P2-1: RuntimeAdapter 文档化

在 `zero-cli/src/runtime/traits.rs` 添加完整文档：
- 当前实现状态说明
- 未来运行时设计（DockerRuntime, WorkersRuntime, EmbeddedRuntime）
- 何时添加新运行时的指导
- 示例实现代码

### P2-3: CLAUDE.md 更新

| 修改项 | 原值 | 新值 |
|--------|------|------|
| Agent 数量 | 23 | 31 (28 可见 + 3 隐藏) |
| 端口配置 | 缺少 4433 | 添加 Zero Browser: 4433 |

新增 Agent 分类：
- 主模式 (4): build、plan、writer、autonomous
- 逆向工程 (2): code-reverse、jar-code-reverse
- 工程质量 (6): general、explore、code-reviewer、security-reviewer、tdd-guide、architect
- 内容创作 (5): expander、expander-fiction、expander-nonfiction、proofreader、verifier
- 祝融说系列 (8): observer、decision、macro、trader、picker、miniproduct、ai-engineer、value-analyst
- 产品与可行性 (2): prd-generator、feasibility-assess
- 其他 (1): synton-assistant
- 系统隐藏 (3): compaction、title、summary

### P3: dead_code 标记审核

审核了 70+ 处 `#[allow(dead_code)]` 标记，分类如下：

| 类别 | 数量 | 处理 |
|------|------|------|
| API 响应结构体字段 | ~30 | 保留 - 来自外部 API，未来可能使用 |
| "Reserved for future" | ~20 | 保留 - 有明确的未来使用意图 |
| 调试/内省字段 | ~5 | 保留 - 用于日志和调试 |
| 初始化/注册方法 | ~10 | 保留并添加注释 - daemon 初始化时使用 |
| 待定功能 | ~5 | 保留 - 等待功能完整实现 |

结论：所有 `#[allow(dead_code)]` 标记均为有意保留，无需删除。clippy 检查无警告。

---

## P1 技术要点

### 4. HybridDecisionMaker 实现

MacroOrchestrator 现在正式实现 `HybridDecisionMaker` trait:

```rust
#[async_trait]
impl HybridDecisionMaker for MacroOrchestrator {
    type Context = MacroContext;
    type RuleResult = MacroEnvironment;
    type AgentResult = AgentAnalysis;
    type Decision = MacroDecision;
    type Trigger = AnalysisTrigger;
    // ... methods
}
```

同时，相关类型实现了 marker traits:
- `AnalysisTrigger` 实现 `zero_common::hybrid::AnalysisTrigger`
- `AgentAnalysis` 实现 `zero_common::hybrid::AgentResult`
- `MacroDecision` 实现 `zero_common::hybrid::HybridDecision`

### 5. comparePeriods() 实现

完整实现了时间段性能对比功能:
- 对比两个时间段的 traces 统计
- 按服务和函数维度对比
- 标记回归 (>10% 变慢) 和改善 (>10% 变快)
- 生成格式化的对比报告

### 6. 死代码清理

从 trace 模块移除的未使用导出:
- `formatColoredEntry` - 已被 trace.ts 中的 local `formatEntry` 替代
- `formatTimeline` - 设计但从未使用
- `formatServiceFlow` - 设计但从未使用
- `findRelatedTraces` - 无 CLI 命令使用

### 7. Event Sourcing Task #12 完成 (P2-2)

完成了 ImProgressHandler 的 Redis Stream 消费重构:

**修复的问题**:
- `stream_event_to_sse()` 遗漏了 `AgentInfo` 和 `SkillUse` 事件的转换
- 这导致这些事件被忽略，无法在 IM 渠道中显示

**修复内容**:
```rust
StreamTaskEvent::AgentInfo(data) => {
    Some(TaskEvent::AgentInfo(crate::sse::AgentInfoData {
        agent: data.agent.clone(),
        display_name: data.display_name.clone(),
        is_primary: Some(data.is_primary),
        duration_ms: data.duration_ms,
    }))
}
StreamTaskEvent::SkillUse(data) => {
    Some(TaskEvent::SkillUse(crate::sse::SkillUseData {
        skill: data.skill.clone(),
        args: data.args.clone(),
        duration_ms: data.duration_ms,
    }))
}
```

**事件转换完整映射**:
| StreamTaskEvent | SSE TaskEvent | 处理方法 |
|-----------------|---------------|---------|
| Thought | Thought | on_thought |
| ToolUse | ToolUse | on_tool_use |
| Progress | Progress | on_progress |
| Output | Output | on_output |
| Confirmation | Confirmation | (TODO) |
| DebugInfo | DebugInfo | on_debug_info |
| AgentInfo | AgentInfo | on_agent_info |
| SkillUse | SkillUse | on_skill_use |
| TaskCompleted | Finish(success=true) | on_finish |
| TaskFailed | Finish(success=false) | on_finish |
| TaskCreated/TaskStarted/Heartbeat/AgentSwitch | (ignored) | - |
