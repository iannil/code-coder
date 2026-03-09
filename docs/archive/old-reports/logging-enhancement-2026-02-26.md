# 日志增强实现报告

**日期:** 2026-02-26
**状态:** 已完成
**作者:** AI Assistant

## 概述

实现了 `ops.sh tail log` 命令的业务上下文日志增强，包括噪音过滤和更详细的追踪信息。

## 实现内容

### 1. Rust 日志格式增强 (logging.rs)

**文件:** `services/zero-common/src/logging.rs`

**变更:**
- 添加了 `NOISY_MODULES` 常量，默认过滤以下噪音模块：
  - `hyper`, `hyper_util`, `reqwest`, `h2`, `rustls`, `tokio_util`, `tower_http`, `tungstenite`
- 创建了 `build_filter()` 函数，自动将噪音模块设置为 `warn` 级别
- 添加了 `init_logging_with_exclusions()` 函数，支持自定义排除列表
- 新增宏：
  - `request_span!` - 创建带 trace_id 的请求 span
  - `channel_span!` - 创建 IM 渠道消息处理 span
  - `api_call_span!` - 创建 API 调用 span

### 2. 配置扩展 (config.rs)

**文件:** `services/zero-common/src/config.rs`

**变更:**
- `ObservabilityConfig` 新增字段：
  - `show_trace_id: bool` (默认 true) - 在 pretty 格式中显示 trace_id
  - `excluded_targets: Vec<String>` - 自定义排除的噪音模块列表

### 3. ops.sh 增强

**文件:** `ops.sh`

**变更:**
- 添加 `NOISE_FILTER_PATTERN` 变量，定义要过滤的噪音模式
- 修改 `tail_all_logs()` 函数：
  - 新增 `raw` 参数支持
  - 默认启用噪音过滤
  - 使用 `grep -vE` 过滤连接池/HTTP2 帧等底层日志
- 添加 `--raw` 选项支持显示全部日志
- 更新帮助文档说明新功能

### 4. 业务上下文日志 (bridge.rs, telegram/mod.rs)

**文件:** `services/zero-channels/src/bridge.rs`, `services/zero-channels/src/telegram/mod.rs`

**变更:**
- `call_codecoder()` 方法：
  - 将 `tracing::debug` 升级为 `tracing::info`
  - 添加结构化字段：`trace_id`, `user_id`, `channel`, `endpoint`, `agent`
  - 响应日志包含：`duration_ms`, `tokens`, `agent`
- Telegram 消息接收：
  - 简化日志输出，保留关键字段：`trace_id`, `user_id`, `channel_id`, `message_id`

## 使用方式

```bash
# 默认模式 - 过滤底层噪音日志
./ops.sh tail all

# 显示全部日志（含噪音）
./ops.sh tail all --raw

# 按 trace_id 搜索日志
./ops.sh logs trace <trace_id>
```

## 预期日志输出

过滤后的日志示例：
```
[zero-channels] 2026-02-26 12:11:12 INFO trace_id=abc123 user_id=765318302
  Telegram message received
[zero-channels] 2026-02-26 12:11:12 INFO trace_id=abc123 endpoint=/api/v1/chat
  → Calling CodeCoder API
[zero-channels] 2026-02-26 12:11:29 INFO trace_id=abc123 duration_ms=17100 tokens=17880
  ← API response received
```

## 验证

```bash
# 检查编译
cargo check --workspace  # ✓ 通过

# 运行测试
cargo test -p zero-common logging  # ✓ 9 测试通过
```

## 文件修改清单

| 文件 | 修改类型 |
|------|---------|
| `services/zero-common/src/logging.rs` | 增强 |
| `services/zero-common/src/config.rs` | 扩展 |
| `services/zero-channels/src/bridge.rs` | 增强 |
| `services/zero-channels/src/telegram/mod.rs` | 增强 |
| `ops.sh` | 增强 |

## 后续建议

1. **gateway 日志增强**: 可以在 `zero-gateway/src/routes.rs` 中添加类似的请求追踪日志
2. **日志聚合**: 考虑使用 Loki + Grafana 进行集中日志分析
3. **trace_id 关联**: 实现跨服务 trace_id 传播，便于全链路追踪
