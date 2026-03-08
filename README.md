# CodeCoder

> AI Agent Observation System - Observe, trust, and evolve with AI agents through progressive autonomy from manual to automatic modes.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Bun](https://img.shields.io/badge/Bun-1.3+-black?logo=bun)](https://bun.sh)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.md) | [中文](README.zh-CN.md)

---

## Observation as Creation

> Every interaction with AI is an act of "observation" — collapsing infinite possibilities into concrete reality.

Based on **Zhurong Philosophy (祝融说)**, CodeCoder treats each AI interaction not as mere querying, but as a creative observation that shapes outcomes. You choose how to participate in this observation process: control every step yourself, or trust the Agent to complete it independently.

### Progressive Trust Journey

Trust isn't binary — it's a spectrum. CodeCoder lets you start cautiously and evolve naturally:

| Trust Level | Autonomy | Your Role | When to Use |
|-------------|----------|-----------|-------------|
| **Timid** | Minimal | Confirm every step | First-time use, sensitive tasks |
| **Bold** | Low | Approve key decisions | Learning phase |
| **Wild** | Medium | Set boundaries | Growing trust |
| **Crazy** | High | Exception handling only | Established trust |
| **Insane** | Very High | Critical alerts only | High confidence |
| **Lunatic** | Full | Set goal, walk away | Complete trust |

**This is a two-way observation:**

- **You observe the Agent** → understand its capabilities → build trust
- **Agent observes you** → learns your preferences → personalizes service

---

## Key Features

- **Observation Philosophy** - Based on Zhurong Theory: every interaction collapses possibilities into reality
- **Progressive Trust** - 6 autonomy levels from Timid (manual) to Lunatic (automatic)
- **Dual Observation** - You observe Agent → build trust; Agent observes you → learns preferences
- **31 Specialized Agents** - Three-mode design (@build, @writer, @decision) covering engineering, content, and decisions
- **30+ AI Providers** - Claude, OpenAI, Google, Ollama, Groq, Mistral, Azure, Bedrock, and more
- **Memory as Observation Record** - Daily notes (flow) + Long-term memory (sediment)
- **CLOSE Framework** - Sustainable decision-making over optimal decisions
- **MCP Protocol** - Full Model Context Protocol support with OAuth
- **30+ LSP Integrations** - TypeScript, Python, Go, Rust, Java, and more

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      OBSERVATION (祝融说)                               │
│   "Every interaction collapses possibilities into reality"              │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                      PROGRESSIVE TRUST                                  │
│                                                                         │
│   ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐             │
│   │  Timid  │───►│  Wild   │───►│  Crazy  │───►│ Lunatic │             │
│   │(Manual) │    │(Hybrid) │    │ (Auto)  │    │ (Full)  │             │
│   └─────────┘    └─────────┘    └─────────┘    └─────────┘             │
│        │              │              │              │                   │
│   Confirm         Approve        Exception      Set goal                │
│   every step    key decisions    handling only  walk away               │
│                                                                         │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                           EXECUTION                                     │
│   31 Agents  │  30+ AI Providers  │  Memory  │  CLOSE Framework         │
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

# Configure AI provider
export ANTHROPIC_API_KEY="your-api-key"
```

### Start Your Trust Journey

```bash
# Start in Manual Mode - observe and learn
bun dev

# Set autonomy level (as trust grows)
bun dev --autonomy wild    # Medium trust
bun dev --autonomy crazy   # High trust

# Full automatic mode (when trust is established)
bun dev run --agent autonomous "Build a REST API"
```

---

## Trust Journey: From Manual to Automatic

### Starting: Manual Mode

Begin here. Every operation can be confirmed or modified before execution.

```bash
bun dev
```

**What you experience:**

- See every step the Agent plans to take
- Approve, modify, or reject each action
- Observe how the Agent thinks and decides
- Build understanding of its capabilities and limits

**Best for:** First-time use, sensitive tasks, learning exploration

### Growing: Hybrid Mode

As trust builds, let low-risk operations run automatically while keeping control over high-impact decisions.

```bash
bun dev --autonomy wild
```

**What you experience:**

- File reads and searches run automatically
- File writes and system commands pause for approval
- System learns your risk thresholds
- Interruptions decrease as patterns establish

**Best for:** Regular development work, growing familiarity

### Established: Automatic Mode

Set your goal and let the Agent work independently. You're notified only when something unexpected occurs.

```bash
bun dev run --agent autonomous "Implement user authentication with JWT"
```

**What you experience:**

- Define the objective, not the steps
- Agent handles planning and execution
- Real-time progress updates
- Intervention only on critical decisions or errors

**Best for:** Well-defined tasks, trusted environments, maximum productivity

---

## Philosophy: Zhurong (祝融说)

CodeCoder incorporates a unique decision-making philosophy based on "Zhurong Theory".

### Observation Collapses Reality

The universe is an infinite field of possibilities. "Observation" is a creative act that collapses these possibilities into concrete reality. Every conversation with AI is a shared observation — you and the Agent together "vote" on what becomes real.

This explains the need for both automatic and manual modes:

- **Manual Mode** = You participate in every observation collapse
- **Automatic Mode** = You trust the Agent to observe and collapse independently

### Dual Observation

Trust is built through observation, and it flows both ways:

| Direction | Process | Outcome |
|-----------|---------|---------|
| You → Agent | Observe behavior, verify reasoning | Understand capabilities, build trust |
| Agent → You | Learn preferences, remember patterns | Personalized service, reduced friction |

### CLOSE Decision Framework

Every decision is evaluated across five dimensions:

| Dimension | Meaning | Question |
|-----------|---------|----------|
| **C**apacity | Current capability | Can we do this? |
| **L**everage | Amplification potential | What's the multiplier? |
| **O**pportunity | Timing and context | Is now the right time? |
| **S**ustainability | Long-term viability | Can we keep doing this? |
| **E**xit | Reversibility | Can we undo if needed? |

### Available Margin

**Core Insight:** Sustainable decisions > Optimal decisions.

Preserving unexploited potential (available margin) is more important than achieving the "best" outcome. The ability to "try again" matters more than the "perfect" solution.

---

## Agent System

CodeCoder uses a 3-mode agent system for simplified interaction:

| Mode | Primary Agent | Capabilities |
|------|---------------|--------------|
| **@build** (default) | build | code-reviewer, security-reviewer, tdd-guide, architect, explore, general, code-reverse, jar-code-reverse, verifier, prd-generator, feasibility-assess |
| **@writer** | writer | expander, expander-fiction, expander-nonfiction, proofreader, verifier |
| **@decision** | decision | macro, trader, value-analyst, picker, miniproduct, ai-engineer, synton-assistant |

### Usage

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
  <i>Every observation collapses possibilities into reality.<br/>Choose how you participate: step by step, or let it flow.</i>
</p>
