# Web 端功能缺失实现报告

**日期**: 2026-02-16
**状态**: 已完成
**更新时间**: 2026-02-16 (P3 功能完成)

## 概述

根据 `docs/reports/2026-02-16-feature-catalog.md` 文档中的功能缺失评估，实现了 Web 端 P0、P1 和 P2 级功能，包括模型选择器、提供商管理、MCP 管理、会话操作增强、权限配置完善，以及文档写作、记忆系统、钩子配置和 LSP 配置的后端 API。

## 阶段一: P0 核心功能

### 1. 后端 API 端点

#### Provider API (`handlers/provider.ts`)
- `GET /api/providers` - 获取所有提供商及其连接状态
- `GET /api/providers/connected` - 获取已连接的提供商
- `GET /api/providers/auth` - 获取所有提供商的认证方法
- `GET /api/providers/:providerId` - 获取特定提供商详情
- `GET /api/providers/:providerId/models` - 获取特定提供商的模型列表

#### MCP API (`handlers/mcp.ts`)
- `GET /api/mcp/status` - 获取所有 MCP 服务器状态
- `GET /api/mcp/tools` - 获取已连接 MCP 服务器提供的工具
- `GET /api/mcp/resources` - 获取已连接 MCP 服务器的资源
- `POST /api/mcp/:name/connect` - 连接 MCP 服务器
- `POST /api/mcp/:name/disconnect` - 断开 MCP 服务器
- `POST /api/mcp/:name/toggle` - 切换 MCP 服务器状态
- `GET /api/mcp/:name/auth-status` - 获取 MCP 认证状态
- `POST /api/mcp/:name/auth/start` - 启动 OAuth 认证流程
- `POST /api/mcp/:name/auth/finish` - 完成 OAuth 认证

### 2. 前端类型定义 (`lib/types.ts`)

新增类型:
- `ProviderModel` - 模型信息
- `ProviderInfo` - 提供商信息
- `ProviderListResponse` - 提供商列表响应
- `ProviderAuthMethod` - 认证方法
- `ModelSelection` - 模型选择
- `McpStatus` - MCP 状态（支持多种状态类型）
- `McpTool` - MCP 工具
- `McpResource` - MCP 资源
- `McpAuthStatus` - MCP 认证状态

### 3. API 客户端扩展 (`lib/api.ts`)

新增方法:
- Provider 相关: `listProviders`, `listConnectedProviders`, `getProviderAuthMethods`, `getProvider`, `getProviderModels`
- MCP 相关: `getMcpStatus`, `getMcpTools`, `getMcpResources`, `connectMcp`, `disconnectMcp`, `toggleMcp`, `getMcpAuthStatus`, `startMcpAuth`, `finishMcpAuth`

### 4. 状态管理 (Zustand Stores)

#### Provider Store (`stores/provider.ts`)
- 管理提供商列表和连接状态
- 模型选择状态
- 收藏和最近使用历史
- 本地存储持久化

#### MCP Store (`stores/mcp.ts`)
- MCP 服务器状态管理
- 工具和资源列表
- 连接/断开操作

### 5. UI 组件

#### Model Selector (`components/model/ModelSelector.tsx`)
- 模型选择对话框
- 支持模糊搜索
- 收藏功能
- 最近使用历史
- 按提供商分组显示
- `ModelSelectorTrigger` 触发按钮组件

#### Settings 页面更新 (`pages/Settings.tsx`)
新增标签页:
- **Providers** - 显示所有提供商及连接状态
- **MCP** - MCP 服务器管理，支持启用/禁用

配置表单更新:
- `model` - 默认模型
- `small_model` - 小模型（用于标题生成等）
- `default_agent` - 默认 Agent
- `username` - 显示名称
- `theme` - 主题
- `logLevel` - 日志级别

---

## 阶段二: P1 重要功能

### 1. Session 页面增强 (`pages/Session.tsx`)

#### 会话分叉功能
- 在会话菜单中添加 "Fork session" 选项
- 调用 `api.forkSession()` 创建分叉
- 分叉后自动跳转到新会话
- 显示分叉标识（GitFork 图标）

#### 模型选择器集成
- 在 Agent 选择栏旁添加 `ModelSelectorTrigger`
- 支持在会话中快速切换模型
- 页面加载时自动获取提供商列表

### 2. 消息详情面板 (`components/message/MessageDetails.tsx`)

可展开的详情面板，显示:
- **Model** - 提供商和模型 ID
- **Token Usage** - 输入/输出/推理 token 统计，缓存读写
- **Cost** - 成本显示
- **Timing** - 开始时间、完成时间、持续时间
- **Context** - 工作目录路径
- **Finish reason** - 完成原因

### 3. 权限配置增强 (`pages/Settings.tsx`)

#### 支持的权限类型 (11 种)
- `Read` - 读取文件
- `Edit` - 编辑文件
- `Write` - 创建文件
- `Glob` - 文件搜索
- `Grep` - 内容搜索
- `Bash` - 执行命令
- `Task` - 启动子代理
- `WebFetch` - 获取网页
- `WebSearch` - 网页搜索
- `MCP` - 使用 MCP 工具
- `NotebookEdit` - 编辑 Jupyter

#### 规则管理功能
- 添加新规则（选择类型 + 可选模式匹配）
- 删除规则
- 快速切换规则动作 (allow/prompt/deny)
- 颜色编码的动作状态
- 权限类型说明图例

---

## 阶段三: P2 扩展功能

### 1. 文档/写作系统 API (`handlers/document.ts`)

完整的文档写作系统后端支持:

#### Document API
- `GET /api/documents` - 获取文档列表
- `GET /api/documents/:id` - 获取文档详情
- `POST /api/documents` - 创建文档
- `PUT /api/documents/:id` - 更新文档
- `DELETE /api/documents/:id` - 删除文档
- `GET /api/documents/:id/stats` - 获取文档统计
- `GET /api/documents/:id/export` - 导出文档 (Markdown/HTML)

#### Chapter API
- `GET /api/documents/:id/chapters` - 获取章节列表
- `GET /api/documents/:id/chapters/:chapterId` - 获取章节详情
- `PUT /api/documents/:id/chapters/:chapterId` - 更新章节

#### Entity API
- `GET /api/documents/:id/entities` - 获取实体列表
- `POST /api/documents/:id/entities` - 创建实体
- `PUT /api/documents/:id/entities/:entityId` - 更新实体
- `DELETE /api/documents/:id/entities/:entityId` - 删除实体

#### Volume API
- `GET /api/documents/:id/volumes` - 获取卷列表
- `POST /api/documents/:id/volumes` - 创建卷

### 2. 记忆系统 API (`handlers/memory.ts`)

#### Daily Notes (流层)
- `GET /api/memory/daily` - 获取每日笔记日期列表
- `GET /api/memory/daily/:date` - 获取特定日期的笔记
- `POST /api/memory/daily` - 追加今日笔记

#### Long-term Memory (沉积层)
- `GET /api/memory/long-term` - 获取长期记忆内容
- `GET /api/memory/sections` - 获取记忆分类
- `PUT /api/memory/category/:category` - 更新分类内容
- `POST /api/memory/category/:category/merge` - 合并内容到分类

#### Consolidation
- `GET /api/memory/consolidation/stats` - 获取整合统计
- `POST /api/memory/consolidation` - 触发记忆整合
- `GET /api/memory/summary` - 获取记忆摘要

### 3. 钩子系统 API (`handlers/hooks.ts`)

- `GET /api/hooks` - 获取所有配置的钩子
- `GET /api/hooks/:lifecycle` - 按生命周期获取钩子
- `GET /api/hooks/settings` - 获取钩子设置
- `GET /api/hooks/locations` - 获取配置文件位置
- `GET /api/hooks/action-types` - 获取可用动作类型

支持的生命周期: `PreToolUse`, `PostToolUse`, `PreResponse`, `Stop`

### 4. LSP 配置 API (`handlers/lsp.ts`)

#### 状态与配置
- `GET /api/lsp/status` - 获取 LSP 服务器状态
- `GET /api/lsp/diagnostics` - 获取诊断信息
- `GET /api/lsp/config` - 获取 LSP 配置
- `GET /api/lsp/available` - 检查文件的 LSP 可用性
- `POST /api/lsp/init` - 初始化 LSP
- `POST /api/lsp/touch` - 触发文件分析

#### LSP 操作
- `POST /api/lsp/hover` - 获取悬停信息
- `POST /api/lsp/definition` - 跳转到定义
- `POST /api/lsp/references` - 查找引用
- `POST /api/lsp/workspace-symbols` - 工作区符号搜索
- `POST /api/lsp/document-symbols` - 文档符号

### 5. 前端类型定义 (`lib/types.ts`)

新增 P2 类型:

**Document Types:**
- `DocumentStatus`, `ChapterStatus`, `EntityType`
- `DocumentOutline`, `DocumentOutlineChapter`
- `DocumentStyleGuide`, `DocumentGlobalSummary`
- `DocumentMetadata`, `DocumentChapter`
- `DocumentEntity`, `DocumentVolume`, `DocumentStats`

**Memory Types:**
- `DailyEntryType`, `DailyEntry`
- `MemoryCategory`, `MemorySection`
- `MemorySummary`, `ConsolidationStats`

**Hooks Types:**
- `HookLifecycle`, `HookActionType`
- `HookAction`, `HookDefinition`, `HookEntry`
- `HookSettings`, `HookLocation`, `HookActionTypeInfo`

**LSP Types:**
- `LspServerStatus`, `LspStatus`
- `LspRange`, `LspDiagnostic`, `LspFileDiagnostics`
- `LspConfig`, `LspSymbol`, `LspDocumentSymbol`, `LspLocation`

### 6. API 客户端扩展 (`lib/api.ts`)

新增方法:

**Document APIs:**
- `listDocuments`, `getDocument`, `createDocument`, `updateDocument`, `deleteDocument`
- `getDocumentStats`, `exportDocument`
- `listChapters`, `getChapter`, `updateChapter`
- `listEntities`, `createEntity`, `updateEntity`, `deleteEntity`
- `listVolumes`, `createVolume`

**Memory APIs:**
- `listDailyDates`, `getDailyNotes`, `appendDailyNote`
- `getLongTermMemory`, `getMemorySections`
- `updateMemoryCategory`, `mergeToMemoryCategory`
- `getConsolidationStats`, `triggerConsolidation`, `getMemorySummary`

**Hooks APIs:**
- `listHooks`, `getHooksByLifecycle`
- `getHooksSettings`, `getHookLocations`, `getHookActionTypes`

**LSP APIs:**
- `getLspStatus`, `getLspDiagnostics`, `getLspConfig`
- `checkLspAvailable`, `initLsp`, `touchLspFile`
- `getLspHover`, `getLspDefinition`, `getLspReferences`
- `getLspWorkspaceSymbols`, `getLspDocumentSymbols`

---

## 文件变更清单

### 阶段一新增文件
```
packages/ccode/src/api/server/handlers/provider.ts
packages/ccode/src/api/server/handlers/mcp.ts
packages/web/src/stores/provider.ts
packages/web/src/stores/mcp.ts
packages/web/src/components/model/ModelSelector.tsx
packages/web/src/components/model/index.ts
```

### 阶段二新增文件
```
packages/web/src/components/message/MessageDetails.tsx
```

### 阶段三新增文件
```
packages/ccode/src/api/server/handlers/document.ts
packages/ccode/src/api/server/handlers/memory.ts
packages/ccode/src/api/server/handlers/hooks.ts
packages/ccode/src/api/server/handlers/lsp.ts
packages/web/src/stores/document.ts
packages/web/src/stores/memory.ts
packages/web/src/stores/hooks.ts
packages/web/src/stores/lsp.ts
packages/web/src/components/memory/MemoryPanel.tsx
packages/web/src/components/memory/index.ts
packages/web/src/components/hooks/HooksPanel.tsx
packages/web/src/components/hooks/index.ts
packages/web/src/components/lsp/LspPanel.tsx
packages/web/src/components/lsp/index.ts
packages/web/src/pages/Documents.tsx
```

### 修改文件
```
packages/ccode/src/api/server/router.ts    - 添加新路由 (P0-P2)
packages/web/src/lib/types.ts              - 添加新类型 (P0-P2)
packages/web/src/lib/api.ts                - 添加 API 方法 (P0-P2)
packages/web/src/stores/index.ts           - 导出新 store (P0-P2)
packages/web/src/pages/Settings.tsx        - Provider/MCP/权限/Memory/Hooks/LSP 配置
packages/web/src/pages/Session.tsx         - 分叉功能、模型选择器
packages/web/src/components/message/MessageItem.tsx - 集成详情面板
packages/web/src/router.ts                 - 添加 /documents 路由
packages/web/src/App.tsx                   - 添加 Documents 导航链接
```

## 验证

- TypeScript 类型检查通过 (`npx tsc --noEmit`)
- 所有新代码遵循项目代码风格指南

## P2 功能完成情况

### 已完成
1. ✅ 记忆系统可视化组件 (`MemoryPanel`)
   - 每日笔记浏览
   - 长期记忆查看
   - 整合控制面板
2. ✅ 钩子配置界面组件 (`HooksPanel`)
   - 按生命周期分组的钩子列表
   - 钩子设置查看
   - 配置文件位置
   - Action 类型参考
3. ✅ LSP 配置界面组件 (`LspPanel`)
   - 服务器状态监控
   - 诊断信息展示
   - LSP 配置查看
4. ✅ 文档写作界面 (`Documents` 页面)
   - 文档列表（创建/删除）
   - 章节导航
   - 章节内容编辑
   - 实体管理（按类型分组）
   - 统计面板（进度/章节/剩余字数）
   - `/documents` 独立路由

## 下一步建议

### P3 级功能
1. 键绑定配置
2. 主题选择
3. 存储管理
4. 可观测性面板
5. 命令面板

---

## 阶段四: P3 扩展功能

### 1. 主题系统

#### ThemeProvider (`hooks/use-theme.tsx`)
- 支持 light/dark/system 三种模式
- 自动同步系统主题偏好
- localStorage 持久化

#### ThemeToggle (`components/theme/ThemeToggle.tsx`)
- 下拉菜单切换主题
- 显示当前主题图标

#### ThemeSelector
- 按钮组选择器（用于 Settings 页面）

### 2. 命令面板 (`components/command/CommandPalette.tsx`)

#### 特性
- VS Code 风格命令面板
- `Cmd/Ctrl + K` 快捷键打开
- 模糊搜索支持
- 键盘导航 (↑↓ Enter Esc)

#### 命令分类
- **Navigation**: 跳转到各页面
- **Session**: 新建/刷新会话
- **Theme**: 切换主题

#### 集成
- 最近 5 个会话快速跳转
- 与 ThemeProvider 集成

### 3. 存储管理 (`components/storage/StoragePanel.tsx`)

#### 信息展示
- 会话数量
- localStorage 使用量
- 健康状态

#### 缓存管理
- 刷新数据
- 清除缓存（保留主题偏好）

#### 存储详情
- 按大小排序的 localStorage 条目
- 单条删除功能

### 4. Settings 页面更新

新增标签页:
- **Appearance**: 主题选择
- **Storage**: 存储管理

---

## P3 新增文件
```
packages/web/src/hooks/use-theme.tsx
packages/web/src/components/theme/ThemeToggle.tsx
packages/web/src/components/theme/index.ts
packages/web/src/components/command/CommandPalette.tsx
packages/web/src/components/command/index.ts
packages/web/src/components/storage/StoragePanel.tsx
packages/web/src/components/storage/index.ts
```

## P3 修改文件
```
packages/web/src/main.tsx           - 添加 ThemeProvider, CommandPaletteProvider
packages/web/src/App.tsx            - 添加 ThemeToggle 到 header
packages/web/src/pages/Settings.tsx - 添加 Appearance, Storage 标签页
```

---

## 完成总结

| 优先级 | 功能 | 状态 |
|--------|------|------|
| P0 | 模型选择器 | ✅ |
| P0 | 提供商管理 | ✅ |
| P0 | MCP 管理 | ✅ |
| P1 | 会话分叉 | ✅ |
| P1 | 消息详情 | ✅ |
| P1 | 权限配置 | ✅ |
| P2 | 文档写作系统 API | ✅ |
| P2 | 记忆系统 API | ✅ |
| P2 | 钩子系统 API | ✅ |
| P2 | LSP API | ✅ |
| P2 | Memory Panel | ✅ |
| P2 | Hooks Panel | ✅ |
| P2 | LSP Panel | ✅ |
| P2 | Documents 页面 | ✅ |
| P3 | 主题选择 | ✅ |
| P3 | 命令面板 | ✅ |
| P3 | 存储管理 | ✅ |

## 相关文档

- 功能清单: `docs/reports/2026-02-16-feature-catalog.md`
- 架构评估: `docs/reports/2026-02-16-architecture-assessment.md`
