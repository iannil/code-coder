# Review Report Template

> 文档类型: template
> 创建时间: 2025-01-29
> 用途: 代码审查和技术评审报告模板

## 元数据

```markdown
# {Review Subject} 审查报告

> 审查时间: YYYY-MM-DD
> 审查者: {Name/Agent}
> 审查类型: code-review | security-review | architecture-review | performance-review
> 审查范围: {Files/Modules}
```

## 报告结构

### 1. 概述 (Overview)

简要说明审查的目的、范围和方法。

```markdown
## 概述

### 审查目标

{本次审查的目标}

### 审查范围

- 文件: {List of files reviewed}
- 代码行数: {Approximate lines of code}
- 审查方法: {Manual analysis | Automated tools | Both}
```

### 2. 发现 (Findings)

按严重程度分类发现的问题。

```markdown
## 发现

### CRITICAL

{需要立即阻止发布的关键问题}

### HIGH

{应该尽快修复的重要问题}

### MEDIUM

{建议在下一个迭代修复的问题}

### LOW

{可以推迟的改进建议}

### INFO

{观察和建议，非问题}
```

### 3. 统计 (Statistics)

提供量化的审查结果。

```markdown
## 统计

| 指标 | 数值 |
|------|------|
| 审查文件数 | X |
| 发现问题数 | Y |
| CRITICAL | a |
| HIGH | b |
| MEDIUM | c |
| LOW | d |
```

### 4. 详细问题 (Detailed Issues)

对重要问题提供详细描述和建议。

```markdown
## 详细问题

### Issue #{N}: {Title}

**严重程度**: CRITICAL | HIGH | MEDIUM | LOW
**位置**: `{file}:{line}`
**类别**: Security | Performance | Maintainability | Correctness

#### 描述

{详细描述问题}

#### 建议

{如何修复}

#### 代码示例

```diff
- // 原始代码
+ // 修复后代码
```
```

### 5. 正面发现 (Positive Findings)

记录做得好的地方。

```markdown
## 正面发现

- {Good practice observed}
- {Well-designed component}
```

### 6. 结论 (Conclusion)

总结审查结果和后续行动。

```markdown
## 结论

### 总体评估

{整体质量评价}

### 后续行动

- [ ] {Action item 1}
- [ ] {Action item 2}

### 下次审查建议

{建议何时或如何进行下一次审查}
```

## 使用示例

```markdown
# Agent.ts 安全审查报告

> 审查时间: 2026-02-05
> 审查者: security-reviewer
> 审查类型: security-review
> 审查范围: packages/ccode/src/agent/

## 概述

### 审查目标

评估 Agent 模块的安全风险，包括输入验证、权限检查、敏感数据处理。

### 审查范围

- 文件: agent.ts, tool-registry.ts, prompt/
- 代码行数: ~1500
- 审查方法: 自动化工具 + 人工审查

...
```
