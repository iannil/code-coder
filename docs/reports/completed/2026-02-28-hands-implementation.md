# Hands 系统实现完成报告

**完成时间**: 2026-02-28 13:10

## 实现内容

为 CodeCoder 创建了 4 个定时任务 Hands，覆盖高价值 Agents。

### 新增 Hands

| Hand ID | 名称 | 调度时间 | Agent | 自治级别 | 下次执行 (UTC) |
|---------|------|----------|-------|----------|----------------|
| `daily-health-check` | 每日代码健康检查 | 工作日 08:00 | verifier | bold | 2026-03-01 08:00 |
| `weekly-value-analysis` | 周度价值分析 | 周一 09:00 | value-analyst | crazy | 2026-03-01 09:00 |
| `ai-engineer-weekly` | AI 工程学习周报 | 周五 18:00 | ai-engineer | wild | 2026-03-05 18:00 |
| `miniproduct-tracker` | 极小产品进度追踪 | 周日 20:00 | miniproduct | crazy | 2026-02-28 20:00 |

### 文件位置

```
~/.codecoder/hands/
├── daily-health-check/HAND.md    (1096 bytes)
├── weekly-value-analysis/HAND.md (1025 bytes)
├── ai-engineer-weekly/HAND.md    (907 bytes)
└── miniproduct-tracker/HAND.md   (983 bytes)
```

## 代码修复

### 1. State 共享问题

**问题**: `start()` 和 `build_router()` 各自创建独立的 `WorkflowState`，导致 hands_scheduler 设置在一个 state 上，但 API 使用的是另一个 state。

**解决方案**: 在 `lib.rs` 中添加 `build_router_with_state()` 方法：

```rust
/// Build the workflow router with an existing state.
fn build_router_with_state(&self, state: Arc<WorkflowState>) -> Router {
    // ... 使用传入的 state 而非创建新的
}
```

修改 `start()` 方法使用新方法：
```rust
let router = self.build_router_with_state(state.clone());
```

### 2. Cron 表达式问题

**问题**: `cron` crate 不支持 `0` 作为周日的表示。

**解决方案**: 将 `miniproduct-tracker` 的 schedule 从 `0 0 20 * * 0` 改为 `0 0 20 * * 7`。

### 3. 配置启用

在 `~/.codecoder/config.json` 的 `workflow` 部分添加：
```json
"hands": { "enabled": true, "max_history": 100 }
```

## 验证结果

```bash
# API 返回 4 个已加载的 hands
curl http://localhost:4432/api/v1/hands
# {"success":true,"data":[...4 hands...]}

# 状态显示调度器已启用
curl http://localhost:4432/api/v1/hands/status
# {"success":true,"data":{"enabled":true,"hands_count":4,...}}
```

## 验证命令

```bash
# 列出所有 hands
curl http://localhost:4432/api/v1/hands

# 查看调度器状态
curl http://localhost:4432/api/v1/hands/status

# 手动触发测试
curl -X POST http://localhost:4432/api/v1/hands/daily-health-check/trigger

# 查看执行历史
curl http://localhost:4432/api/v1/hands/daily-health-check/executions
```

## 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `services/zero-workflow/src/lib.rs` | 添加 `build_router_with_state()` 方法，修复 state 共享 |
| `~/.codecoder/config.json` | 添加 `hands.enabled: true` 配置 |
| `~/.codecoder/hands/miniproduct-tracker/HAND.md` | 修正 cron 表达式 |
| `~/.codecoder/hands/daily-health-check/HAND.md` | 新建 |
| `~/.codecoder/hands/weekly-value-analysis/HAND.md` | 新建 |
| `~/.codecoder/hands/ai-engineer-weekly/HAND.md` | 新建 |
