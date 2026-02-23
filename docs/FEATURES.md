# CodeCoder 功能清单

本文档详细梳理 CodeCoder 项目的所有功能模块，按照系统架构分层组织。

---

## 1. 项目概览

**CodeCoder** 是一个**个人智囊系统**（Personal Brain Trust System），融合工程能力与决策智慧的 AI 顾问平台。

### 核心特性

- **多 AI 提供商支持**: Claude、OpenAI、Google、Amazon Bedrock、Azure 等 30+ 提供商
- **多模式交互**: CLI 命令行、TUI 终端界面、无头 API 服务器
- **客户端/服务器架构**: 支持本地和远程操作
- **MCP 协议支持**: 本地和远程 MCP 服务器，支持 OAuth 认证
- **LSP 集成**: 30+ 语言服务器协议集成
- **23 个专业 Agent**: 覆盖工程、逆向、内容创作、决策咨询等领域

### 三层智慧架构

| 层级 | 定位 | 涵盖内容 |
|------|------|----------|
| 工程智囊层 | 代码与系统 | 代码审查、安全分析、TDD、架构设计、逆向工程 |
| 领域智囊层 | 专业知识 | 宏观经济、交易分析、选品策略、极小产品、AI 工程 |
| 思维智囊层 | 决策框架 | 祝融说哲学体系、CLOSE 决策框架、观察者理论 |

---

## 2. CLI 命令系统

CLI 入口: `packages/ccode/src/index.ts`

### 2.1 主要命令

| 命令 | 功能描述 | 关键参数 |
|------|----------|----------|
| `codecoder [message..]` (默认) | 启动 TUI 终端界面 | `<path>` - 指定工作目录 |
| `codecoder run [message..]` | 以命令行模式运行 | `--model`, `--agent`, `--continue`, `--session`, `--file`, `--format`, `--title`, `--variant` |
| `codecoder serve` | 启动无头 API 服务器 | `--port` (默认 4400) |

### 2.2 认证命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder auth login [provider]` | 登录指定提供商 |
| `codecoder auth logout [provider]` | 登出指定提供商 |
| `codecoder auth list` | 列出已认证的提供商 |

### 2.3 Agent 管理命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder agent list` | 列出所有可用 Agent |
| `codecoder agent generate` | 自动生成 Agent 配置 |

### 2.4 模型管理命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder models list` | 列出所有可用模型 |
| `codecoder models default` | 显示/设置默认模型 |

### 2.5 Session 管理命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder session list` | 列出所有会话 |
| `codecoder session show <id>` | 显示会话详情 |
| `codecoder session delete <id>` | 删除指定会话 |
| `codecoder session export <id>` | 导出会话 |

### 2.6 MCP 管理命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder mcp list` | 列出 MCP 服务器状态 |
| `codecoder mcp auth <name>` | 对 MCP 服务器进行 OAuth 认证 |
| `codecoder mcp connect <name>` | 连接 MCP 服务器 |
| `codecoder mcp disconnect <name>` | 断开 MCP 服务器 |

### 2.7 文档生成命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder document` | 长文档生成入口 |
| `codecoder chapter` | 章节管理 |

### 2.8 逆向工程命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder reverse <url>` | 网站逆向工程分析 |
| `codecoder jar-reverse <file>` | JAR 文件逆向工程分析 |

### 2.9 记忆系统命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder memory show` | 显示记忆内容 |
| `codecoder memory clear` | 清除记忆 |

### 2.10 调试命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder debug config` | 显示配置信息 |
| `codecoder debug lsp` | LSP 调试信息 |
| `codecoder debug ripgrep` | Ripgrep 测试 |
| `codecoder debug file` | 文件读取测试 |
| `codecoder debug skill` | Skill 调试信息 |
| `codecoder debug snapshot` | 快照调试 |
| `codecoder debug agent` | Agent 调试信息 |
| `codecoder debug paths` | 显示全局路径 |
| `codecoder debug wait` | 无限等待（调试用） |

### 2.11 辅助命令

| 命令 | 功能描述 |
|------|----------|
| `codecoder completion` | 生成 Shell 自动补全脚本 |
| `codecoder --help` | 显示帮助信息 |
| `codecoder --version` | 显示版本号 |

---

## 3. Agent 系统

Agent 定义: `packages/ccode/src/agent/agent.ts`
Prompt 目录: `packages/ccode/src/agent/prompt/`

### 3.1 主模式 Agent

| Agent | 名称 | 功能描述 | 模式 |
|-------|------|----------|------|
| `build` | 构建模式 | 默认主模式，支持提问和计划进入 | primary |
| `plan` | 计划模式 | 计划编写模式，只能编辑计划文件 | primary |

### 3.2 逆向工程 Agent

| Agent | 名称 | 功能描述 | 特性 |
|-------|------|----------|------|
| `code-reverse` | 网站逆向 | 像素级网站复刻规划，分析技术栈、提取设计系统 | temperature: 0.3, color: cyan |
| `jar-code-reverse` | JAR 逆向 | Java 源码重建，分析框架库、提取类结构 | temperature: 0.3, color: magenta |

### 3.3 工程类 Agent

| Agent | 名称 | 功能描述 | 特性 |
|-------|------|----------|------|
| `general` | 通用 Agent | 研究复杂问题、执行多步骤任务 | 禁用 todoread/todowrite |
| `explore` | 探索 Agent | 快速代码库探索、文件模式搜索、关键词搜索 | 只读权限 |
| `code-reviewer` | 代码审查 | 全面代码质量审查，提供具体可行的反馈 | subagent |
| `security-reviewer` | 安全审查 | 代码安全漏洞分析和最佳实践检查 | subagent |
| `tdd-guide` | TDD 指导 | 强制测试驱动开发方法论 | subagent |
| `architect` | 架构师 | 系统架构设计、接口定义、模式建立 | subagent |

### 3.4 内容创作 Agent

| Agent | 名称 | 功能描述 | 特性 |
|-------|------|----------|------|
| `writer` | 写作 Agent | 长文写作专家（20k+ 字），大纲生成、章节写作、风格一致性 | temperature: 0.7 |
| `proofreader` | 校对 Agent | 长文校对专家，使用 PROOF 框架检查语法、拼写、风格等 | temperature: 0.3 |

### 3.5 祝融说系列 Agent (ZRS)

| Agent | 名称 | 功能描述 | 特性 |
|-------|------|----------|------|
| `observer` | 观察者 | 基于"祝融说"观察者理论分析问题，揭示可能性空间 | temperature: 0.7 |
| `decision` | 决策智慧师 | 基于 CLOSE 五维评估框架分析选择，保持可用余量 | temperature: 0.6 |
| `macro` | 宏观经济分析师 | 基于 18 章课程体系解读 GDP、货币政策等数据 | temperature: 0.5 |
| `trader` | 交易指南 | 超短线交易分析，情绪周期、模式识别、仓位管理 | temperature: 0.5 |
| `picker` | 选品专家 | 基于"爆品之眼"方法论，七宗罪选品法识别市场机会 | temperature: 0.6 |
| `miniproduct` | 极小产品教练 | 指导独立开发者 0-1 构建可盈利软件产品 | temperature: 0.6 |
| `ai-engineer` | AI 工程师导师 | Python 基础到 LLM 应用开发、RAG 系统、微调优化 | temperature: 0.5 |

### 3.6 工具辅助 Agent

| Agent | 名称 | 功能描述 | 特性 |
|-------|------|----------|------|
| `synton-assistant` | SYNTON-DB 助手 | 帮助理解和使用 LLM 记忆数据库、PaQL 查询、Graph-RAG | temperature: 0.5 |

### 3.7 系统隐藏 Agent

| Agent | 名称 | 功能描述 | 特性 |
|-------|------|----------|------|
| `compaction` | 压缩 Agent | 会话上下文压缩 | hidden: true |
| `title` | 标题生成 | 自动生成会话标题 | hidden: true, temperature: 0.5 |
| `summary` | 摘要生成 | 生成会话摘要 | hidden: true |

### 3.8 Agent 配置参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `name` | string | Agent 名称 |
| `description` | string | 使用场景描述 |
| `mode` | enum | `primary`/`subagent`/`all` |
| `prompt` | string | 系统提示词 |
| `model` | object | 指定模型 `{providerID, modelID}` |
| `temperature` | number | 生成温度 |
| `topP` | number | Top-P 采样 |
| `color` | string | 颜色标识（十六进制） |
| `hidden` | boolean | 是否隐藏 |
| `steps` | number | 最大迭代次数 |
| `permission` | Ruleset | 权限规则集 |
| `options` | object | 额外选项 |

---

## 4. 工具系统 (Tools)

工具定义: `packages/ccode/src/tool/`

### 4.1 文件操作工具

| 工具 | 文件 | 功能描述 | 关键参数 |
|------|------|----------|----------|
| `read` | `read.ts` | 读取文件内容（支持文本、图片、PDF） | `filePath`, `offset`, `limit` |
| `write` | `write.ts` | 写入文件内容 | `filePath`, `content` |
| `edit` | `edit.ts` | 精确字符串替换编辑 | `filePath`, `oldString`, `newString`, `replaceAll` |
| `multiedit` | `multiedit.ts` | 批量多文件编辑 | `edits[]` |

### 4.2 搜索工具

| 工具 | 文件 | 功能描述 | 关键参数 |
|------|------|----------|----------|
| `glob` | `glob.ts` | 文件模式匹配搜索 | `pattern`, `path` |
| `grep` | `grep.ts` | 基于 ripgrep 的内容搜索 | `pattern`, `path`, `glob`, `type`, `output_mode` |
| `codesearch` | `codesearch.ts` | 代码语义搜索 | `query` |
| `ls` | `ls.ts` | 目录列表 | `path` |

### 4.3 执行工具

| 工具 | 文件 | 功能描述 | 关键参数 |
|------|------|----------|----------|
| `bash` | `bash.ts` | Shell 命令执行 | `command`, `timeout`, `workdir`, `description` |
| `task` | `task.ts` | 子任务调度 | `prompt`, `agent`, `model` |

### 4.4 网络工具

| 工具 | 文件 | 功能描述 | 关键参数 |
|------|------|----------|----------|
| `webfetch` | `webfetch.ts` | Web 内容获取和处理 | `url`, `prompt` |
| `websearch` | `websearch.ts` | Web 搜索 | `query`, `allowed_domains`, `blocked_domains` |

### 4.5 交互工具

| 工具 | 文件 | 功能描述 | 关键参数 |
|------|------|----------|----------|
| `question` | `question.ts` | 用户交互问答 | `questions[]` |
| `plan` | `plan.ts` | 计划模式切换 | `enter`/`exit` |
| `skill` | `skill.ts` | 技能调用 | `skill`, `args` |

### 4.6 任务管理工具

| 工具 | 文件 | 功能描述 |
|------|------|----------|
| `todo` | `todo.ts` | 任务列表管理 |

### 4.7 其他工具

| 工具 | 文件 | 功能描述 |
|------|------|----------|
| `apply_patch` | `apply_patch.ts` | 应用补丁文件 |
| `batch` | `batch.ts` | 批量工具调用 |
| `lsp` | `lsp.ts` | LSP 诊断获取 |
| `network-analyzer` | `network-analyzer.ts` | 网络分析 |
| `truncation` | `truncation.ts` | 输出截断处理 |
| `registry` | `registry.ts` | 工具注册表 |

### 4.8 工具定义结构

```typescript
interface Tool.Info {
  id: string
  init: (ctx?: InitContext) => Promise<{
    description: string
    parameters: z.ZodType
    execute(args, ctx): Promise<{
      title: string
      metadata: object
      output: string
      attachments?: FilePart[]
    }>
  }>
}
```

---

## 5. AI Provider 系统

Provider 定义: `packages/ccode/src/provider/provider.ts`

### 5.1 内置 Provider（直接支持）

| Provider ID | 名称 | SDK 包 |
|-------------|------|--------|
| `anthropic` | Anthropic Claude | `@ai-sdk/anthropic` |
| `openai` | OpenAI | `@ai-sdk/openai` |
| `google` | Google Gemini | `@ai-sdk/google` |
| `google-vertex` | Google Vertex AI | `@ai-sdk/google-vertex` |
| `google-vertex-anthropic` | Vertex AI (Claude) | `@ai-sdk/google-vertex/anthropic` |
| `amazon-bedrock` | Amazon Bedrock | `@ai-sdk/amazon-bedrock` |
| `azure` | Azure OpenAI | `@ai-sdk/azure` |
| `azure-cognitive-services` | Azure Cognitive | `@ai-sdk/azure` |
| `github-copilot` | GitHub Copilot | `@ai-sdk/github-copilot` |
| `github-copilot-enterprise` | GitHub Copilot Enterprise | `@ai-sdk/github-copilot` |
| `xai` | xAI | `@ai-sdk/xai` |
| `mistral` | Mistral AI | `@ai-sdk/mistral` |
| `groq` | Groq | `@ai-sdk/groq` |
| `deepinfra` | DeepInfra | `@ai-sdk/deepinfra` |
| `cerebras` | Cerebras | `@ai-sdk/cerebras` |
| `cohere` | Cohere | `@ai-sdk/cohere` |
| `togetherai` | Together AI | `@ai-sdk/togetherai` |
| `perplexity` | Perplexity | `@ai-sdk/perplexity` |
| `openrouter` | OpenRouter | `@openrouter/ai-sdk-provider` |
| `vercel` | Vercel AI | `@ai-sdk/vercel` |
| `gitlab` | GitLab Duo | `@gitlab/gitlab-ai-provider` |
| `ccode` | CodeCoder 默认 | 内置 |

### 5.2 认证方式

| 认证类型 | 描述 | 支持的 Provider |
|----------|------|-----------------|
| `api` | API Key | 所有 Provider |
| `oauth` | OAuth 认证 | anthropic, openai, github-copilot, gitlab |
| `env` | 环境变量 | 所有 Provider |
| `wellknown` | Well-Known 配置 | 企业级部署 |

### 5.3 模型属性

```typescript
interface Model {
  id: string
  providerID: string
  name: string
  family?: string
  api: { id: string, url: string, npm: string }
  capabilities: {
    temperature: boolean
    reasoning: boolean
    attachment: boolean
    toolcall: boolean
    input: { text, audio, image, video, pdf: boolean }
    output: { text, audio, image, video, pdf: boolean }
    interleaved: boolean | { field: string }
  }
  cost: {
    input: number     // 每百万 token
    output: number
    cache: { read: number, write: number }
  }
  limit: {
    context: number
    input?: number
    output: number
  }
  status: 'alpha' | 'beta' | 'deprecated' | 'active'
}
```

### 5.4 Provider 配置示例

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "sk-..."
      }
    },
    "custom-provider": {
      "api": "https://api.example.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "models": {
        "custom-model": {
          "name": "Custom Model",
          "limit": { "context": 128000, "output": 8192 }
        }
      }
    }
  }
}
```

---

## 6. MCP (Model Context Protocol) 系统

MCP 定义: `packages/ccode/src/mcp/index.ts`

### 6.1 服务器类型

| 类型 | 配置字段 | 描述 |
|------|----------|------|
| `local` | `command`, `environment` | 本地进程 MCP 服务器 |
| `remote` | `url`, `headers`, `oauth` | 远程 HTTP/SSE MCP 服务器 |

### 6.2 本地服务器配置

```json
{
  "mcp": {
    "my-server": {
      "type": "local",
      "command": ["npx", "my-mcp-server"],
      "environment": {
        "API_KEY": "..."
      },
      "enabled": true,
      "timeout": 30000
    }
  }
}
```

### 6.3 远程服务器配置

```json
{
  "mcp": {
    "remote-server": {
      "type": "remote",
      "url": "https://mcp.example.com/sse",
      "headers": {
        "Authorization": "Bearer ..."
      },
      "oauth": {
        "clientId": "...",
        "clientSecret": "...",
        "scope": "read write"
      },
      "enabled": true,
      "timeout": 30000
    }
  }
}
```

### 6.4 OAuth 认证流程

1. **发现**: 自动发现 OAuth 元数据
2. **注册**: 动态客户端注册（RFC 7591）或使用预配置 clientId
3. **授权**: 浏览器跳转授权
4. **回调**: 本地服务器接收回调
5. **令牌**: 交换并存储令牌

### 6.5 MCP 状态

| 状态 | 描述 |
|------|------|
| `connected` | 已连接 |
| `disabled` | 已禁用 |
| `failed` | 连接失败 |
| `needs_auth` | 需要认证 |
| `needs_client_registration` | 需要客户端注册 |

### 6.6 MCP 功能

- **工具调用**: 调用 MCP 服务器提供的工具
- **Prompt 模板**: 使用 MCP 提供的 prompt 模板
- **资源访问**: 读取 MCP 资源

---

## 7. LSP (Language Server Protocol) 集成

LSP 定义: `packages/ccode/src/lsp/server.ts`

### 7.1 支持的语言服务器

| ID | 语言/框架 | 扩展名 | 自动安装 |
|----|----------|--------|----------|
| `typescript` | TypeScript/JavaScript | .ts, .tsx, .js, .jsx | 需要项目安装 |
| `deno` | Deno | .ts, .tsx, .js, .jsx | 需要系统安装 |
| `vue` | Vue.js | .vue | 自动安装 |
| `svelte` | Svelte | .svelte | 自动安装 |
| `astro` | Astro | .astro | 自动安装 |
| `eslint` | ESLint | .ts, .tsx, .js, .jsx | 自动构建 |
| `oxlint` | Oxlint | .ts, .tsx, .js, .jsx | 需要项目安装 |
| `biome` | Biome | .ts, .tsx, .js, .jsx, .json | 需要安装 |
| `gopls` | Go | .go | 自动安装 |
| `rust` | Rust | .rs | 需要系统安装 |
| `pyright` | Python | .py, .pyi | 自动安装 |
| `ty` | Python (Ty) | .py, .pyi | 实验性 |
| `ruby-lsp` | Ruby | .rb, .rake | 自动安装 |
| `elixir-ls` | Elixir | .ex, .exs | 自动安装 |
| `zls` | Zig | .zig, .zon | 自动安装 |
| `csharp` | C# | .cs | 自动安装 |
| `fsharp` | F# | .fs, .fsi | 自动安装 |
| `sourcekit-lsp` | Swift | .swift | 需要 Xcode |
| `clangd` | C/C++ | .c, .cpp, .h | 自动安装 |
| `jdtls` | Java | .java | 自动安装 |
| `kotlin-ls` | Kotlin | .kt, .kts | 自动安装 |
| `yaml-ls` | YAML | .yaml, .yml | 自动安装 |
| `lua-ls` | Lua | .lua | 自动安装 |
| `php intelephense` | PHP | .php | 自动安装 |
| `prisma` | Prisma | .prisma | 需要安装 |
| `dart` | Dart | .dart | 需要系统安装 |
| `ocaml-lsp` | OCaml | .ml, .mli | 需要安装 |
| `bash` | Bash/Shell | .sh, .bash, .zsh | 自动安装 |
| `terraform` | Terraform | .tf, .tfvars | 自动安装 |
| `texlab` | LaTeX | .tex, .bib | 自动安装 |
| `dockerfile` | Dockerfile | Dockerfile | 自动安装 |
| `gleam` | Gleam | .gleam | 需要安装 |
| `clojure-lsp` | Clojure | .clj, .cljs | 需要安装 |
| `nixd` | Nix | .nix | 需要安装 |
| `tinymist` | Typst | .typ | 自动安装 |
| `haskell-language-server` | Haskell | .hs, .lhs | 需要安装 |

### 7.2 LSP 配置

```json
{
  "lsp": {
    "typescript": {
      "disabled": false
    },
    "custom-lsp": {
      "command": ["my-lsp", "--stdio"],
      "extensions": [".custom"],
      "env": { "DEBUG": "1" },
      "initialization": { "customOption": true }
    }
  }
}
```

### 7.3 LSP 功能

- **诊断信息**: 编译错误、警告、提示
- **自动根目录检测**: 基于项目文件（package.json、go.mod 等）
- **多服务器支持**: 同时运行多个 LSP 服务器
- **自动安装**: 自动下载和安装缺失的 LSP 服务器

---

## 8. Session 会话系统

Session 定义: `packages/ccode/src/session/index.ts`

### 8.1 会话结构

```typescript
interface Session.Info {
  id: string           // 会话 ID
  slug: string         // URL 友好的短标识
  projectID: string    // 项目 ID
  directory: string    // 工作目录
  parentID?: string    // 父会话 ID（用于分支）
  title: string        // 会话标题
  version: string      // 版本号
  time: {
    created: number    // 创建时间
    updated: number    // 更新时间
    compacting?: number // 压缩时间
    archived?: number   // 归档时间
  }
  permission?: Ruleset  // 权限规则
  summary?: {
    additions: number
    deletions: number
    files: number
    diffs: FileDiff[]
  }
  revert?: {
    messageID: string
    partID?: string
    snapshot?: string
    diff?: string
  }
}
```

### 8.2 会话操作

| 操作 | 函数 | 描述 |
|------|------|------|
| 创建 | `Session.create()` | 创建新会话 |
| 获取 | `Session.get(id)` | 获取会话详情 |
| 更新 | `Session.update(id, editor)` | 更新会话 |
| 删除 | `Session.remove(id)` | 删除会话及其子会话 |
| 列表 | `Session.list()` | 列出所有会话 |
| 分支 | `Session.fork({ sessionID, messageID })` | 从指定消息分支 |
| 子会话 | `Session.children(parentID)` | 获取子会话 |

### 8.3 消息类型

| 类型 | 描述 |
|------|------|
| `user` | 用户消息 |
| `assistant` | 助手消息 |
| `system` | 系统消息 |

### 8.4 消息部分类型

| Part 类型 | 描述 |
|-----------|------|
| `text` | 文本内容 |
| `reasoning` | 推理过程 |
| `tool` | 工具调用 |
| `file` | 文件附件 |
| `step-start` | 步骤开始标记 |
| `step-finish` | 步骤结束标记 |

### 8.5 会话事件

| 事件 | 描述 |
|------|------|
| `session.created` | 会话创建 |
| `session.updated` | 会话更新 |
| `session.deleted` | 会话删除 |
| `session.diff` | 文件差异 |
| `session.error` | 会话错误 |
| `session.idle` | 会话空闲 |

---

## 9. 配置系统

配置定义: `packages/ccode/src/config/config.ts`

### 9.1 配置文件位置（优先级从低到高）

1. Well-Known 远程配置
2. 全局配置: `~/.config/codecoder/codecoder.json(c)`
3. `CCODE_CONFIG` 环境变量指定的配置
4. 项目配置: `./codecoder.json(c)`
5. `.codecoder/` 目录下的配置
6. `CCODE_CONFIG_CONTENT` 环境变量

### 9.2 主要配置项

```typescript
interface Config.Info {
  $schema?: string
  theme?: string                    // 主题名称
  keybinds?: Keybinds              // 快捷键配置
  logLevel?: Log.Level             // 日志级别
  tui?: TUI                        // TUI 设置
  server?: Server                  // 服务器配置
  command?: Record<string, Command> // 自定义命令
  model?: string                   // 默认模型 (provider/model)
  small_model?: string             // 小模型（用于标题生成等）
  default_agent?: string           // 默认 Agent
  username?: string                // 显示用户名
  agent?: Record<string, Agent>    // Agent 配置
  provider?: Record<string, Provider> // Provider 配置
  mcp?: Record<string, Mcp>        // MCP 配置
  lsp?: Record<string, LSP>        // LSP 配置
  formatter?: Record<string, Formatter> // 格式化器配置
  permission?: Permission          // 权限配置
  instructions?: string[]          // 额外指令文件
  disabled_providers?: string[]    // 禁用的 Provider
  enabled_providers?: string[]     // 仅启用的 Provider
  compaction?: {                   // 压缩设置
    auto?: boolean
    prune?: boolean
  }
  experimental?: {                 // 实验性功能
    batch_tool?: boolean
    openTelemetry?: boolean
    mcp_timeout?: number
    // ...
  }
}
```

### 9.3 快捷键配置

| 分类 | 示例快捷键 |
|------|------------|
| 应用控制 | `app_exit`, `editor_open`, `theme_list` |
| 会话管理 | `session_new`, `session_list`, `session_fork` |
| 消息滚动 | `messages_page_up`, `messages_page_down` |
| 模型操作 | `model_list`, `model_cycle_recent` |
| Agent 操作 | `agent_list`, `agent_cycle` |
| 输入编辑 | `input_clear`, `input_submit`, `input_newline` |

### 9.4 环境变量支持

配置文件中支持环境变量替换：

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

### 9.5 文件引用支持

```json
{
  "agent": {
    "custom": {
      "prompt": "{file:./prompts/custom.md}"
    }
  }
}
```

---

## 10. 记忆系统

记忆系统位置: `./memory/`

### 10.1 存储结构

| 层级 | 路径 | 类型 | 用途 |
|------|------|------|------|
| 流层（每日） | `./memory/daily/{YYYY-MM-DD}.md` | 追加日志 | 每日交互记录、决策、任务 |
| 沉积层（长期） | `./memory/MEMORY.md` | 结构化知识 | 用户偏好、项目上下文、关键决策 |

### 10.2 操作规则

| 操作 | 时机 | 行为 |
|------|------|------|
| 读取 | 会话初始化 | 加载 MEMORY.md + 当日/前日笔记 |
| 即时写入 | 重要交互后 | 追加到当日笔记（不可变） |
| 整合写入 | 检测到重要信息 | 更新 MEMORY.md（合并/替换） |

### 10.3 长期记忆分类

- `## 用户偏好`
- `## 项目上下文`
- `## 关键决策`
- `## 经验教训`

---

## 11. Skill 技能系统

Skill 定义: `packages/ccode/src/skill/skill.ts`

### 11.1 Skill 扫描路径

1. 内置 Skills: `packages/ccode/src/skill/builtin/*/SKILL.md`
2. 项目 Skills: `.claude/skills/**/SKILL.md`
3. 项目 Skills: `.codecoder/skills/**/SKILL.md`
4. 全局 Skills: `~/.claude/skills/**/SKILL.md`
5. 配置目录 Skills

### 11.2 Skill 定义格式

```markdown
---
name: my-skill
description: 技能描述
---

技能的详细提示词内容...
```

### 11.3 Skill 结构

```typescript
interface Skill.Info {
  name: string        // 技能名称
  description: string // 技能描述
  location: string    // 文件路径
}
```

### 11.4 内置 Skill

| Skill | 描述 |
|-------|------|
| `commit` | 创建规范的 Git 提交 |

---

## 12. Hook 钩子系统

Hook 定义: `packages/ccode/src/hook/hook.ts`

### 12.1 生命周期钩子

| 钩子 | 触发时机 |
|------|----------|
| `PreToolUse` | 工具执行前 |
| `PostToolUse` | 工具执行后 |
| `PreResponse` | 响应生成前 |
| `Stop` | 会话停止时 |

### 12.2 钩子动作类型

| 动作类型 | 描述 |
|----------|------|
| `scan` | 模式扫描（输入/输出） |
| `scan_content` | 文件内容扫描 |
| `check_env` | 环境变量检查 |
| `check_style` | 风格检查 |
| `notify_only` | 仅通知 |
| `run_command` | 运行命令 |
| `analyze_changes` | 分析变更 |
| `scan_files` | 扫描文件 |

### 12.3 钩子配置

```json
// .codecoder/hooks/hooks.json
{
  "hooks": {
    "PreToolUse": {
      "secret-scanner": {
        "pattern": "bash",
        "actions": [
          {
            "type": "scan",
            "patterns": ["API_KEY", "SECRET"],
            "message": "检测到敏感信息: {match}",
            "block": true
          }
        ]
      }
    }
  },
  "settings": {
    "enabled": true,
    "blocking_mode": "interactive"
  }
}
```

### 12.4 钩子上下文

```typescript
interface Hook.Context {
  tool?: string
  input?: Record<string, unknown>
  output?: string
  filePath?: string
  fileContent?: string
  command?: string
  sessionID?: string
  diff?: string
}
```

---

## 13. Permission 权限系统

Permission 定义: `packages/ccode/src/permission/next.ts`

### 13.1 权限动作

| 动作 | 描述 |
|------|------|
| `allow` | 允许 |
| `deny` | 拒绝 |
| `ask` | 询问用户 |

### 13.2 权限类型

| 权限 | 描述 |
|------|------|
| `read` | 文件读取 |
| `edit` | 文件编辑（含 write, patch, multiedit） |
| `glob` | 文件搜索 |
| `grep` | 内容搜索 |
| `list` | 目录列表 |
| `bash` | 命令执行 |
| `task` | 子任务 |
| `external_directory` | 外部目录访问 |
| `todowrite` | 任务写入 |
| `todoread` | 任务读取 |
| `question` | 用户提问 |
| `webfetch` | Web 获取 |
| `websearch` | Web 搜索 |
| `codesearch` | 代码搜索 |
| `lsp` | LSP 诊断 |
| `doom_loop` | 循环检测 |
| `plan_enter` | 进入计划模式 |
| `plan_exit` | 退出计划模式 |

### 13.3 权限配置示例

```json
{
  "permission": {
    "*": "allow",
    "bash": {
      "*": "ask",
      "git *": "allow",
      "rm -rf *": "deny"
    },
    "read": {
      "*": "allow",
      "*.env": "ask"
    },
    "edit": {
      "*": "allow",
      "*.lock": "deny"
    }
  }
}
```

### 13.4 权限规则评估

规则按顺序评估，后定义的规则覆盖前面的：

1. Agent 默认规则
2. 用户配置规则
3. 会话规则

---

## 14. TUI 终端界面

TUI 目录: `packages/ccode/src/cli/cmd/tui/`

### 14.1 技术栈

- **框架**: SolidJS
- **终端**: OpenTUI
- **样式**: TailwindCSS

### 14.2 主要组件

| 组件 | 功能 |
|------|------|
| `App` | 应用主入口 |
| `Thread` | 会话线程显示 |
| `Input` | 用户输入框 |
| `Messages` | 消息列表 |
| `Sidebar` | 侧边栏导航 |
| `StatusBar` | 状态栏 |
| `Modal` | 模态对话框 |
| `Toast` | 通知提示 |

### 14.3 TUI 配置

```json
{
  "tui": {
    "scroll_speed": 1.0,
    "scroll_acceleration": {
      "enabled": true
    },
    "diff_style": "auto"
  }
}
```

### 14.4 主要快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+C` / `Ctrl+D` | 退出 |
| `Ctrl+X, e` | 打开编辑器 |
| `Ctrl+X, m` | 模型列表 |
| `Ctrl+X, a` | Agent 列表 |
| `Ctrl+X, n` | 新建会话 |
| `Ctrl+X, l` | 会话列表 |
| `Escape` | 中断当前操作 |
| `Tab` | 切换 Agent |
| `Ctrl+T` | 切换模型变体 |

---

## 15. 共享工具库 (packages/util)

工具库目录: `packages/util/src/`

### 15.1 工具函数

| 模块 | 文件 | 主要导出 |
|------|------|----------|
| 数组操作 | `array.ts` | 数组工具函数 |
| 二进制操作 | `binary.ts` | 二进制数据处理 |
| 编码 | `encode.ts` | 编码/解码工具 |
| 错误处理 | `error.ts` | `NamedError` 错误类 |
| 函数工具 | `fn.ts` | `fn` 类型安全函数包装 |
| 标识符 | `identifier.ts` | ID 生成工具 |
| IIFE | `iife.ts` | 立即执行函数工具 |
| 路径 | `path.ts` | 路径处理工具 |
| 重试 | `retry.ts` | 重试逻辑 |
| Slug | `slug.ts` | URL 友好标识符生成 |
| 惰性求值 | `lazy.ts` | 惰性初始化 |

### 15.2 NamedError 使用

```typescript
import { NamedError } from "@codecoder-ai/util/error"

export const MyError = NamedError.create(
  "MyError",
  z.object({
    code: z.number(),
    message: z.string(),
  })
)

// 使用
throw new MyError({ code: 404, message: "Not found" })

// 检查
if (MyError.isInstance(error)) {
  console.log(error.data.code)
}
```

---

## 16. 脚本系统

脚本目录: `script/`

### 16.1 主要脚本

| 脚本 | 功能 |
|------|------|
| `generate.ts` | SDK 生成脚本 |

### 16.2 构建命令

```bash
# 安装依赖
bun install

# 开发运行
bun dev

# 构建可执行文件
bun run --cwd packages/ccode build

# 类型检查
bun turbo typecheck

# 运行测试
cd packages/ccode && bun test

# 重新生成 SDK
./script/generate.ts
```

---

## 附录

### A. 文件路径参考

| 模块 | 关键文件路径 |
|------|-------------|
| CLI 入口 | `packages/ccode/src/index.ts` |
| Agent 定义 | `packages/ccode/src/agent/agent.ts` |
| Tool 定义 | `packages/ccode/src/tool/*.ts` |
| Provider | `packages/ccode/src/provider/provider.ts` |
| MCP | `packages/ccode/src/mcp/index.ts` |
| LSP | `packages/ccode/src/lsp/server.ts` |
| Session | `packages/ccode/src/session/index.ts` |
| Config | `packages/ccode/src/config/config.ts` |
| Hook | `packages/ccode/src/hook/hook.ts` |
| Skill | `packages/ccode/src/skill/skill.ts` |
| Permission | `packages/ccode/src/permission/next.ts` |
| TUI | `packages/ccode/src/cli/cmd/tui/**/*.tsx` |
| 共享工具 | `packages/util/src/*.ts` |

### B. 环境变量

| 变量 | 描述 |
|------|------|
| `CCODE_CONFIG` | 自定义配置文件路径 |
| `CCODE_CONFIG_CONTENT` | 内联配置内容（JSON） |
| `CCODE_CONFIG_DIR` | 额外配置目录 |
| `CCODE_PERMISSION` | 内联权限配置（JSON） |
| `CCODE_DISABLE_PROJECT_CONFIG` | 禁用项目配置 |
| `CCODE_DISABLE_LSP_DOWNLOAD` | 禁用 LSP 自动下载 |
| `CCODE_DISABLE_AUTOCOMPACT` | 禁用自动压缩 |
| `CCODE_DISABLE_PRUNE` | 禁用输出修剪 |
| `CCODE_ENABLE_EXPERIMENTAL_MODELS` | 启用实验性模型 |

### C. 版本信息

- **运行时**: Bun 1.3+
- **构建**: Turborepo
- **前端**: Solid.js、OpenTUI、TailwindCSS
- **后端**: Hono（HTTP）

---

*文档生成时间: 2026-02-10*
