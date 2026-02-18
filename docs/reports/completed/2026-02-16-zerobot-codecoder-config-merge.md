# ZeroBot 配置合并到 CodeCoder

**日期**: 2026-02-16
**状态**: 已完成

## 概述

将 ZeroBot 配置合并到 CodeCoder 配置文件中，实现配置统一管理。用户只需在 `~/.codecoder/codecoder.json` 中配置 API keys 和其他设置，ZeroBot 即可自动读取并使用。

## 修改内容

### Phase 1: CodeCoder 侧 - 添加 ZeroBot 配置 Schema

**文件**: `packages/ccode/src/config/config.ts`

添加了以下 Zod schema:
- `ZeroBotObservability` - 可观测性配置 (none, log, prometheus, otel)
- `ZeroBotAutonomy` - 自主性配置 (readonly, supervised, full)
- `ZeroBotRuntime` - 运行时配置 (native, docker, cloudflare)
- `ZeroBotReliability` - 可靠性配置 (重试、回退等)
- `ZeroBotHeartbeat` - 心跳配置
- `ZeroBotMemory` - 内存配置 (sqlite, markdown, none)
- `ZeroBotGateway` - 网关配置 (端口、主机、配对)
- `ZeroBotTunnel` - 隧道配置 (cloudflare, tailscale, ngrok)
- `ZeroBotChannels` - 渠道配置 (telegram, discord, slack, whatsapp)
- `ZeroBotBrowser` - 浏览器配置
- `ZeroBotIdentity` - 身份配置 (openclaw, aieos)
- `ZeroBot` - 顶层 ZeroBot 配置

在 `Info` schema 中添加了 `zerobot` 字段。

### Phase 2: ZeroBot 侧 - 支持读取 JSON 配置

**文件**: `services/zero-bot/src/config/schema.rs`

添加了以下功能:

1. **JSON 中间结构体**: 用于从 JSON 反序列化的结构体族
   - `ZeroBotJsonConfig`
   - `ZeroBotJsonObservability`
   - `ZeroBotJsonAutonomy`
   - ... 等

2. **JSONC 注释剥离**: `strip_json_comments()` 函数
   - 支持 `//` 单行注释
   - 支持 `/* */` 多行注释
   - 保留字符串内的斜杠

3. **环境变量解析**: `resolve_env_vars()` 函数
   - 支持 `{env:VAR}` 语法
   - 与 CodeCoder 保持一致

4. **配置加载**: `load_from_codecoder()` 方法
   - 尝试读取 `~/.codecoder/codecoder.json`、`codecoder.jsonc`、`config.json`
   - 从 `provider` 节提取 API key
   - 转换为 ZeroBot 配置结构

5. **配置转换**: `from_json_config()` 方法
   - 将 JSON 配置转换为 Config 结构
   - 对未指定字段使用默认值

6. **向后兼容**: 修改 `load_or_init()` 方法
   - 优先尝试从 CodeCoder 配置加载
   - 若失败则回退到 TOML 配置

### 测试覆盖

添加了 17 个新测试:
- `strip_json_comments_*` - JSONC 注释剥离测试
- `resolve_env_vars_*` - 环境变量解析测试
- `zerobot_json_config_*` - JSON 配置解析测试
- `config_from_json_*` - 配置转换测试

## 配置示例

```jsonc
{
  "$schema": "https://code-coder.com/config.json",

  // 共享配置（CodeCoder + ZeroBot 都使用）
  "model": "anthropic/claude-sonnet-4",
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    },
    "openrouter": {
      "options": {
        "apiKey": "{env:OPENROUTER_API_KEY}"
      }
    }
  },

  // ZeroBot 特有配置
  "zerobot": {
    "default_provider": "openrouter",
    "default_model": "anthropic/claude-sonnet-4-20250514",
    "default_temperature": 0.7,

    "memory": {
      "backend": "sqlite",
      "auto_save": true
    },

    "gateway": {
      "port": 8080,
      "host": "127.0.0.1"
    },

    "channels": {
      "telegram": {
        "bot_token": "{env:TELEGRAM_BOT_TOKEN}",
        "allowed_users": ["your_username"]
      }
    },

    "autonomy": {
      "level": "supervised",
      "workspace_only": true,
      "max_actions_per_hour": 20
    }
  }
}
```

## 验证步骤

1. ✅ CodeCoder TypeScript 类型检查 - `bun turbo typecheck` (ZeroBot schema 验证通过)
2. ✅ ZeroBot Rust 构建 - `cargo build` (通过)
3. ✅ ZeroBot 测试 - `cargo test config::schema::tests` (80 个测试通过)

## 迁移指南

用户可以将现有的 `~/.codecoder/config.toml` 配置迁移到 `~/.codecoder/codecoder.json`:

1. 在 `codecoder.json` 中添加 `zerobot` 节
2. 将 API keys 移至 `provider` 节
3. ZeroBot 会自动检测并使用 JSON 配置
4. 原有 TOML 配置作为回退继续工作

## 后续工作

- [ ] 添加 `zero-bot migrate config` 命令自动迁移
- [ ] 更新 ZeroBot 向导以写入 JSON 格式
- [ ] 添加更多集成测试
