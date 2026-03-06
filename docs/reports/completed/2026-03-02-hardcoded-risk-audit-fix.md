# CodeCoder 硬编码风险审计 - 修复进度

**日期**: 2026-03-02 (Phase 3 更新: 2026-03-06)
**状态**: Phase 1, 2 & 3 完成

## 进度概述

| 阶段 | 状态 | 说明 |
|------|------|------|
| Phase 1: CRITICAL | 已完成 | 交易风险参数配置化 |
| Phase 2: HIGH | 已完成 | 服务端点配置化 |
| Phase 3: MEDIUM | 已完成 | 超时配置化、限流参数、E2E 测试路径 |

---

## 已完成的修复

### Phase 1: CRITICAL

#### 1. 交易风险参数配置化

**文件**: `services/zero-common/src/config/types.rs`

新增 `T1RiskConfig` 结构体，包含以下可配置参数：

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `stop_loss_pct` | 5.0 | 止损百分比 |
| `take_profit_pct` | 10.0 | 止盈百分比 |
| `max_loss_per_trade_pct` | 2.0 | 单笔最大亏损百分比 |
| `gap_down_tolerance` | 2.0 | 缺口下跌容忍度 |
| `gap_up_threshold` | 3.0 | 缺口上涨阈值 |
| `limit_pct` | 10.0 | 涨跌停百分比 |
| `typical_overnight_gap_pct` | 2.0 | 典型隔夜缺口百分比 |
| `initial_capital` | 100000.0 | 初始资金 |

**原问题**:
- `t1_risk.rs:86` 硬编码 `-2.0` 缺口容忍度
- `t1_risk.rs:96` 硬编码 `3.0` 缺口上涨阈值
- `t1_risk.rs:195-196` 硬编码 `0.9/1.1` (10%涨跌停)
- `execution/mod.rs:110` 硬编码 `100000.0` 初始资金

#### 2. T1风险管理器配置化

**文件**: `services/zero-trading/src/execution/t1_risk.rs`

修改内容：
- 更新 `T1RiskConfig` 结构体，添加缺口容忍度和涨跌停配置
- `next_day_decision()` 使用 `self.config.gap_down_tolerance` 替代硬编码值
- `next_day_decision()` 使用 `self.config.gap_up_threshold` 替代硬编码值
- `evaluate_gap_risk()` 使用 `self.config.limit_pct` 替代硬编码值

#### 3. 执行引擎资金配置化

**文件**: `services/zero-trading/src/execution/mod.rs`

修改内容：
- 从 `trading.risk_params` 加载风险配置
- 从配置读取 `initial_capital` 替代硬编码 `100000.0`
- 使用 `risk_manager.calculate_position_size()` 替代硬编码风险比例

### Phase 2: HIGH

#### 4. HITL 客户端网关地址配置化

**文件**: `packages/ccode/src/hitl/client.ts`

修改内容：
```typescript
// 修改前
const DEFAULT_GATEWAY_URL = "http://127.0.0.1:4430"

// 修改后
const DEFAULT_GATEWAY_URL =
  process.env.GATEWAY_URL ||
  process.env.ZERO_GATEWAY_URL ||
  "http://127.0.0.1:4430"
```

### 示例配置更新

**文件**: `example/trading.json`

新增 `risk_params` 配置节：

```json
{
  "risk_params": {
    "stop_loss_pct": 5.0,
    "take_profit_pct": 10.0,
    "max_loss_per_trade_pct": 2.0,
    "gap_down_tolerance": 2.0,
    "gap_up_threshold": 3.0,
    "limit_pct": 10.0,
    "typical_overnight_gap_pct": 2.0,
    "initial_capital": 100000.0
  }
}
```

### Phase 3: MEDIUM (2026-03-06)

#### 5. 限流参数配置文档化

**文件**: `services/zero-trading/src/data/itick.rs`、`services/zero-trading/src/data/lixin.rs`

修改内容：
- 重命名 `RATE_LIMIT_RETRY_SECS` 为 `DEFAULT_RETRY_DELAY_SECS`
- 添加配置文档：`rate_limit_rpm` 和 `retry_delay_secs` 可通过 `data_sources` 配置覆盖

```rust
/// Default rate limit: 5 requests per second = 300 per minute
/// Can be overridden via data_sources config: `rate_limit_rpm`
const DEFAULT_RATE_LIMIT_RPM: u32 = 300;

/// Default retry delay after rate limit error (seconds)
/// Can be overridden via data_sources config: `retry_delay_secs`
const DEFAULT_RETRY_DELAY_SECS: u64 = 2;
```

**注意**: 选股阈值 (`screener/config.rs`) 已在之前版本中完成配置化，无需修改。

#### 6. 超时配置化

**文件**: `packages/ccode/src/config/timeouts.ts` (新建)

创建集中式超时配置模块，包含 30+ 超时常量：

| 类别 | 常量 | 默认值 | 环境变量 |
|------|------|--------|----------|
| Tool | WEBFETCH_DEFAULT_TIMEOUT_MS | 30s | CCODE_TIMEOUT_WEBFETCH_DEFAULT |
| Tool | WEBFETCH_MAX_TIMEOUT_MS | 2min | CCODE_TIMEOUT_WEBFETCH_MAX |
| Tool | SCHEDULER_REQUEST_TIMEOUT_MS | 10s | CCODE_TIMEOUT_SCHEDULER_REQUEST |
| HITL | HITL_CLIENT_TIMEOUT_MS | 30s | CCODE_TIMEOUT_HITL_CLIENT |
| HITL | HITL_HEALTH_CHECK_TIMEOUT_MS | 5s | CCODE_TIMEOUT_HITL_HEALTH |
| Autonomous | HANDS_BRIDGE_TIMEOUT_MS | 30s | CCODE_TIMEOUT_HANDS_BRIDGE |
| Autonomous | AUTONOMOUS_HEALTH_CHECK_TIMEOUT_MS | 5s | CCODE_TIMEOUT_AUTONOMOUS_HEALTH |
| ... | ... | ... | ... |

已更新的文件：
- `packages/ccode/src/tool/webfetch.ts`
- `packages/ccode/src/tool/scheduler.ts`
- `packages/ccode/src/hitl/client.ts`
- `packages/ccode/src/autonomous/hands/bridge.ts`

#### 7. E2E 测试路径配置化

**文件**: `packages/ccode/test/helpers/paths.ts` (新建)

创建测试路径助手模块：

```typescript
export function getProjectRoot(): string {
  return resolve(import.meta.dir, "../..")
}

export const TestPaths = {
  get cwd(): string {
    return getProjectRoot()
  },
  // ...
} as const
```

已更新的测试文件 (12 个文件，51 处替换)：
- `test/e2e/tui/visual/*.test.ts` (3 files)
- `test/e2e/tui/critical/*.test.ts` (3 files)
- `test/e2e/tui/high/*.test.ts` (4 files)
- `test/e2e/tui/medium/*.test.ts` (2 files)
```

---

## 验证结果

### Phase 1 & 2: Rust 编译检查

```bash
cd services && cargo check -p zero-trading
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.93s
```

### Phase 3: TypeScript 类型检查

```bash
bun turbo typecheck --filter=ccode
# Tasks:    1 successful, 1 total
```

### Phase 3: 硬编码路径验证

```bash
grep -r "/Users/iannil" packages/ccode/test/e2e/tui
# Found 0 total occurrences across 0 files.
```

---

## 所有阶段已完成

Phase 1, 2, 3 的所有硬编码风险项目已修复完成。

### 后续维护建议

1. **新增超时配置**: 在 `packages/ccode/src/config/timeouts.ts` 中添加新常量
2. **新增数据源配置**: 在 `data_sources.sources[].config` 中添加 provider 特定配置
3. **新增测试文件**: 使用 `TestPaths.cwd` 替代硬编码路径

---

## 配置使用说明

### 环境变量覆盖

```bash
# HITL 客户端网关地址
export GATEWAY_URL=http://custom-gateway:4430
# 或
export ZERO_GATEWAY_URL=http://custom-gateway:4430

# 超时配置覆盖 (Phase 3 新增)
export CCODE_TIMEOUT_WEBFETCH_DEFAULT=60000  # 60 seconds
export CCODE_TIMEOUT_HITL_CLIENT=60000       # 60 seconds
```

### trading.json 配置

将 `risk_params` 添加到 `~/.codecoder/trading.json`:

```json
{
  "risk_params": {
    "initial_capital": 200000.0,
    "stop_loss_pct": 3.0,
    "limit_pct": 20.0
  }
}
```

注意：只需指定需要覆盖的字段，其他字段使用默认值。

### 科创板/创业板配置示例

```json
{
  "risk_params": {
    "limit_pct": 20.0
  }
}
```

---

## 关键代码位置

| 功能 | 文件 | 说明 |
|------|------|------|
| T1RiskConfig (config) | `services/zero-common/src/config/types.rs:~2940` | 配置定义 |
| T1RiskConfig (runtime) | `services/zero-trading/src/execution/t1_risk.rs:17-43` | 运行时结构 |
| ExecutionEngine | `services/zero-trading/src/execution/mod.rs:100-130` | 资金初始化 |
| HITL 网关 | `packages/ccode/src/hitl/client.ts:175-179` | 地址配置 |
| 超时配置 | `packages/ccode/src/config/timeouts.ts` | 集中式超时定义 |
| 测试路径助手 | `packages/ccode/test/helpers/paths.ts` | 测试路径工具 |
| iTick 限流 | `services/zero-trading/src/data/itick.rs:48-53` | 限流参数文档 |
| Lixin 限流 | `services/zero-trading/src/data/lixin.rs:66-71` | 限流参数文档 |
