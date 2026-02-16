# Web 端 shadcn/ui 组件标准化 - 完成报告

## 完成时间

2026-02-16

## 任务概述

将 CodeCoder Web 包中的自定义组件实现替换为 shadcn/ui 标准组件,确保一致的视觉风格、更好的可访问性和减少维护成本。

## 已完成的工作

### Phase 1: 安装 shadcn 组件

**新增依赖:**
- `@radix-ui/react-collapsible`
- `@radix-ui/react-select`
- `@radix-ui/react-alert-dialog`
- `@radix-ui/react-switch`
- `@radix-ui/react-tooltip`
- `cmdk`

**新增 UI 组件文件:**
| 文件 | 描述 |
|------|------|
| `src/components/ui/Badge.tsx` | Badge 组件,支持 10 种 variant |
| `src/components/ui/Command.tsx` | Command 命令面板组件 |
| `src/components/ui/Collapsible.tsx` | 可折叠组件 |
| `src/components/ui/Select.tsx` | 下拉选择组件 |
| `src/components/ui/AlertDialog.tsx` | 确认对话框组件 |
| `src/components/ui/Switch.tsx` | 开关组件 |
| `src/components/ui/Tooltip.tsx` | 悬停提示组件 |
| `src/components/ui/Skeleton.tsx` | 加载骨架屏组件 |

### Phase 2: 组件重构

#### 2.1 CommandPalette → shadcn Command
- **文件:** `src/components/command/CommandPalette.tsx`
- **改动:** 使用 `CommandDialog`, `CommandInput`, `CommandList`, `CommandGroup`, `CommandItem`, `CommandShortcut`
- **效果:** 代码减少约 50%,保留完整功能

#### 2.2 Sidebar CollapsibleSection → shadcn Collapsible
- **文件:** `src/components/layout/Sidebar.tsx`
- **改动:** 使用 `Collapsible`, `CollapsibleTrigger`, `CollapsibleContent`
- **效果:** 移除自定义展开/折叠逻辑

#### 2.3 ThemeToggle → DropdownMenu
- **文件:** `src/components/theme/ThemeToggle.tsx`
- **改动:** 使用 `DropdownMenu`, `DropdownMenuTrigger`, `DropdownMenuContent`, `DropdownMenuItem`
- **效果:** 移除自定义 click-outside 逻辑

#### 2.4 Badge 组件标准化
- **涉及文件:**
  - `src/components/agent/AgentSelector.tsx` - AgentModeBadge
  - `src/components/hooks/HooksPanel.tsx` - LifecycleBadge
  - `src/components/lsp/LspPanel.tsx` - SeverityBadge
  - `src/pages/Settings.tsx` - 状态徽章
  - `src/pages/Documents.tsx` - StatusBadge
- **改动:** 统一使用 shadcn Badge 组件
- **Badge variants:** `default`, `secondary`, `destructive`, `outline`, `success`, `warning`, `info`, `purple`, `pink`, `orange`

#### 2.5 Select 组件替换
- **文件:** `src/pages/Settings.tsx` (Permission Settings)
- **改动:** 使用 `Select`, `SelectTrigger`, `SelectContent`, `SelectItem`
- **效果:** 统一风格,更好的可访问性

#### 2.6 AlertDialog 替换 confirm()
- **涉及文件:**
  - `src/components/storage/StoragePanel.tsx`
  - `src/pages/Documents.tsx`
- **改动:** 使用 `AlertDialog`, `AlertDialogTrigger`, `AlertDialogContent`
- **效果:** 更好的视觉反馈和用户体验

#### 2.7 Skeleton 统一加载状态
- **涉及文件:**
  - `src/pages/Settings.tsx`
  - `src/pages/Documents.tsx`
  - `src/pages/Files.tsx`
  - `src/pages/Dashboard.tsx`
  - `src/components/hooks/HooksPanel.tsx`
  - `src/components/lsp/LspPanel.tsx`
  - `src/components/memory/MemoryPanel.tsx`
  - `src/components/message/MessageList.tsx`
- **改动:** 使用 shadcn `Skeleton` 替换 `animate-pulse` + 自定义 div
- **效果:** 统一加载状态样式

## 文件变更清单

| 文件路径 | 变更类型 |
|----------|----------|
| `packages/web/package.json` | 依赖更新 |
| `packages/web/src/components/ui/Badge.tsx` | 新增 |
| `packages/web/src/components/ui/Command.tsx` | 新增 |
| `packages/web/src/components/ui/Collapsible.tsx` | 新增 |
| `packages/web/src/components/ui/Select.tsx` | 新增 |
| `packages/web/src/components/ui/AlertDialog.tsx` | 新增 |
| `packages/web/src/components/ui/Switch.tsx` | 新增 |
| `packages/web/src/components/ui/Tooltip.tsx` | 新增 |
| `packages/web/src/components/ui/Skeleton.tsx` | 新增 |
| `packages/web/src/components/command/CommandPalette.tsx` | 重构 |
| `packages/web/src/components/layout/Sidebar.tsx` | 修改 |
| `packages/web/src/components/theme/ThemeToggle.tsx` | 修改 |
| `packages/web/src/components/agent/AgentSelector.tsx` | 修改 |
| `packages/web/src/components/hooks/HooksPanel.tsx` | 修改 |
| `packages/web/src/components/lsp/LspPanel.tsx` | 修改 |
| `packages/web/src/components/memory/MemoryPanel.tsx` | 修改 |
| `packages/web/src/components/message/MessageList.tsx` | 修改 |
| `packages/web/src/components/storage/StoragePanel.tsx` | 修改 |
| `packages/web/src/pages/Settings.tsx` | 修改 |
| `packages/web/src/pages/Documents.tsx` | 修改 |
| `packages/web/src/pages/Files.tsx` | 修改 |
| `packages/web/src/pages/Dashboard.tsx` | 修改 |

## 验证结果

- **TypeScript 检查:** 通过 ✅
- **依赖安装:** 成功 ✅

## 保留的 animate-pulse 用法

以下位置保留了 `animate-pulse`,因为它们是合理的用法:
1. `Session.tsx:388` - 在线状态指示器(绿色脉冲圆点)
2. `Skeleton.tsx:4` - Skeleton 组件本身的实现
3. `Markdown.tsx:321` - Markdown 渲染的加载回退

## 后续建议

1. **视觉检查:** 启动 dev server,检查所有页面组件渲染正常
2. **交互测试:**
   - 命令面板 (Cmd+K) 打开/搜索/选择
   - Sidebar 折叠/展开
   - 主题切换
   - Settings 页面各项功能
   - Documents 页面删除确认对话框
3. **可访问性:** 测试键盘导航、屏幕阅读器支持
