# CodeCoder 功能清单

**生成时间**: 2026-02-16
**版本**: 基于当前代码库分析

---

## 1. 核心界面模式

### 1.1 TUI 终端界面

- **功能描述**: 基于 SolidJS 和 OpenTUI 的交互式终端界面
- **关键文件**: `packages/ccode/src/cli/cmd/tui/`
- **子功能**:
  - 会话管理界面
  - 消息渲染与滚动
  - 键盘快捷键系统
  - 剪贴板操作
  - 外部编辑器集成
  - Spinner 加载状态

### 1.2 CLI 命令行模式

- **功能描述**: 通过命令行参数直接执行任务
- **关键文件**: `packages/ccode/src/cli/cmd/`
- **使用方式**: `bun dev <path>` 或 `ccode <path>`

### 1.3 HTTP API 服务器

- **功能描述**: 无头模式 HTTP API 服务，支持远程访问
- **关键文件**: `packages/ccode/src/api/server/`
- **使用方式**: `bun dev serve --port 8080`
- **API 端点**:
  - `/api/sessions` - 会话管理
  - `/api/config` - 配置管理
  - `/api/permissions` - 权限管理
  - `/api/files` - 文件搜索
  - `/api/events` - SSE 事件流
  - `/api/agents` - Agent 调用
  - `/api/v1/tasks` - 异步任务流

---

## 2. Agent 系统

### 2.1 主模式 Agent (Primary)

| Agent | 描述 | 特性 |
|-------|------|------|
| `build` | 默认开发模式 | 完整工具集，支持提问和计划模式 |
| `plan` | 计划模式 | 只读编辑受限文件，用于设计实现方案 |
| `writer` | 长文写作 | 支持 20k+ 字数，章节管理，风格一致性 |
| `autonomous` | 自主模式 | 完全自主执行，CLOSE 决策框架 |

### 2.2 工程类 Agent (Subagent)

| Agent | 描述 | 用途 |
|-------|------|------|
| `general` | 通用代理 | 复杂多步任务，并行执行 |
| `explore` | 探索代理 | 代码库搜索，快速定位 |
| `code-reviewer` | 代码审查 | 质量检查，可操作建议 |
| `security-reviewer` | 安全审查 | 漏洞检测，最佳实践 |
| `tdd-guide` | TDD 引导 | 测试驱动开发，覆盖率验证 |
| `architect` | 架构设计 | 系统设计，接口定义 |
| `verifier` | 验证代理 | 构建检查，类型检查，测试执行 |

### 2.3 内容创作 Agent

| Agent | 描述 | 特性 |
|-------|------|------|
| `proofreader` | 校对代理 | PROOF 框架，语法/风格检查 |
| `expander` | 内容扩展 | 创意扩展，框架构建 |
| `expander-fiction` | 小说扩展 | 世界观/角色/叙事结构 |
| `expander-nonfiction` | 非虚构扩展 | 论证/证据框架 |

### 2.4 逆向工程 Agent

| Agent | 描述 | 用途 |
|-------|------|------|
| `code-reverse` | 网站逆向 | 像素级复刻规划，技术栈识别 |
| `jar-code-reverse` | JAR 逆向 | Java 源码重建，框架识别 |

### 2.5 祝融说系列 Agent (ZRS)

| Agent | 描述 | 哲学基础 |
|-------|------|----------|
| `observer` | 观察者 | 可能性基底，观察收敛理论 |
| `decision` | 决策师 | CLOSE 五维评估框架 |
| `macro` | 宏观分析 | 18 章课程体系，经济数据解读 |
| `trader` | 交易指南 | 情绪周期，模式识别 |
| `picker` | 选品专家 | 爆品之眼，七宗罪选品法 |
| `miniproduct` | 极小产品 | 独立开发者，0 到 1 构建 |
| `ai-engineer` | AI 工程师 | LLM 应用，RAG 系统，微调 |
| `synton-assistant` | SYNTON-DB | 张量图存储，PaQL 查询 |

### 2.6 系统隐藏 Agent

| Agent | 用途 |
|-------|------|
| `compaction` | 上下文压缩 |
| `title` | 会话标题生成 |
| `summary` | 会话摘要生成 |

---

## 3. AI 提供商集成

### 3.1 内置提供商

| 提供商 | SDK | 认证方式 |
|--------|-----|----------|
| Anthropic | `@ai-sdk/anthropic` | API Key / OAuth (Claude Max) |
| OpenAI | `@ai-sdk/openai` | API Key / OAuth (ChatGPT Plus/Pro) |
| Google Gemini | `@ai-sdk/google` | API Key |
| Google Vertex AI | `@ai-sdk/google-vertex` | GCP 凭证 |
| Azure OpenAI | `@ai-sdk/azure` | Azure 认证 |
| Amazon Bedrock | `@ai-sdk/amazon-bedrock` | AWS 凭证链 |
| GitHub Copilot | `@ai-sdk/github-copilot` | OAuth |
| OpenRouter | `@openrouter/ai-sdk-provider` | API Key |
| xAI | `@ai-sdk/xai` | API Key |
| Mistral | `@ai-sdk/mistral` | API Key |
| Groq | `@ai-sdk/groq` | API Key |
| DeepInfra | `@ai-sdk/deepinfra` | API Key |
| Cerebras | `@ai-sdk/cerebras` | API Key |
| Cohere | `@ai-sdk/cohere` | API Key |
| TogetherAI | `@ai-sdk/togetherai` | API Key |
| Perplexity | `@ai-sdk/perplexity` | API Key |
| Vercel AI | `@ai-sdk/vercel` | API Key |
| Cloudflare AI Gateway | 自定义 | API Token |

### 3.2 提供商特性

- **自动发现**: 根据环境变量自动检测可用提供商
- **模型白名单/黑名单**: 配置文件限制可用模型
- **自定义提供商**: 支持通过配置添加自定义提供商
- **超时控制**: 每个提供商可配置请求超时
- **变体支持**: 模型可配置多个变体（如不同参数配置）

---

## 4. 工具系统

### 4.1 文件操作工具

| 工具 | 文件 | 功能 |
|------|------|------|
| `read` | `tool/read.ts` | 读取文件内容 |
| `write` | `tool/write.ts` | 写入文件 |
| `edit` | `tool/edit.ts` | 精确字符串替换 |
| `multiedit` | `tool/multiedit.ts` | 多处编辑 |
| `glob` | `tool/glob.ts` | 文件模式匹配 |
| `grep` | `tool/grep.ts` | 内容搜索 (ripgrep) |
| `ls` | `tool/ls.ts` | 目录列表 |

### 4.2 执行工具

| 工具 | 文件 | 功能 |
|------|------|------|
| `bash` | `tool/bash.ts` | Shell 命令执行 |
| `task` | `tool/task.ts` | 子代理任务 |

### 4.3 Web 工具

| 工具 | 文件 | 功能 |
|------|------|------|
| `webfetch` | `tool/webfetch.ts` | 网页内容获取 |
| `websearch` | `tool/websearch.ts` | 网络搜索 |
| `codesearch` | `tool/codesearch.ts` | 代码搜索 |
| `network-analyzer` | `tool/network-analyzer.ts` | 网络分析 |

### 4.4 辅助工具

| 工具 | 文件 | 功能 |
|------|------|------|
| `question` | `tool/question.ts` | 向用户提问 |
| `plan` | `tool/plan.ts` | 计划模式控制 |
| `todo` | `tool/todo.ts` | 任务列表管理 |
| `skill` | `tool/skill.ts` | 技能调用 |
| `lsp` | `tool/lsp.ts` | LSP 集成工具 |
| `apply_patch` | `tool/apply_patch.ts` | 应用补丁 |
| `batch` | `tool/batch.ts` | 批量操作 |
| `truncation` | `tool/truncation.ts` | 内容截断处理 |

---

## 5. MCP (Model Context Protocol) 集成

### 5.1 MCP 配置

- **本地 MCP 服务器**: 通过命令启动本地进程
- **远程 MCP 服务器**: 通过 HTTP/SSE 连接
- **OAuth 认证**: 支持 OAuth 2.0 流程
- **超时配置**: 可配置请求超时

### 5.2 关键文件

| 文件 | 功能 |
|------|------|
| `mcp/index.ts` | MCP 管理器 |
| `mcp/auth.ts` | 认证管理 |
| `mcp/oauth-callback.ts` | OAuth 回调 |
| `mcp/oauth-provider.ts` | OAuth 提供商 |

---

## 6. 配置系统

### 6.1 配置文件层级 (优先级从低到高)

1. 全局配置: `~/.config/codecoder/codecoder.jsonc`
2. 用户 home: `~/.ccode/config.jsonc`
3. 项目配置: `.ccode/config.jsonc`
4. 环境变量: `CCODE_CONFIG_CONTENT`

### 6.2 主要配置项

| 配置项 | 描述 |
|--------|------|
| `model` | 默认模型 (provider/model 格式) |
| `small_model` | 小模型 (标题生成等) |
| `default_agent` | 默认 Agent |
| `provider` | 提供商配置 |
| `mcp` | MCP 服务器配置 |
| `agent` | Agent 配置 |
| `command` | 自定义命令 |
| `keybinds` | 键绑定配置 |
| `permission` | 权限配置 |
| `compaction` | 压缩配置 |
| `lsp` | LSP 服务器配置 |
| `formatter` | 格式化器配置 |
| `tui` | TUI 设置 |
| `server` | API 服务器设置 |

### 6.3 键绑定配置

支持 70+ 键绑定配置，包括：

- 应用控制 (退出、挂起)
- 会话操作 (新建、列表、导出、分叉)
- 消息导航 (滚动、翻页)
- 模型切换
- Agent 切换
- 输入编辑

---

## 7. 存储系统

### 7.1 存储架构

- **位置**: `~/.config/codecoder/storage/`
- **格式**: JSON 文件
- **锁机制**: 读写锁保护并发访问

### 7.2 数据类型

| 目录 | 内容 |
|------|------|
| `project/` | 项目元数据 |
| `session/` | 会话信息 |
| `message/` | 消息内容 |
| `part/` | 消息部分 |
| `_backup/` | 备份文件 |
| `_corrupted/` | 损坏文件隔离 |

### 7.3 数据完整性

- **自动备份**: 写入前创建备份
- **自动恢复**: JSON 解析失败时从备份恢复
- **损坏隔离**: 损坏文件移至 `_corrupted`
- **健康检查**: `healthCheck()` 函数验证数据完整性

---

## 8. 会话管理

### 8.1 会话功能

| 文件 | 功能 |
|------|------|
| `session/index.ts` | 会话核心 |
| `session/message.ts` | 消息处理 |
| `session/message-v2.ts` | 消息 V2 格式 |
| `session/compaction.ts` | 上下文压缩 |
| `session/summary.ts` | 摘要生成 |
| `session/system.ts` | 系统提示 |
| `session/todo.ts` | TODO 管理 |
| `session/llm.ts` | LLM 调用 |
| `session/processor.ts` | 消息处理器 |
| `session/prompt.ts` | 提示构建 |
| `session/retry.ts` | 重试逻辑 |
| `session/revert.ts` | 回滚操作 |
| `session/status.ts` | 状态管理 |

### 8.2 会话操作

- 创建/删除会话
- 消息发送/接收
- 会话分叉
- 上下文压缩
- 消息撤销/重做
- 会话导出

---

## 9. 自主模式 (Autonomous Mode)

### 9.1 自主级别

| 级别 | 中文 | 描述 |
|------|------|------|
| `lunatic` | 完全自主 | 无任何人工干预 |
| `insane` | 高度自主 | 几乎不需要干预 |
| `crazy` | 显著自主 | 偶需帮助 (默认) |
| `wild` | 部分自主 | 需定期确认 |
| `bold` | 谨慎自主 | 频繁暂停 |
| `timid` | 几乎无法自主 | 需持续监督 |

### 9.2 资源限制

- `maxTokens`: 最大 token 消耗
- `maxCostUSD`: 最大成本 (USD)
- `maxDurationMinutes`: 最大时长
- `maxFilesChanged`: 最大修改文件数
- `maxActions`: 最大操作数

### 9.3 自主模式架构

| 文件 | 功能 |
|------|------|
| `autonomous/agent/` | 自主代理 |
| `autonomous/config/` | 配置模式 |
| `autonomous/decision/` | 决策引擎 |
| `autonomous/execution/` | 执行器 |
| `autonomous/integration/` | 集成 |
| `autonomous/metrics/` | 指标评分 |
| `autonomous/orchestration/` | 编排器 |
| `autonomous/safety/` | 安全护栏 |
| `autonomous/state/` | 状态机 |

### 9.4 使用方式

```bash
bun dev autonomous "实现用户登录功能" --autonomy-level crazy --max-tokens 500000 --max-cost 5.0
```

---

## 10. 文档/长文写作系统

### 10.1 核心功能

| 文件 | 功能 |
|------|------|
| `document/writer.ts` | 写作提示生成 |
| `document/proofreader.ts` | 校对功能 |
| `document/editor.ts` | 文档编辑 |
| `document/entity.ts` | 实体管理 |
| `document/schema.ts` | 文档结构 |
| `document/summary.ts` | 摘要处理 |
| `document/templates.ts` | 模板系统 |
| `document/version.ts` | 版本管理 |
| `document/volume.ts` | 卷管理 |

### 10.2 写作流程

1. 生成文档大纲
2. 章节分配和规划
3. 上下文智能选择
4. 章节写作
5. 摘要提取
6. 一致性验证

---

## 11. LSP 集成

### 11.1 支持的语言服务器

- TypeScript/JavaScript
- Python
- Rust
- Go
- 更多可通过配置添加

### 11.2 功能

- 代码补全
- 定义跳转
- 引用查找
- 诊断信息
- 代码操作

---

## 12. 记忆系统

### 12.1 双层记忆架构

| 层级 | 路径 | 用途 |
|------|------|------|
| 每日笔记 | `memory/daily/{YYYY-MM-DD}.md` | 流动上下文日志 |
| 长期记忆 | `memory/MEMORY.md` | 沉积知识和偏好 |

### 12.2 操作规则

- **读取**: 加载 MEMORY.md + 当日笔记
- **写入**: 追加到每日笔记，合并到 MEMORY.md
- **透明性**: 纯 Markdown，Git 友好

---

## 13. 钩子系统 (Hooks)

### 13.1 钩子类型

| 类型 | 触发时机 |
|------|----------|
| `file_edited` | 文件编辑后 |
| `session_completed` | 会话完成后 |

### 13.2 配置示例

```jsonc
{
  "experimental": {
    "hook": {
      "file_edited": {
        "*.ts": [
          {
            "command": ["prettier", "--write", "{file}"]
          }
        ]
      }
    }
  }
}
```

---

## 14. 权限系统

### 14.1 权限类型

- `read` - 读取文件
- `edit` - 编辑文件
- `glob` - 文件匹配
- `grep` - 内容搜索
- `bash` - Shell 执行
- `task` - 子任务
- `external_directory` - 外部目录访问
- `webfetch` - 网页获取
- `websearch` - 网络搜索
- `codesearch` - 代码搜索
- `lsp` - LSP 工具
- `question` - 用户提问
- `doom_loop` - 循环检测

### 14.2 权限动作

- `allow` - 允许
- `deny` - 拒绝
- `ask` - 询问用户

---

## 15. ZeroBot 集成

### 15.1 配置选项

| 分类 | 选项 |
|------|------|
| 可观测性 | 日志/Prometheus/OpenTelemetry |
| 自主级别 | readonly/supervised/full |
| 运行时 | native/docker/cloudflare |
| 可靠性 | 重试/回退/限流 |
| 心跳 | 健康检查间隔 |
| 记忆 | SQLite/Markdown/无 |
| 网关 | 端口/主机/配对 |
| 隧道 | Cloudflare/Tailscale/ngrok |
| 渠道 | CLI/Telegram/Discord/Slack/WhatsApp |

---

## 16. CLI 命令

### 16.1 主要命令

| 命令 | 描述 |
|------|------|
| `(default)` | 启动 TUI |
| `serve` | 启动 HTTP API 服务器 |
| `autonomous` | 自主模式执行 |
| `session` | 会话管理 |
| `models` | 模型列表 |
| `mcp` | MCP 管理 |
| `memory` | 记忆管理 |
| `document` | 文档/写作 |
| `reverse` | 网站逆向 |
| `jar-reverse` | JAR 逆向 |
| `book-writer` | 书籍写作 |
| `get-started` | 入门引导 |

### 16.2 调试命令

| 命令 | 描述 |
|------|------|
| `debug agent` | Agent 调试 |
| `debug config` | 配置调试 |
| `debug file` | 文件调试 |
| `debug lsp` | LSP 调试 |
| `debug ripgrep` | Ripgrep 调试 |
| `debug skill` | 技能调试 |
| `debug snapshot` | 快照调试 |

---

## 17. 事件总线

### 17.1 架构

- **全局单例**: `Bus.publish()` / `Bus.subscribe()`
- **类型安全**: 使用 Zod schema 定义事件

### 17.2 事件类型

- 会话事件
- 工具调用事件
- 自主模式事件
- 错误事件

---

## 18. 可观测性

### 18.1 日志系统

- 结构化 JSON 日志
- 多级别: debug/info/warn/error
- 服务标识

### 18.2 追踪

- OpenTelemetry 集成 (可选)
- API 调用追踪
- 性能指标

---

## 附录: 项目结构

```
packages/ccode/src/
├── agent/          # Agent 定义和提示
├── api/            # HTTP API 服务器
├── autonomous/     # 自主模式
├── bus/            # 事件总线
├── cli/            # CLI 入口和命令
├── command/        # 命令系统
├── config/         # 配置管理
├── context/        # 上下文管理
├── document/       # 文档/写作系统
├── env/            # 环境变量
├── file/           # 文件操作
├── flag/           # 功能标志
├── format/         # 格式化器
├── global/         # 全局状态
├── hook/           # 钩子系统
├── id/             # ID 生成
├── lsp/            # LSP 集成
├── mcp/            # MCP 集成
├── memory-markdown/# Markdown 记忆
├── permission/     # 权限系统
├── project/        # 项目管理
├── provider/       # AI 提供商
├── session/        # 会话管理
├── storage/        # 存储系统
├── tool/           # 工具定义
└── util/           # 工具函数
```

---

*此文档由 Claude Code 自动生成*
