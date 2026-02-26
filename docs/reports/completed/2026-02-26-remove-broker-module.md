# 重构报告：移除 zero-trading 自动执行交易逻辑

**日期**: 2026-02-26
**状态**: 已完成

## 背景

用户因缺少可用的自动执行交易接口（如 Futu OpenAPI 需要 OpenD 桌面程序），决定暂时放弃自动执行交易功能。系统转变为：

**信号生成** → **IM 推送** → **用户手动执行**

## 删除范围

### 已删除文件（4个）

| 文件 | 说明 |
|------|------|
| `src/broker/mod.rs` | Broker trait 定义和 AccountInfo 类型 |
| `src/broker/futu.rs` | Futu OpenAPI 适配器 |
| `src/broker/push_handler.rs` | 推送消息处理 |
| `src/broker/errors.rs` | Broker 错误类型 |

### 已修改文件（4个）

| 文件 | 修改内容 |
|------|------|
| `src/lib.rs` | 移除 `pub mod broker` |
| `src/execution/mod.rs` | 删除 `execute_via_broker()` 方法，移除 `LiveExecutor` 导出 |
| `src/execution/executor.rs` | 删除 `LiveExecutor`，将 `AccountInfo` 移入本模块，简化 `create_executor()` |
| `src/routes.rs` | `broker_connected` 固定返回 `false` |

## 保留模块

| 模块 | 说明 |
|------|------|
| `notification.rs` | 核心功能，负责推送信号到 IM |
| `strategy/` | 信号生成逻辑 |
| `data/` | 市场数据获取 |
| `paper_trading/` | 保留用于策略验证（不涉及真实资金） |
| `execution/` | 保留 Order、Position、PaperExecutor 等基础结构 |
| `loop/` | 保留交易循环，用于定时扫描信号 |
| `macro_agent/` | 宏观分析，辅助决策 |

## 验证结果

- `cargo build -p zero-trading`: 编译通过（仅有预存的 warnings）
- `cargo test -p zero-trading`: 154 单元测试 + 18 集成测试全部通过

## 架构变更说明

### 执行器架构（之前）

```text
TradingExecutor (trait)
       |
   +---+---+
   |       |
   v       v
PaperExecutor  LiveExecutor
(simulation)   (real orders)
                   |
                   v
                Broker
```

### 执行器架构（之后）

```text
TradingExecutor (trait)
       |
       v
PaperExecutor
(simulation)
```

### API 兼容性

- `GET /api/v1/status` 中的 `broker_connected` 字段保留但固定返回 `false`
- 其他所有 API 端点保持不变
- Paper trading API 完全保留

## 后续建议

1. 如果未来需要恢复 broker 功能，可以重新实现 `LiveExecutor` 并更新 `create_executor()`
2. 当前信号通知流程：信号生成 → `notification.rs` → Telegram/其他 IM → 用户手动执行
3. Paper trading 可用于策略验证，无需真实资金
