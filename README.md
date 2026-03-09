# CodeCoder

> An observation-centric AI agent system — observe first, control always.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Bun](https://img.shields.io/badge/Bun-1.3+-black?logo=bun)](https://bun.sh/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)

[中文文档](./README_ZH.md)

## Overview

CodeCoder is an **observation-centric AI agent system** built on "祝融说" (Zhu Rong Philosophy). Unlike traditional AI assistants that wait for commands, CodeCoder:

1. **Observes continuously** — Four watchers monitor code, world, self, and meta-layer
2. **Forms consensus** — Consensus engine determines what's happening and what matters
3. **Responds controllably** — Gear System (P/N/D/S/M) lets you dial in exactly how autonomous the response should be

The core insight: **Observation before action, control over automation.**

## Gear System: Your Autonomy Dial

Like a car's transmission, CodeCoder lets you control how autonomous the AI should be:

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Gear Selector: P  N  D  S  M                   │
│                                                                     │
│              ┌───────────┬───────────┬───────────┐                  │
│              │  Observe  │  Decide   │    Act    │   Three Dials    │
│              │   0-100   │   0-100   │   0-100   │   (Manual mode)  │
│              └───────────┴───────────┴───────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Five Gears

| Gear | Mode | Observe | Decide | Act | Use Case |
|------|------|---------|--------|-----|----------|
| **P** | Park | 0% | 0% | 0% | System inactive, no resource consumption |
| **N** | Neutral | 50% | 0% | 0% | Pure observation, no intervention |
| **D** | Drive | 70% | 60% | 40% | Balanced daily operation (default) |
| **S** | Sport | 90% | 80% | 70% | Full autonomous mode |
| **M** | Manual | Custom | Custom | Custom | Fine-grained control via three dials |

### Three Dials

When in Manual (M) mode, you have independent control over three dimensions:

- **Observe (0-100%)**: How actively the system scans for changes
  - 0% = passive wait, 100% = aggressive active scanning
- **Decide (0-100%)**: How autonomously the system makes decisions
  - 0% = suggest only, 100% = decide without asking
- **Act (0-100%)**: How autonomously the system executes actions
  - 0% = wait for confirmation, 100% = immediate execution

### Quick Start with Gears

```bash
# Start in Drive mode (default - balanced autonomy)
bun dev

# Start in Sport mode (high autonomy)
bun dev --gear S

# Start in Neutral mode (observe only)
bun dev --gear N

# Manual mode with custom dial values
bun dev --gear M --observe 80 --decide 30 --act 10
```

## Features

**Observation Layer:**
- **Observer Network** — Four watchers (Code, World, Self, Meta) with consensus engine
- **Continuous monitoring** — Always-on observation with configurable depth

**Control Layer:**
- **Gear System** — P/N/D/S/M transmission-like autonomy control
- **Three dials** — Fine-grained control over Observe/Decide/Act dimensions

**Response Layer:**
- **31 AI Agents** — Specialized responders organized in 3 modes (build, writer, decision)
- **Multi-provider support** — Claude, GPT, Gemini, Ollama, and 20+ providers

**Infrastructure:**
- **Dual-language architecture** — TypeScript for intelligence, Rust for security boundaries
- **Markdown-based memory** — Transparent, Git-friendly knowledge system
- **Multi-platform IM** — Telegram, Discord, Slack, Feishu, Email

## Quick Start

### Prerequisites

- [Bun](https://bun.sh/) 1.3+ (required)
- [Rust](https://www.rust-lang.org/) 1.75+ (optional, for zero-* services)

### Installation

```bash
# Clone the repository
git clone https://github.com/iannil/code-coder.git
cd code-coder

# Install dependencies
bun install

# Run CodeCoder TUI
bun dev

# Or run in a specific directory
bun dev /path/to/your/project
```

### Configuration

Create `~/.codecoder/config.json`:

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "your-api-key"
    }
  }
}
```

For sensitive credentials, use `~/.codecoder/secrets.json` (chmod 600).

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Gear Control Layer                              │
│              ┌─────────────────────────────────────────┐                │
│              │     Gear Selector: P  N  D  S  M        │                │
│              ├───────────┬───────────┬───────────┬─────┤                │
│              │  Observe  │  Decide   │    Act    │Gear │                │
│              │   0-100   │   0-100   │   0-100   │ ↑↓  │                │
│              └───────────┴───────────┴───────────┴─────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Observer Network                               │
│   ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐              │
│   │ CodeWatch │ │WorldWatch │ │ SelfWatch │ │ MetaWatch │              │
│   │  (Code)   │ │ (Market)  │ │ (Agent)   │ │ (System)  │              │
│   └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘              │
│         └─────────────┴─────────────┴─────────────┘                     │
│                              │                                          │
│                    ┌─────────▼─────────┐                                │
│                    │  Consensus Engine │                                │
│                    └───────────────────┘                                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           User Access Layer                             │
│   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐  │
│   │   TUI   │   │   Web   │   │   CLI   │   │Telegram │   │ Discord │  │
│   │  :4400  │   │  :4401  │   │         │   │   Bot   │   │   Bot   │  │
│   └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘  │
└────────┼─────────────┼─────────────┼─────────────┼─────────────┼────────┘
         │             │             │             │             │
         └─────────────┴──────┬──────┴─────────────┴─────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Core Services (TypeScript/Bun)                       │
│   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                │
│   │  API Server  │   │ Agent Engine │   │ Memory System│                │
│   │    :4400     │◄─►│  (31 Agents) │◄─►│  (Markdown)  │                │
│   └──────────────┘   └──────────────┘   └──────────────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Rust Services (5 Crates)                           │
│              ┌──────────────────────────────────────┐                   │
│              │       Zero CLI Daemon :4402          │                   │
│              │    (Unified Entry + Orchestration)   │                   │
│              └─────────────────┬────────────────────┘                   │
│   ┌────────────────────────────┼────────────────────────────┐           │
│   │                            │                            │           │
│   ▼                            ▼                            ▼           │
│ ┌──────────┐            ┌──────────────┐            ┌──────────┐        │
│ │zero-core │            │  zero-hub    │            │zero-     │        │
│ │ (Tools)  │            │(Gateway+IM+  │            │trading   │        │
│ │          │            │  Workflow)   │            │          │        │
│ └──────────┘            └──────────────┘            └──────────┘        │
└─────────────────────────────────────────────────────────────────────────┘
```

### Core Principle

> **"TypeScript for intelligence, Rust for security boundaries"**

| Task Type | Best Tool | Reason |
|-----------|-----------|--------|
| Protocol parsing, signature verification, scheduling | Rust (zero-*) | Rules are clear, needs high performance and security |
| Intent understanding, code generation, decision advice | TypeScript (LLM) | Requires reasoning and domain knowledge |

## Observer Network

The Observer Network is the core of CodeCoder's observation-centric design. It transforms the system from an execution-centric tool to an observation-centric one.

### Four Watchers

| Watcher | Observes | Related Agents |
|---------|----------|----------------|
| **CodeWatch** | Codebase changes, Git activity, build status | explore |
| **WorldWatch** | Market data, news, API changes | macro, trader |
| **SelfWatch** | Agent behavior, decision logs, error patterns | code-reviewer, security-reviewer, decision |
| **MetaWatch** | Observation quality, system health, blind spots | observer |

### Observation → Consensus → Response

```
Events → Buffer → Aggregate → Consensus → Mode Controller → Response
                                              │
                              ┌───────────────┼───────────────┐
                              ▼               ▼               ▼
                          Notifier       Analyzer        Executor
```

The system continuously observes, builds consensus about what's happening, and responds according to your gear setting.

## Agent System

CodeCoder features 31 specialized AI agents organized in a 3-mode system:

### Modes

| Mode | Primary Agent | Use Case |
|------|--------------|----------|
| **@build** (default) | `build` | Software development |
| **@writer** | `writer` | Long-form content creation |
| **@decision** | `decision` | Decision-making with Zhu Rong philosophy |

### Usage

```bash
# Default build mode
bun dev

# Writer mode for content creation
bun dev -m writer

# Decision mode for philosophical analysis
bun dev -m decision

# Access specific capabilities
bun dev @build:security-review
bun dev @decision:macro
```

### Agent Categories

| Category | Agents | Purpose |
|----------|--------|---------|
| **Main Modes (4)** | build, plan, writer, autonomous | Primary interaction modes |
| **Reverse Engineering (2)** | code-reverse, jar-code-reverse | Code analysis |
| **Engineering Quality (6)** | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | Code quality assurance |
| **Content Creation (5)** | expander, expander-fiction, expander-nonfiction, proofreader, verifier | Writing assistance |
| **Zhu Rong Series (8)** | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | Decision & domain expertise |
| **Product (2)** | prd-generator, feasibility-assess | Product requirements |
| **System (3)** | compaction, title, summary | Internal use (hidden) |

## Development

### Commands

```bash
# Install dependencies
bun install

# Run TUI in current directory
bun dev

# Start headless API server
bun dev serve --port 4400

# Type checking
bun turbo typecheck

# Run tests (from specific package)
cd packages/ccode && bun test

# Build standalone executable
cd packages/ccode && bun run build

# Build Rust services
./ops.sh build rust

# Service management
./ops.sh start all      # Start all services
./ops.sh stop           # Stop services
./ops.sh status         # Check status
./ops.sh health         # Health check
```

### Port Configuration

| Service | Port | Description |
|---------|------|-------------|
| CodeCoder API | 4400 | Main API server |
| Web Frontend | 4401 | React web UI |
| Zero Daemon | 4402 | Rust unified entry |
| Whisper | 4403 | Voice transcription |
| MCP Server | 4420 | Model Context Protocol |

### Monorepo Structure

```
codecoder/
├── packages/                    # TypeScript packages
│   ├── ccode/                   # Core CLI (entry point)
│   │   ├── src/agent/           # 31 Agent definitions
│   │   ├── src/cli/cmd/tui/     # Terminal UI (Solid.js)
│   │   └── src/observer/        # Observer Network + Dials
│   ├── memory/                  # Memory module
│   ├── util/                    # Shared utilities
│   └── web/                     # Web frontend (React)
│
├── services/                    # Rust services (5 crates)
│   ├── zero-cli/                # CLI + Daemon
│   ├── zero-core/               # Core tools (grep/glob/edit, NAPI)
│   ├── zero-hub/                # Gateway + Channels + Workflow
│   ├── zero-trading/            # Trading system
│   └── zero-common/             # Shared config, logging, events
│
├── memory/                      # Project memory
│   ├── daily/                   # Daily notes (stream)
│   └── MEMORY.md                # Long-term memory (sediment)
│
└── docs/                        # Documentation
```

## Design Philosophy

CodeCoder is built on "祝融说" (Zhu Rong Philosophy):

> **"Sustainable decisions are more important than optimal decisions"**

This manifests in the gear system design:
- **Park (P)**: Preserve resources when not needed
- **Neutral (N)**: Observe before acting — understand before intervening
- **Drive (D)**: Balance autonomy with human oversight
- **Sport (S)**: Trust the system for routine tasks
- **Manual (M)**: Always provide fine-grained human control

The CLOSE framework evaluates decisions:
- **C**apacity: Can I do this?
- **L**everage: What's the ROI?
- **O**pportunity: What am I giving up?
- **S**ustainability: Can I keep doing this? ← **Veto power**
- **E**xit: How do I retreat if it fails?

For more details, see [Design Philosophy](./docs/architecture/DESIGN_PHILOSOPHY.md).

## Documentation

- [Architecture Overview](./docs/architecture/ARCHITECTURE.md)
- [Design Philosophy](./docs/architecture/DESIGN_PHILOSOPHY.md)
- [Beginner's Guide](./docs/guides/beginners-guide.md)
- [Project Instructions](./CLAUDE.md)

## Contributing

We welcome contributions! Please:

1. Open an issue first to discuss major changes
2. Follow conventional commit format (`feat:`, `fix:`, `docs:`, etc.)
3. Keep PRs small and focused
4. Include tests for new features

## License

[MIT](./LICENSE)
