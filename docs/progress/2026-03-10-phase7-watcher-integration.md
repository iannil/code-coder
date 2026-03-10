# Phase 7: Watcher 集成与 TypeScript 清理

> 日期: 2026-03-10
> 状态: ✅ 完成
> 时间: 完成

## 概述

完成 Watcher 系统与 ObserverNetwork 的集成，并清理 TypeScript 冗余代码。这是架构重构的最后阶段。

## 实施内容

### 1. WatcherManager 完整实现

**文件**: `services/zero-cli/src/observer/watchers/mod.rs`

新增 `run_observation_loop` 方法，支持持续观察并通过 channel 发送观察结果：

```rust
pub async fn run_observation_loop(
    &self,
    tx: mpsc::Sender<Vec<Observation>>,
    interval_ms: u64,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    // 使用 tokio::select! 处理定时器和关闭信号
    // 批量收集观察结果并发送到 ConsensusEngine
}
```

**关键特性**:
- 基于 `tokio::select!` 的并发事件处理
- 通过 `watch` channel 实现优雅关闭
- 批量发送观察结果以提高效率

### 2. WatcherManager → ObserverNetwork 连接

**文件**: `services/zero-cli/src/observer/network.rs`

**变更**:

1. `ObserverNetworkState` 新增字段:
   - `watcher_manager: Arc<RwLock<WatcherManager>>` - Watcher 管理器
   - `shutdown_tx/rx` - 关闭信号 channel

2. `ObserverNetworkConfig` 新增:
   - `observation_interval_ms: u64` - 观察间隔配置 (默认 5000ms)

3. `start()` 方法增强:
   - 启动所有注册的 Watcher
   - 启动观察循环任务 (observation loop)
   - 启动共识喂送任务 (consensus feeder)
   - 观察结果自动路由到 ConsensusEngine

4. `stop()` 方法增强:
   - 发送关闭信号
   - 停止所有 Watcher
   - 停止 ConsensusEngine

5. 新增方法:
   - `get_watcher_statuses()` - 获取所有 Watcher 状态

6. 新增事件类型:
   - `WatcherStarted { watcher_id, watcher_type }`
   - `WatcherStopped { watcher_id, watcher_type }`

### 3. Daemon 初始化集成

**文件**: `services/zero-cli/src/daemon/mod.rs`

在 daemon 启动时注册四大 Watcher:

```rust
// 注册四大 Watcher
let mut watcher_manager = observer_state.get_watcher_manager_mut().await;

// CodeWatch: 观察代码变更
watcher_manager.register(Box::new(CodeWatch::new(CodeWatchConfig {
    git_root: Some(config.workspace_dir.clone()),
    track_build: true,
    enable_typecheck: false,
    ..Default::default()
})));

// WorldWatch: 观察外部世界
watcher_manager.register(Box::new(WorldWatch::new(WorldWatchConfig::default())));

// SelfWatch: 观察系统行为
watcher_manager.register(Box::new(SelfWatch::new(SelfWatchConfig::default())));

// MetaWatch: 观察观察者网络本身
watcher_manager.register(Box::new(MetaWatch::new(MetaWatchConfig::default())));
```

### 4. TypeScript Watcher 清理

**受影响文件**:
- `packages/ccode/src/observer/watchers/base-watcher.ts` - @deprecated
- `packages/ccode/src/observer/watchers/code-watch.ts` - @deprecated
- `packages/ccode/src/observer/watchers/world-watch.ts` - @deprecated
- `packages/ccode/src/observer/watchers/self-watch.ts` - @deprecated
- `packages/ccode/src/observer/watchers/meta-watch.ts` - @deprecated
- `packages/ccode/src/observer/watchers/index.ts` - 更新导出说明

所有文件均添加了 `@deprecated` JSDoc 注释，指向 Rust 实现并说明迁移状态。

## 测试结果

```
test result: ok. 60 passed; 0 failed; 0 ignored
```

所有 observer 模块测试通过，包括新增的:
- `test_watcher_manager_count`
- `test_observation_loop_shutdown`

## 架构总结

完成 Phase 7 后，Observer Network 架构如下：

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Daemon Startup                                   │
│   1. Create ObserverNetwork                                         │
│   2. Register 4 Watchers (Code/World/Self/Meta)                    │
│   3. Start ObserverNetwork                                         │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    ObserverNetwork.start()                          │
│   1. Start ConsensusEngine                                          │
│   2. Start all Watchers                                            │
│   3. Spawn observation loop → mpsc → ConsensusEngine               │
│   4. Spawn consensus update loop                                   │
└─────────────────────────────────────────────────────────────────────┘
```

**数据流**:
```
Watchers → run_observation_loop() → mpsc<Vec<Observation>>
    → consensus_feeder → ConsensusEngine.add_observation()
    → WorldModel 更新 → ObserverNetworkEvent 广播
```

## 关键文件变更

| 文件 | 变更类型 | 变更内容 |
|------|----------|----------|
| `observer/watchers/mod.rs` | 修改 | 添加 run_observation_loop, watcher_count |
| `observer/network.rs` | 修改 | 集成 WatcherManager, 添加 shutdown channel |
| `daemon/mod.rs` | 修改 | 注册四大 Watcher |
| `unified_api/observer.rs` | 修改 | 处理新事件类型 |
| `packages/ccode/.../watchers/*.ts` | 修改 | 添加 @deprecated 注释 |

## 后续工作

- [ ] 完善 WorldWatch 的 Agent 轮询功能
- [ ] 添加 Watcher 配置的运行时动态调整
- [ ] 实现 Watcher 热重启能力
- [ ] 完善 CLOSE 评估与 Gear 自动切换

## 验收检查

- [x] 所有测试通过 (60 passed)
- [x] 代码编译无错误
- [x] Watcher 在 daemon 启动时正确注册
- [x] 观察循环正确运行并路由到 ConsensusEngine
- [x] TypeScript 文件标记为 @deprecated
