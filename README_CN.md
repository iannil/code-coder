# CodeCoder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-blue.svg)](https://www.typescriptlang.org/)
[![Bun](https://img.shields.io/badge/Runtime-Bun-black.svg)](https://bun.sh/)

[English](./README.md)

你的 AI 工作站 - 一个功能强大的 CLI 工具，配备 23 个专业 AI Agent，覆盖软件开发、代码分析和智能自动化。

## 功能特性

- 23 个专业 AI Agent - 三层智慧架构，涵盖工程、领域专业、思维框架和内容创作
- 20+ AI 提供商支持 - Anthropic Claude、OpenAI、Google Gemini、AWS Bedrock、Azure、OpenRouter 等
- 丰富的工具生态 - 文件操作、代码执行、网络获取等
- MCP 协议支持 - Model Context Protocol 扩展能力
- 网站逆向工程 - 分析并生成像素级还原的开发计划
- 长文档写作 - AI 辅助生成 10 万字以上的长篇文档
- 自主执行 - 自主完成任务，配备安全护栏
- 形式化验证 - 基于属性的测试和不变量验证

## 安装

### npm

```bash
npm install -g @codecoder/ccode
```

### Bun

```bash
bun install -g @codecoder/ccode
```

### 从源码安装

```bash
git clone https://github.com/iannil/code-coder.git
cd code-coder/packages/ccode
bun install
bun run build
```

## 快速开始

```bash
# 启动交互式会话
ccode run

# 发送直接消息
ccode run "解释这个代码库的架构"

# 继续上次会话
ccode run --continue

# 使用特定 Agent
ccode run --agent architect "设计一个用户管理的 REST API"

# 使用特定模型
ccode run --model anthropic/claude-sonnet-4-20250514 "审查我的代码"
```

## AI Agent

CodeCoder 采用三层智慧架构，配备 23 个专业 Agent：

### 主要模式（Primary Modes）

| Agent                | 描述                         | 权限     |
| -------------------- | ---------------------------- | -------- |
| build            | 主开发模式，完整文件操作能力 | 完全读写 |
| plan             | 代码探索和规划模式           | 只读     |
| code-reverse     | 网站逆向工程                 | 只读     |
| jar-code-reverse | JAR 文件逆向工程             | 只读     |

### 工程

| Agent                 | 用途                                            |
| --------------------- | ----------------------------------------------- |
| code-reviewer     | 代码质量审查 - 识别代码异味、命名问题、可维护性 |
| security-reviewer | 安全漏洞分析 - OWASP Top 10、注入风险、认证问题 |
| tdd-guide         | 测试驱动开发指导 - 红-绿-重构循环、覆盖率       |
| architect         | 系统架构设计 - 接口定义、设计模式、技术决策     |
| explore           | 快速代码库探索 - 模式搜索、结构理解             |
| general           | 多步骤任务执行 - 复杂工作流、并行处理           |
| verifier          | 形式化验证 - 基于属性的测试、不变量验证         |

### 领域

| Agent                | 用途                                                |
| -------------------- | --------------------------------------------------- |
| macro            | 宏观经济分析 - GDP、通胀、货币政策、贸易数据        |
| trader           | 超短线交易指导 - 情绪周期、模式识别（仅供教育参考） |
| picker           | 选品策略专家 - 七宗罪选品法、市场机会发现           |
| miniproduct      | 极小产品教练 - MVP 设计、AI 辅助开发、变现策略      |
| ai-engineer      | AI 工程师导师 - Python、LLM 应用、RAG、微调、MLOps  |
| synton-assistant | SYNTON-DB 助手 - 张量图存储、PaQL 查询、Graph-RAG   |

### 思维

| Agent        | 用途                                                          |
| ------------ | ------------------------------------------------------------- |
| observer | 观察者理论顾问 - 可能性空间分析、认知框架                     |
| decision | 决策智慧师 - CLOSE 五维评估（收敛、杠杆、选择权、余量、演化） |

### 内容

| Agent           | 用途                                      |
| --------------- | ----------------------------------------- |
| writer      | 长文写作专家 - 2 万字以上文档、章节规划   |
| proofreader | 内容校对专家 - 语法、风格、PROOF 框架验证 |

## 工具集

### 文件操作

| 工具    | 描述                       |
| ------- | -------------------------- |
| `read`  | 读取文件内容，支持分页     |
| `write` | 写入或覆盖文件             |
| `edit`  | 在文件中执行精确字符串替换 |
| `glob`  | 基于模式的文件匹配         |
| `grep`  | 基于正则的内容搜索         |
| `ls`    | 列出目录内容               |

### 执行

| 工具   | 描述                          |
| ------ | ----------------------------- |
| `bash` | 在持久会话中执行 Shell 命令   |
| `task` | 启动专业子 Agent 处理复杂任务 |

### 网络

| 工具               | 描述                    |
| ------------------ | ----------------------- |
| `webfetch`         | 获取并分析网页内容      |
| `websearch`        | 搜索网络信息            |
| `network-analyzer` | 分析网络流量和 API 端点 |

### 其他

| 工具       | 描述             |
| ---------- | ---------------- |
| `question` | 交互式用户提示   |
| `todo`     | 任务列表管理     |
| `skill`    | 加载专业技能文档 |
| `patch`    | 应用统一差异补丁 |

## 支持的 AI 提供商

CodeCoder 开箱即支持 20+ AI 提供商：

- Anthropic - Claude 3.5、Claude 4 系列
- OpenAI - GPT-4o、GPT-4.1、o1、o3 系列
- Google - Gemini 2.0、Gemini 2.5 系列
- AWS Bedrock - 多种基础模型
- Azure OpenAI - 企业级 OpenAI 访问
- OpenRouter - 通过统一 API 访问 200+ 模型
- Groq - 超快推理
- Cerebras - 高速 AI 推理
- Mistral - Mistral 和 Codestral 模型
- Cohere - Command 系列
- xAI - Grok 模型
- DeepInfra - 高性价比推理
- Together AI - 开源模型托管
- Perplexity - 搜索增强模型
- GitHub Copilot - GitHub AI 助手
- GitLab - GitLab AI 功能
- Vercel AI Gateway - 统一模型访问

## 配置

### 全局配置

位于 `~/.ccode/config.json`：

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

### 自定义 Agent

在 `~/.ccode/agents.json` 中定义自定义 Agent：

```json
{
  "agents": [
    {
      "name": "my-advisor",
      "description": "我的自定义顾问",
      "mode": "subagent",
      "permission": "read",
      "systemPrompt": "你是...方面的专家",
      "temperature": 0.6
    }
  ]
}
```

### 项目配置

在项目根目录创建 `AGENTS.md` 文件，添加项目特定指令：

```markdown
# 项目指令

## 构建命令

- npm run build
- npm run test

## 代码风格

- 使用 TypeScript 严格模式
- 优先使用函数式编程模式
```

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      CodeCoder CLI                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   TUI App   │  │  CLI cmds   │  │   Server    │         │
│  │  (SolidJS)  │  │  (yargs)    │  │   (API)     │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                      Core Engine                            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │
│  │  Agent   │ │  Session │ │   Tool   │ │ Provider │      │
│  │  System  │ │  Manager │ │  System  │ │  System  │      │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘      │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │
│  │   MCP    │ │  Memory  │ │Verifier  │ │Autonomous│      │
│  │ Protocol │ │  System  │ │  Engine  │ │  Engine  │      │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘      │
├─────────────────────────────────────────────────────────────┤
│                      AI Providers                           │
│  Anthropic │ OpenAI │ Google │ AWS │ Azure │ OpenRouter... │
└─────────────────────────────────────────────────────────────┘
```

### 技术栈

- 运行时: Bun + TypeScript ESM 模块
- UI 框架: OpenTUI + SolidJS
- 验证: Zod schemas
- AI SDK: Vercel AI SDK 多提供商支持
- 构建: Bun 原生打包器
- 测试: Bun test 框架

## 内置斜杠命令

| 命令       | 描述                     |
| ---------- | ------------------------ |
| `/help`    | 获取 CodeCoder 使用帮助  |
| `/accept`  | 接受并实现更改           |
| `/docs`    | 生成文档                 |
| `/issues`  | 分析并修复 GitHub Issues |
| `/next`    | 建议下一步开发工作       |
| `/readme`  | 生成或更新 README 文件   |
| `/roadmap` | 创建项目路线图           |

## 命令行

```bash
# 认证
ccode auth login <provider>
ccode auth logout <provider>
ccode auth status

# 会话管理
ccode session list
ccode session show <id>

# Agent 管理
ccode agent list
ccode agent show <name>

# 模型管理
ccode models list
ccode models show <provider/model>

# MCP 服务器管理
ccode mcp list
ccode mcp add <name> <command>
ccode mcp remove <name>

# 记忆管理
ccode memory show
ccode memory clear

# 文档生成
ccode document create --title "我的书" --words 100000
ccode chapter next

# 逆向工程
ccode reverse analyze <url> --output ./report
ccode jar-reverse analyze <jar-file>

# 调试模式
ccode debug <session-id>
```

## 贡献

欢迎贡献代码！详情请参阅[贡献指南](CONTRIBUTING.md)。

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m '添加某功能'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 开发

```bash
# 安装依赖
bun install

# 开发模式运行
bun run dev

# 类型检查
bun run typecheck

# 运行测试
bun test

# 运行特定测试文件
bun test test/tool/tool.test.ts

# 构建
bun run build
```

## 许可证

本项目基于 MIT 许可证开源 - 详情请参阅 [LICENSE](LICENSE) 文件。

## 致谢

- 使用 [Bun](https://bun.sh) 构建 - 快速的一体化 JavaScript 运行时
- 由 [Vercel AI SDK](https://sdk.vercel.ai) 驱动 - 多提供商 AI 集成
- UI 由 [OpenTUI](https://opentui.com) 提供支持 - 终端 UI 框架

---

<p align="center">
  CodeCoder 团队用 ❤️ 制作
</p>
