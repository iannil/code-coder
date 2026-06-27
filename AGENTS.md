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

## Architecture

设计共识（详见 [[codecoder-design]]）：

| 层 | 职责 |
|---|------|
| **Event Bus** | 核心骨架 — 统一事件路由（用户消息 / 定时器 / 文件事件） |
| **Agent Loop** | 事件驱动的感知-思考-行动循环 |
| **Skill System** | 分层能力：markdown skill → Rust/WASM 插件 |
| **LLM Layer** | 分层 LLM：微 LLM（本地）处理常规事件，大 LLM（云端）处理复杂推理 |
| **Tools** | 内置六件套：read_file, write_file, run_command, llm_call, search_web, list_directory |
| **Sandbox** | 分级：L0 纯数据 → L1 WASM → L2 Docker → L3 VM |
| **REPL** | 零配置交互式终端 |

源码目录计划：
```
src/
├── main.rs       # 入口：启动事件循环
├── event.rs      # Event enum + EventBus trait
├── agent.rs      # AgentLoop 骨架
├── skill/        # Skill trait + 发现/加载
├── llm/          # 分层 LLM 封装
├── tools/        # 内置工具
└── repl.rs       # 交互式终端
```

## Conventions

- **先设计后编码** — 重大架构决策先走 /grill-me 充分讨论
- **纯 markdown 优先** — 能力扩展优先用 markdown skill，不够用时再写 Rust
- **文件系统即 API** — `skills/`、`memory/`、`tools/` 目录是系统的"自我"，不要硬编码能力列表
- **渐进式增强** — MVP 先跑通 Event Bus + Agent Loop，不要一开始做全功能

## Notes

(预留)
