# CodeCoder

> Personal Brain Trust System - AI-powered development tool that combines engineering capabilities with decision wisdom.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Bun](https://img.shields.io/badge/Bun-1.3+-black?logo=bun)](https://bun.sh)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.md) | [中文](README.zh-CN.md)

---

## What is CodeCoder?

CodeCoder is an AI advisor platform that merges **engineering capabilities** with **decision wisdom**. It's not just a coding assistant - it's a comprehensive system designed to enhance both technical execution and strategic thinking.

### Three-Layer Wisdom Architecture

| Layer | Capabilities |
|-------|--------------|
| **Engineering** | Code review, Security analysis, TDD, Architecture design, Reverse engineering |
| **Domain** | Macro-economics, Trading analysis, Product selection, Minimum viable products, AI engineering |
| **Thinking** | Zhurong Philosophy (祝融说), CLOSE decision framework, Observer theory |

---

## Key Features

- **31 Specialized Agents** - Three-mode design (@build, @writer, @decision) covering engineering, content creation, and decision consulting
- **30+ AI Providers** - Claude, OpenAI, Google, Ollama, Groq, Mistral, Azure, Bedrock, and more
- **MCP Protocol** - Full Model Context Protocol support for local and remote servers with OAuth
- **30+ LSP Integrations** - TypeScript, Python, Go, Rust, Java, and many more language servers
- **Multi-mode Interface** - TUI (terminal), CLI, Web, and Headless API
- **Dual-layer Memory** - Transparent Markdown-based memory system (daily notes + long-term memory)
- **Secure Sandbox** - Process, Docker, and WASM execution backends
- **Hands Autonomous Agent** - 6-level autonomy system with CLOSE framework integration

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           User Interface                                │
│     TUI (:4400)    │    Web (:4401)    │    CLI    │  Telegram/Discord │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                    packages/ccode (TypeScript/Bun)                      │
│      Agent Engine (31 Agents)  │  AI Providers  │  Memory System        │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                      services/zero-* (Rust)                             │
│   zero-cli  │  zero-core  │  zero-hub  │  zero-trading  │  zero-common  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Core Principle:** TypeScript handles intelligence (agents, reasoning); Rust handles security boundaries (tools, sandboxing, protocols).

---

## Quick Start

### Prerequisites

- [Bun](https://bun.sh) 1.3+ (required)
- [Rust](https://rustup.rs) 1.75+ (optional, for Rust services)

### Installation

```bash
# Clone the repository
git clone https://github.com/iannil/code-coder.git
cd code-coder

# Install dependencies
bun install

# Start TUI (interactive terminal interface)
bun dev

# Or start headless API server
bun dev serve --port 4400
```

### First Run

On first run, configure at least one AI provider:

```bash
# Using environment variable
export ANTHROPIC_API_KEY="your-api-key"

# Or configure via config file (~/.codecoder/config.json)
```

---

## Agent System

CodeCoder uses a 3-mode agent system for simplified user interaction:

| Mode | Primary Agent | Capabilities |
|------|---------------|--------------|
| **@build** (default) | build | code-reviewer, security-reviewer, tdd-guide, architect, explore, general, code-reverse, jar-code-reverse, verifier, prd-generator, feasibility-assess |
| **@writer** | writer | expander, expander-fiction, expander-nonfiction, proofreader, verifier |
| **@decision** | decision | macro, trader, value-analyst, picker, miniproduct, ai-engineer, synton-assistant |

### Usage Examples

```bash
# Default @build mode
bun dev

# Writer mode for content creation
bun dev -m writer

# Decision mode for strategic analysis
bun dev -m decision

# Access specific capabilities
bun dev @build:security-reviewer
bun dev @decision:macro
```

### Agent Categories (31 Total)

| Category | Agents | Purpose |
|----------|--------|---------|
| **Primary (4)** | build, plan, writer, autonomous | Main development/creation modes |
| **Reverse Engineering (2)** | code-reverse, jar-code-reverse | Code analysis and reconstruction |
| **Engineering Quality (6)** | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | Code quality assurance |
| **Content Creation (5)** | expander, expander-fiction, expander-nonfiction, proofreader, verifier | Long-form writing |
| **Zhurong Series (8)** | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | Decision and domain consulting |
| **Product (2)** | prd-generator, feasibility-assess | Requirements and feasibility |
| **Other (1)** | synton-assistant | SYNTON-DB assistant |
| **System Hidden (3)** | compaction, title, summary | Internal use |

---

## Philosophy: Zhurong (祝融说)

CodeCoder incorporates a unique decision-making philosophy based on "Zhurong Theory":

- **Possibility Substrate** - The universe as an infinite field of potential
- **Observation Collapses** - Observation as a creative act that "collapses" possibilities into certainty
- **Available Margin** - The unexploited potential space, source of free will and creativity
- **CLOSE Framework** - Five-dimensional evaluation for sustainable decision-making:
  - **C**apacity - Current capability and resources
  - **L**everage - Amplification potential
  - **O**pportunity - Timing and context
  - **S**ustainability - Long-term viability
  - **E**xit - Reversibility and options

**Core Insight:** Sustainable decisions > Optimal decisions. Maintaining the ability to "try again" is more important than pursuing the "best" solution.

---

## Configuration

Configuration files are located in `~/.codecoder/`:

```
~/.codecoder/
├── config.json       # Core configuration
├── secrets.json      # Credentials (gitignored, 600 permissions)
├── providers.json    # LLM provider configuration
├── trading.json      # Trading module configuration
└── channels.json     # IM channel configuration
```

### Environment Variables

```bash
ANTHROPIC_API_KEY     # Anthropic Claude API key
OPENAI_API_KEY        # OpenAI API key
CCODE_CONFIG          # Custom config file path
CCODE_CONFIG_CONTENT  # Inline configuration (JSON)
```

---

## Development Commands

```bash
# Install dependencies
bun install

# Run TUI (development)
bun dev

# Run headless server
bun dev serve --port 4400

# Type checking (all packages)
bun turbo typecheck

# Run tests (from specific package)
cd packages/ccode && bun test

# Build standalone executable
bun run --cwd packages/ccode build

# Regenerate SDK after API changes
./script/generate.ts
```

### Rust Services (Optional)

```bash
# Build Rust services
./ops.sh build rust

# Start all services
./ops.sh start all

# Check status
./ops.sh status

# View logs
./ops.sh logs zero-daemon
```

---

## Project Structure

```
codecoder/
├── packages/
│   ├── ccode/              # Core CLI & Agent engine (TypeScript)
│   │   ├── src/
│   │   │   ├── agent/      # 31 agent definitions
│   │   │   ├── tool/       # Tool implementations
│   │   │   ├── provider/   # AI provider integrations
│   │   │   ├── mcp/        # MCP protocol
│   │   │   └── cli/cmd/tui/# Terminal UI (Solid.js)
│   │   └── test/
│   ├── web/                # Web frontend (React)
│   └── util/               # Shared utilities
├── services/               # Rust services (5 crates)
│   ├── zero-cli/           # CLI + Daemon (entry point)
│   ├── zero-core/          # Core tools, NAPI bindings
│   ├── zero-hub/           # Service hub (gateway/channels/workflow)
│   ├── zero-trading/       # Trading system
│   └── zero-common/        # Shared library
├── memory/                 # Dual-layer memory system
│   ├── daily/              # Daily notes (append-only)
│   └── MEMORY.md           # Long-term memory (curated)
└── docs/                   # Documentation
```

---

## Documentation

- **Architecture:** [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md)
- **Features:** [`docs/FEATURES.md`](docs/FEATURES.md)
- **Design Philosophy:** [`docs/architecture/DESIGN_PHILOSOPHY.md`](docs/architecture/DESIGN_PHILOSOPHY.md)
- **Project Overview:** [`docs/PROJECT-OVERVIEW.md`](docs/PROJECT-OVERVIEW.md)

---

## Contributing

We welcome contributions! Please follow these guidelines:

1. **Fork** the repository and create a feature branch
2. **Reference** an existing issue or create one first
3. **Follow** conventional commits (`feat:`, `fix:`, `docs:`, etc.)
4. **Include** tests for new functionality
5. **Keep** PRs small and focused

---

## License

MIT License

Copyright (c) 2024-2026 CodeCoder Contributors

See [LICENSE](LICENSE) for details.

---

<p align="center">
  <i>Built with Observer Theory in mind: every interaction collapses possibilities into reality.</i>
</p>
