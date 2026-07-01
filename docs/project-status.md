# CodeCoder 项目状态报告

> **生成时间**: 2026-07-01
> **用途**: 为 LLM agent 提供完整的项目全局视图，以便理解上下文并执行后续迭代

---

## 1. 项目概览

CodeCoder 是一个基于 AI 大模型、用 Rust 编写的高度自主 agent 系统。核心设计理念是 **事件驱动架构** + **文件系统即自我**。

| 属性 | 值 |
|------|-----|
| 语言 | Rust (edition 2024) |
| 总源码 | 20,450 行（51 个 `.rs` 文件） |
| 测试 | 696 tests, 0 failed (560 个断言) |
| 编译 | ✅ 通过（37 个 warning: dead code / unused） |
| 模块数 | 16+ 核心模块 |
| 内置工具 | 20 个 |
| 外部依赖 | 20+ crates |
| 架构决策 | 7 ADRs（`docs/adr/`） |
| 状态 | **Phase 1-6 完成 ✅，Phase 7 已编码待集成 🔧** |

### 关键组件

```
src/
├── main.rs          # 入口（351 行）CLI 解析 + TUI/REPL 启动
├── agent.rs         # Agent 循环（1,206 行）感知-思考-行动
├── autonomous.rs    # 自主模式（353 行）定时器/调度器
├── event.rs         # 事件总线（181 行）
├── config.rs        # 配置管理（447 行）
├── session.rs       # 会话持久化（658 行）
├── self_evolve.rs   # Phase 7 自进化（736 行）
├── context.rs       # 上下文（74 行）
├── repl.rs          # REPL 模式（318 行）
├── llm/mod.rs       # LLM 客户端（665 行）
├── skill/mod.rs     # 技能系统（553 行）
├── memory/mod.rs    # 记忆存储（180 行）
├── mcp/             # MCP 协议（3 文件, 936 行）
├── sandbox/         # 分级沙箱（3 文件, 679 行）
├── permission/      # 权限引擎（3 文件, 346 行）
├── tools/           # 工具系统（20 文件, ~5,000 行）
└── tui/             # TUI 界面（11 文件, 7,735 行）
```

---

## 2. Phase 进度详情

### Phase 1: Event Bus + Agent Loop ✅

**文件**: `src/event.rs`, `src/agent.rs`, `src/autonomous.rs`

| 组件 | 状态 | 说明 |
|------|------|------|
| EventBus trait | ✅ | 统一事件路由接口 |
| 事件类型 | ✅ | System、User、Assistant、ToolCall、ToolResult、Timer、FileChange |
| AgentLoop | ✅ | 感知-思考-行动循环，支持同步/异步模式 |
| 自主调度器 | ✅ | 定时器事件驱动，`Scheduler` 结构管理间隔任务 |

**关键结构** (`src/agent.rs`):
```rust
pub struct AgentLoop {
    llm: Box<dyn LlmClient>,
    tools: ToolRegistry,
    skills: SkillRegistry,
    self_evolve: Option<SelfEvolve>,
    session: SessionStore,
    // ...
}
```

### Phase 2: LLM 客户端 ✅

**文件**: `src/llm/mod.rs` (665 行)

| 客户端 | 说明 |
|--------|------|
| `OpenAiClient` | 真正的 LLM 调用（支持 streaming） |
| `StubClient` | 模拟 LLM 响应（无 API key 时用于测试） |

**配置**:
- 环境变量: `CODECODER_API_KEY`, `CODECODER_MODEL`（默认 gpt-4o）, `CODECODER_API_BASE`, `CODECODER_MAX_TOKENS`, `CODECODER_TEMPERATURE`
- 支持 streaming response（`StreamingLLmClient`）

### Phase 3: 工具系统 ✅

**文件**: `src/tools/` (20 个工具, ~5,000 行)

| # | 工具 | 文件 | 行数 | 功能 |
|---|------|------|------|------|
| 1 | `ReadFile` | `tools/read_file.rs` | 50 | 读取文件内容 |
| 2 | `WriteFile` | `tools/write_file.rs` | 108 | 写入文件 |
| 3 | `RunCommand` | `tools/run_command.rs` | 129 | 执行 shell 命令 |
| 4 | `SearchWeb` | `tools/search_web.rs` | 161 | HTTP 获取 URL |
| 5 | `ListDir` | `tools/list_dir.rs` | 61 | 列出目录 |
| 6 | `GenerateSkill` | `tools/generate.rs` | 325 | 生成 skill 文件 |
| 7 | `GeneratePrompt` | `tools/generate.rs` | 325 | 生成 prompt 模板 |
| 8 | `GenerateTool` | `tools/generate.rs` | 325 | 生成可执行脚本 |
| 9 | `SearchGitHub` | `tools/search_github.rs` | 238 | GitHub API 搜索 |
| 10 | `ReverseApi` | `tools/reverse_api.rs` | 304 | API 文档 → endpoint 提取 |
| 11 | `GlobTool` | `tools/glob.rs` | 103 | 文件 glob 搜索 |
| 12 | `Grep` | `tools/grep.rs` | 977 | 文本搜索 + AST 查询 |
| 13 | `TodoTool` | `tools/todo.rs` | 225 | Todo 管理 |
| 14 | `DiffTool` | `tools/diff.rs` | 239 | 文件差异比较 |
| 15 | `EditFileTool` | `tools/edit_file.rs` | 156 | 精确文本替换编辑 |
| 16 | `CommitTool` | `tools/commit.rs` | 146 | Git 提交 |
| 17 | `ReviewTool` | `tools/review.rs` | 227 | 代码审查 |
| 18 | `PlanTool` | `tools/plan.rs` | 186 | 任务计划 |
| 19 | `AskUserTool` | `tools/ask_agent.rs` | 270 | 用户交互 |
| 20 | `RunInSandbox` | `tools/run_in_sandbox.rs` | 98 | 沙箱执行 |

**MCP 扩展**: `src/mcp/` (3 文件, 936 行)
- protocol.rs: MCP JSON-RPC 消息定义
- transport.rs: stdio transport
- mod.rs: 注册、发现、调用

### Phase 4: 分级沙箱 ✅

**文件**: `src/sandbox/` (3 文件, 679 行)

| 等级 | 实现 | 状态 | 适用场景 |
|------|------|------|---------|
| L0 | 纯数据处理 | ✅ | 阅读文档、API 提取 |
| L1 | WASM (wasmtime) | ✅ | Rust, Go, C, C++, WAT |
| L2 | Docker | ✅ | Python, JS, Ruby 等 |

- WASM 运行器基于 wasmtime
- Docker 运行器使用 `docker exec` 命令
- 自动降级: 如果 WASM 不可用，回退到 Docker

### Phase 5: TUI 终端界面 ✅

**文件**: `src/tui/` (11 文件, 7,735 行)

基于 ratatui 的终端界面，是项目中最大的模块。

| 文件 | 行数 | 职责 |
|------|------|------|
| `mod.rs` | 794 | TUI 主循环 + 事件分发 |
| `app.rs` | 593 | `TuiApp` 状态机（Mode、Dialog、Context） |
| `commands.rs` | 1,304 | 斜杠命令分发 + 会话 CRUD |
| `dialogs.rs` | 1,661 | 各类对话框（权限/计划/确认/帮助/模型选择） |
| `input_area.rs` | 1,575 | 多行输入框（undo/redo/kill-ring） |
| `message_list.rs` | 611 | 消息列表渲染 + 虚拟滚动 + 搜索 |
| `markdown.rs` | 904 | Markdown 渲染 + diff 渲染 |
| `diff.rs` | 529 | Diff 渲染管线（gutter + 语法高亮） |
| `status_bar.rs` | 323 | 状态栏（模式/模型/上下文量/忙闲） |
| `theme.rs` | 130 | 主题（dark/light 双主题） |
| `completion.rs` | 131 | 斜杠命令 + 文件路径补全 |

**状态机 Mode**:
```
INSERT → (正常输入)
SEARCH → (Ctrl+F 搜索)
R-SEARCH → (Ctrl+R 反向搜索)
DIALOG → (权限/计划/确认/提问覆盖层)
HELP → (帮助窗口)
MODEL → (模型选择弹窗)
SLASH → (斜杠命令补全)
BROWSE → (浏览消息历史)
```

### Phase 6: REPL + 会话持久化 ✅

**文件**: `src/repl.rs` (318 行), `src/session.rs` (658 行)

- REPL 模式: 非 TUI 的命令行交互界面
- 会话持久化: 对话保存到 `sessions/` 目录的 JSON 文件
- Schema 版本迁移: `schema_version: 1`，支持向后兼容
- 会话命令: `/resume`, `/sessions`, `/clear`

### Phase 7: 自我进化 🔧 (代码已实现，待完全集成)

**文件**: `src/self_evolve.rs` (736 行)
**设计文档**: `docs/design/phase-7-self-evolution.md`

核心能力:
1. **失败检测**: LLM 自我声明 → 工具错误累积 → 用户反馈（三级升级）
2. **Gap 分析**: 检查 `skills/` 目录，识别能力缺口
3. **Skill 生成**: 自动写 markdown skill 文件补齐缺口
4. **自省循环**: 在 agent loop 中周期性检查

**集成状态**:
- `AgentLoop` 已包含 `self_evolve: Option<SelfEvolve>` 字段
- 自省循环逻辑已实现但**默认未启用**（Option 为 None）
- 需要在启动时或配置中启用 `self_evolve`

### Phase 8+: 待设计 📋

| 项目 | 说明 |
|------|------|
| Rust 工具生成 | 动态编译 Rust 工具 |
| 文件系统 watcher | notify crate 已依赖，待集成 |
| 跨会话学习 | 跨 session 的知识积累 |

---

## 3. 编译状态

```bash
cargo check     # ✅ 通过，37 warnings
cargo build     # ✅ 通过
cargo test      # ✅ 696 passed, 0 failed
cargo doc       # ✅ 541 页文档
```

### 警告分类（37 个）

| 类别 | 数量 | 说明 |
|------|------|------|
| `unused import` | 3 | 两处 `Color` 导入未被使用 |
| `never constructed` | 3 | `ToolCall`/`ToolResult` 变体、`ListToolsParams` 结构体 |
| `never read` | 10+ | 各结构体字段（如 `jsonrpc`, `id`, `finish_reason`）|
| `never used` | 8 | 方法/函数（如 `render_markdown`, `format_context_bar`）|
| 其他 | ~10 | 派生 impl 被忽略、测试中未使用的 etc. |

**说明**: 多数警告是"字段定义了但未读取"类型，属于预期行为（序列化时需要这些字段），不影响运行。

---

## 4. 测试覆盖

| 模块 | 测试数 | 特点 |
|------|--------|------|
| TUI 模块 | 最多 | theme(6), status_bar(6), markdown(2), diff(2) |
| 工具模块 | 广泛 | 每个工具至少 1 个测试，grep/run_command/search_web 等较多 |
| 沙箱 | 3 | WASM 回退、Docker 不可用、沙箱工具 |
| 会话 | 5 | 创建/保存/列表/加载/删除 |
| 自主模式 | 2 | 调度器事件、多任务 |
| 全量测试 | 696 个 | 覆盖所有模块 |

---

## 5. 数据持久化

| 存储 | 路径 | 格式 | 规模 |
|------|------|------|------|
| 会话 | `sessions/` | JSON (schema_version: 1) | 238 个文件, 1.0 MB |
| 记忆 | `memory/` | Markdown KV | 2 个文件 |
| 技能 | `skills/` | Markdown + frontmatter | 1 个 (greeter.md) |
| 配置 | `codecoder.json` | JSON (未创建) | — |

---

## 6. 已知问题和技术债务

### P0 - 功能性阻塞
（无）

### P1 - 重要
1. **Phase 7 未完整集成**: `self_evolve.rs` 代码存在但默认不启用，需要显式初始化 `SelfEvolve` 并插入 agent 循环
2. **仅有 1 个示例 skill**: `skills/greeter.md` 是唯一的 skill 文件，缺乏实用的生产级 skill

### P2 - 应修复
3. **37 个编译警告**: 大部分是 dead_code / unused field，可通过 `#[allow(dead_code)]` 或 `_` 前缀+删除清理
4. **`.superpowers/sdd/` 临时文件**: 58 个 review diffs 和 task reports 应清理
5. **README.md 测试数过时**: 写的是 "56 个测试"，实际是 696（已更新在 AGENTS.md 中）

### P3 - 低优先级
6. **238 个会话文件**: 许多是空的测试产物（~332 bytes），可清理
7. **无 codecov 集成**: 有覆盖率报告目录但无 CI 集成
8. **`prompts/`, `tools/`, `knowledge/` 目录未创建**: README 中提到了但实际不存在

### 被废弃/不用的代码

| 文件/符号 | 说明 |
|-----------|------|
| `EventBus` trait (`src/event.rs`) | 定义了但未被任何具体实现使用 |
| `ToolCall` / `ToolResult` 事件变体 | 定义了但未在事件循环中构造 |
| `render_markdown` 函数 | 独立的 markdown 渲染函数，可能被 TUI 的 markdown.rs 取代 |
| `format_context_bar` 函数 | 在 status_bar 中有重复实现 |
| `SkillRegistry::promote`, `::record_usage` 等方法 | 定义但未被调用 |
| `build_frontmatter`, `update_frontmatter_in_file` | 定义但未被调用 |

---

## 7. 外部依赖

```toml
[dependencies]
anyhow = "1"                          # 错误处理
atty = "0.2"                          # 终端检测
reqwest = "0.12"                      # HTTP 客户端（json/stream/blocking）
serde = "1"                           # 序列化
serde_json = "1"                      # JSON
tempfile = "3"                        # 临时文件
ratatui = "0.29"                      # TUI 框架
crossterm = "0.28"                    # 终端控制
pulldown-cmark = "0.13"              # Markdown 解析
syntect = "5"                         # 语法高亮
similar = "3"                         # Diff 算法
unicode-width = "0.2"                 # Unicode 宽度
tokio = "1"                           # 异步运行时
async-trait = "0.1"                   # 异步 trait
futures-util = "0.3"                  # 异步工具
ignore = "0.4"                        # .gitignore 感知的递归搜索
globset = "0.4"                       # Glob 模式匹配
regex = "1"                           # 正则
tree-sitter = "0.23"                  # AST 解析
tree-sitter-rust = "0.23"            # Rust 语法
tree-sitter-typescript = "0.23"      # TypeScript 语法
tree-sitter-python = "0.23"          # Python 语法
notify = "7"                          # 文件系统监控
```

---

## 8. 文件系统结构（完整）

```
/ (project root)
├── AGENTS.md                   # 系统身份声明 → 注入 LLM system prompt
├── CONTEXT.md                  # 项目术语表
├── README.md                   # 用户快速入门
├── Cargo.toml                  # 构建配置
├── Cargo.lock                  # 依赖锁定
├── .gitignore                  # 忽略配置
├── .codecoder_session.json     # 当前会话摘要
├── codecoder.log               # 运行日志
│
├── src/                        # Rust 源码（16+ 模块, 20,450 行）
├── docs/                       # 文档
│   ├── README.md               # 文档索引
│   ├── adr/                    # 架构决策记录（7 个）
│   ├── audit/                  # 审计报告（2 个）
│   ├── design/                 # 设计规格（1 个）
│   └── archive/                # 已完成计划的归档
│
├── skills/                     # Markdown skill（自动注册）
├── memory/                     # Key-value 记忆存储
├── sessions/                   # 对话会话（238 个 JSON）
│
├── archived/                   # 参考项目存档
│   ├── claude-code/            # Bun/TS CLI agent 参考
│   └── nocobase/               # Node.js 无代码平台参考
│
├── .reasonix/                  # Reasonix 调试技能
├── .superpowers/sdd/           # SDD 工作流记录（待清理）
├── coverage_report/            # 代码覆盖率报告
└── target/                     # 编译产物（gitignored）
```

---

## 9. 关键架构决策摘要

| ADR | 标题 | 核心决定 |
|-----|------|---------|
| 0001 | 键位绑定与模式语义 | TUI 采用 Mode 状态机替代事件回调 |
| 0002 | 斜杠命令本地分发 | 斜杠命令在 TUI 线程本地处理，不发送到 LLM |
| 0003 | 中心化主题 | 单个 `Theme` 结构体持有所有颜色定义 |
| 0004 | 会话持久化与迁移 | JSON 文件 + schema_version 实现向后兼容 |
| 0005 | 权限作用域 | `Once` / `AlwaysThisSession` / `AlwaysThisProject` 三级 |
| 0006 | 确认对话框模式 | 使用 `Dialog` 枚举实现模态覆盖层 |
| 0007 | 提示注入式斜杠命令 | LLM 看到的展开文本不同于用户输入 |

---

## 10. 工作流建议

### 给 LLM agent 的指导

1. **阅读顺序**: 先读本文件 → AGENTS.md → CONTEXT.md → 目标模块的源码
2. **代码组织**: 每个 `.rs` 文件包含 `struct` 定义、`impl`、`pub fn`、测试模块
3. **测试惯例**: `#[cfg(test)]` 内部模块；使用 `#[test]` 和辅助函数
4. **错误处理**: 使用 `anyhow::Result` 和 `anyhow!` 宏
5. **配置**: 默认通过 `CODECODER_*` 环境变量配置；`codecoder.json` 可选
6. **TUI 架构**: 每一帧从 `TuiApp` 读取状态 → `render` → `crossterm::execute`

### 下一个迭代建议

| 优先级 | 任务 | 涉及文件 |
|--------|------|---------|
| 🔴 P0 | 启用 Phase 7 self_evolve | `src/self_evolve.rs`, `src/agent.rs` |
| 🟡 P1 | 减少编译警告 | 各处 `unused` 标记 |
| 🟡 P1 | 清理 `.superpowers/sdd/` | 58 个临时文件 |
| 🟢 P2 | 添加生产级 skill 示例 | `skills/` |
| 🟢 P2 | 清理空会话文件 | `sessions/` |
| 🔵 P3 | 配置 CI / coverage | `.github/workflows/` |
