# CodeCoder 硬编码风险审计 - 修复进度

**日期**: 2026-03-02
**状态**: Phase 1 & 2 完成

## 进度概述

| 阶段 | 状态 | 说明 |
|------|------|------|
| Phase 1: CRITICAL | 已完成 | 交易风险参数配置化 |
| Phase 2: HIGH | 已完成 | 服务端点配置化 |
| Phase 3: MEDIUM | 待处理 | 后续迭代 |

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

---

## 验证结果

### Rust 编译检查

```bash
cd services && cargo check -p zero-trading
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.93s
```

---

## 待处理项目 (Phase 3: MEDIUM)

| 项目 | 文件 | 说明 |
|------|------|------|
| 超时配置化 | 多文件 | 各服务的超时时间 |
| 限流参数适配 | `itick.rs` | API 提供商限流参数 |
| 选股阈值可配置 | `screener/config.rs` | ROE、毛利、负债率阈值 |
| E2E 测试路径 | `test/e2e/**/*.ts` | 42+ 文件中的绝对路径 |

---

## 配置使用说明

### 环境变量覆盖

```bash
# HITL 客户端网关地址
export GATEWAY_URL=http://custom-gateway:4430
# 或
export ZERO_GATEWAY_URL=http://custom-gateway:4430
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
