# Phase 4: Agent 精简为 3 模式 - CLI 集成完成报告

## 执行日期
2026-03-08

## 完成内容

### Step 1: 添加 `--mode` CLI 选项 ✅

**修改文件:**

1. `packages/ccode/src/cli/cmd/tui/thread.ts`
   - 添加 `--mode` 选项，支持 `build`, `writer`, `decision` 三种模式
   - 默认值为 `build`
   - 传递 mode 参数到 TUI args

2. `packages/ccode/src/cli/cmd/run.ts`
   - 添加 `--mode` 选项
   - 添加 `getMode` 导入
   - 更新 `resolvedAgent` 逻辑，根据 mode 设置默认 agent

3. `packages/ccode/src/cli/cmd/tui/context/args.tsx`
   - 扩展 `Args` 接口，添加 `mode?: string` 字段

### Step 2: 连接模式选择到 Agent 启动 ✅

**修改文件:**

1. `packages/ccode/src/cli/cmd/tui/context/local.tsx`
   - 添加 `getMode` 导入
   - 更新 agent 初始化逻辑，按优先级选择初始 agent:
     1. 显式指定的 `args.agent`
     2. 模式的主 agent (`mode.primaryAgent`)
     3. 第一个可用 agent

### Step 3: 支持 `@mode:capability` 语法 ✅

**修改文件:**

1. `packages/ccode/src/cli/cmd/tui/component/prompt/index.tsx`
   - 添加 `parseModeCapability`, `getMode`, `validateCapability` 导入
   - 在 `submit()` 函数中添加 `@mode:capability` 解析逻辑
   - 支持格式: `@build:security-review`, `@decision:macro` 等
   - 解析成功后切换到指定的 capability agent
   - 显示 toast 提示用户当前使用的 agent
   - 验证 capability 是否属于指定 mode

## 使用示例

```bash
# 模式切换 (CLI)
bun dev                      # 默认 @build 模式
bun dev --mode writer        # 写作模式
bun dev --mode decision      # 决策模式

# 与其他选项组合
bun dev --mode writer --model claude/claude-sonnet-4

# run 命令也支持
bun dev run "Review the code" --mode build

# TUI 内输入 (能力调用)
@build:security-review Check auth.ts for vulnerabilities
@decision:macro Analyze the PMI data
@writer:proofreader Check grammar in README.md
```

## 技术实现细节

### 模式定义 (mode.ts)

```typescript
// 3 个核心模式
MODES = {
  build: {
    primaryAgent: "build",
    alternativePrimaries: ["plan", "autonomous"],
    capabilities: ["code-reviewer", "security-reviewer", ...]
  },
  writer: {
    primaryAgent: "writer",
    capabilities: ["expander", "proofreader", ...]
  },
  decision: {
    primaryAgent: "decision",
    alternativePrimaries: ["observer"],
    capabilities: ["macro", "trader", ...]
  }
}
```

### @mode:capability 解析

```typescript
// 在 submit() 中
const modeCapMatch = trimmed.match(/^@(\w+):(\w+(?:-\w+)*)\s*/)
if (modeCapMatch) {
  const [fullMatch, modeId, capabilityName] = modeCapMatch
  const mode = getMode(modeId)
  if (mode && validateCapability(modeId, capabilityName)) {
    targetAgent = capabilityName
    inputToProcess = store.prompt.input.slice(fullMatch.length).trim()
  }
}
```

## 验证结果

### 功能测试

```
✅ getMode("build") 返回正确的模式定义
✅ getMode("invalid") 返回 undefined
✅ parseModeCapability("@build:security-review") 正确解析
✅ validateCapability("build", "security-reviewer") 返回 true
✅ validateCapability("build", "macro") 返回 false (macro 属于 decision)
```

### 类型检查

```
✅ 修改的文件无类型错误
⚠️ 存在预有的 observability 模块错误 (非本次修改引入)
```

## 文件变更清单

| 文件 | 操作 | 行数变化 |
|------|------|----------|
| `packages/ccode/src/cli/cmd/tui/thread.ts` | 修改 | +7 |
| `packages/ccode/src/cli/cmd/run.ts` | 修改 | +15 |
| `packages/ccode/src/cli/cmd/tui/context/args.tsx` | 修改 | +1 |
| `packages/ccode/src/cli/cmd/tui/context/local.tsx` | 修改 | +16 |
| `packages/ccode/src/cli/cmd/tui/component/prompt/index.tsx` | 修改 | +30 |

## 遗留问题

无

## 下一步

Phase 4 CLI 集成完成后，可以继续:
- Phase 5: 配置统一
- Phase 6: 文档更新 + 验收
