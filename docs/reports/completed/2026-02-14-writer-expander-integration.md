# Writer 与 Expander 集成实施报告

**日期**: 2026-02-14
**状态**: 已完成

## 背景

writer 和 expander 是两个独立的内容创作 agent，需要让 writer 能够调用 expander 的系统化写作能力。

| Agent | 功能 | 模式 | 能力 |
|--------|------|------|------|
| `writer` | 书籍写作 | primary | 探索文件→生成大纲→逐章写作 |
| `expander-fiction` | 小说扩展 | subagent | 世界观/角色/故事弧线 |
| `expander-nonfiction` | 非小说扩展 | subagent | 论证/知识框架 |

## 实施内容

### Step 1: 修改 writer.txt prompt

**文件**: `/packages/ccode/src/agent/prompt/writer.txt`

添加了 `Chapter Writing with Expander Integration` 章节，包含：

1. **Content Type Detection** - 区分小说与非小说
2. **Expander Selection** - 使用 Task 工具调用对应的 expander
3. **Context to Provide** - 需要传递给 expander 的上下文信息
4. **After Expander Returns** - expander 返回后的处理流程

更新了 `Example Workflow` 以包含 expander 调用步骤。

### Step 2: 验证 expander 输出格式

检查 `expander-fiction.txt` 和 `expander-nonfiction.txt` 的输出格式：

- **expander-fiction**: 输出包含 Chapter Summary（叙事进展、角色发展、世界观元素等）
- **expander-nonfiction**: 输出包含 Chapter Summary（主要论证、证据支持、论题关联等）

两者都与 writer 的期望格式对齐。

### Step 3: 确认 writer 配置

**文件**: `/packages/ccode/src/agent/agent.ts`

确认：
- writer 的 `permission` 已包含 Task 工具权限（`defaults` 中 `"*": "allow"`）
- maxOutputTokens 保持 128k（writer 需要呈现完整章节给用户）
- temperature: 0.7（适合创作性工作）

## 验证方式

```bash
# 启动 TUI
bun dev

# 切换到 writer agent
@writer

# 提供写作任务（示例）
写一本关于[主题]的书

# 验证流程：
# 1. writer 探索文件
# 2. writer 生成大纲
# 3. writer 逐章写作时调用 expander-fiction/nonfiction
# 4. 章节内容完整且有系统化框架支持
```

## 技术细节

### Writer → Expander 调用模式

```
用户请求
    ↓
writer (主入口)
    ├─ 探索文件 (Glob/Read)
    ├─ 生成大纲
    ├─ 展示给用户确认
    └─ 逐章写作
            ↓
        检测内容类型
            ↓
        ┌───────────────┴───────────────┐
        │                               │
    Fiction                          Non-Fiction
        │                               │
        ↓                               ↓
Task → expander-fiction            Task → expander-nonfiction
        │                               │
        └───────────────┬───────────────┘
                        ↓
                呈现完整章节给用户
                        ↓
                    等待反馈/继续下一章
```

## 完成状态

- [x] writer.txt 添加 expander 集成指令
- [x] 验证 expander 输出格式对齐
- [x] 确认 writer 有 Task 工具权限
- [x] 更新示例工作流程

## 后续建议

1. **测试验证**: 在实际使用中测试 writer → expander 的调用流程
2. **性能监控**: 观察调用的延迟和 token 消耗
3. **用户反馈**: 根据实际使用体验优化 prompt 指令
