# TUI Text 渲染错误修复报告

## 修复时间
2026-02-14

## 问题描述
TUI 在渲染时出现致命错误：
```
TextNodeRenderable only accepts strings, TextNodeRenderable instances, or StyledText instances
```

错误发生在 `packages/ccode/src/cli/cmd/tui/routes/session/index.tsx`，具体是在 SDK 事件处理和 SolidJS 渲染阶段。

## 问题根因

@opentui 的 `<text>` 元素不支持嵌套另一个 `<text>` 元素。代码中存在三处这样的嵌套问题：

1. **ToolTitle 组件** - 在 `<Show>` 的 `fallback` 属性中使用了 `<text>` 元素
2. **InlineTool 组件** - 在 `<Show>` 的 `fallback` 属性中使用了 `<text>` 元素
3. **Spinner 组件** - 返回 `<text>` 元素，但被用在 InlineTool 的 `<text>` 元素内部

## 修复内容

### 文件修改

#### 1. packages/ccode/src/cli/cmd/tui/routes/session/index.tsx

**ToolTitle 组件 (第 1538 行)**
```tsx
// 修改前
<Show fallback={<text>~ {safeText(props.fallback)}</text>} when={props.when}>

// 修改后
<Show fallback={<>~ {safeText(props.fallback)}</>} when={props.when}>
```

**InlineTool 组件 (第 1644 行)**
```tsx
// 修改前
<Show fallback={<text>~ {safePending()}</text>} when={props.complete || isRunning()}>

// 修改后
<Show fallback={<>~ {safePending()}</>} when={props.complete || isRunning()}>
```

**InlineTool 中的 Spinner 调用 (第 1646 行)**
```tsx
// 修改前
<Spinner color={props.iconColor ?? theme.accent} />

// 修改后
<Spinner color={props.iconColor ?? theme.accent} inline={true} />
```

#### 2. packages/ccode/src/cli/cmd/tui/ui/progress-bar.tsx

修改 Spinner 组件，添加 `inline` 属性以支持在 `<text>` 元素内部使用：

```tsx
export interface SpinnerProps {
  color?: RGBA
  /** When true, renders as <span> for use inside <text> elements. When false (default), renders as <text> */
  inline?: boolean
}

export function Spinner(props: SpinnerProps) {
  const { theme } = useTheme()
  const color = () => props.color ?? theme.accent

  const animationFrames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
  const frameIndex = () => Math.floor(Date.now() / 80) % animationFrames.length
  const frame = () => animationFrames[frameIndex()]

  // Use span when inline (inside text elements) to avoid nesting text elements
  return props.inline ? (
    <span style={{ fg: color() }}>{frame()}</span>
  ) : (
    <text fg={color()}>{frame()}</text>
  )
}
```

#### 3. packages/ccode/src/cli/cmd/tui/util/safe-text.ts

增强 `safeText()` 函数的防御性检查：
- 明确处理数组类型
- 明确处理对象类型
- 添加最终 fallback 处理未知类型

```typescript
export function safeText(value: unknown): string {
  if (value === null || value === undefined) return ""
  if (typeof value === "string") return value
  if (typeof value === "number" || typeof value === "boolean") return String(value)
  if (value instanceof Error) return value.message
  if (Array.isArray(value)) {
    try {
      return JSON.stringify(value)
    } catch {
      return "[Array]"
    }
  }
  if (typeof value === "object") {
    try {
      return JSON.stringify(value)
    } catch {
      return "[Object]"
    }
  }
  try {
    return String(value)
  } catch {
    return "[Unknown]"
  }
}
```

## 验证方式
1. 运行 TUI: `cd packages/ccode && bun dev`
2. 执行各种工具操作 (Read, Write, Edit, Bash, Glob 等)
3. 观察是否还有渲染错误

## 状态
已完成
