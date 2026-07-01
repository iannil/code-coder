# CodeCoder

> 自主 AI agent 系统 — 事件驱动，文件系统即自我

基于 AI 大模型、用 Rust 编写的高度自主 agent。它能读/写文件、搜索网络、搜索 GitHub、逆向 API、在沙箱中运行代码，还能用工具来扩展自身能力。

## 快速开始

```bash
# 1. 设置 API key
export CODECODER_API_KEY=sk-your-key-here
export CODECODER_MODEL=gpt-4o  # 默认 gpt-4o

# 2. 启动
cargo run

# 3. 在 REPL 中试试
cc> 列出当前目录的文件
cc> 搜索 GitHub 上的 Rust Web 框架
cc> /help      # 查看所有命令
cc> /exit      # 退出
```

## 无需 API key 运行

不设置 API key 时使用 StubClient（模拟 LLM 响应），可用于测试：

```bash
cargo run
```

## 内置工具（20 个）

| 工具 | 功能 |
|------|------|
| `read_file` | 读取文件内容 |
| `write_file` | 写入文件（自动创建父目录） |
| `run_command` | 执行 shell 命令 |
| `list_directory` | 列出目录内容 |
| `search_web` | 抓取 URL 内容 |
| `search_github` | 搜索 GitHub 仓库 (`repos:`) 或代码 (`code:`) |
| `reverse_api` | 抓取文档页面，提取 API endpoint 签名 |
| `generate_skill` | 生成 skill 文件到 `skills/` |
| `generate_prompt` | 生成 prompt 模板 |
| `generate_tool` | 生成可执行脚本（自动 chmod +x） |
| `run_in_sandbox` | 在沙箱中运行代码（自动选择 WASM L1 / Docker L2） |
| `glob` | 文件 glob 搜索（支持 ** 递归） |
| `grep` | 文本搜索 + AST 查询（tree-sitter） |
| `diff` | 文件差异比较 |
| `edit_file` | 精确文本替换编辑 |
| `commit` | Git 提交 |
| `review` | 代码审查 |
| `plan` | 任务计划 |
| `ask_user` | 用户交互 |
| `agent` | 子代理调用 |

## 文件系统即自我

```
project/
├── AGENTS.md     ← 系统身份声明 → 自动注入 LLM system prompt
├── CONTEXT.md    ← 项目术语表
├── skills/       ← .md 文件自动注册为技能
├── memory/       ← 系统持久化的 key-value 记忆
├── sessions/     ← 对话历史 JSON 文件
├── docs/         ← 设计文档、ADR、审计报告
│   ├── adr/      ← 架构决策记录
│   ├── audit/    ← TUI 保真度审计
│   └── design/   ← 设计规格
├── archived/     ← 参考项目存档
└── target/       ← 编译产物
```

## REPL 命令

```
/exit, /quit    退出 REPL
/help           显示帮助
/reload         重载 context 和 skills
/clear          清除对话历史
/history        显示历史消息数
/tools          列出可用工具
/skills         列出已加载技能
/memory         列出持久化记忆
```

## 沙箱系统

自动分级选择：

| 等级 | 方式 | 适用语言 |
|------|------|---------|
| L0 | 纯数据 | 阅读文档、API 提取 |
| L1 | WASM | Rust, Go, C, C++, WAT |
| L2 | Docker | Python, JS, Ruby, 及其他 |

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `CODECODER_API_KEY` | — | LLM API key（必需） |
| `CODECODER_MODEL` | `gpt-4o` | 模型名称 |
| `CODECODER_API_BASE` | `https://api.openai.com/v1` | API 端点 |
| `CODECODER_MAX_TOKENS` | `4096` | 最大 token 数 |
| `CODECODER_TEMPERATURE` | `0.7` | 温度参数 |
| `CODECODER_ROOT` | 当前目录 | 项目根目录 |
| `GITHUB_TOKEN` | — | GitHub API token（提升 rate limit） |

## 开发

```bash
cargo build      # 编译
cargo test       # 运行 696 个测试
cargo run        # 启动 TUI / REPL
```

## 架构

参考 `docs/` 目录下的 ADR 和设计文档。
