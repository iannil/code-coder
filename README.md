# CodeCoder

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue.svg)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Bun](https://img.shields.io/badge/Bun-1.3+-pink.svg)](https://bun.sh/)

<!-- Logo placeholder: Add your logo here -->

---

## Beyond Code Completion

CodeCoder isn't just another AI coding assistant. It's a **work station** that combines engineering prowess with decision wisdom.

At its core is **Observer Theory** from the 祝融说 (Zhurongsuo) philosophy: every observation collapses infinite possibilities into concrete reality. When you ask CodeCoder to review code, design architecture, or analyze a market trend, you're not just getting answers—you're participating in a creative act that shapes outcomes from a field of potential.

This philosophical foundation manifests in the **CLOSE Framework** for decision-making:

- **C**lear reversibility assessment
- **L**everage option preservation
- **O**ption space expansion
- **S**ustainability over optimality
- **E**xit strategy planning

The key insight: **sustainable decisions outperform optimal ones**. CodeCoder helps you maintain "possibility margin" (可用余量)—the unfixed potential that enables adaptation, creativity, and the ability to "play again" when circumstances change.

---

## What is CodeCoder?

A three-layer wisdom architecture that goes beyond code:

| Layer | Focus | Capabilities |
|-------|-------|--------------|
| **Engineering** | Code & Systems | Code review, security analysis, TDD, architecture design, reverse engineering |
| **Domain** | Expert Knowledge | Macro economics, trading analysis, product selection, miniproduct development, AI engineering |
| **Thinking** | Decision Frameworks | 祝融说 philosophy, CLOSE framework, observer theory |

**Core Features:**

- **20+ AI Providers** — Claude, OpenAI, Google, Amazon Bedrock, Azure, xAI, and more
- **24 Specialized Agents** — Each with distinct expertise and personality
- **MCP Protocol** — Local, remote, and OAuth-authenticated Model Context Protocol servers
- **30+ LSP Integrations** — TypeScript, Rust, Go, Python, Java, and more
- **Multiple Modes** — CLI, TUI (terminal UI), and headless API server
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
```

---

## Architecture

```
codecoder/
├── packages/                    # TypeScript packages
│   ├── ccode/                   # Core CLI & business logic
│   │   ├── src/
│   │   │   ├── agent/           # Agent definitions & prompts
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
│   ├── zero-workflow/           # Webhooks, Cron, Git
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
| Zero CLI Daemon | 4402 | Rust (combined: gateway + channels + scheduler) |
| Whisper STT Server | 4403 | Docker |
| Zero Gateway | 4410 | Rust (standalone) |
| Zero Channels | 4411 | Rust (standalone) |
| Zero Workflow | 4412 | Rust (standalone) |
| MCP Server (HTTP) | 4420 | Protocol (Model Context Protocol) |

---

## Agent System

CodeCoder includes 24 specialized agents organized into 6 categories:

### Main Modes (3)

| Agent | Description |
|-------|-------------|
| `build` | Default development mode with full capabilities |
| `plan` | Planning mode for structured implementation design |
| `autonomous` | Self-directed execution with CLOSE decision framework |

### Reverse Engineering (2)

| Agent | Description |
|-------|-------------|
| `code-reverse` | Pixel-perfect website recreation planning |
| `jar-code-reverse` | Java JAR file decompilation and reconstruction |

### Engineering (7)

| Agent | Description |
|-------|-------------|
| `general` | Multi-step task execution and research |
| `explore` | Fast codebase exploration and pattern search |
| `code-reviewer` | Comprehensive code quality review |
| `security-reviewer` | Security vulnerability analysis |
| `tdd-guide` | Test-driven development enforcement |
| `architect` | System architecture design |
| `verifier` | Formal verification and property testing |

### Content Creation (5)

| Agent | Description |
|-------|-------------|
| `writer` | Long-form content writing (20k+ words) |
| `proofreader` | Grammar, style, and consistency checking |
| `expander` | Systematic content expansion framework |
| `expander-fiction` | Fiction-specific worldbuilding and narrative |
| `expander-nonfiction` | Non-fiction argumentation and evidence |

### 祝融说 (Zhurongsuo) Series (8)

| Agent | Description |
|-------|-------------|
| `observer` | Observer theory analysis—revealing possibility spaces |
| `decision` | CLOSE framework decision advisor |
| `macro` | Macroeconomic data interpretation (GDP, policy, etc.) |
| `trader` | Ultra-short-term trading pattern recognition |
| `picker` | Product selection using "Seven Sins" methodology |
| `miniproduct` | Solo developer 0-to-1 product coaching |
| `ai-engineer` | AI/ML engineering mentor |
| `synton-assistant` | SYNTON-DB memory database assistant |

### System (3, hidden)

| Agent | Description |
|-------|-------------|
| `compaction` | Context compression for long sessions |
| `title` | Automatic session title generation |
| `summary` | Session summary generation |

### Usage Examples

```bash
# Code review
bun dev run --agent code-reviewer "Review src/api/server.ts"

# Security analysis
bun dev run --agent security-reviewer "Audit the authentication system"

# Decision consulting
bun dev run "@decision Use CLOSE framework to analyze this job offer"

# Macro analysis
bun dev run "@macro Interpret this month's PMI data"

# Architecture design
bun dev run --agent architect "Design a microservices migration plan"
```

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

### Configuration

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "sk-ant-..."
      }
    },
    "openai": {
      "options": {
        "apiKey": "sk-proj-..."
      }
    }
  },
  "model": "anthropic/claude-sonnet-4-5"
}
```

---

## Rust Services

For production deployments, CodeCoder includes high-performance Rust services:

### Zero Daemon (Combined Service)

The `zero-daemon` runs all services in a single process—ideal for development and single-machine deployments:

```bash
# Build Rust services
./ops.sh build rust

# Start the daemon
./ops.sh start zero-daemon
```

### Standalone Services (Modular Deployment)

For distributed deployments, run services independently:

| Service | Purpose |
|---------|---------|
| `zero-gateway` | Authentication, routing, rate limiting, sandboxing |
| `zero-channels` | Telegram, Discord, Slack, Email integrations |
| `zero-workflow` | Webhook handlers, cron jobs, Git operations |

```bash
# Start individual services
./ops.sh start zero-gateway
./ops.sh start zero-channels
./ops.sh start zero-workflow

# View all service status
./ops.sh status

# View logs
./ops.sh logs zero-workflow
```

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
    },
    "remote-server": {
      "type": "remote",
      "url": "https://mcp.example.com/sse",
      "oauth": {
        "clientId": "...",
        "scope": "read write"
      }
    }
  },

  "permission": {
    "*": "allow",
    "bash": {
      "rm -rf *": "deny",
      "git *": "allow"
    }
  }
}
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
- **Categories:** User preferences, project context, key decisions, lessons learned

### Operation Rules

| Operation | When | Behavior |
|-----------|------|----------|
| **Read** | Session init | Load MEMORY.md + current/previous daily notes |
| **Immediate write** | After important interactions | Append to daily note (immutable) |
| **Consolidation** | Detecting significant info | Update MEMORY.md (merge/replace outdated) |

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

## Contributing

We welcome contributions! Please see our contributing guidelines:

- PRs must reference an existing issue
- PR titles follow conventional commits (`feat:`, `fix:`, `docs:`, etc.)
- UI changes require screenshots/videos
- Keep PRs small and focused

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <i>Built with Observer Theory in mind: every interaction collapses possibilities into reality.</i>
</p>
