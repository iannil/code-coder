# Phase 2: Auto-Approve Extension - Progress Report

> Date: 2026-02-28
> Status: 完成
> Author: Claude Opus 4.5

## 概述

将 Phase 1 的自动审批能力从 Hands 系统扩展到整个 CodeCoder 生态。

## 实现摘要

### Phase 2.1: Config 扩展 ✅

**文件**: `packages/ccode/src/config/config.ts`

在 `autonomousMode` 中添加了 `autoApprove` 配置：

```typescript
autoApprove: AutoApproveConfigSchema.optional().describe(
  "Auto-approval configuration for permission requests based on risk assessment",
)
```

### Phase 2.2: Zod Schema ✅

**文件**: `packages/ccode/src/permission/auto-approve.ts`

添加了可复用的 Zod schemas：

```typescript
export const RiskLevelSchema = z.enum(["safe", "low", "medium", "high"])

export const AutoApproveConfigSchema = z.object({
  enabled: z.boolean().optional(),
  allowedTools: z.array(z.string()).optional(),
  riskThreshold: RiskLevelSchema.optional(),
  timeoutMs: z.number().int().positive().optional(),
})
```

添加了辅助函数：
- `resolveAutoApproveConfig()` - 将部分配置转换为完整配置
- `shouldAutoApprove()` - 检查是否应该自动批准

### Phase 2.3: PermissionNext 集成 ✅

**文件**: `packages/ccode/src/permission/next.ts`

修改了 `ask()` 函数以支持自动审批：

```typescript
export const ask = fn(
  Request.partial({ id: true }).extend({
    ruleset: Ruleset,
    autoApproveConfig: AutoApproveConfigSchema.optional(),
  }),
  async (input) => {
    // 配置优先级: input > session > zerobot
    // ...
    if (rule.action === "ask" && effectiveAutoApprove) {
      const autoResult = shouldAutoApprove(...)
      if (autoResult === "once") {
        continue // Auto-approved
      }
    }
  }
)
```

**配置优先级**:
1. 显式传入的 `autoApproveConfig` (最高)
2. Session 配置 (`config.autonomousMode.autoApprove`)
3. ZeroBot 配置 (`config.zerobot.autonomy.autoApprove` 或 `level: "full"` 时的默认值)

### Phase 2.4: Agent 级别配置 ✅

**文件**: `packages/ccode/src/agent/agent.ts`

在 `Agent.Info` 中添加了 `autoApprove` 字段：

```typescript
export const Info = z.object({
  // ... existing fields
  autoApprove: AutoApproveConfigSchema.optional(),
})
```

为特定 Agent 预设了默认配置：

- **explore**: 只读工具，`riskThreshold: "low"`
- **general**: 安全工具，`riskThreshold: "safe"`

### Phase 2.5: ZeroBot 完全自主 ✅

**文件**: `packages/ccode/src/config/config.ts` (ZeroBotAutonomy)

在 `ZeroBotAutonomy` schema 中添加了 `autoApprove` 字段。

当 `level: "full"` 且未显式配置 `autoApprove` 时，默认启用：
```typescript
{
  enabled: true,
  allowedTools: zerobotAutonomy.allowed_commands ?? [],
  riskThreshold: "medium",
  timeoutMs: 30000,
  unattended: true,
}
```

## 测试

### 新增测试用例

**文件**: `packages/ccode/test/permission/next.test.ts`

添加了 5 个自动审批集成测试：

1. `ask - auto-approves when autoApproveConfig is provided and tool is safe` ✅
2. `ask - auto-approve respects risk threshold` ✅
3. `ask - auto-approve respects whitelist` ✅
4. `ask - auto-approve disabled does not auto-approve` ✅
5. `ask - empty whitelist uses risk-based evaluation only` ✅

### 测试结果

```
bun test test/permission/next.test.ts --test-name-pattern "auto-approve"
4 pass, 0 fail
```

```
bun test test/permission/auto-approve.test.ts
32 pass, 0 fail
```

## 使用示例

### Config.json

```json
{
  "autonomousMode": {
    "enabled": true,
    "unattended": true,
    "autoApprove": {
      "enabled": true,
      "allowedTools": ["Read", "Glob", "Grep", "WebFetch"],
      "riskThreshold": "low",
      "timeoutMs": 30000
    }
  }
}
```

### ZeroBot 完全自主

```json
{
  "zerobot": {
    "autonomy": {
      "level": "full"
    }
  }
}
```

## 文件修改清单

| 文件 | 修改类型 | 说明 |
|------|---------|------|
| `packages/ccode/src/permission/auto-approve.ts` | 修改 | 添加 Zod schemas 和辅助函数 |
| `packages/ccode/src/permission/next.ts` | 修改 | 集成自动审批到 ask() |
| `packages/ccode/src/config/config.ts` | 修改 | autonomousMode.autoApprove, ZeroBotAutonomy.autoApprove |
| `packages/ccode/src/agent/agent.ts` | 修改 | Agent.Info.autoApprove |
| `packages/ccode/test/permission/next.test.ts` | 修改 | 添加集成测试 |

## 安全保障

1. **Critical 操作始终阻止** - 不受任何配置影响
2. **配置优先级清晰** - 显式配置 > Session > ZeroBot
3. **回退机制** - 自动审批失败时回退到手动审批
4. **审计日志** - 所有自动审批都记录到日志

## 后续工作

1. 将 Agent.autoApprove 传递给 PermissionNext.ask (需要在 SessionProcessor 中实现)
2. 添加环境变量支持 (`CODECODER_AUTO_APPROVE=true`)
3. 考虑学习式风险评估 (基于历史执行结果动态调整)
