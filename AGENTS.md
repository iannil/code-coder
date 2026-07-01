# CodeCoder

基于 AI 大模型、高度自主的 Rust agent 系统。事件驱动架构，文件系统即自我，支持分层能力进化。

## Project

- **状态**: Phase 1-6 编码完成并测试通过（696 tests, 0 failed）
- **栈**: Rust (edition 2024, 20,450 行源码)
- **入口**: `src/main.rs`
- **架构**: 7 ADRs 文档化于 `docs/adr/`
- **参考项目**: `archived/claude-code/` (Bun/TS CLI agent), `archived/nocobase/` (Node.js 无代码平台)

## 快速统计

| 指标 | 值 |
|------|-----|
| 源码行数 | 20,450 (`.rs` files) |
| 测试数 | 696 (`cargo test` all pass) |
| 编译警告 | 37 (dead code / unused imports) |
| 模块数 | 20+ 个 Rust 模块 |
| 内置工具 | 20 个 |
| TUI 组件 | 11 个 |
| ADR | 7 个 |
| Session 存储 | 238 个会话 JSON |
| 编译 | `cargo build` / `cargo check` 通过 |

## Commands

```bash
cargo build              # 编译（dev profile）
cargo check              # 快速检查编译错误
cargo run                # 启动 REPL / TUI
cargo test               # 运行 696 个测试
cargo test -- --nocapture # 显示测试输出
cargo fix                # 应用编译器建议
```

## 模块架构

| 层 | 模块 | 行数 | 职责 |
|---|------|------|------|
| **入口** | `src/main.rs` | 351 | CLI 解析、日志初始化、REPL/TUI 启动 |
| **Agent 循环** | `src/agent.rs` | 1,206 | 感知-思考-行动循环，集成 self_evolve |
| **自主模式** | `src/autonomous.rs` | 353 | 定时器/调度器、自主运行 |
| **事件总线** | `src/event.rs` | 416 | `EventBus` trait + 事件类型定义 |
| **LLM 客户端** | `src/llm/mod.rs` | 665 | OpenAI 兼容 LLM 客户端（含 StubClient） |
| **工具系统** | `src/tools/` (20 文件) | 5,500+ | 20 个内置工具 |
| **技能系统** | `src/skill/mod.rs` | 553 | Skill 注册、扫描、frontmatter 解析 |
| **自进化** | `src/self_evolve.rs` | 736 | Phase 7: 失败检测 → gap 分析 → skill 生成 |
| **会话** | `src/session.rs` | 658 | 会话持久化 + schema 迁移 |
| **内存** | `src/memory/mod.rs` | 157 | 文件型 key-value 存储 |
| **配置** | `src/config.rs` | 447 | codecoder.json 配置管理 |
| **MCP** | `src/mcp/` (3 文件) | 843 | MCP 协议扩展支持 |
| **沙箱** | `src/sandbox/` (3 文件) | 447 | 分级沙箱：L0 数据 / L1 WASM / L2 Docker |
| **权限** | `src/permission/` (3 文件) | 218 | 路径验证 + shell 分类 |
| **TUI** | `src/tui/` (11 文件) | 7,735 | 全屏终端界面（ratatui） |
| **REPL** | `src/repl.rs` | 286 | 非 TUI 命令行交互 |

## Design Docs

- `docs/adr/0001-0007` — 7 个架构决策记录
- `docs/design/phase-7-self-evolution.md` — Phase 7: 自我进化型 Agent 设计
- `docs/audit/` — TUI 保真度审计文档（已完成）
- `docs/superpowers/` — SDD 实施计划与设计规格

## Architecture

（详见各 ADR 和源码）

| 层 | 职责 | 实现状态 |
|---|------|---------|
| **Event Bus** | 统一事件路由（用户消息 / 定时器 / 文件事件） | ✅ 已实现 |
| **Agent Loop** | 事件驱动的感知-思考-行动循环 | ✅ 已实现 |
| **Self-Evolve** | 失败驱动的自省循环，自动生成 skill | ✅ 代码实现，待完全集成 |
| **Skill System** | 分层能力：markdown skill → Rust/WASM 插件 | ✅ 已实现 |
| **LLM Layer** | OpenAI 兼容接口（含 StubClient 测试模式） | ✅ 已实现 |
| **Tools** | 20 个内置工具 + MCP 扩展 | ✅ 已实现 |
| **Sandbox** | 分级：L0 纯数据 → L1 WASM → L2 Docker | ✅ 已实现 |
| **TUI/REPL** | 全屏终端界面 + 命令行 REPL | ✅ 已实现 |

## Conventions

- **先设计后编码** — 重大架构决策先走 ADR 流程
- **纯 markdown 优先** — 能力扩展优先用 markdown skill，不够用时再写 Rust
- **文件系统即 API** — `skills/`、`memory/`、`tools/` 目录是系统的"自我"，不要硬编码能力列表
- **渐进式增强** — Phase 7 从失败驱动的自省 MVP 开始
- **分层失败检测** — LLM 自我声明 → 工具错误累积 → 用户反馈，逐级升级

## Phase Status

| Phase | 状态 | 说明 |
|-------|------|------|
| 1 | ✅ 完成 | Event Bus + Agent Loop |
| 2 | ✅ 完成 | LLM 客户端（OpenAI 兼容） |
| 3 | ✅ 完成 | 20 个内置工具 + MCP |
| 4 | ✅ 完成 | 分级沙箱（WASM / Docker） |
| 5 | ✅ 完成 | TUI 终端界面（ratatui） |
| 6 | ✅ 完成 | REPL + 会话持久化 |
| 7 | 🔧 代码实现 | 自我进化（self_evolve.rs），待完全集成 |
| 8+ | 📋 待设计 | Rust 工具生成、文件系统 watcher、跨会话学习 |

## Known Issues

- 37 个编译警告（dead code / unused imports / 未读字段）
- `.superpowers/sdd/` 有 58 个临时文件（review diff / task reports）需清理
- `skills/greeter.md` 是唯一的示例 skill，缺乏实用 skill
- `src/self_evolve.rs` 已实现但尚未在 agent 循环中完全集成
- 238 个 session JSON 文件，其中许多可能是空的测试产物

## Memory & Knowledge

- `memory/` — 系统持久化的 key-value 记忆
- `sessions/` — 对话会话 JSON 文件
- `skills/` — markdown skill 文件（自动注册）
