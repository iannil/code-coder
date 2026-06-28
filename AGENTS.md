# CodeCoder

基于 AI 大模型、高度自主的 Rust agent 系统。事件驱动架构，文件系统即自我，支持分层能力进化。

## Project

- **状态**: 设计完成，待编码（尚无 Cargo.toml）
- **栈**: Rust (planned)
- **入口**: `src/main.rs` (planned)
- **设计文档**: [[codecoder-design]] — 13 个架构决策已达成共识
- **参考项目**: `archived/claude-code/` (Bun/TS CLI agent), `archived/nocobase/` (Node.js 无代码平台)

## Commands

项目尚未初始化 Rust。开始编码后：

```bash
# 初始化 Rust 项目
cargo init

# 构建
cargo build

# 运行
cargo run

# 测试
cargo test
```

## Design Docs

- [[codecoder-design]] — Phase 1-6 核心架构（Event Bus / Agent Loop / LLM / Tools / Sandbox）
- [[phase-7-self-evolution]] — Phase 7: 自我进化型 Agent 设计（14 项决策树）

## Architecture

设计共识（详见 [[codecoder-design]]，当前实现见源码 `src/`）：

| 层 | 职责 |
|---|------|
| **Event Bus** | 核心骨架 — 统一事件路由（用户消息 / 定时器 / 文件事件） |
| **Agent Loop** | 事件驱动的感知-思考-行动循环 |
| **Self-Evolve** | 失败驱动的自省循环，自动生成 skill 补足能力缺口 |
| **Skill System** | 分层能力：markdown skill → Rust/WASM 插件 |
| **LLM Layer** | 分层 LLM：微 LLM（本地）处理常规事件，大 LLM（云端）处理复杂推理 |
| **Tools** | 21 个内置工具（含 MCP 扩展工具） |
| **Sandbox** | 分级：L0 纯数据 → L1 WASM → L2 Docker |
| **REPL** | 零配置交互式终端 + TUI |

## Conventions

- **先设计后编码** — 重大架构决策先走 /grill-me 充分讨论
- **纯 markdown 优先** — 能力扩展优先用 markdown skill，不够用时再写 Rust
- **文件系统即 API** — `skills/`、`memory/`、`tools/` 目录是系统的"自我"，不要硬编码能力列表
- **渐进式增强** — Phase 7 从失败驱动的自省 MVP 开始，后续逐步加入 Rust 工具生成和文件系统 watcher
- **分层失败检测** — LLM 自我声明 → 工具错误累积 → 用户反馈，逐级升级

## Phase Status

| Phase | 状态 | 说明 |
|-------|------|------|
| 1-6 | ✅ 完成 | Event Bus / Agent Loop / LLM / Tools / Sandbox / TUI |
| 7 | 📐 设计完成 | 自我进化型 Agent — 见 docs/phase-7-self-evolution.md |
| 8+ | 📋 待设计 | Rust 工具生成、文件系统 watcher、跨会话学习 |

## Notes

(预留)
