# 记忆系统架构

> 文档类型: guide
> 创建时间: 2026-02-05
> 最后更新: 2026-02-16
> 状态: active

## 概述

CodeCoder 采用多层记忆系统架构，分别服务于不同的使用场景：

1. **技术记忆系统** - 用于代码理解和索引
2. **Markdown 记忆系统** - 用于用户偏好、决策、经验教训的持久化
3. **ZeroBot 记忆系统** - 跨系统共享记忆 (SQLite)
4. **记忆路由器** - 统一写入入口，自动路由到正确存储层
5. **记忆桥接层** - 组合多系统上下文，带 TTL 缓存

## 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                     MemoryRouter                             │
│            (统一写入入口，自动路由)                            │
└────────────────────────┬────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│  Markdown   │  │  Technical  │  │   ZeroBot   │
│   Memory    │  │   Memory    │  │   Memory    │
│ (MEMORY.md) │  │ (patterns)  │  │ (brain.db)  │
└─────────────┘  └─────────────┘  └─────────────┘
         │               │               │
         └───────────────┼───────────────┘
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   MemoryBridge                               │
│         (组合上下文，30秒 TTL 缓存)                           │
└─────────────────────────────────────────────────────────────┘
```

## 系统对比

| 特性 | 技术记忆系统 | Markdown 记忆系统 | ZeroBot 记忆系统 |
|------|-------------|------------------|------------------|
| **代码路径** | `src/memory/` | `src/memory-markdown/` | `src/memory-zerobot/` |
| **存储位置** | 内部数据结构 | 项目 `./memory/` | `~/.codecoder/workspace/memory/brain.db` |
| **存储格式** | 二进制/内部 | Markdown 文本 | SQLite + FTS5 |
| **人类可读** | 否 | 是 | 部分 (SQL) |
| **Git 友好** | 否 | 是 | 否 |
| **检索方式** | 向量搜索 | 文本读取 | FTS5 全文搜索 |
| **主要用途** | 代码结构、API 端点 | 用户偏好、关键决策 | 跨系统共享 |
| **默认访问** | 读写 | 读写 | **只读** |

## 技术记忆系统

### 目录结构

```
packages/ccode/src/memory/
├── index.ts          # 主入口
├── vector.ts         # 向量存储
├── code-index.ts     # 代码索引
└── pattern-learning.ts # 模式学习
```

### 功能

- 向量搜索
- 代码结构索引
- API 端点识别
- 模式学习

### 使用场景

当 Agent 需要理解代码库结构时，技术记忆系统提供：
- 类和函数定义的位置
- API 端点的签名
- 模块之间的依赖关系

## Markdown 记忆系统

### 目录结构

```
packages/ccode/src/memory-markdown/
├── types.ts          # 核心类型定义
├── util.ts           # 工具函数
├── daily.ts          # 流层管理
├── long-term.ts      # 沉积层管理
├── loader.ts         # 上下文加载器
├── consolidate.ts    # 自动整合机制
├── config.ts         # 配置加载
├── project.ts        # 项目检测
├── storage.ts        # 存储抽象
└── index.ts          # 公共 API
```

### 双层架构

#### 流层 (Flow Layer)

**路径**: `./memory/daily/{YYYY-MM-DD}.md`

**特性**:
- 仅追加模式，不可变日志
- 按时间顺序记录所有活动
- 支持时间范围查询

**示例格式**:

```markdown
# 2026-02-05

## 10:30 - 决策

选择使用 Turborepo 作为 monorepo 构建工具，理由：
- 社区活跃
- 性能优秀
- 与 Bun 兼容好

## 14:20 - 任务完成

实现了 Markdown 记忆系统的 consolidate 功能
```

#### 沉积层 (Sediment Layer)

**路径**: `./memory/MEMORY.md`

**特性**:
- 结构化的分类知识
- 支持智能合并和更新
- 包含：用户偏好、项目上下文、关键决策、经验教训

**示例格式**:

```markdown
# 项目记忆

## 用户偏好

- 使用 TypeScript 进行所有新开发
- 优先使用 Bun API 而非 Node.js API
- 保持代码行数 < 300 行/文件

## 项目上下文

CodeCoder 是一个开源 AI 编程代理，使用 Turborepo 和 Bun 构建。

## 关键决策

### 2026-02-05: 采用双层记忆架构

决定使用 Markdown 文件存储非技术性记忆，理由：
- 人类可读
- Git 友好
- 易于审计

## 经验教训

- 避免过早抽象
- 保持小文件原则
```

### API 设计

```typescript
// 每日笔记（流层）
appendDailyNote(entry: DailyEntry)
loadDailyNotes(date: Date, days?: number): Promise<string[]>

// 长期记忆（沉积层）
loadLongTermMemory(): Promise<string>
updateCategory(category: MemoryCategory, content: string): Promise<void>
mergeToCategory(category: MemoryCategory, update: string): Promise<void>

// 上下文加载
loadMarkdownMemoryContext(options?: LoadOptions): Promise<MemoryContext>

// 自动整合
consolidateMemory(options?: ConsolidateOptions): Promise<ExtractionResult[]>
getConsolidationStats(): Promise<ConsolidationStats>
```

### CLI 命令

```bash
# 查看所有记忆
codecoder memory view

# 查看特定类别
codecoder memory view 用户偏好

# 查看最近7天的每日笔记
codecoder memory view daily --days 7

# 编辑今天的笔记
codecoder memory edit daily

# 整合最近7天的笔记
codecoder memory consolidate --days 7

# 查看统计信息
codecoder memory stats
```

## 记忆桥接层

### 代码路径

`packages/ccode/src/agent/memory-bridge.ts`

### 功能

将多套记忆系统组合成统一的上下文，并提供 TTL 缓存减少重复加载：

```typescript
// 构建组合上下文（带 30 秒缓存）
buildMemoryContext(options?: BuildMemoryContextOptions): Promise<{
  technical: AgentContext      // 来自技术记忆系统
  markdown: MemoryContext      // 来自 Markdown 记忆系统
  formatted: string            // 组合后的格式化上下文
}>

// 强制跳过缓存
buildMemoryContext({ skipCache: true })

// 手动失效缓存
invalidateMemoryCache(): void
```

### 缓存策略

- **TTL**: 30 秒自动过期
- **Options Hash**: 不同参数使用独立缓存
- **自动失效**: `routeMemoryWrite()` 成功后自动失效
- **手动失效**: 调用 `invalidateMemoryCache()`

## 记忆路由器

### 代码路径

`packages/ccode/src/agent/memory-router.ts`

### 功能

统一写入入口，根据数据类型自动路由到正确的存储层：

| 类型 | 路由目标 | 说明 |
|------|----------|------|
| `preference` | MEMORY.md/用户偏好 | 长期用户偏好 |
| `decision` | MEMORY.md/关键决策 | 需要审计的决策 |
| `lesson` | MEMORY.md/经验教训 | 知识沉淀 |
| `context` | MEMORY.md/项目上下文 | 项目特定上下文 |
| `daily` | daily/*.md | 每日流水日志 |
| `pattern` | preferences/patterns | 代码模式学习 |

### API

```typescript
// 主路由函数
routeMemoryWrite(request: MemoryWriteRequest): Promise<MemoryWriteResult>

// 批量写入
batchMemoryWrite(requests: MemoryWriteRequest[]): Promise<MemoryWriteResult[]>

// 便捷函数
writePreference(key: string, content: string): Promise<MemoryWriteResult>
writeDecision(key: string, content: string): Promise<MemoryWriteResult>
writeLesson(key: string, content: string): Promise<MemoryWriteResult>
writeDailyNote(key: string, content: string, entryType?: DailyEntryType): Promise<MemoryWriteResult>
learnPattern(pattern: string): Promise<MemoryWriteResult>
```

### 使用示例

```typescript
import { routeMemoryWrite, writePreference } from "@/agent/memory-router"

// 使用主函数
await routeMemoryWrite({
  type: "preference",
  key: "editor",
  content: "Uses Neovim for all editing"
})

// 使用便捷函数
await writePreference("lang", "Prefers Rust over Python")
await writeDecision("arch", "Adopted event-driven architecture")
await writeLesson("testing", "Always mock external services")
```

## ZeroBot 记忆系统

### 代码路径

`packages/ccode/src/memory-zerobot/`

### 功能

访问 ZeroBot (Rust) 的 SQLite 记忆数据库，实现跨系统共享：

```typescript
import { createZeroBotMemory } from "@/memory-zerobot"

const memory = createZeroBotMemory()  // 默认只读

if (memory.isAvailable()) {
  // 查询记忆
  const results = memory.recall("programming language", 5)

  // 获取特定记忆
  const pref = memory.get("user_preference")

  // 检查是否可写
  if (memory.isWritable()) {
    memory.store("key", "content", "core")
  }

  // 安全写入（只读模式返回 false）
  const success = memory.tryStore("key", "content", "core")
}
```

### 安全设计

- **默认只读**: `readOnly` 默认为 `true`，防止意外修改 ZeroBot 数据
- **显式可写**: 需要 `createZeroBotMemory({ readOnly: false })` 才能写入
- **安全写入**: `tryStore()` 在只读模式返回 `false` 而不是抛异常

## 集成到 Agent

### 系统提示集成

代码路径: `packages/ccode/src/session/system.ts`

```typescript
export function markdownMemory(): string {
  // 加载 Markdown 记忆并格式化为系统提示
}
```

### 自动加载

Agent 启动时自动加载：
1. 技术记忆（代码上下文）
2. Markdown 记忆（用户偏好、项目上下文）

## 设计原则

### LLM 友好

- 小文件：最大 321 行
- 清晰命名：`appendDailyNote`, `loadLongTermMemory`
- 显式类型：完整的 TypeScript 定义
- 单一职责：每个文件职责明确

### 透明性

- 所有记忆文件都是标准 Markdown
- 可以手动编辑审查
- Git 可追踪变更

### 独立性

- 两套系统完全独立
- 无导入依赖
- 可单独使用

## 相关文档

- [CLAUDE.md - 记忆系统](../../CLAUDE.md#记忆系统)
- [Architecture Guide](../Architecture-Guide.md)
- [Markdown 记忆层完成报告](../reports/completed/2026-02-05-memory-markdown-completion.md)
