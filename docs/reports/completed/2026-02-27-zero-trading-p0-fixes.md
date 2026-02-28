# Zero Trading P0 问题修复

## 修改时间

2026-02-27

## 修改概要

根据 Zero Trading 执行情况评估报告，完成了以下 P0/P1 级别问题的修复：

### 1. 仓位大小计算 (P0)

**位置**: `src/loop/trading_loop.rs`

**问题**: `execute_entry()` 方法中硬编码 `quantity: 100.0`

**修复**: 使用 `ExecutionEngine` 返回的 `Order.quantity`，该值基于风险计算：
```
quantity = (2% of capital) / (entry_price - stop_loss)
```

### 2. 日内盈亏追踪 (P0)

**位置**: `src/execution/executor.rs`

**问题**: `PaperExecutor::get_account()` 始终返回 `realized_pnl_today: 0.0`

**修复**:
- 新增 `DailyPnlState` 结构体追踪每日已实现盈亏
- 卖出时计算盈亏：`(exit_price - entry_price) * quantity`
- 每日自动重置（检测日期变化）

### 3. ScreenerScheduler 集成 (P0)

**位置**: `src/lib.rs`, `src/routes.rs`

**问题**: 5 处 TODO 标记的 screener 路由未实现

**修复**:
- 新增 `ScreenerSchedulerImpl` 类型别名（使用 `LixinAdapter` 提供基本面数据）
- 在 `TradingState` 中新增 `screener_scheduler: Option<Arc<ScreenerSchedulerImpl>>`
- 实现完整的 screener API 路由：
  - `GET /api/v1/screener/status` - 获取调度器状态
  - `POST /api/v1/screener/run` - 触发筛选扫描
  - `GET /api/v1/screener/results` - 获取最新结果
  - `GET /api/v1/screener/history` - 获取历史记录
  - `POST /api/v1/screener/sync` - 触发数据同步

**依赖**: 需要配置 Lixin API Token (`secrets.external.lixin`)

### 4. 理杏仁 API gzip 合规性修复 (P0)

**位置**: `services/Cargo.toml:23`

**问题**: reqwest 未启用 `gzip` feature，导致请求不会自动添加 `Accept-Encoding: gzip` 头，不符合理杏仁 API 要求

**要求对照**:
| 要求 | 状态 | 说明 |
|------|------|------|
| 频率限制 ≤ 1000 次/分钟 | ✅ | 默认 100 次/分钟 |
| 429 状态码处理 | ✅ | 正确返回 `RateLimited` 错误 |
| Content-Type: application/json | ✅ | `.json()` 方法自动设置 |
| accept-encoding: gzip | ✅ 已修复 | 启用 reqwest `gzip` feature |

**修复**:
```diff
- reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream", "multipart"] }
+ reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream", "multipart", "gzip"] }
```

**验证**: `cargo build -p zero-trading` 成功，`cargo test lixin` 通过

### 5. 死代码警告修复 (P1)

**位置**:
- `src/data/lixin.rs` - `LixinIncomeStatement::report_date`, `LixinCashFlow::report_date`
- `src/task_scheduler/mod.rs` - `TaskScheduler::strategy`

**修复**: 添加 `#[allow(dead_code)]` 注释说明保留原因（API 反序列化/组件所有权）

## 验证结果

```bash
# 编译
cargo build -p zero-trading
# 结果: 成功，1 个警告 (unused_comparisons)

# 测试
cargo test
# 结果: 380 个测试通过，0 失败
```

## 遗留问题

根据评估报告，以下问题暂未在本次修复范围内：

1. **unwrap/expect 使用** (226 处): 建议分批处理关键路径
2. **Lixin 统计 API** (`data/lixin.rs:909`): 低优先级占位符
3. **策略/执行/组合模块单元测试**: 中期补充

## 相关文件

- `services/Cargo.toml` (gzip feature)
- `services/zero-trading/src/loop/trading_loop.rs`
- `services/zero-trading/src/execution/executor.rs`
- `services/zero-trading/src/lib.rs`
- `services/zero-trading/src/routes.rs`
- `services/zero-trading/src/data/lixin.rs`
- `services/zero-trading/src/task_scheduler/mod.rs`
