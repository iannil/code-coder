# Write/Edit/MultiEdit 工具大参数截断问题修复

**日期**: 2026-02-13
**状态**: 已完成

## 问题概述

当使用 autonomous agent 或其他 agent 为外部项目生成大文件时，Write 工具调用失败：

```
Error message: JSON Parse error: Unterminated string
```

### 问题场景

用户使用 code-coder 的 auto agent 为另一个项目（compliance-review-platform-lite）编写大型测试文件 `ServiceTests.java`，该文件包含：

- 600+ 行 Java 代码
- 包含完整的测试套件（AuthService、ProjectService、RuleService、StatisticsService）
- 预估文件大小约 20-30KB

当 agent 生成 Write 工具调用时：
1. 模型生成了完整的工具调用 JSON
2. 但 `content` 参数值过大
3. JSON 在传输/解析过程中被截断
4. 解析器报错：`Unterminated string`

---

## 根本原因分析

### 工具参数大小限制

这是一个**工具调用参数大小限制**问题，而非模型输出 token 限制问题。

| 问题类型 | Token 限制 | 参数大小限制 |
|---------|-----------|-------------|
| **发生位置** | 模型输出时 | 工具调用解析时 |
| **已有解决方案** | maxOutputTokens 配置 | 无 |
| **表现** | 输出被截断 | JSON 解析失败 |
| **错误类型** | 流式输出结束 | `Unterminated string` |

### 为什么 Bash 工具没有这个问题？

**Bash 工具**：
- 输出在命令执行**后**产生
- 可以在返回前截断并保存到文件
- 使用 `Truncate.output()` 处理大输出

**Write 工具**：
- `content` 是输入参数，在工具调用**前**由模型生成
- 如果太大，JSON 在到达工具前就被截断
- 无法在工具内部处理

---

## 修复方案

### 方案 1：Prompt 层面指导（已实施）

在工具描述中添加大文件处理指导，让 agent 自觉使用分批策略。

#### 修改的文件

1. **`packages/ccode/src/tool/write.txt`**
   ```markdown
   Large File Handling:
   - For large files (>500 lines or >20KB), use a two-step approach:
     1. Write the file structure and initial content using Write tool
     2. Use Edit tool for subsequent large additions
   - Alternative: Use Bash tool with echo/heredoc to write very large content
   - This prevents JSON parameter size limits from causing "Unterminated string" errors
   ```

2. **`packages/ccode/src/tool/edit.txt`**
   ```markdown
   Large Edit Handling:
   - For very large newString values (>5KB), consider splitting into multiple smaller edits
   - This prevents JSON parameter size limits from causing tool call failures
   - Alternative: Write large content to a temporary file, then use Edit for precise modifications
   ```

3. **`packages/ccode/src/tool/multiedit.txt`**
   ```markdown
   Large File Handling:
   - For very large newString values (>5KB), consider splitting into multiple smaller edits
   - This prevents JSON parameter size limits from causing tool call failures
   - Alternative: Use Write tool for initial content, then Edit for modifications
   ```

#### 优点
- ✅ 不增加代码复杂度
- ✅ 教会 agent 正确的处理方式
- ✅ 适用于所有类似场景

#### 缺点
- ❌ 依赖 agent 遵守指导
- ❌ 不是硬性限制

---

## 测试验证

### 手动测试

1. 启动 code-coder：
   ```bash
   cd /Users/iannil/Code/zproducts/code-coder
   bun dev
   ```

2. 使用 autonomous agent 生成大文件：
   ```
   > @autonomous 为 compliance-review-platform-lite 项目编写完整的 ServiceTests.java 测试文件
   ```

3. 验证：
   - Agent 应使用分批策略
   - 或使用 Bash 工具写入大文件
   - 不应出现 "Unterminated string" 错误

---

## 相关问题

### 与 Autonomous Truncation 修复的区别

| 修复 | 日期 | 问题 | 解决方案 |
|------|------|------|----------|
| **Autonomous Truncation** | 2026-02-13 | Thinking mode + maxOutputTokens 冲突 | 禁用 thinking，改进计算逻辑 |
| **Write Tool Truncation** | 2026-02-13 | 工具参数 JSON 大小限制 | Prompt 指导分批策略 |

### 相关文档

- `docs/progress/2026-02-13-autonomous-truncation-fix.md` - Autonomous agent thinking 模式问题
- `docs/progress/2026-02-12-writer-truncation-fix.md` - Writer agent 输出限制问题

---

## 后续可选改进

### 方案 2：代码层面检测和警告（未实施）

在工具执行前检测参数大小，给出明确警告：

```typescript
// packages/ccode/src/tool/write.ts
const MAX_CONTENT_SIZE = 20 * 1024 // 20KB

if (params.content.length > MAX_CONTENT_SIZE) {
  return {
    title: path.relative(Instance.worktree, filepath),
    output: `Content too large (${params.content.length} bytes). Please use Edit tool for large additions or split into multiple writes.`,
  }
}
```

**缺点**：
- 无法阻止 JSON 解析失败（错误发生在工具调用前）
- 只能在成功解析后给出提示

### 方案 3：间接文件写入机制（未实施）

新增工具参数，支持从文件读取内容：

```typescript
parameters: z.object({
  content: z.string().optional(),
  contentFromFile: z.string().optional().describe("Read content from this file path instead"),
  // ...
})
```

**优点**：
- 完全绕过 JSON 大小限制

**缺点**：
- 增加复杂度
- 需要两步操作（先写临时文件，再移动）

---

## 修改文件清单

| 文件 | 修改类型 | 状态 |
|------|----------|------|
| `packages/ccode/src/tool/write.txt` | 添加大文件处理指导 | ✅ 完成 |
| `packages/ccode/src/tool/edit.txt` | 添加大编辑处理指导 | ✅ 完成 |
| `packages/ccode/src/tool/multiedit.txt` | 添加大文件处理指导 | ✅ 完成 |
| `memory/MEMORY.md` | 记录决策和经验 | ✅ 完成 |
| `docs/progress/2026-02-13-write-tool-truncation-fix.md` | 本文档 | ✅ 完成 |

---

## 总结

本次修复通过 **Prompt 工程** 解决了工具参数大小限制导致的 JSON 解析失败问题。

**核心思路**：
1. 在工具描述中明确告知参数大小限制
2. 提供替代方案（分批写入、使用 Bash 工具）
3. 让 agent 学会正确处理大文件的策略

**与之前修复的区别**：
- Autonomous truncation: 修改了配置和 token 计算逻辑
- Writer truncation: 增加了 maxOutputTokens 配置
- **Write tool truncation**: 依靠 prompt 指导（因为无法在代码层面阻止）

Agent 现在应该能够：
- 识别即将生成的大文件
- 主动使用分批策略
- 或使用 Bash 工具的 echo/heredoc 写入大内容
- 避免 "Unterminated string" 错误
