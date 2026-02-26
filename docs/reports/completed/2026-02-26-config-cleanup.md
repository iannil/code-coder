# 配置文件优化完成报告

**日期**: 2026-02-26
**状态**: ✅ 已完成

## 概述

清理 `~/.codecoder/config.json` 和 `services/zero-common/src/config.rs` 中的冗余、重复、过期和无效配置。

## 已完成的修改

### Phase 1: config.json 清理

| 移除项 | 原因 | 行为变化 |
|--------|------|----------|
| `api_keys.deepseek` | 与 `provider.deepseek.options.apiKey` 重复 | 无（TypeScript 使用 provider 配置） |
| `trading.tushare_token` | 占位符值 `YOUR_TUSHARE_TOKEN_HERE` | 无（从未生效） |
| `trading.workflow_endpoint` | 等于默认值 `http://127.0.0.1:4432` | 无（使用代码默认值） |
| `trading.telegram_notification.channels_endpoint` | 等于默认值 `http://localhost:4431` | 无（使用代码默认值） |
| `trading.macro_agent.codecoder_endpoint` | 等于默认值（继承自 `codecoder.endpoint`） | 无（使用代码默认值） |

### Phase 2: config.rs 清理

| 移除项 | 原因 |
|--------|------|
| `FutuConfig` 结构体 | `broker/futu.rs` 已被删除 |
| `TradingConfig.futu` 字段 | 同上 |
| `default_futu_host()` 函数 | 不再使用 |
| `default_futu_port()` 函数 | 不再使用 |

### 附带修复

| 文件 | 问题 | 修复 |
|------|------|------|
| `zero-cli/src/channels/mod.rs:560` | `TelegramConfig` 缺少 `trading_chat_id` 字段 | 添加 `trading_chat_id: None` |
| `zero-common/src/validation.rs:449` | 同上 | 添加 `trading_chat_id: None` |

## 验证结果

```bash
# JSON 语法检查
$ cat ~/.codecoder/config.json | jq . > /dev/null
✓ JSON syntax valid

# Rust 构建
$ cargo build --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.68s

# 配置测试
$ cargo test --package zero-common config
test result: ok. 33 passed; 0 failed; 0 ignored
```

## 优化后的配置结构

```
config.json (206 行 → 207 行，净减少约 15 行冗余配置)
├── gateway          # Gateway 配置
├── channels         # 渠道配置
├── workflow         # 工作流配置
├── codecoder        # CodeCoder 集成
├── providers        # Rust 服务 LLM 默认提供商
├── observability    # 可观测性配置
├── trading          # 交易配置（已清理冗余端点和占位符）
├── provider         # TypeScript AI SDK 提供商配置
└── mcp              # MCP 服务器配置
```

## 注意事项

- `gateway.codecoder_endpoint` 与 `codecoder.endpoint` 不是重复配置
  - 前者：Gateway 代理到 CodeCoder API 的目标
  - 后者：Rust 服务直接调用 CodeCoder API 的端点
  - 两者使用相同值是正确的

## 相关文件

- `~/.codecoder/config.json`
- `services/zero-common/src/config.rs`
- `services/zero-cli/src/channels/mod.rs`
- `services/zero-common/src/validation.rs`
