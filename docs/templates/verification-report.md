# {{Feature Name}} 验证报告

> 验收日期: {{YYYY-MM-DD}}
> Agent: verifier
> Session ID: {{session-id}}
> 相关需求: {{REQ-XXX}}

## 执行摘要

| 维度 | 状态 | 详情 |
|------|------|------|
| 前置条件 | {{status}} | {{X/Y}} 已验证 |
| 后置条件 | {{status}} | {{X/Y}} 已验证 |
| 不变量 | {{status}} | {{X 保持 / Y 违反}} |
| 属性 | {{status}} | {{X 已证明 / Y 被证伪}} |
| 测试覆盖率 | {{XX%}} | 目标: 80% |
| 验收标准 | {{status}} | {{X/Y}} 已满足 |

**最终判决**: {{PASS | PASS_WITH_WARNINGS | FAIL | BLOCKED}}

## 功能目标

### 目标: V-{{PROJECT}}-{{FEATURE}}-{{NNN}}

**需求追溯**: REQ-001, REQ-002
**测试追溯**: TC-001, TC-002

#### 前置条件

| ID | 形式化陈述 | 状态 | 证据 |
|----|-----------|------|------|
| P-001 | forall x: x != null | verified | test/login.test.ts:42 |
| P-002 | | | |

#### 后置条件

| ID | 形式化陈述 | 状态 | 证据 |
|----|-----------|------|------|
| Q-001 | exists token: isValid(token) | verified | test/login.test.ts:55 |
| Q-002 | | | |

#### 不变量

| ID | 不变量 | 作用域 | 状态 |
|----|--------|--------|------|
| I-001 | sorted(arr) implies forall i: arr[i] <= arr[i+1] | function | holds |
| I-002 | | | |

#### 数学属性

| ID | 属性 | 形式化陈述 | 状态 | 证明方法 |
|----|------|-----------|------|----------|
| PR-001 | 幂等性 | f(f(x)) = f(x) | proven | 属性测试 |
| PR-002 | 往返 | decode(encode(x)) = x | proven | 形式化证明 |
| PR-003 | | | | |

#### 验收标准

| ID | 标准 (SMART) | 阈值 | 实测 | 状态 |
|----|--------------|------|------|------|
| AC-001 | 响应时间 < 100ms (95%) | 100ms | 87ms | PASS |
| AC-002 | 错误率 < 0.1% | 0.1% | 0.05% | PASS |
| AC-003 | | | | |

## 覆盖率分析

### 需求-测试矩阵

| 需求ID | 测试用例 | 状态 | 覆盖度 |
|--------|----------|------|--------|
| REQ-001 | TC-001, TC-002 | PASS, PASS | 完整 |
| REQ-002 | TC-003 | FAIL | 无 |
| REQ-003 | | | |

**未覆盖需求**: {{list of uncovered requirements}}

## 数学证明

### 证明: {{Property Name}}

**定理**: {{形式化陈述}}

**证明**:
1. {{第一步}}
2. {{第二步}}
   2.1. 子步骤
   2.2. 子步骤
3. {{第三步}}
...
N. {{最终步骤}}

**QED**: {{结论}}

## 发现

### 严重（必须修复）

{{阻塞性问题列表}}

- **ISS-001**: {{标题}}
  - 类别: {{precondition_violation | postcondition_violation | invariant_violation | property_disproven}}
  - 描述: {{详细描述}}
  - 位置: {{file.ts:line}}
  - 建议: {{修复建议}}

### 重要（应当修复）

{{重要问题列表}}

- **ISS-002**: {{标题}}
  - 类别: {{acceptance_not_met | missing_test}}
  - 描述: {{详细描述}}
  - 建议: {{修复建议}}

### 建议（可以改进）

{{改进建议列表}}

- **ISS-003**: {{标题}}
  - 描述: {{改进建议}}

## 新生成测试

### 通过 tdd-guide 生成的测试

| 测试ID | 目标 | 文件路径 | 状态 |
|--------|------|----------|------|
| TC-NEW-001 | 覆盖 REQ-002 | test/feature.test.ts | PASS |
| TC-NEW-002 | | | |

## 附录

### A. 测试输出

```
{{原始测试输出}}
```

### B. 代码引用

| 位置 | 摘要 |
|------|------|
| file.ts:42 | {{相关代码}} |
| file.ts:123 | {{相关代码}} |

### C. 验证元数据

| 字段 | 值 |
|------|-----|
| 验收日期 | {{YYYY-MM-DD HH:MM:SS}} |
| 验收耗时 | {{duration}}ms |
| Session ID | {{session-id}} |
| 验收人 | verifier agent |
| 温度设置 | 0.1 |

---
*本报告由 CodeCoder Verifier Agent 自动生成*
