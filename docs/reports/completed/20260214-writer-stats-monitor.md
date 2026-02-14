# Writer/Expander 执行统计与增量保存功能实现报告

## 概述

实现了 Writer/Expander 执行统计功能，当 writer agent 调用 expander/expander-fiction/expander-nonfiction 生成长文本时：
1. 系统每 30 秒输出一次当前执行状态
2. 每 2000 字或每 60 秒自动保存草稿，防止进度丢失

## 完成时间

2026-02-14

## 修改历史

### v1.2 (2026-02-14) - 增量保存功能

新增增量保存机制，解决长篇章节（15000字）生成过程中进度丢失的问题：

1. **新增 `ChapterDraftManager`**：
   - 管理章节草稿的增量保存
   - 每 2000 字或每 60 秒自动保存
   - 保存到 `.draft` 后缀文件
   - 生成完成后合并到最终文件

2. **修改 expander prompts**：
   - 添加长篇内容生成指导
   - 要求连续输出，不要最后一次性输出
   - 使用场景/章节结构组织内容
   - 添加进度标记 (<!-- PROGRESS: ~3000 words -->)

3. **新增 TUI 事件**：
   - `ChapterDraftSaved`: 草稿保存通知
   - `ChapterDraftFinalized`: 章节完成通知

### v1.1 (2026-02-14) - 字数统计优化

修复了两个问题：
1. **字数估算错误**：使用正确的字数统计方法（中文字符 + 英文单词数）
2. **章节完成判断不准确**：追踪 Write tool 的调用状态

### v1.0 (2026-02-14) - 初始实现

- 基础的执行统计监控
- 每 30 秒报告一次状态

## 修改文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `packages/ccode/src/agent/writer-stats-monitor.ts` | 修改 | 集成草稿保存功能 |
| `packages/ccode/src/agent/chapter-draft-manager.ts` | 新增 | 章节草稿管理器 |
| `packages/ccode/src/cli/cmd/tui/event.ts` | 修改 | 添加草稿事件类型 |
| `packages/ccode/src/cli/cmd/tui/app.tsx` | 修改 | 订阅草稿事件 |
| `packages/ccode/src/tool/task.ts` | 修改 | 异步调用 stop |
| `packages/ccode/src/agent/prompt/expander.txt` | 修改 | 长篇内容生成指导 |
| `packages/ccode/src/agent/prompt/expander-fiction.txt` | 修改 | 小说长篇生成指导 |
| `packages/ccode/src/agent/prompt/expander-nonfiction.txt` | 修改 | 非虚构长篇生成指导 |

## 实现细节

### 1. ChapterDraftManager (`chapter-draft-manager.ts`)

核心功能：
- `start(input)`: 开始追踪章节草稿
- `updateContent(sessionID, content)`: 更新内容，检查是否需要保存
- `finalize(sessionID, finalContent)`: 完成章节，保存最终文件
- `recoverDraft(chapterPath)`: 从草稿文件恢复

保存策略：
- 每 2000 新字保存一次
- 每 60 秒检查一次（如有 500+ 新字则保存）
- 草稿保存到 `{chapterPath}.draft`
- 完成后删除草稿，写入最终文件

### 2. WriterStatsMonitor 集成

新增字段：
```typescript
interface MonitoredSession {
  // ... 原有字段
  chapterPath?: string
  lastDraftSaveWords: number
  lastDraftSaveTime: number
  draftSaveCount: number
}
```

新增方法：
- `setChapterPath(sessionID, path)`: 设置章节路径（后期绑定）
- `getAccumulatedContent(sessionID)`: 获取已生成内容

### 3. 新增 TUI 事件

```typescript
ChapterDraftSaved: BusEvent.define(
  "chapter.draft.saved",
  z.object({
    sessionID: z.string(),
    chapterPath: z.string(),
    wordsWritten: z.number(),
    saveCount: z.number(),
  }),
),
ChapterDraftFinalized: BusEvent.define(
  "chapter.draft.finalized",
  z.object({
    sessionID: z.string(),
    chapterPath: z.string(),
    wordsWritten: z.number(),
    totalSaves: z.number(),
  }),
),
```

### 4. TUI 显示

```
💾 草稿已保存 (3) | 6.5k字
✅ 章节已完成 | 15.2k字 | 共7次保存
```

### 5. Expander Prompt 更新

添加了长篇内容生成指导：
- 连续输出，不要最后一次性输出
- 场景/章节结构组织（每个 2000-3000 字）
- 进度标记 `<!-- PROGRESS: ~3000 words -->`
- 15000 字章节的结构模板

## 架构图

```
writer agent
    │
    ▼ (Task tool 调用)
expander-* subagent
    │
    ├─> WriterStatsMonitor.start(sessionID, chapterPath)
    │   │
    │   ├─> ChapterDraftManager.start(sessionID, chapterPath)
    │   │
    │   ├─> 订阅 MessageV2.Event.PartUpdated
    │   │   ├─> 追踪 text parts 的完整内容
    │   │   ├─> 追踪 Write tool 的执行状态
    │   │   └─> 检查是否需要保存草稿
    │   │
    │   ├─> 定时器每 30s 发布 TuiEvent.WriterStats
    │   │
    │   └─> 定时器每 60s 检查并保存草稿
    │       └─> ChapterDraftManager.updateContent()
    │           └─> 发布 TuiEvent.ChapterDraftSaved
    │
    └─> WriterStatsMonitor.stop(sessionID)
        └─> ChapterDraftManager.finalize()
            └─> 发布 TuiEvent.ChapterDraftFinalized
```

## 测试验证

### 功能测试
1. 切换到 writer agent: `@writer`
2. 请求写一个长篇章节（15000字）
3. 观察：
   - 每 30 秒显示统计 toast
   - 每 2000 字或 60 秒显示草稿保存 toast
   - 完成时显示总字数和保存次数

### 边界情况
- 短章节（<2000字）：不触发草稿保存
- 生成中断：草稿文件保留，可恢复
- 正常完成：草稿文件删除，最终文件写入

### 草稿恢复
```typescript
const draft = await ChapterDraftManager.recoverDraft(chapterPath)
if (draft) {
  // 从草稿恢复
}
```

## 注意事项

1. **草稿文件**：保存在 `{chapterPath}.draft`，完成后自动删除
2. **异步 stop**：`WriterStatsMonitor.stop()` 现在是异步的，需要 await
3. **后期绑定**：可以用 `setChapterPath()` 在监控开始后设置章节路径
4. **进度标记**：expander 输出的进度标记 `<!-- PROGRESS: ... -->` 不影响最终内容
