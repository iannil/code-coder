# Markdown 记忆层实施

> 开始时间: 2026-02-05
> 状态: completed
> 负责人: Claude Code

## 目标

在现有记忆模块 (`packages/ccode/src/memory/`) 基础上，新增一层 Markdown 双层架构，实现透明的、Git 友好的长期记忆存储。

## 背景

### 现有系统
现有记忆模块使用向量搜索、代码索引和模式学习，主要服务于技术性记忆（代码结构、API 端点等）。

### 新增需求
根据 CLAUDE.md 规范，项目需要：
- 用户偏好、关键决策、经验教训等"软知识"的持久化
- 人类可读的 Markdown 格式
- 透明的、Git 友好的存储
- 不使用复杂的嵌入检索

### 设计原则
**核心原则**: 现有记忆模块完全不受影响，新增 `memory-markdown/` 模块独立运行。

## 进展

### 已完成

- [x] 创建 `packages/ccode/src/memory-markdown/` 模块
  - [x] `types.ts` - 类型定义 (DailyEntry, MemoryCategory, LoadOptions)
  - [x] `util.ts` - 工具函数 (日期格式化、Markdown 格式化)
  - [x] `daily.ts` - 流层管理 (./memory/daily/{YYYY-MM-DD}.md)
  - [x] `long-term.ts` - 沉积层管理 (./memory/MEMORY.md)
  - [x] `loader.ts` - 上下文加载器
  - [x] `index.ts` - 公共 API 导出
- [x] 创建 `packages/ccode/src/agent/memory-bridge.ts` - 两套系统的桥接层
- [x] 通过类型检查和构建验证
- [x] 验证现有记忆模块未被修改
- [x] **集成到 Agent 上下文加载流程** (2026-02-05)
- [x] **编写单元测试** (2026-02-05)
- [x] **添加 CLI 命令** (2026-02-05)
- [x] **实现自动整合机制** (2026-02-05)

### 新增功能 (2026-02-05)

1. **Agent 上下文集成**
   - 在 `session/system.ts` 中添加 `markdownMemory()` 函数
   - 在 `session/prompt.ts` 中集成到系统提示加载流程
   - Agent 现在可以自动访问 Markdown 记忆内容

2. **CLI 命令**
   - `codecoder memory view [category]` - 查看记忆内容
   - `codecoder memory edit [category]` - 编辑记忆文件
   - `codecoder memory list` - 列出每日笔记日期
   - `codecoder memory consolidate` - 整合每日笔记到长期记忆
   - `codecoder memory stats` - 显示记忆统计信息

3. **自动整合机制**
   - 创建 `consolidate.ts` 模块
   - 智能提取每日笔记中的重要信息
   - 按重要性评分自动分类
   - 去重并合并到长期记忆

4. **单元测试**
   - `test/unit/memory-markdown/util.test.ts` - 工具函数测试
   - `test/unit/memory-markdown/consolidate.test.ts` - 整合机制测试
   - 37 个测试用例全部通过

### 待办

- [ ] 添加遗忘策略（过期内容清理）
- [ ] 实现自动记忆整合的定时触发
- [ ] 添加更多集成测试

## 阻塞问题

| 问题 | 影响 | 解决方案 |
|------|------|----------|
| 无 | - | - |

## 变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `packages/ccode/src/memory-markdown/types.ts` | 新建 | 核心类型定义 |
| `packages/ccode/src/memory-markdown/util.ts` | 新建 | 工具函数 |
| `packages/ccode/src/memory-markdown/daily.ts` | 新建 | 流层（每日笔记）管理 |
| `packages/ccode/src/memory-markdown/long-term.ts` | 新建 | 沉积层（长期记忆）管理 |
| `packages/ccode/src/memory-markdown/loader.ts` | 新建 | 上下文加载器 |
| `packages/ccode/src/memory-markdown/index.ts` | 新建/修改 | 公共 API 导出（新增 consolidate） |
| `packages/ccode/src/memory-markdown/consolidate.ts` | 新建 | 自动整合机制 |
| `packages/ccode/src/agent/memory-bridge.ts` | 新建 | 记忆系统桥接层 |
| `packages/ccode/src/session/system.ts` | 修改 | 添加 markdownMemory() 函数 |
| `packages/ccode/src/session/prompt.ts` | 修改 | 集成 Markdown 记忆到系统提示 |
| `packages/ccode/src/cli/cmd/memory.ts` | 新建 | Memory CLI 命令 |
| `packages/ccode/src/index.ts` | 修改 | 注册 memory 命令 |
| `packages/ccode/test/unit/memory-markdown/util.test.ts` | 新建 | 工具函数单元测试 |
| `packages/ccode/test/unit/memory-markdown/consolidate.test.ts` | 新建 | 整合机制单元测试 |

## 相关文档

- [CLAUDE.md - 记忆系统](../../CLAUDE.md#记忆系统)
- [Architecture Guide](../Architecture-Guide.md)
- [完成报告](../reports/completed/2026-02-05-memory-markdown-completion.md)

## 技术细节

### 双层架构

```
./memory/
├── daily/              # 流层 (Flow Layer)
│   ├── 2026-02-05.md   # 按日期的只追加日志
│   └── 2026-02-04.md
└── MEMORY.md           # 沉积层 (Sediment Layer)
                        # 结构化的长期知识
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

### 桥接层设计

```typescript
// memory-bridge.ts
buildMemoryContext(): Promise<{
  technical: AgentContext      // 来自现有系统
  markdown: MemoryContext      // 来自新 Markdown 层
  formatted: string            // 组合后的格式化上下文
}>
```

### CLI 使用示例

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
