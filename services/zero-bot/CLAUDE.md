# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此代码仓库中工作时提供指导。

## 项目概述

ZeroBot 是一个用 Rust 编写的轻量级、基于 trait 的 AI 助手基础设施。它提供可插拔的 providers（24 个 LLM 后端）、channels（Telegram、Discord、Slack 等）、memory 后端和 tools。二进制文件约 3.4MB，内存占用小于 5MB。

## 构建与开发命令

```bash
cargo build                    # 开发构建
cargo build --release          # 发布构建（约 3.4MB）
cargo test                     # 运行所有测试（1,811 个测试）
cargo clippy -- -D warnings    # 代码检查（必须通过，零警告）
cargo fmt                      # 格式化代码

# 运行特定测试
cargo test test_name

# 运行 SQLite vs Markdown 内存基准测试
cargo test --test memory_comparison -- --nocapture

# 本地安装用于 CLI 测试
cargo install --path . --force
```

### Pre-push Hook

启用 pre-push hook（推送前运行 fmt、clippy、tests）：
```bash
git config core.hooksPath .githooks
```

快速迭代时跳过：`git push --no-verify`

## 架构

每个子系统都是一个 **trait** — 通过配置切换实现，无需修改代码。

```
src/
├── providers/       # LLM 后端          → Provider trait
├── channels/        # 消息通道          → Channel trait
├── tools/           # Agent 能力        → Tool trait
├── memory/          # 持久化存储        → Memory trait
├── observability/   # 指标/日志         → Observer trait
├── runtime/         # 平台适配器        → RuntimeAdapter trait
├── tunnel/          # 隧道提供者        → Tunnel trait
├── security/        # 沙箱安全          → SecurityPolicy
├── agent/           # Agent 循环编排
├── gateway/         # HTTP webhook 服务器（axum）
├── daemon/          # 长期运行的自主运行时
├── config/          # TOML 配置 schema 和加载
├── onboard/         # 设置向导
└── skills/          # TOML manifest + SKILL.md 加载器
```

### 核心 Traits

| Trait | 位置 | 用途 |
|-------|------|------|
| `Provider` | `src/providers/traits.rs` | LLM 聊天接口：`chat_with_system()` |
| `Channel` | `src/channels/traits.rs` | 消息通道：`send()`、`listen()`、`health_check()` |
| `Tool` | `src/tools/traits.rs` | Agent 能力：`execute()`、`parameters_schema()` |
| `Memory` | `src/memory/traits.rs` | 持久化：`store()`、`recall()`、`forget()` |
| `Observer` | `src/observability/traits.rs` | 指标：`record_event()`、`record_metric()` |

### 添加新实现

1. 创建 `src/<subsystem>/your_impl.rs`
2. 实现对应的 trait
3. 在 `src/<subsystem>/mod.rs` 的工厂函数中注册

示例：新 provider 注册位置在 `src/providers/mod.rs` 的 `create_provider()` 工厂函数。

## 代码风格

- **生产代码禁止 unwrap** — 使用 `?`、`anyhow::Result` 或 `thiserror`
- **最小化依赖** — 每个 crate 都会增加二进制大小
- **内联测试** — 在每个文件底部使用 `#[cfg(test)] mod tests {}`
- **默认安全** — 沙箱化一切，使用白名单（而非黑名单）
- **Clippy pedantic** — 强制执行 `#![warn(clippy::all, clippy::pedantic)]`

### 安全优先级

`src/security/` 目录下的更改需要最高级别的审查。注意检查：
- 硬编码的密钥
- 缺失的输入验证
- 没有充分理由的 unsafe 代码
- 路径遍历问题（文件工具中已有符号链接逃逸检测）

## 配置

配置文件：`~/.codecoder/config.toml`

主要配置部分：`[memory]`、`[gateway]`、`[autonomy]`、`[tunnel]`、`[identity]`、`[codecoder]`

目前仅支持 `runtime.kind = "native"`；不支持的类型会快速失败。

### CodeCoder 集成

ZeroBot 可以通过 `codecoder` Tool 调用 CodeCoder 的 23 个 AI Agent。

配置示例：
```toml
[codecoder]
enabled = true
endpoint = "http://localhost:4096"
```

使用方法：
1. 启动 CodeCoder API 服务器：`cd ../.. && bun dev serve`
2. 在 ZeroBot 中启用 codecoder tool
3. Agent 会自动通过 HTTP 调用 CodeCoder 的专业 Agent

可用 Agent：
- **主模式**: build, plan
- **工程质量**: code-reviewer, security-reviewer, tdd-guide, architect
- **祝融说系列**: observer, decision, macro, trader, picker, miniproduct, ai-engineer
- **内容创作**: writer, proofreader
- **工具辅助**: explore, general

## CLI 命令

```bash
zero-bot onboard              # 快速设置
zero-bot onboard --interactive # 完整向导
zero-bot agent -m "Hello"     # 单条消息
zero-bot agent                # 交互式聊天
zero-bot gateway              # 启动 webhook 服务器
zero-bot daemon               # 完整自主运行时
zero-bot status               # 系统状态
zero-bot doctor               # 诊断
```
