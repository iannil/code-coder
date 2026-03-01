# CodeCoder

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue.svg)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Bun](https://img.shields.io/badge/Bun-1.3+-pink.svg)](https://bun.sh/)

[中文文档](README.zh-CN.md)

---

## Autonomous Evolution: The Core Differentiator

**CodeCoder** is the first AI workstation with **self-building and autonomous evolution capabilities**.

Unlike traditional AI coding assistants that only execute predefined tasks, CodeCoder can:

- **Detect capability gaps** — Identify when it lacks the right tools or knowledge
- **Build new concepts autonomously** — Create Agent, Prompt, Skill, Tool, Hand, Memory, or Workflow
- **Learn from experience** — Extract reusable patterns from successful solutions
- **Make sustainable decisions** — Use the CLOSE framework to evaluate options

### How It Works: The 5-Step Evolution Loop

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        AUTONOMOUS EVOLUTION LOOP                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Proactive Resource Retrieval (主动资源检索)                              │
│     └─ Search documentation and web when confidence is low                  │
│                                                                             │
│  2. Tool Discovery (工具发现)                                               │
│     └─ Check for existing reusable tools before generating code             │
│                                                                             │
│  3. Dynamic Code Generation (动态编程保底)                                  │
│     └─ Write and execute temporary scripts as fallback                      │
│                                                                             │
│  4. Self-Reflection & Retry (自主反思与重试)                                │
│     └─ Analyze errors like a human programmer and correct                   │
│                                                                             │
│  5. Knowledge Sedimentation (沉淀与进化)                                    │
│     └─ Store successful solutions as reusable tools/skills                  │
│                                                                             │
│              ┌─────────────────────────────────────────────────┐            │
│              │         FAILURE TRIGGERS AUTO-BUILDER           │            │
│              │  ┌──────────────────────────────────────────┐  │            │
│              │  │ Detect Gap → CLOSE Eval → Build Concept │  │            │
│              │  └──────────────────────────────────────────┘  │            │
│              └─────────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### CLOSE Decision Framework

Every autonomous decision is evaluated using the **CLOSE framework**:

| Dimension | Range | Description |
|-----------|-------|-------------|
| **C**onvergence | 0-10 | How much this limits future options (lower = more reversible) |
| **L**everage | 0-10 | Value gained relative to risk taken |
| **O**ptionality | 0-10 | Ability to undo or pivot later |
| **S**urplus | 0-10 | Resource margin preserved |
| **E**volution | 0-10 | Learning value for future decisions |

**Key insight**: Sustainable decisions > Optimal decisions. Maintaining "available surplus" (可用余量) — the ability to "play again" — matters more than one-time optimization.

### Autonomy Levels

| Level | Score | Description |
|-------|-------|-------------|
| **Lunatic** | 90+ | Fully autonomous, operates independently |
| **Insane** | 75-89 | Highly autonomous, minimal intervention |
| **Crazy** | 60-74 | Significantly autonomous, periodic check-ins |
| **Wild** | 40-59 | Partially autonomous, needs guidance |
| **Bold** | 20-39 | Cautiously autonomous, frequent pauses |
| **Timid** | <20 | Barely autonomous, constant supervision |

---

## What is CodeCoder?

A three-layer AI workstation combining engineering capability with decision wisdom:

| Layer | Focus | Capabilities |
|-------|-------|--------------|
| **Engineering** | Code & Systems | Code review, security analysis, TDD, architecture, reverse engineering |
| **Domain** | Expert Knowledge | Macro economics, trading, product selection, AI engineering |
| **Thinking** | Decision Frameworks | Observer theory, CLOSE framework, sustainable decisions |

### Core Capabilities

- **31 Specialized Agents** — Each with distinct expertise and personality
- **Autonomous Evolution** — Self-building capability through gap detection
- **20+ AI Providers** — Claude, OpenAI, Google, Amazon Bedrock, xAI, and more
- **Multiple Interfaces** — CLI, TUI (terminal UI), Web, and IM bots
- **MCP Protocol** — Local, remote, and OAuth-authenticated servers
- **Rust Microservices** — High-performance services for production deployment
- **Markdown Memory** — Human-readable, Git-friendly knowledge persistence

---

## Quick Start

### Prerequisites

- **Bun** 1.3+ ([install](https://bun.sh/docs/installation))
- **Node.js** 22+ (for some dependencies)
- **Rust** 1.75+ (optional, for Rust services)

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

### Basic Usage

```bash
# Start TUI interface (default)
bun dev

# Run in CLI mode with a message
bun dev run "Review the authentication code in src/auth"

# Start headless API server
bun dev serve --port 4400

# Use a specific agent
bun dev run --agent code-reviewer "Analyze src/api/server.ts"

# Use @ syntax for agents
bun dev run "@decision Analyze this career choice using CLOSE framework"

# Use autonomous mode (self-directed execution)
bun dev run --agent autonomous "Build a REST API for user management"
```

---

## Architecture Overview

```
codecoder/
├── packages/                    # TypeScript packages
│   ├── ccode/                   # Core CLI & business logic
│   │   ├── src/
│   │   │   ├── agent/           # Agent definitions & prompts
│   │   │   ├── autonomous/      # Autonomous evolution system
│   │   │   ├── api/             # HTTP API server (Hono)
│   │   │   ├── cli/             # CLI commands & TUI
│   │   │   ├── config/          # Configuration management
│   │   │   ├── mcp/             # MCP protocol support
│   │   │   ├── provider/        # AI provider adapters
│   │   │   ├── session/         # Session management
│   │   │   └── tool/            # Tool definitions
│   │   └── test/                # Tests
│   ├── util/                    # Shared utilities
│   └── web/                     # Web UI (React + Vite)
├── services/                    # Rust services
│   ├── zero-cli/                # CLI daemon (combined service)
│   ├── zero-gateway/            # Auth, routing, quota
│   ├── zero-channels/           # Telegram, Discord, Slack
│   ├── zero-workflow/           # Webhooks, Cron, Git + Hands
│   ├── zero-browser/            # Browser automation
│   ├── zero-common/             # Shared config
│   ├── zero-agent/              # Agent execution (lib)
│   ├── zero-memory/             # Memory persistence (lib)
│   └── zero-tools/              # Tool definitions (lib)
├── memory/                      # Markdown memory system
│   ├── MEMORY.md                # Long-term knowledge
│   └── daily/                   # Daily notes
├── docs/                        # Documentation
└── script/                      # Build scripts
```

### Port Configuration

| Service | Port | Technology |
|---------|------|------------|
| CodeCoder API Server | 4400 | Bun/TypeScript |
| Web Frontend | 4401 | Vite/React |
| Zero CLI Daemon | 4402 | Rust (combined) |
| Whisper STT Server | 4403 | Docker |
| MCP Server | 4420 | Model Context Protocol |
| Zero Gateway | 4430 | Rust |
| Zero Channels | 4431 | Rust |
| Zero Workflow | 4432 | Rust |
| Zero Browser | 4433 | Rust |

---

## 8 Core Concepts

CodeCoder is built on 8 foundational concepts:

| Concept | Description | Location |
|---------|-------------|----------|
| **AGENT** | Intelligent execution units with specific roles | `packages/ccode/src/agent/` |
| **PROMPT** | Agent behavior definition files | `packages/ccode/src/agent/prompt/*.txt` |
| **SKILL** | Reusable cross-project capabilities | `~/.codecoder/skills/*/SKILL.md` |
| **TOOL** | Execution tools for environment interaction | `packages/ccode/src/tool/` |
| **CHANNEL** | Message channels (IM platforms) | `services/zero-channels/` |
| **MEMORY** | Two-layer Markdown memory system | `memory/` directory |
| **WORKFLOW** | Automation engine (Cron, Webhook, Git) | `services/zero-workflow/` |
| **HAND** | Autonomous persistent agents | `services/zero-workflow/src/hands/` |

### HAND = WORKFLOW + AGENT + MEMORY

A **HAND** is the highest-level abstraction — a declaratively defined autonomous agent:

```markdown
---
id: "market-sentinel"
name: "Market Sentinel"
schedule: "0 */30 * * * *"        # Every 30 minutes
agent: "macro"                     # Use macro Agent
enabled: true
autonomy:
  level: "crazy"                   # Autonomy level
  unattended: true
decision:
  use_close: true                  # Use CLOSE framework
---

# Market Sentinel

Monitor macroeconomic data changes and identify market signals.
```

---

## Agent List

CodeCoder includes 31 specialized agents:

### Main Modes (4)

| Agent | Description |
|-------|-------------|
| `build` | Default development mode with full capabilities |
| `plan` | Planning mode for structured implementation design |
| `writer` | Long-form content writing (20k+ words) |
| `autonomous` | Self-directed execution with CLOSE decision framework |

### Engineering Quality (7)

| Agent | Description |
|-------|-------------|
| `code-reviewer` | Comprehensive code quality review |
| `security-reviewer` | Security vulnerability analysis |
| `tdd-guide` | Test-driven development enforcement |
| `architect` | System architecture design |
| `explore` | Fast codebase exploration and pattern search |
| `general` | Multi-step task execution and research |
| `verifier` | Build, type, and test verification |

### Reverse Engineering (2)

| Agent | Description |
|-------|-------------|
| `code-reverse` | Pixel-perfect website recreation planning |
| `jar-code-reverse` | Java JAR decompilation and reconstruction |

### Content Creation (5)

| Agent | Description |
|-------|-------------|
| `expander` | Systematic content expansion framework |
| `expander-fiction` | Fiction-specific worldbuilding and narrative |
| `expander-nonfiction` | Non-fiction argumentation and evidence |
| `proofreader` | Grammar, style, and consistency checking |
| `writer` | Long-form content writing |

### 祝融说 Series (8)

| Agent | Description |
|-------|-------------|
| `observer` | Observer theory analysis |
| `decision` | CLOSE framework decision advisor |
| `macro` | Macroeconomic data interpretation |
| `trader` | Ultra-short-term trading pattern recognition |
| `picker` | Product selection using "Seven Sins" methodology |
| `miniproduct` | Solo developer 0-to-1 product coaching |
| `ai-engineer` | AI/ML engineering mentor |
| `value-analyst` | Value investment analysis |

### Product & Feasibility (2)

| Agent | Description |
|-------|-------------|
| `prd-generator` | Product requirements document generation |
| `feasibility-assess` | Technical feasibility analysis |

### System (3)

| Agent | Description |
|-------|-------------|
| `synton-assistant` | SYNTON-DB memory database assistant |
| `compaction` | Context compression (hidden) |
| `title` | Automatic session title generation (hidden) |
| `summary` | Session summary generation (hidden) |

---

## Autonomous Evolution Deep Dive

### Self-Building Capability

When CodeCoder encounters a task it cannot complete with existing capabilities, it can:

1. **Detect the gap** — Analyze failure patterns to identify missing capabilities
2. **Evaluate with CLOSE** — Decide whether building the capability is worthwhile
3. **Generate the concept** — Create Agent, Prompt, Skill, Tool, Hand, Memory, or Workflow
4. **Validate** — Ensure the generated concept meets quality standards
5. **Register** — Make the new concept available for future use

### What Can Be Auto-Built

| Concept | Risk | Example |
|---------|------|---------|
| **TOOL** | Low | A reusable script for JSON transformation |
| **PROMPT** | Low | Behavior template for a new specialist |
| **SKILL** | Low | TDD methodology for a specific framework |
| **AGENT** | Medium | New domain specialist like "database-optimizer" |
| **MEMORY** | Medium | Structured knowledge base for a project |
| **HAND** | High | Persistent autonomous agent for monitoring |
| **WORKFLOW** | High | Multi-step automation pipeline |

### Safety Mechanisms

- **Resource Limits** — Token, cost, and time budgets prevent runaway execution
- **Loop Detection** — Identifies and breaks infinite loops
- **Checkpoint System** — Creates checkpoints before risky operations
- **Rollback Capability** — Maintains ability to revert any change
- **CLOSE Evaluation** — Every decision is scored before execution

---

## AI Providers

CodeCoder supports 20+ AI providers out of the box:

| Provider | Auth Methods |
|----------|--------------|
| Anthropic Claude | API Key, Claude Max (OAuth) |
| OpenAI | API Key, ChatGPT Plus/Pro (OAuth) |
| Google Gemini | API Key |
| Google Vertex AI | Service Account |
| Amazon Bedrock | IAM, Profile, Web Identity |
| Azure OpenAI | API Key |
| GitHub Copilot | OAuth |
| xAI | API Key |
| Mistral AI | API Key |
| Groq | API Key |
| DeepInfra | API Key |
| Cerebras | API Key |
| Cohere | API Key |
| Together AI | API Key |
| Perplexity | API Key |
| OpenRouter | API Key |
| Vercel AI | API Key |
| GitLab Duo | OAuth |

---

## Configuration

Configuration files are loaded in priority order (later overrides earlier):

1. Well-Known remote configuration
2. Global: `~/.config/codecoder/codecoder.json`
3. `CCODE_CONFIG` environment variable
4. Project: `./codecoder.json`
5. `.codecoder/` directory
6. `CCODE_CONFIG_CONTENT` environment variable

### Example Configuration

```json
{
  "$schema": "https://codecoder.ai/schema/config.json",
  "model": "anthropic/claude-sonnet-4-5",
  "small_model": "anthropic/claude-haiku-4-5",
  "default_agent": "build",

  "provider": {
    "anthropic": {
      "options": { "apiKey": "{env:ANTHROPIC_API_KEY}" }
    }
  },

  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["npx", "@anthropic/mcp-server-filesystem", "/home/user"]
    }
  },

  "permission": {
    "*": "allow",
    "bash": {
      "rm -rf *": "deny"
    }
  }
}
```

---

## Rust Microservices

For production deployments, CodeCoder includes high-performance Rust services:

### Zero Daemon (Combined Service)

The `zero-daemon` runs all services in a single process—ideal for development and single-machine deployments.

```bash
# Build Rust services
./ops.sh build rust

# Start the daemon
./ops.sh start zero-daemon
```

### Standalone Services

For distributed deployments:

| Service | Purpose | Port |
|---------|---------|------|
| `zero-gateway` | Authentication, routing, rate limiting | 4430 |
| `zero-channels` | IM integrations | 4431 |
| `zero-workflow` | Automation engine | 4432 |
| `zero-browser` | Browser automation | 4433 |

```bash
# Start individual services
./ops.sh start zero-gateway
./ops.sh start zero-channels
./ops.sh start zero-workflow

# View status
./ops.sh status

# View logs
./ops.sh logs zero-workflow
```

---

## Memory System

CodeCoder uses a transparent, Git-friendly two-layer memory architecture:

### Layer 1: Daily Notes (Flow)

- **Path:** `./memory/daily/{YYYY-MM-DD}.md`
- **Type:** Append-only log
- **Purpose:** Captures daily interactions, decisions, and tasks

### Layer 2: Long-term Memory (Sediment)

- **Path:** `./memory/MEMORY.md`
- **Type:** Curated, structured knowledge
- **Categories:** User preferences, project context, key decisions

### Operation Rules

| Operation | When | Behavior |
|-----------|------|----------|
| **Read** | Session init | Load MEMORY.md + current/previous daily notes |
| **Immediate write** | After interactions | Append to daily note (immutable) |
| **Consolidation** | Significant info detected | Update MEMORY.md |

All memory files are standard Markdown—edit them directly when needed.

---

## Development Commands

```bash
# Install dependencies
bun install

# Run TUI interface
bun dev

# Run in specific directory
bun dev /path/to/project

# Start API server
bun dev serve --port 4400

# Type checking
bun turbo typecheck

# Run tests (from package directory)
cd packages/ccode && bun test

# Build standalone executable
bun run --cwd packages/ccode build

# Regenerate SDK after API changes
./script/generate.ts

# Service operations
./ops.sh start          # Start core services
./ops.sh start all      # Start all services
./ops.sh stop           # Stop all services
./ops.sh status         # View service status
./ops.sh build rust     # Build Rust services
./ops.sh logs api       # View API logs
```

---

## Documentation

- [Architecture Overview](docs/architecture/README.md)
- [Core Concepts](docs/architecture/CORE_CONCEPTS.md) — AGENT, PROMPT, SKILL, TOOL, CHANNEL, MEMORY, WORKFLOW, HAND
- [Design Philosophy](docs/architecture/DESIGN_PHILOSOPHY.md) — Zhurongsuo philosophy and CLOSE framework
- [CLAUDE.md](CLAUDE.md) — Project-specific instructions for Claude Code

---

## Contributing

We welcome contributions! Please:

- PRs must reference an existing issue
- PR titles follow conventional commits (`feat:`, `fix:`, `docs:`, etc.)
- UI changes require screenshots/videos
- Keep PRs small and focused

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

<p align="center">
  <i>Built with Observer Theory in mind: every interaction collapses possibilities into reality.</i>
</p>
