# CodeCoder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-blue.svg)](https://www.typescriptlang.org/)
[![Bun](https://img.shields.io/badge/Runtime-Bun-black.svg)](https://bun.sh/)

[中文](./README_CN.md)

Your AI-powered work station - A powerful CLI tool with 23 specialized AI agents for software development, code analysis, and intelligent automation.

## Features

- 23 Specialized AI Agents - Three-layer intelligence architecture covering engineering, domain expertise, thinking frameworks, and content creation
- 20+ AI Provider Support - Anthropic Claude, OpenAI, Google Gemini, AWS Bedrock, Azure, OpenRouter, and many more
- Rich Tool Ecosystem - File operations, code execution, web fetching, and more
- MCP Protocol Support - Model Context Protocol for extended capabilities
- Website Reverse Engineering - Analyze and recreate pixel-perfect UI implementations
- Long-form Document Writing - Generate 100k+ word documents with AI assistance
- Autonomous Execution - Self-directed task completion with safety guardrails
- Formal Verification - Property-based testing and invariant validation

## Installation

### npm

```bash
npm install -g @codecoder/ccode
```

### Bun

```bash
bun install -g @codecoder/ccode
```

### From Source

```bash
git clone https://github.com/iannil/code-coder.git
cd code-coder/packages/ccode
bun install
bun run build
```

## Quick Start

```bash
# Start an interactive session
ccode run

# Send a direct message
ccode run "Explain the architecture of this codebase"

# Continue last session
ccode run --continue

# Use a specific agent
ccode run --agent architect "Design a REST API for user management"

# Use a specific model
ccode run --model anthropic/claude-sonnet-4-20250514 "Review my code"
```

## AI Agents

CodeCoder features a three-layer intelligence architecture with 23 specialized agents:

### Primary Modes

| Agent                | Description                                     | Permissions     |
| -------------------- | ----------------------------------------------- | --------------- |
| build            | Main development mode with full file operations | Full read/write |
| plan             | Code exploration and planning mode              | Read-only       |
| code-reverse     | Website reverse engineering                     | Read-only       |
| jar-code-reverse | JAR file reverse engineering                    | Read-only       |

### Engineering Layer

| Agent                 | Purpose                                                                      |
| --------------------- | ---------------------------------------------------------------------------- |
| code-reviewer     | Code quality review - identifies code smells, naming issues, maintainability |
| security-reviewer | Security vulnerability analysis - OWASP Top 10, injection risks, auth issues |
| tdd-guide         | Test-driven development guidance - red-green-refactor cycles, coverage       |
| architect         | System architecture design - interfaces, patterns, technical decisions       |
| explore           | Fast codebase exploration - pattern search, structure understanding          |
| general           | Multi-step task execution - complex workflows, parallel processing           |
| verifier          | Formal verification - property-based testing, invariant validation           |

### Domain Layer

| Agent                | Purpose                                                                           |
| -------------------- | --------------------------------------------------------------------------------- |
| macro            | Macroeconomic analysis - GDP, inflation, monetary policy, trade data              |
| trader           | Short-term trading guidance - sentiment cycles, pattern recognition (educational) |
| picker           | Product selection expert - seven deadly sins method, market opportunity discovery |
| miniproduct      | Micro-product coach - MVP design, AI-assisted development, monetization           |
| ai-engineer      | AI engineering mentor - Python, LLM apps, RAG, fine-tuning, MLOps                 |
| synton-assistant | SYNTON-DB assistant - tensor graph storage, PaQL queries, Graph-RAG               |

### Thinking Layer

| Agent        | Purpose                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------ |
| observer | Observer theory advisor - possibility space analysis, cognitive framework                  |
| decision | Decision wisdom - CLOSE framework (Convergence, Leverage, Optionality, Surplus, Evolution) |

### Content Layer

| Agent           | Purpose                                                           |
| --------------- | ----------------------------------------------------------------- |
| writer      | Long-form writing expert - 20k+ word documents, chapter planning  |
| proofreader | Content proofreading - grammar, style, PROOF framework validation |

## Tools

### File Operations

| Tool    | Description                                |
| ------- | ------------------------------------------ |
| `read`  | Read file contents with pagination support |
| `write` | Write or overwrite files                   |
| `edit`  | Perform exact string replacements in files |
| `glob`  | Pattern-based file matching                |
| `grep`  | Regex-based content search                 |
| `ls`    | List directory contents                    |

### Execution

| Tool   | Description                                     |
| ------ | ----------------------------------------------- |
| `bash` | Execute shell commands with persistent sessions |
| `task` | Launch specialized subagents for complex tasks  |

### Web

| Tool               | Description                               |
| ------------------ | ----------------------------------------- |
| `webfetch`         | Fetch and analyze web content             |
| `websearch`        | Search the web for information            |
| `network-analyzer` | Analyze network traffic and API endpoints |

### Other

| Tool       | Description                          |
| ---------- | ------------------------------------ |
| `question` | Interactive user prompts             |
| `todo`     | Task list management                 |
| `skill`    | Load specialized skill documentation |
| `patch`    | Apply unified diff patches           |

## Supported AI Providers

CodeCoder supports 20+ AI providers out of the box:

- Anthropic - Claude 3.5, Claude 4 series
- OpenAI - GPT-4o, GPT-4.1, o1, o3 series
- Google - Gemini 2.0, Gemini 2.5 series
- AWS Bedrock - Multiple foundation models
- Azure OpenAI - Enterprise OpenAI access
- OpenRouter - 200+ models through unified API
- Groq - Ultra-fast inference
- Cerebras - High-speed AI inference
- Mistral - Mistral and Codestral models
- Cohere - Command series
- xAI - Grok models
- DeepInfra - Cost-effective inference
- Together AI - Open-source model hosting
- Perplexity - Search-enhanced models
- GitHub Copilot - GitHub's AI assistant
- GitLab - GitLab AI features
- Vercel AI Gateway - Unified model access

## Configuration

### Global Configuration

Located at `~/.ccode/config.json`:

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "your-api-key"
      }
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

### Custom Agents

Define custom agents in `~/.ccode/agents.json`:

```json
{
  "agents": [
    {
      "name": "my-advisor",
      "description": "My custom advisor",
      "mode": "subagent",
      "permission": "read",
      "systemPrompt": "You are an expert in...",
      "temperature": 0.6
    }
  ]
}
```

### Project Configuration

Create `AGENTS.md` in your project root for project-specific instructions:

```markdown
# Project Instructions

## Build Commands

- npm run build
- npm run test

## Code Style

- Use TypeScript strict mode
- Prefer functional programming patterns
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CodeCoder CLI                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   TUI App   │  │  CLI cmds   │  │   Server    │          │
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
│  │   MCP    │ │  Memory  │ │Verifier  │ │Autonomous│        │
│  │ Protocol │ │  System  │ │  Engine  │ │  Engine  │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
├─────────────────────────────────────────────────────────────┤
│                      AI Providers                           │
│  Anthropic │ OpenAI │ Google │ AWS │ Azure │ OpenRouter...  │
└─────────────────────────────────────────────────────────────┘
```

### Tech Stack

- Runtime: Bun with TypeScript ESM modules
- UI Framework: OpenTUI + SolidJS
- Validation: Zod schemas
- AI SDK: Vercel AI SDK with multi-provider support
- Build: Bun native bundler
- Testing: Bun test framework

## Built-in Commands

| Command    | Description                     |
| ---------- | ------------------------------- |
| `/help`    | Get help with using CodeCoder   |
| `/accept`  | Accept and implement changes    |
| `/docs`    | Generate documentation          |
| `/issues`  | Analyze and fix GitHub issues   |
| `/next`    | Suggest next development steps  |
| `/readme`  | Generate or update README files |
| `/roadmap` | Create project roadmap          |

## Commands

```bash
# Authentication
ccode auth login <provider>
ccode auth logout <provider>
ccode auth status

# Session management
ccode session list
ccode session show <id>

# Agent management
ccode agent list
ccode agent show <name>

# Model management
ccode models list
ccode models show <provider/model>

# MCP server management
ccode mcp list
ccode mcp add <name> <command>
ccode mcp remove <name>

# Memory management
ccode memory show
ccode memory clear

# Document generation
ccode document create --title "My Book" --words 100000
ccode chapter next

# Reverse engineering
ccode reverse analyze <url> --output ./report
ccode jar-reverse analyze <jar-file>

# Debug mode
ccode debug <session-id>
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Development

```bash
# Install dependencies
bun install

# Run in development mode
bun run dev

# Type check
bun run typecheck

# Run tests
bun test

# Run specific test file
bun test test/tool/tool.test.ts

# Build
bun run build
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Bun](https://bun.sh) - Fast all-in-one JavaScript runtime
- Powered by [Vercel AI SDK](https://sdk.vercel.ai) - Multi-provider AI integration
- UI powered by [OpenTUI](https://opentui.com) - Terminal UI framework

---

<p align="center">
  Made with ❤️ by the CodeCoder Team
</p>
