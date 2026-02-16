# CodeCoder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-blue.svg)](https://www.typescriptlang.org/)
[![Bun](https://img.shields.io/badge/Runtime-Bun-black.svg)](https://bun.sh/)

[中文](./README_CN.md)

**An AI-powered personal workstation that combines engineering capabilities with decision-making wisdom.**

CodeCoder is not just another coding assistant. It's a comprehensive AI platform designed for everyone whose work is being transformed by AI - developers, analysts, writers, decision-makers, and independent creators. Built on the [祝融说 (Zhurongshuo)](https://zhurongshuo.com) philosophical framework, it integrates structured thinking with practical tooling.

## The Problem We Solve

In an age of AI abundance, the challenge isn't accessing AI - it's knowing **how to think with AI**.

Most AI tools focus on answering questions. CodeCoder focuses on helping you ask better questions and make better decisions. We believe:

- **Multi-dimensional assistance** - Not just code, but decisions, analysis, and content creation
- **Philosophical grounding** - The CLOSE framework for sustainable decision-making
- **Provider flexibility** - 30+ AI providers through a unified interface
- **Specialized expertise** - 25+ purpose-built agents for different domains

## Core Philosophy

CodeCoder is built on **祝融说 (Zhurongshuo)**, a unique philosophical framework that reframes how we approach uncertainty and decision-making.

### Foundational Concepts

**Possibility Substrate**
- Reality emerges from an infinite field of potential
- Every observation converges possibilities into actuality
- Understanding this unlocks creative problem-solving

**Available Surplus**
- The unfixed potential space in any situation
- Source of flexibility, creativity, and system resilience
- Sustainable decisions preserve surplus; optimal decisions often consume it

**Observer Convergence**
- "Observation" is a creative act, not passive reception
- Macro-world stability comes from multi-level observer "voting"
- Social reality is a "symbolic consensus" of collective observation

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

## Three-Layer Architecture

CodeCoder organizes its capabilities into three interconnected layers:

| Layer | Capabilities |
|-------|-------------|
| **Engineering** | Code review, security analysis, TDD guidance, architecture design, reverse engineering, verification |
| **Domain** | Macroeconomics, trading analysis, product selection, indie product development, AI engineering |
| **Thinking** | Observer theory analysis, CLOSE framework decision evaluation, possibility space exploration |

## Specialized Agents (25+)

### Primary Modes

| Agent | Description |
|-------|-------------|
| `build` | Main development mode with full capabilities |
| `plan` | Read-only exploration and planning mode |
| `autonomous` | Self-directed task completion with safety guardrails |

### Engineering Agents

| Agent | Purpose |
|-------|---------|
| `general` | General-purpose research agent - multi-step tasks, parallel work execution |
| `code-reviewer` | Code quality review - identifies code smells, naming issues, maintainability |
| `security-reviewer` | Security vulnerability analysis - OWASP Top 10, injection risks, auth issues |
| `tdd-guide` | Test-driven development guidance - red-green-refactor cycles, coverage |
| `architect` | System architecture design - interfaces, patterns, technical decisions |
| `verifier` | Comprehensive validation - build, type, lint checks, test suites, coverage |
| `explore` | Fast codebase exploration - pattern search, structure understanding |

### Reverse Engineering

| Agent | Purpose |
|-------|---------|
| `code-reverse` | Website pixel-perfect reconstruction planning |
| `jar-code-reverse` | Java JAR file analysis and source reconstruction |

### Domain Agents (祝融说 Series)

| Agent | Purpose |
|-------|---------|
| `macro` | Macroeconomic analysis - GDP, inflation, monetary policy, trade data |
| `trader` | Short-term trading guidance - sentiment cycles, pattern recognition (educational) |
| `picker` | Product selection - seven deadly sins method, market opportunity discovery |
| `miniproduct` | Indie product coaching - MVP design, AI-assisted development, monetization |
| `ai-engineer` | AI engineering mentorship - Python, LLM apps, RAG, fine-tuning, MLOps |
| `synton-assistant` | SYNTON-DB helper - LLM memory databases, tensor graph storage, PaQL queries |

### Thinking Agents (祝融说 Series)

| Agent | Purpose |
|-------|---------|
| `observer` | Observer theory advisor - possibility space analysis, cognitive framework |
| `decision` | Decision coaching - CLOSE framework evaluation, sustainable choice-making |

### Content Agents

| Agent | Purpose |
|-------|---------|
| `writer` | Long-form writing - 20k+ word documents, chapter planning, style consistency |
| `proofreader` | Content proofreading - grammar, style, PROOF framework validation |
| `expander` | Content expansion - transform ideas into comprehensive books |
| `expander-fiction` | Fiction expansion - novels with consistent worldbuilding, character arcs |
| `expander-nonfiction` | Non-fiction expansion - logical argumentation, evidence frameworks |

## Features

### Multi-Provider AI Support

Connect to 30+ AI providers with a unified interface:

| Category | Providers |
|----------|-----------|
| Major | Anthropic Claude, OpenAI, Google Gemini, Amazon Bedrock, Azure |
| Specialized | xAI Grok, Mistral, Groq, Cerebras, Cohere, Perplexity |
| Aggregators | OpenRouter, Together AI, DeepInfra, Vercel AI |
| Enterprise | GitHub Copilot, GitLab Duo, Google Vertex AI |
| Local | Any OpenAI-compatible endpoint via MCP |

### Built-in Tools

CodeCoder includes 20+ built-in tools:

| Category | Tools |
|----------|-------|
| File Operations | `read`, `write`, `edit`, `glob`, `grep`, `ls` |
| Code Intelligence | `codesearch`, `lsp`, `bash` |
| Web | `webfetch`, `websearch` |
| Planning | `task`, `todo`, `plan`, `question` |
| Advanced | `batch`, `multiedit`, `apply_patch`, `network-analyzer` |

### Language Server Protocol (LSP)

Built-in support for 30+ language servers with auto-installation:

TypeScript, Go, Rust, Python, Ruby, Java, Kotlin, C/C++, C#, Swift, Dart, Elixir, Zig, PHP, Lua, OCaml, Haskell, Clojure, Gleam, and more.

### Model Context Protocol (MCP)

- Local and remote MCP servers
- OAuth 2.0 authentication flow
- Dynamic tool discovery
- SSE streaming support

### Terminal UI

A rich terminal interface built with SolidJS and OpenTUI:

- Full keyboard navigation
- Session management with branching
- Real-time streaming responses
- Diff visualization for code changes

### Memory System

Transparent, Git-friendly memory architecture:

```
memory/
├── daily/              # Append-only daily logs
│   └── {YYYY-MM-DD}.md
└── MEMORY.md           # Curated long-term knowledge
```

All memory is human-readable Markdown, version-controlled, and manually editable.

### HTTP API Server

Run CodeCoder as a headless server:

```bash
ccode serve --port 4096
```

Exposes all agents via HTTP API for integration with other tools.

### ZeroBot Integration

CodeCoder includes [ZeroBot](./services/zero-bot/), a lightweight Rust-based AI assistant gateway:

| Feature | Description |
|---------|-------------|
| Multi-channel | Telegram, Discord, Slack, WhatsApp, Matrix, iMessage, Email |
| 24 Providers | OpenAI, Claude, Gemini, Groq, Mistral, and more |
| Tiny Footprint | ~3.4MB binary, <5MB memory |
| Bidirectional | ZeroBot calls CodeCoder agents via HTTP API |

```toml
# ~/.codecoder/config.toml
[codecoder]
enabled = true
endpoint = "http://localhost:4096"
```

The two systems are complementary:
- **ZeroBot**: Lightweight gateway for multi-channel message access
- **CodeCoder**: Full-featured workbench for complex AI tasks

## Installation

### Requirements

- [Bun](https://bun.sh/) 1.3 or higher
- macOS, Linux, or Windows (WSL recommended)

### From Source

```bash
git clone https://github.com/iannil/code-coder.git
cd code-coder
bun install
bun dev
```

### Build Standalone Executable

```bash
cd packages/ccode
bun run build
```

## Quick Start

```bash
# Start interactive TUI
ccode

# Or specify a working directory
ccode /path/to/project

# CLI mode with direct message
ccode run "Explain the architecture of this codebase"

# Continue last session
ccode run --continue

# Use a specific agent
ccode run --agent architect "Design a REST API for user management"
ccode run --agent decision "Should I accept this job offer?"
ccode run --agent macro "Interpret the latest PMI data"

# Headless server mode
ccode serve --port 4096
```

## Configuration

### Configuration Locations (Priority Order)

1. Global: `~/.config/codecoder/codecoder.json`
2. Project: `./codecoder.json` or `./.ccode/codecoder.json`
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
  },
  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["npx", "@anthropic/mcp-filesystem"]
    }
  }
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CodeCoder CLI                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   TUI App   │  │  CLI Cmds   │  │   Server    │          │
│  │  (SolidJS)  │  │  (yargs)    │  │   (API)     │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│                      Core Engine                            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │  Agent   │ │  Session │ │   Tool   │ │ Provider │        │
│  │  System  │ │  Manager │ │  System  │ │  System  │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │   MCP    │ │  Memory  │ │   LSP    │ │   Hook   │        │
│  │ Protocol │ │  System  │ │  System  │ │  System  │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
├─────────────────────────────────────────────────────────────┤
│                      AI Providers                           │
│  Anthropic │ OpenAI │ Google │ AWS │ Azure │ OpenRouter...  │
└─────────────────────────────────────────────────────────────┘
```

### Tech Stack

- **Runtime**: Bun 1.3+ with TypeScript ESM
- **UI Framework**: OpenTUI + SolidJS
- **Validation**: Zod schemas
- **AI SDK**: Vercel AI SDK with multi-provider support
- **Build**: Bun native bundler

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
│   └── zero-bot/        # Rust message gateway (optional)
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
```

## Commands Reference

```bash
# Authentication
ccode auth login <provider>
ccode auth logout <provider>
ccode auth list

# Session management
ccode session list
ccode session show <id>
ccode session delete <id>

# Agent management
ccode agent list
ccode agent generate

# Model management
ccode models list
ccode models default

# MCP server management
ccode mcp list
ccode mcp auth <name>
ccode mcp connect <name>

# Memory management
ccode memory show
ccode memory clear

# Reverse engineering
ccode reverse <url>
ccode jar-reverse <file>
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](./docs/CONTRIB.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes following [conventional commits](https://www.conventionalcommits.org/)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Bun](https://bun.sh) - Fast all-in-one JavaScript runtime
- Powered by [Vercel AI SDK](https://sdk.vercel.ai) - Multi-provider AI integration
- UI powered by [OpenTUI](https://github.com/sst/opentui) - Terminal UI framework

---

<p align="center">
  <strong>CodeCoder</strong> - Where engineering meets wisdom
</p>
