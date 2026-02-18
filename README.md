# CodeCoder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-blue.svg)](https://www.typescriptlang.org/)
[![Bun](https://img.shields.io/badge/Runtime-Bun-black.svg)](https://bun.sh/)
[![Rust](https://img.shields.io/badge/Gateway-Rust-orange.svg)](https://www.rust-lang.org/)

[中文文档](./README_CN.md)

**AI-powered personal workstation that combines engineering capabilities with decision-making wisdom.**

CodeCoder is not just another coding assistant. It's a comprehensive AI platform designed for everyone whose work is being transformed by AI—developers, analysts, writers, decision-makers, and independent creators. Built on the [祝融说](https://zhurongshuo.com) philosophical framework, it integrates structured thinking with practical tooling.

## Why CodeCoder

In an age of AI abundance, the challenge isn't accessing AI—it's knowing **how to think with AI**.

Most AI tools focus on answering questions. CodeCoder focuses on helping you ask better questions and make better decisions:

- **Multi-dimensional assistance** — Not just code, but decisions, analysis, and content creation
- **Philosophical grounding** — The CLOSE framework for sustainable decision-making
- **Provider flexibility** — 30+ AI providers through a unified interface
- **Specialized expertise** — 25+ purpose-built agents for different domains

## Key Features

| Feature | Description |
|---------|-------------|
| **25+ Specialized Agents** | Engineering, domain analysis, decision-making, and content creation |
| **30+ AI Providers** | Anthropic, OpenAI, Google, AWS Bedrock, Azure, local models via MCP |
| **20+ Built-in Tools** | File operations, code search, web fetch, task management |
| **LSP Support** | 30+ language servers with auto-installation |
| **MCP Protocol** | Local/remote servers, OAuth 2.0, dynamic tool discovery |
| **ZeroBot Gateway** | Lightweight Rust gateway (~3.4MB) for multi-channel access |
| **Memory System** | Transparent, Git-friendly Markdown-based memory |

## Philosophy

### 祝融说 (Zhurongshuo)

CodeCoder is built on **祝融说**, a unique philosophical framework that reframes how we approach uncertainty and decision-making.

| Concept | Description |
|---------|-------------|
| **Possibility Substrate** | Reality emerges from an infinite field of potential. Every observation converges possibilities into actuality. |
| **Observer Convergence** | "Observation" is a creative act, not passive reception. Macro-world stability comes from multi-level observer "voting". |
| **Available Surplus** | The unfixed potential space in any situation. Source of flexibility, creativity, and system resilience. |

### The CLOSE Decision Framework

A five-dimensional evaluation system for making sustainable choices:

| Dimension | Question | Focus |
|-----------|----------|-------|
| **C**onvergence | How much does this narrow possibilities? | Preserving future options |
| **L**everage | Is there asymmetric upside? | Risk-reward asymmetry |
| **O**ptionality | Can this be reversed? At what cost? | Reversibility |
| **S**urplus | How much buffer does this consume? | Resource preservation |
| **E**volution | Does this create growth opportunities? | Learning potential |

> "Sustainable decisions > optimal decisions. The ability to play again matters more than winning once."

## Architecture

### Three-Layer Wisdom Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Thinking Layer                          │
│   observer · decision                                        │
├─────────────────────────────────────────────────────────────┤
│                      Domain Layer                            │
│   macro · trader · picker · miniproduct · ai-engineer        │
├─────────────────────────────────────────────────────────────┤
│                     Engineering Layer                        │
│   code-reviewer · security-reviewer · tdd-guide · architect  │
│   code-reverse · jar-code-reverse · explore · general        │
├─────────────────────────────────────────────────────────────┤
│                      Content Layer                           │
│   writer · proofreader · expander                            │
└─────────────────────────────────────────────────────────────┘
```

### Tech Stack

- **Runtime**: Bun 1.3+ with TypeScript ESM
- **TUI Framework**: OpenTUI + SolidJS
- **AI SDK**: Vercel AI SDK with multi-provider support
- **Gateway**: ZeroBot (Rust, ~3.4MB binary)
- **Validation**: Zod schemas

## Agents Overview

### Primary Modes

| Agent | Description |
|-------|-------------|
| `build` | Main development mode with full capabilities |
| `plan` | Read-only exploration and planning mode |
| `autonomous` | Self-directed task completion with safety guardrails |

### Engineering Agents

| Agent | Purpose |
|-------|---------|
| `general` | Multi-step tasks, parallel work execution |
| `code-reviewer` | Code quality, naming, maintainability |
| `security-reviewer` | OWASP Top 10, injection risks, auth issues |
| `tdd-guide` | Red-green-refactor cycles, coverage |
| `architect` | System design, interfaces, patterns |
| `verifier` | Build, type, lint checks, test suites |
| `explore` | Fast codebase exploration |

### Reverse Engineering

| Agent | Purpose |
|-------|---------|
| `code-reverse` | Website pixel-perfect reconstruction planning |
| `jar-code-reverse` | Java JAR analysis and source reconstruction |

### Domain Agents (祝融说 Series)

| Agent | Purpose |
|-------|---------|
| `macro` | Macroeconomic analysis—GDP, inflation, monetary policy |
| `trader` | Short-term trading guidance (educational only) |
| `picker` | Product selection—seven deadly sins method |
| `miniproduct` | Indie product coaching—MVP, monetization |
| `ai-engineer` | AI engineering—Python, LLM apps, RAG, MLOps |

### Thinking Agents (祝融说 Series)

| Agent | Purpose |
|-------|---------|
| `observer` | Possibility space analysis, cognitive framework |
| `decision` | CLOSE framework evaluation, sustainable choices |

### Content Agents

| Agent | Purpose |
|-------|---------|
| `writer` | Long-form writing (20k+ words), chapter planning |
| `proofreader` | Grammar, style, PROOF framework validation |
| `expander` | Transform ideas into comprehensive books |

## Quick Start

### Requirements

- [Bun](https://bun.sh/) 1.3 or higher
- macOS, Linux, or Windows (WSL recommended)

### Installation

```bash
git clone https://github.com/iannil/code-coder.git
cd code-coder
bun install
```

### Run

```bash
# Start interactive TUI
bun dev

# Or specify a working directory
bun dev /path/to/project

# Start headless API server
bun dev serve --port 4096
```

### Build Standalone Executable

```bash
bun run --cwd packages/ccode build
```

## Configuration

### Configuration Locations (Priority Order)

1. Global: `~/.config/codecoder/codecoder.json`
2. Project: `./codecoder.json` or `./.codecoder/codecoder.json`
3. Environment: `CCODE_CONFIG` or `CCODE_CONFIG_CONTENT`

### Example Configuration

```json
{
  "$schema": "https://raw.githubusercontent.com/iannil/code-coder/main/packages/ccode/schema.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "default_agent": "build",
  "provider": {
    "anthropic": {
      "options": { "apiKey": "{env:ANTHROPIC_API_KEY}" }
    }
  },
  "permission": {
    "bash": {
      "*": "ask",
      "git *": "allow",
      "npm *": "allow"
    }
  }
}
```

## Usage Examples

### Code Review

```bash
ccode run --agent code-reviewer "Review the authentication module"
```

### Decision Analysis (CLOSE Framework)

```bash
ccode run --agent decision "Should I accept this job offer?"
```

### Economic Data Analysis

```bash
ccode run --agent macro "Interpret the latest PMI data"
```

### Product Selection

```bash
ccode run --agent picker "Analyze market opportunities for AI writing tools"
```

## Project Structure

```
codecoder/
├── packages/
│   ├── ccode/           # Core CLI and business logic
│   │   ├── src/
│   │   │   ├── agent/   # Agent definitions and prompts
│   │   │   ├── cli/     # CLI commands and TUI
│   │   │   ├── provider/# AI provider integrations
│   │   │   ├── mcp/     # MCP protocol support
│   │   │   ├── lsp/     # LSP integrations
│   │   │   └── tool/    # Built-in tools
│   │   └── test/        # Test suites
│   └── util/            # Shared utilities
├── services/
│   └── zero-bot/        # Rust message gateway
├── script/              # Build and generation scripts
├── docs/                # Documentation
└── memory/              # Memory storage
```

## Development

```bash
# Install dependencies
bun install

# Run in development mode
bun dev

# Type checking
bun turbo typecheck

# Run tests (from package directory)
cd packages/ccode && bun test

# Build executable
bun run --cwd packages/ccode build

# Regenerate SDK from OpenAPI
./script/generate.ts
```

### Port Configuration

| Service | Port |
|---------|------|
| CodeCoder API Server | 4400 |
| Web Frontend (Vite) | 4401 |
| ZeroBot Daemon | 4402 |
| Faster Whisper Server | 4403 |

## Contributing

We welcome contributions! Please see our [Contributing Guide](./docs/CONTRIB.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes following [conventional commits](https://www.conventionalcommits.org/)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License—see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Bun](https://bun.sh)—Fast all-in-one JavaScript runtime
- Powered by [Vercel AI SDK](https://sdk.vercel.ai)—Multi-provider AI integration
- UI powered by [OpenTUI](https://github.com/sst/opentui)—Terminal UI framework
