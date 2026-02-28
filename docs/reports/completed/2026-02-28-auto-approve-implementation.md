# Auto-Approve Permission Handler Implementation

> 日期: 2026-02-28
> 状态: 已完成

## 概述

实现了基于风险级别的自动审批功能，使 Hands 系统能够在无人值守模式下自主执行低/中风险操作。

## 实现内容

### Phase 1: 扩展 HAND.md 清单格式 ✅

**文件**: `services/zero-workflow/src/hands/manifest.rs`

新增类型:
- `RiskThreshold` 枚举: Safe, Low, Medium, High
- `AutoApproveConfig` 结构体: enabled, allowed_tools, risk_threshold, timeout_ms

扩展 `AutonomyConfig`:
- 新增 `auto_approve: Option<AutoApproveConfig>` 字段

### Phase 2: 创建 Auto-Approve Permission Handler ✅

**新文件**: `packages/ccode/src/permission/auto-approve.ts`

核心功能:
- `assessToolRisk(tool, input)`: 评估工具操作的风险级别
- `createAutoApproveHandler(config)`: 创建自动审批回调
- `getAuditLog()`: 获取审计日志

风险评估逻辑:
- 基于工具类型的基础风险 (Read=safe, Bash=high)
- Bash 命令模式检测 (sudo→critical, git push→high)
- 敏感文件检测 (.env, .pem→high)

### Phase 3: 集成 Guardrails Risk Assessment ✅

**文件**: `packages/ccode/src/autonomous/safety/integration.ts`

新增方法:
- `assessToolRisk(tool, input)`: 返回 risk, reason, autoApprovable
- `shouldAutoApprove(tool, input, riskThreshold)`: 检查是否应自动批准

### Phase 4: 扩展 Autonomous Bridge ✅

**文件**: `services/zero-workflow/src/hands/autonomous_bridge.rs`

新增类型:
- `BridgeRiskThreshold` 枚举
- `AutoApproveConfig` 结构体

扩展 `AutonomousRequest`:
- 新增 `auto_approve_config: Option<AutoApproveConfig>` 字段

更新 `execute_hand()`:
- 从 Hand manifest 构建 auto-approve 配置

### Phase 5: 扩展 Autonomous API Handler ✅

**文件**: `packages/ccode/src/api/server/handlers/autonomous.ts`

新增类型:
- `AutoApproveConfigInput` 接口

更新 `executeAutonomous()`:
- 接收 `autoApproveConfig` 参数
- 启用时设置 Permission handler

## 风险级别定义

| 级别 | 说明 | 工具示例 |
|------|------|----------|
| safe | 无副作用只读 | Read, Glob, Grep, LS |
| low | 外部只读 | WebFetch, WebSearch |
| medium | 本地可逆写入 | Write, Edit, NotebookEdit |
| high | 外部副作用/不可逆 | Bash (git push), Task |
| critical | 系统级/破坏性 | Bash (sudo, rm -rf /) |

## Bash 命令风险检测

Critical 模式:
- `sudo` - 需要提升权限
- `rm -rf /` - 根目录删除
- `shutdown/reboot/init` - 系统控制
- `git push --force` - 强制推送

High 模式:
- `git push` - 外部效果
- `git reset --hard` - 丢失更改
- `npm publish` - 包发布
- `curl -X POST/PUT/DELETE` - HTTP 变更

## HAND.md 示例

```yaml
autonomy:
  level: "crazy"
  unattended: true
  max_iterations: 3
  auto_approve:
    enabled: true
    allowed_tools:
      - "Read"
      - "Glob"
      - "Grep"
      - "WebFetch"
      - "WebSearch"
    risk_threshold: "low"
    timeout_ms: 30000
```

## 安全保障

1. **Critical 操作始终阻止** - 不受任何配置影响
2. **审计日志** - 所有自动审批决策都记录
3. **双重检查** - 白名单 + 风险阈值都必须通过
4. **超时机制** - 仅在 unattended 模式下生效

## 验证

- [x] TypeScript 类型检查通过
- [x] Rust 编译通过
- [x] 单元测试 (32 tests passing)
- [ ] 集成测试

## 文件修改清单

| 文件 | 修改类型 |
|------|---------|
| `services/zero-workflow/src/hands/manifest.rs` | 修改 |
| `packages/ccode/src/permission/auto-approve.ts` | 新建 |
| `packages/ccode/src/permission/index.ts` | 修改 |
| `packages/ccode/src/autonomous/safety/integration.ts` | 修改 |
| `services/zero-workflow/src/hands/autonomous_bridge.rs` | 修改 |
| `packages/ccode/src/api/server/handlers/autonomous.ts` | 修改 |
| `docs/hands-examples/market-sentinel/HAND.md` | 修改 |
