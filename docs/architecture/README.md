# CodeCoder 架构文档

> 生成时间: 2026-02-25
> 版本: 0.0.1

本目录包含 CodeCoder 项目的完整架构文档。

## 文档索引

| 文档 | 说明 |
|------|------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 系统架构总览 |
| [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md) | 设计思想与哲学框架 |
| [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md) | packages/ccode 与 services/zero-* 关系 |
| [diagrams/](./diagrams/) | ASCII 架构图 |

## 快速导航

### 核心概念

- **祝融说哲学** - 见 [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md#一核心设计哲学)
- **CLOSE 决策框架** - 见 [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md#12-close-决策框架)
- **双语言策略** - 见 [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md#21-双语言策略)
- **确定性 vs 不确定性划分** - 见 [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md#六确定性-vs-不确定性深度解析) / [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md#六底层原则确定性-vs-不确定性)

### 技术架构

- **系统总览** - 见 [ARCHITECTURE.md](./ARCHITECTURE.md#2-系统架构总览)
- **Agent 系统** - 见 [ARCHITECTURE.md](./ARCHITECTURE.md#4-agent-系统架构)
- **Rust 微服务** - 见 [ARCHITECTURE.md](./ARCHITECTURE.md#5-rust-微服务架构)
- **Trading 模块** - 见 [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md#四zero-trading-模块设计思想)

### 关系与通信

- **ccode 与 zero-* 关系** - 见 [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md)
- **通信方式** - 见 [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md#一通信方式)
- **部署模式** - 见 [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md#五部署模式)

## 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        CodeCoder 系统                           │
├─────────────────────────────────────────────────────────────────┤
│  客户端层     │  TUI (Solid.js) │ Web (React) │ CLI │ IM Bots  │
├─────────────────────────────────────────────────────────────────┤
│  核心服务层   │  ccode API :4400 │ Agent 引擎 │ MCP :4420     │
├─────────────────────────────────────────────────────────────────┤
│  AI Provider  │  Claude │ GPT │ Gemini │ Ollama │ 15+ 提供商   │
├─────────────────────────────────────────────────────────────────┤
│  Rust 微服务  │  Gateway │ Channels │ Workflow │ Trading      │
├─────────────────────────────────────────────────────────────────┤
│  基础设施     │  Redis │ SQLite │ Docker │ Whisper            │
└─────────────────────────────────────────────────────────────────┘
```

## 端口分配

| 端口 | 服务 | 技术栈 |
|------|------|--------|
| 4400 | CodeCoder API | TypeScript/Bun |
| 4401 | Web Frontend | React/Vite |
| 4402 | Zero Daemon | Rust |
| 4403 | Whisper STT | Docker |
| 4420 | MCP Server | TypeScript |
| 4430 | Zero Gateway | Rust |
| 4431 | Zero Channels | Rust |
| 4432 | Zero Workflow | Rust |
| 4434 | Zero Trading | Rust |
| 6379 | Redis | Docker |
