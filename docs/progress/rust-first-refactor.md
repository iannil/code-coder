# CodeCoder Rust-First 架构重构进展

> 创建时间: 2026-03-12
> 最后更新: 2026-03-12 (Phase 4 进行中: commit, review, agents 完成)
> 状态: 进行中

## 实施概述

将 TypeScript 业务逻辑迁移到 Rust，实现"Rust CLI 独立运行 + TS 极简 UI"的最终形态。

## 进度摘要

| 阶段 | 状态 | 完成日期 | 备注 |
|------|------|----------|------|
| Phase 1: Agent 注册中心 | ✅ 完成 | 2026-03-12 | 模糊搜索已实现 |
| Phase 2: Memory-Markdown | ✅ 完成 | 已有实现 | 发现已存在 Rust 实现 |
| Phase 3: Provider SDK 扩展 | ✅ 完成 | 2026-03-12 | 6 个新 Provider |
| Phase 4: CLI 命令迁移 | ⏳ 进行中 | - | 3/4 命令完成 (commit, review, agents) |
| Phase 5: TypeScript 精简 | ⏳ 待实施 | - | |

---

## Phase 1: Agent 注册中心迁移 ✅

**完成日期**: 2026-03-12

### 实现内容

1. **新增文件**:
   - `services/zero-core/src/agent/metadata.rs` (~700 行)
   - `services/zero-core/src/napi/agent_registry.rs` (~400 行)

2. **核心功能**:
   - `AgentMetadata` - 丰富的 Agent 元数据结构
   - `AgentCapability` - 能力声明
   - `AgentTrigger` - 触发器定义（关键词/正则/事件/上下文）
   - `AgentExample` - 使用示例
   - `AgentCategory` / `AgentRole` - 分类和角色

3. **搜索能力**:
   - `MetadataIndex.search()` - Jaro-Winkler 模糊搜索
   - `MetadataIndex.find_by_trigger()` - 触发器匹配
   - `MetadataIndex.recommend()` - 基于意图的推荐
   - 加权字段评分系统

4. **NAPI 绑定**:
   - `AgentMetadataIndexHandle` - TypeScript 可用的异步句柄
   - 完整的类型转换层

5. **依赖新增**:
   - `strsim = "0.11"` - 字符串相似度计算

### 测试结果

```
running 7 tests
test agent::metadata::tests::test_list_by_mode ... ok
test agent::metadata::tests::test_get_primary_for_mode ... ok
test agent::metadata::tests::test_list_by_category ... ok
test agent::metadata::tests::test_search_fuzzy ... ok
test agent::metadata::tests::test_search_basic ... ok
test agent::metadata::tests::test_recommend ... ok
test agent::metadata::tests::test_find_by_trigger ... ok

test result: ok. 7 passed; 0 failed
```

### TypeScript 迁移说明

`packages/ccode/src/agent/registry.ts` 可逐步切换到调用 Rust NAPI 绑定：

```typescript
// 旧方式 (Fuse.js)
const registry = new AgentRegistry()
const results = registry.search(query)

// 新方式 (Rust NAPI)
import { createAgentMetadataIndexWithBuiltins } from '@codecoder-ai/core'
const index = createAgentMetadataIndexWithBuiltins()
const results = await index.search(query)
```

---

## Phase 2: Memory-Markdown 系统 ✅

**状态**: 已发现完整 Rust 实现

### 现有实现

1. **核心模块**: `services/zero-core/src/memory/markdown.rs` (720 行)
   - `DailyEntry` / `DailyEntryType` - 每日笔记条目
   - `MemoryCategory` - 五大长期记忆分类
   - `MarkdownMemoryStore` - 完整的存储实现
   - `MemoryContext` - 上下文组合

2. **NAPI 绑定**: `services/zero-core/src/napi/memory.rs` (1348 行)
   - `MarkdownMemoryHandle` - TypeScript 可用句柄
   - 完整的每日笔记操作
   - 长期记忆分类管理
   - 上下文加载

### 功能验证

TypeScript 的 `packages/ccode/src/memory-markdown/` 可以完全由 Rust 实现替代。

---

## Phase 3: Provider SDK 扩展 ✅

**完成日期**: 2026-03-12

### 实现内容

1. **新增文件**:
   - `services/zero-core/src/provider/openai_compat.rs` (~800 行)

2. **新增 Provider**:
   | Provider | API 基础 | 工具支持 | 视觉支持 |
   |----------|---------|---------|---------|
   | Ollama | OpenAI 兼容 | ✅ | ✅ |
   | Groq | OpenAI 兼容 | ✅ | ❌ |
   | Mistral | OpenAI 兼容 | ✅ | ❌ |
   | Together | OpenAI 兼容 | ✅ | ✅ |
   | Perplexity | OpenAI 兼容 | ❌ | ❌ |
   | DeepSeek | OpenAI 兼容 | ✅ | ❌ |

3. **架构设计**:
   - `OpenAICompatConfig` - 可配置的基础配置
   - `OpenAICompatProvider` - 通用 OpenAI 兼容实现
   - 每个 Provider 只是薄包装，复用基础实现

4. **API 更新**:
   - `create_provider()` 工厂函数支持所有新 Provider
   - 端点常量: `OLLAMA_API_URL`, `GROQ_API_URL`, 等

### 测试结果

```
running 30 tests
test provider::openai_compat::tests::test_sse_done ... ok
test provider::openai_compat::tests::test_sse_parsing ... ok
test provider::openai_compat::tests::test_ollama_config ... ok
test provider::openai_compat::tests::test_groq_config ... ok
... 其他测试 ...

test result: ok. 30 passed; 0 failed
```

### 使用示例

```rust
use zero_core::provider::{create_provider, ProviderConfig, OllamaProvider};

// 通过工厂函数
let config = ProviderConfig::new("ollama", "");
let provider = create_provider("ollama", config)?;

// 或直接实例化
let provider = OllamaProvider::new(ProviderConfig::new("ollama", ""));
```

---

## 技术决策记录

### 决策 1: 使用 strsim 而非 fuzzy-matcher

**原因**:
- strsim 提供 Jaro-Winkler 算法，适合短字符串匹配
- API 更简洁，单函数调用
- 无需额外的索引构建

### 决策 2: 保留 TypeScript AgentRegistry 兼容层

**原因**:
- 允许渐进式迁移
- 降低一次性切换风险
- 便于 A/B 测试对比结果

### 决策 3: OpenAI 兼容 Provider 使用组合模式

**原因**:
- 所有 OpenAI 兼容 API 共享 95% 代码
- 每个 Provider 只需定义端点和少量差异
- 减少代码重复，提高可维护性

---

## 下一步行动

1. [x] 实施 Phase 3: 添加 Ollama/Groq 等 Provider 支持
2. [x] 更新 TypeScript registry.ts 添加 Rust 调用切换开关
   - 环境变量: `CODECODER_RUST_REGISTRY=1`
   - 新增方法: `searchAsync()`, `findByTriggerAsync()`, `recommendAsync()` 等
   - 导出: `isRustBackendEnabled()`, `isRustBackendAvailable()`
3. [ ] 性能基准测试: Fuse.js vs Rust fuzzy search
4. [ ] 集成测试: 验证 NAPI 绑定在 TUI 中正常工作
5. [ ] Phase 4: CLI 命令迁移 (/commit, /review-pr 等)
   - **发现**: `services/zero-cli/` 已存在完整框架 (17,000+ 行)
   - 包含: daemon, agent, channels, skills, memory 等模块
   - 待办: 添加 git commit 和 PR review 专用命令
6. [ ] Phase 5: TypeScript 代码精简

---

## TypeScript Registry Rust 切换 ✅

**完成日期**: 2026-03-12

### 使用方式

```bash
# 启用 Rust 后端
export CODECODER_RUST_REGISTRY=1

# 验证是否启用
bun -e 'import { isRustBackendEnabled } from "./src/agent/registry"; console.log(isRustBackendEnabled())'
```

### 新增 API

| 方法 | 描述 | Rust 支持 |
|------|------|-----------|
| `searchAsync(query, options)` | 异步模糊搜索 | ✅ |
| `findByTriggerAsync(input)` | 异步触发器匹配 | ✅ |
| `recommendAsync(intent)` | 异步意图推荐 | ✅ |
| `listByModeAsync(modeId)` | 异步模式列表 | ✅ |
| `getPrimaryForModeAsync(modeId)` | 异步主代理获取 | ✅ |
| `isRustBackendEnabled()` | 检查开关状态 | - |
| `isRustBackendAvailable()` | 检查绑定可用 | - |

### 转换层

```typescript
// Rust 元数据转换为 TypeScript 格式
function rustToTsMetadata(r: RustAgentMetadata): AgentMetadata
function rustToTsSearchResult(r: RustSearchResult): SearchResult
```

---

## Phase 4: CLI 命令探索 ⏳

**状态**: 探索完成，核心模块已就绪

### 探索发现

`services/zero-cli/` 已存在完整框架 (~17,000+ 行代码):

1. **main.rs** (608 行): 完整的 clap 命令定义
   - `onboard`, `agent`, `daemon`, `status`, `cron`
   - `channel`, `integrations`, `skills`, `credential`
   - `migrate`, `mcp-server`, `serve-ipc`, `trading`

2. **已有模块**:
   | 模块 | 文件 | 行数 | 描述 |
   |------|------|------|------|
   | agent | `src/agent/` | ~1,000 | 代理执行 |
   | channels | `src/channels/` | ~800 | Telegram/Discord/Slack |
   | daemon | `src/daemon/` | ~600 | 进程编排器 |
   | memory | `src/memory/` | ~500 | 记忆后端 |
   | tools | `src/tools/` | ~1,500 | 工具注册 |
   | unified_api | `src/unified_api/` | ~5,000 | HTTP/WS API |

3. **核心功能已在 zero-core**:
   - `git/mod.rs` (1,338 行): 完整 Git 操作 (commit, diff, status, worktree)
   - `java/mod.rs`: JAR 分析 (JarAnalyzer, ClassFile, Fingerprint)
   - `agent/metadata.rs`: Agent 注册搜索

### 待添加命令

| 命令 | 目标 | 依赖模块 | 状态 |
|------|------|----------|------|
| `ccode commit` | 智能提交 | `git/mod.rs` | ✅ 完成 |
| `ccode review` | 代码审查 | `git/mod.rs` + Provider | ✅ 完成 |
| `ccode jar-reverse` | JAR 逆向 | `java/mod.rs` | 待实现 |
| `ccode agents` | Agent 搜索 | `agent/metadata.rs` | ✅ 完成 |

### `zero-cli commit` 实现 ✅

**完成日期**: 2026-03-12

**功能**:
- AI 生成 conventional commit 消息
- 支持 `--dry-run` 预览模式
- 支持 `-a/--add-all` 暂存所有更改
- 支持 `-m/--message` 自定义消息
- 支持 `--allow-empty` 允许空提交

**使用示例**:
```bash
# 预览将要提交的内容
zero-cli commit --dry-run

# AI 生成消息并提交所有更改
zero-cli commit -a

# 使用自定义消息
zero-cli commit -m "feat: add new feature"
```

**新增文件**:
- `services/zero-cli/src/commit.rs` (~230 行)

### `zero-cli review` 实现 ✅

**完成日期**: 2026-03-12

**功能**:
- AI 驱动的代码审查分析
- 识别安全问题、Bug、性能问题
- 生成结构化的审查建议
- 支持多种输出格式 (text, json, markdown)

**使用示例**:
```bash
# 审查当前分支与 main 的差异
zero-cli review

# 审查特定分支
zero-cli review -t feature-branch -b develop

# JSON 输出
zero-cli review --format json

# 显示完整 diff
zero-cli review --show-diff
```

**新增文件**:
- `services/zero-cli/src/review.rs` (~420 行)

### `zero-cli agents` 实现 ✅

**完成日期**: 2026-03-12

**功能**:
- Agent 模糊搜索 (使用 Rust `strsim` 库)
- 按分类/模式列表 Agent
- 显示 Agent 详细信息
- 基于意图的 Agent 推荐

**子命令**:
- `agents search <query>` - 搜索 Agent
- `agents list [--category]` - 列出 Agent
- `agents info <name>` - 查看 Agent 详情
- `agents recommend <intent>` - 推荐 Agent

**使用示例**:
```bash
# 搜索 Agent
zero-cli agents search "code review"

# 列出所有 Agent
zero-cli agents list

# 按分类列出
zero-cli agents list --category engineering

# 查看详情
zero-cli agents info code-reviewer

# 获取推荐
zero-cli agents recommend "review this code"
```

**新增文件**:
- `services/zero-cli/src/agents.rs` (~260 行)

### 实现方案

```rust
// services/zero-cli/src/main.rs 扩展
#[derive(Subcommand, Debug)]
enum Commands {
    // ... 现有命令 ...

    /// Git commit with AI-generated message
    Commit {
        #[arg(short, long)]
        message: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },

    /// Review pull request changes
    ReviewPr {
        /// PR number or branch name
        target: String,
    },

    /// Reverse engineer JAR file
    JarReverse {
        /// Path to JAR file
        path: std::path::PathBuf,
        /// Output directory
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Agent management
    Agents {
        #[command(subcommand)]
        agent_command: AgentCommands,
    },
}

#[derive(Subcommand, Debug)]
enum AgentCommands {
    /// Search for agents
    Search { query: String },
    /// List all agents
    List,
    /// Show agent info
    Info { name: String },
}
```

### 下一步

1. 在 `zero-cli/src/main.rs` 添加上述命令定义
2. 创建 `zero-cli/src/commit.rs` 实现 AI 生成提交信息
3. 创建 `zero-cli/src/review.rs` 实现 PR 审查
4. 创建 `zero-cli/src/agents.rs` 封装 agent 搜索

