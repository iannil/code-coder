# TUI Session 执行过程详情增强

**完成时间**: 2026-02-13

## 背景

为 TUI 界面中的 session 执行过程添加更多细节信息，包括：
- 工具执行时间
- 进度指示器
- 实时状态（当前执行的工具、队列状态等）
- **模型接口调用状态**

## 实施内容

### Phase 1: 基础工具创建

#### 1.1 进度条组件
**文件**: `packages/ccode/src/cli/cmd/tui/ui/progress-bar.tsx` (新建)

**功能**:
- `ProgressBar` 组件 - 可复用的进度条
  - 支持自定义宽度、颜色、百分比显示
  - 支持动画效果
  - 可选标签显示
- `Spinner` 组件 - 简单的加载动画

#### 1.2 执行时间工具
**文件**: `packages/ccode/src/cli/cmd/tui/util/execution-time.ts` (新建)

**功能**:
- `formatExecutionTime(ms)` - 格式化执行时间（ms → 可读格式）
- `formatPreciseTime(ms)` - 精确时间格式化
- `getToolDuration(part)` - 获取工具执行时长和状态
- `getElapsedTimeFromPart(part)` - 从 Part 获取耗时
- `isToolRunning(part)` - 判断工具是否正在运行

### Phase 2: 工具行内联增强

**文件**: `packages/ccode/src/cli/cmd/tui/routes/session/index.tsx`

**修改内容**:
1. **InlineTool 组件增强**:
   - 运行中：显示 Spinner 动画 + 已用时间
   - 已完成：显示 ✓ + 总耗时
   - 错误：显示 ✗ + 失败信息

2. **Bash 工具增强**:
   - 标题显示执行时间
   - 运行中显示 "Running..." 状态

3. **Write 工具增强**:
   - 标题显示写入耗时

4. **Edit 工具增强**:
   - 标题显示编辑耗时

5. **Task 工具增强**:
   - 显示子任务进度条
   - 显示最近 5 个工具执行状态
   - 显示完成百分比

### Phase 3: Footer 状态栏增强

**文件**: `packages/ccode/src/cli/cmd/tui/routes/session/footer.tsx`

**新增内容**:
- 实时显示当前执行的工具名称
- 显示工具执行时间（实时更新）
- 显示待处理工具数量（+N more）
- 添加 Spinner 动画指示执行状态
- 每 100ms 更新一次时间
- **新增: 模型 API 调用状态显示**
  - `◈ Generating (2.3s)` - 模型正在生成响应
  - Token 使用统计（输入 + 输出 + 缓存）

**显示效果**:
```
~/projects/code-coder  ⠈ Generating (2.3s) | 12,345 tokens  |  △ 1 Permission  • 2 LSP  ⊙ 3 MCP  /status
                       ↑ model call status    ↑ token count
```

### Phase 4: Header 区域增强

**文件**: `packages/ccode/src/cli/cmd/tui/routes/session/header.tsx`

**新增内容**:
- 显示当前执行状态
- Streaming 状态显示 "Generating response..."
- 工具执行状态显示工具名 + 执行时间
- 添加 Spinner 动画

**显示效果**:
```
# Session Title          123,456 tokens  45%  ⠙ read (1.2s)  v0.1.0
```

### Phase 5: 模型接口调用状态

**新增组件**:

#### 5.1 StepStartPart 组件
**文件**: `packages/ccode/src/cli/cmd/tui/routes/session/index.tsx`

**功能**:
- 显示模型 API 调用开始的标记
- 显示实时执行时间（如 `2.3s`）
- 仅在显示时间戳时显示

**显示效果**:
```
◈ Model API call started (2.3s)
```

#### 5.2 StepFinishPart 组件
**文件**: `packages/ccode/src/cli/cmd/tui/routes/session/index.tsx`

**功能**:
- 显示模型 API 调用完成信息
- 显示 Token 使用统计：
  - 输入 tokens
  - 输出 tokens
  - 推理 tokens（如果使用）
  - 缓存命中率（如果有缓存）
- 显示完成原因（如 `stop`、`length` 等）
- 显示 API 调用成本

**显示效果**:
```
◈ Model API call completed - stop
Tokens: 12,345 (in: 8,192, out: 4,153), cache: ↦152 ↦0 (2% hit)
Cost: 0.0234¢
```

## 修改文件清单

1. **packages/ccode/src/cli/cmd/tui/ui/progress-bar.tsx** - 新建进度条组件
2. **packages/ccode/src/cli/cmd/tui/util/execution-time.ts** - 新建时间工具
3. **packages/ccode/src/cli/cmd/tui/routes/session/index.tsx** - 工具行内联增强 + 模型调用状态组件
4. **packages/ccode/src/cli/cmd/tui/routes/session/footer.tsx** - Footer 增强 + 模型调用状态
5. **packages/ccode/src/cli/cmd/tui/routes/session/header.tsx** - Header 增强

## 验证方式

1. 启动 TUI: `bun dev`
2. 执行包含多个工具的请求
3. 验证：
   - ✅ Header 显示当前执行的工具
   - ✅ Footer 显示实时状态和时间
   - ✅ Footer 显示模型调用状态和 token 统计
   - ✅ 工具行显示执行时间
   - ✅ StepStartPart/StepFinishPart 正确显示
   - ✅ Task 工具显示进度条

## 技术要点

### 实时更新机制
使用 Solid.js 的 `createSignal` 和 `setInterval` 实现每 100ms 更新：
```typescript
const [now, setNow] = createSignal(Date.now())

createEffect(() => {
  if (isRunning()) {
    const interval = setInterval(() => setNow(Date.now()), 100)
    return () => clearInterval(interval)
  }
})
```

### 工具状态判断
通过 `ToolState` 的 `status` 和 `time` 字段判断执行状态：
- `pending` - 待执行
- `running` - 执行中
- `completed` - 已完成
- `error` - 错误

### 时间计算
- 运行中: `Date.now() - time.start`
- 已完成: `time.end - time.start`
- 无时间数据: 返回 0

### 模型调用状态检测
- 检查 `lastAssistant.time.completed` 是否为空
- 检查 parts 中是否有 `step-start` 或 `step-finish` 类型的 Part
- 从 `lastAssistant.tokens` 读取实时 token 使用

## 数据结构

### StepStartPart
```typescript
{
  type: "step-start",
  id: string,
  sessionID: string,
  messageID: string,
  snapshot?: string
}
```

### StepFinishPart
```typescript
{
  type: "step-finish",
  id: string,
  sessionID: string,
  messageID: string,
  reason: string,          // 完成原因 (stop, length, etc.)
  snapshot?: string,
  cost: number,            // API 调用成本
  tokens: {
    input: number,
    output: number,
    reasoning: number,
    cache: { read: number, write: number }
  }
}
```

## 后续优化建议

1. **预估剩余时间**: 基于历史工具执行时间预估剩余时间
2. **更详细的进度信息**: 对于某些工具（如 websearch），可以显示更详细的进度
3. **工具队列显示**: 在 Footer 或 Sidebar 显示完整的工具执行队列
4. **性能优化**: 对于大量工具的场景，考虑节流/防抖处理
5. **Token 使用预测**: 基于当前 token 使用速率预测是否会超限
