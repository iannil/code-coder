# CodeCoder 文档标准

本文档定义 CodeCoder 项目的文档结构和编写规范。

## 文档分类

### 1. 进度文档 (`docs/progress/`)

记录正在进行的开发工作。

**命名规范**: `{YYYY-MM-DD}-{topic}.md`

**必需字段**:
```markdown
# 标题

> 开始时间: YYYY-MM-DD
> 状态: in_progress | completed | blocked

## 目标

## 进展

## 阻塞问题

## 下一步行动
```

### 2. 完成报告 (`docs/reports/completed/`)

记录已完成的验收报告。

**命名规范**: `{YYYY-MM-DD}-{feature-name}.md`

**必需字段**:
```markdown
# {Feature Name} 完成报告

> 完成时间: YYYY-MM-DD
> 执行者: {Agent/Developer}

## 变更摘要

## 验收结果

## 相关文件
```

### 3. 审查报告 (`docs/reports/reviews/`)

记录代码审查、安全审查、架构审查等评审结果。

**命名规范**: `{YYYY-MM-DD}-review-{subject}.md`

**必需字段**:
```markdown
# {Subject} 审查报告

> 审查时间: YYYY-MM-DD
> 审查者: {Name/Agent}
> 审查类型: code-review | security-review | architecture-review

## 概述
## 发现
## 统计
## 详细问题
## 正面发现
## 结论
```

## 文档模板

参见 `docs/templates/` 目录:
- `progress-report.md` - 进度报告模板
- `completion-report.md` - 完成报告模板
- `review-report.md` - 审查报告模板

## 命名规范

| 类型 | 规范 | 示例 |
|------|------|------|
| 进度文档 | `{YYYY-MM-DD}-{topic}.md` | `2026-02-05-tui-refactor.md` |
| 完成报告 | `{YYYY-MM-DD}-{feature}.md` | `2026-02-05-mcp-integration.md` |
| 验收报告 | `{YYYY-MM-DD}-review-{topic}.md` | `2026-02-05-review-security.md` |
| 指南文档 | `{topic}.md` | `api-integration.md` |
| 永久文档 | `{Title}-Guide.md` | `Architecture-Guide.md` |

## 元数据格式

所有文档开头应包含以下元数据（如适用）:

```markdown
> 文档类型: progress | report | guide | reference
> 创建时间: YYYY-MM-DD
> 更新时间: YYYY-MM-DD
> 状态: draft | active | deprecated
> 负责人: (可选)
```

## 文档生命周期

### 状态流转

```
docs/progress/ (in_progress)
       ↓
docs/reviews/ (peer review) ← 可选
       ↓
docs/reports/completed/ (completed)
       ↓
docs/archive/ (deprecated)
```

### 详细说明

1. **创建** (`docs/progress/`)
   - 在 `docs/progress/` 创建进度文档
   - 命名: `{YYYY-MM-DD}-{topic}.md`
   - 状态: `in_progress` 或 `draft`

2. **更新** (持续)
   - 每日更新进展，带上时间戳
   - 保持状态字段同步

3. **审查** (`docs/reports/reviews/`) - 可选
   - 进行同行审查或自动化审查
   - 生成审查报告

4. **完成** (`docs/reports/completed/`)
   - 功能完成后移动到 `docs/reports/completed/`
   - 命名: `{YYYY-MM-DD}-{feature}.md`
   - 状态: `completed`

5. **归档** (`docs/archive/`)
   - 过期内容移动到 `docs/archive/`
   - 保留历史参考

### 归档条件

文档符合以下条件之一应归档:
- 功能已被替代或重构
- 文档内容已过时
- 相关代码已删除
- 文档超过 1 年未更新且不反映当前状态

## 编写规范

- 使用中文进行交流和文档编写
- 代码使用英文
- 标题层级不超过 4 级
- 使用 Markdown 标准语法
- 添加有意义的链接
