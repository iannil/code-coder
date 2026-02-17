# 用户全生命周期测试覆盖报告

> 生成时间: 2026-02-17
> 版本: 1.0.0

## 概述

本文档定义了 CodeCoder Web 应用的用户全生命周期测试用例，确保所有用户类型的核心流程都有完整的测试覆盖。

## 用户类型定义

### 1. Developer (软件开发者)

**核心场景**: 代码开发、审查、测试、架构设计

**对应 Agent**:
- `build` - 构建与开发
- `plan` - 规划模式
- `code-reviewer` - 代码审查
- `security-reviewer` - 安全审查
- `tdd-guide` - 测试驱动开发
- `architect` - 架构设计
- `explore` - 代码探索

**生命周期流程**:
```
入口 → Dashboard → 新建会话 → 选择开发类 Agent → 编码/审查/测试 → 文件操作 → 配置管理 → 退出
```

### 2. Creator (内容创作者)

**核心场景**: 长文写作、校对、内容扩展

**对应 Agent**:
- `writer` - 长文写作
- `proofreader` - 校对
- `expander` - 内容扩展
- `expander-fiction` - 小说扩展
- `expander-nonfiction` - 非虚构扩展

**生命周期流程**:
```
入口 → Dashboard → 新建会话 → 选择写作 Agent → 生成大纲 → 章节写作 → 校对 → 文档管理 → 退出
```

### 3. Analyst (决策分析师)

**核心场景**: 决策分析、宏观经济、交易、选品

**对应 Agent** (祝融说系列):
- `observer` - 观察者视角
- `decision` - CLOSE 决策框架
- `macro` - 宏观经济分析
- `trader` - 交易分析
- `picker` - 选品策略
- `miniproduct` - 极小产品
- `ai-engineer` - AI 工程

**生命周期流程**:
```
入口 → Dashboard → 新建会话 → 选择分析 Agent → 决策分析 → 宏观解读 → 交易/选品 → 记忆系统 → 退出
```

### 4. Common (所有用户)

**核心场景**: 通用功能（导航、会话、主题、设置、命令面板）

**生命周期流程**:
```
导航 → 主题切换 → 设置 → 命令面板 → 错误处理
```

---

## 功能覆盖矩阵

| 功能模块 | Developer | Creator | Analyst | Common | API 路由 |
|----------|:---------:|:-------:|:-------:|:------:|----------|
| Dashboard | - | - | - | ✅ | `/` |
| Navigation | - | - | - | ✅ | - |
| Sessions | ✅ | ✅ | ✅ | ✅ | `/api/sessions` |
| Messages | ✅ | ✅ | ✅ | ✅ | `/api/sessions/:id/messages` |
| Agent Selection | ✅ | ✅ | ✅ | ✅ | `/api/agents` |
| Files | ✅ | - | - | - | `/api/files` |
| Documents | - | ✅ | - | - | `/api/documents` |
| Memory | - | - | ✅ | - | `/api/memory` |
| Providers | ✅ | - | - | - | `/api/providers` |
| Theme | - | - | - | ✅ | - |
| Settings | - | - | - | ✅ | `/api/config` |
| Command Palette | - | - | - | ✅ | - |
| Error Handling | - | - | - | ✅ | - |
| **Tasks** | ✅ | - | - | - | `/api/v1/tasks` |
| **LSP** | ✅ | - | - | - | `/api/lsp` |
| **Permissions** | - | - | - | ✅ | `/api/permissions` |
| **MCP** | - | - | - | ✅ | `/api/mcp` |
| **Hooks** | - | - | - | ✅ | `/api/hooks` |

> ✅ = 有专门测试覆盖
> **粗体** = 新增测试覆盖

---

## 测试用例清单

### Common (ULC-CMN-*)

#### Dashboard (ULC-CMN-DASH)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-DASH-001 | should load dashboard | ✅ |
| ULC-CMN-DASH-002 | should display sidebar navigation | ✅ |
| ULC-CMN-DASH-003 | should display main content area | ✅ |

#### Navigation (ULC-CMN-NAV)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-NAV-001 | should navigate to dashboard | ✅ |
| ULC-CMN-NAV-002 | should navigate to settings | ✅ |
| ULC-CMN-NAV-003 | should navigate to files | ✅ |
| ULC-CMN-NAV-004 | should navigate to sessions page | ✅ 新增 |
| ULC-CMN-NAV-005 | should navigate to agents page | ✅ 新增 |
| ULC-CMN-NAV-006 | should navigate to memory page | ✅ 新增 |
| ULC-CMN-NAV-007 | should navigate to tasks page | ✅ 新增 |

#### Session Management (ULC-CMN-SESS)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-SESS-001 | should display new session button | ✅ |
| ULC-CMN-SESS-002 | should create new session | ✅ |
| ULC-CMN-SESS-003 | should display session area in sidebar | ✅ |
| ULC-CMN-SESS-004 | should switch between sessions | ✅ |

#### Theme (ULC-CMN-THME)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-THME-001 | should display theme toggle | ✅ |
| ULC-CMN-THME-002 | should toggle dark mode | ✅ |
| ULC-CMN-THME-003 | should persist theme preference | ✅ |

#### Settings (ULC-CMN-STNG)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-STNG-001 | should load settings page | ✅ |
| ULC-CMN-STNG-002 | should display API key input | ✅ |
| ULC-CMN-STNG-003 | should save settings | ✅ |

#### Agent Selection (ULC-CMN-AGNT)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-AGNT-001 | should display agent selector | ✅ |
| ULC-CMN-AGNT-002 | should list available agents | ✅ |
| ULC-CMN-AGNT-003 | should switch agent | ✅ |

#### Message Interaction (ULC-CMN-MSG)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-MSG-001 | should display message input | ✅ |
| ULC-CMN-MSG-002 | should enable send button with input | ✅ |
| ULC-CMN-MSG-003 | should display message list | ✅ |

#### Command Palette (ULC-CMN-CMD)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-CMD-001 | should open command palette with Cmd+K | ✅ |
| ULC-CMN-CMD-002 | should close command palette with Escape | ✅ |
| ULC-CMN-CMD-003 | should search commands | ✅ |

#### Error Handling (ULC-CMN-ERR)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-ERR-001 | should handle invalid routes gracefully | ✅ |
| ULC-CMN-ERR-002 | should handle network errors gracefully | ✅ |

#### Permission Management (ULC-CMN-PERM) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-PERM-001 | should display permission requests | ✅ 新增 |
| ULC-CMN-PERM-002 | should allow responding to permission | ✅ 新增 |
| ULC-CMN-PERM-003 | should persist permission decisions | ✅ 新增 |

#### MCP Server Management (ULC-CMN-MCP) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-MCP-001 | should load MCP status | ✅ 新增 |
| ULC-CMN-MCP-002 | should display MCP servers list | ✅ 新增 |
| ULC-CMN-MCP-003 | should toggle MCP server | ✅ 新增 |
| ULC-CMN-MCP-004 | should display MCP tools | ✅ 新增 |

#### Hooks Management (ULC-CMN-HOOK) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CMN-HOOK-001 | should load hooks list | ✅ 新增 |
| ULC-CMN-HOOK-002 | should display hooks by lifecycle | ✅ 新增 |
| ULC-CMN-HOOK-003 | should show hook settings | ✅ 新增 |
| ULC-CMN-HOOK-004 | should display action types | ✅ 新增 |

---

### Developer (ULC-DEV-WEB-*)

#### Agent Selection (ULC-DEV-WEB-AGNT)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-AGNT-001 | should have build agent available | ✅ |
| ULC-DEV-WEB-AGNT-002 | should have code-reviewer agent available | ✅ |
| ULC-DEV-WEB-AGNT-003 | should have security-reviewer agent available | ✅ |
| ULC-DEV-WEB-AGNT-004 | should select build agent | ✅ |
| ULC-DEV-WEB-AGNT-005 | should display agent descriptions | ✅ |

#### Session Workflow (ULC-DEV-WEB-SESS)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-SESS-001 | should create coding session | ✅ |
| ULC-DEV-WEB-SESS-002 | should display session in list | ✅ |
| ULC-DEV-WEB-SESS-003 | should maintain agent selection across navigation | ✅ |

#### File Operations (ULC-DEV-WEB-FILE)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-FILE-001 | should load files page | ✅ |
| ULC-DEV-WEB-FILE-002 | should display file tree | ✅ |
| ULC-DEV-WEB-FILE-003 | should expand directories | ✅ |

#### Code Interaction (ULC-DEV-WEB-CODE)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-CODE-001 | should send code-related prompt | ✅ |
| ULC-DEV-WEB-CODE-002 | should display code blocks in response | ✅ |
| ULC-DEV-WEB-CODE-003 | should support syntax highlighting | ✅ |

#### Tool Call Display (ULC-DEV-WEB-TOOL)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-TOOL-001 | should display tool call component | ✅ |

#### Provider Configuration (ULC-DEV-WEB-PROV)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-PROV-001 | should display provider settings | ✅ |
| ULC-DEV-WEB-PROV-002 | should allow API key configuration | ✅ |

#### Task Management (ULC-DEV-WEB-TASK) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-TASK-001 | should load tasks page | ✅ 新增 |
| ULC-DEV-WEB-TASK-002 | should display task list | ✅ 新增 |
| ULC-DEV-WEB-TASK-003 | should show task details | ✅ 新增 |
| ULC-DEV-WEB-TASK-004 | should filter tasks by status | ✅ 新增 |
| ULC-DEV-WEB-TASK-005 | should display task progress | ✅ 新增 |

#### LSP Integration (ULC-DEV-WEB-LSP) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-DEV-WEB-LSP-001 | should display LSP status | ✅ 新增 |
| ULC-DEV-WEB-LSP-002 | should show diagnostics | ✅ 新增 |
| ULC-DEV-WEB-LSP-003 | should navigate to definition | ✅ 新增 |
| ULC-DEV-WEB-LSP-004 | should show references | ✅ 新增 |
| ULC-DEV-WEB-LSP-005 | should display workspace symbols | ✅ 新增 |

---

### Creator (ULC-CRT-WEB-*)

#### Agent Selection (ULC-CRT-WEB-AGNT)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CRT-WEB-AGNT-001 | should have writer agent available | ✅ |
| ULC-CRT-WEB-AGNT-002 | should have proofreader agent available | ✅ |
| ULC-CRT-WEB-AGNT-003 | should have expander agent available | ✅ |
| ULC-CRT-WEB-AGNT-004 | should select writer agent | ✅ |
| ULC-CRT-WEB-AGNT-005 | should display writer description | ✅ |

#### Session Workflow (ULC-CRT-WEB-SESS)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CRT-WEB-SESS-001 | should create writing session | ✅ |
| ULC-CRT-WEB-SESS-002 | should support long-form content input | ✅ |
| ULC-CRT-WEB-SESS-003 | should preserve session content | ✅ |

#### Document Handling (ULC-CRT-WEB-DOCS)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CRT-WEB-DOCS-001 | should load documents page | ✅ |
| ULC-CRT-WEB-DOCS-002 | should display document list | ✅ |
| ULC-CRT-WEB-DOCS-003 | should create new document | ✅ 新增 |
| ULC-CRT-WEB-DOCS-004 | should edit document | ✅ 新增 |
| ULC-CRT-WEB-DOCS-005 | should delete document | ✅ 新增 |

#### Writing Workflow (ULC-CRT-WEB-WRITE)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CRT-WEB-WRITE-001 | should support outline generation prompt | ✅ |
| ULC-CRT-WEB-WRITE-002 | should support chapter writing prompt | ✅ |
| ULC-CRT-WEB-WRITE-003 | should support proofreading request | ✅ |

#### Content Expansion (ULC-CRT-WEB-EXPD)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CRT-WEB-EXPD-001 | should have expander-fiction agent | ✅ |
| ULC-CRT-WEB-EXPD-002 | should have expander-nonfiction agent | ✅ |
| ULC-CRT-WEB-EXPD-003 | should support content expansion prompt | ✅ |

#### Document Export (ULC-CRT-WEB-EXPT) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-CRT-WEB-EXPT-001 | should export document as markdown | ✅ 新增 |
| ULC-CRT-WEB-EXPT-002 | should export document as HTML | ✅ 新增 |

---

### Analyst (ULC-ANL-WEB-*)

#### Agent Selection (ULC-ANL-WEB-AGNT)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-ANL-WEB-AGNT-001 | should have observer agent available | ✅ |
| ULC-ANL-WEB-AGNT-002 | should have decision agent available | ✅ |
| ULC-ANL-WEB-AGNT-003 | should have macro agent available | ✅ |
| ULC-ANL-WEB-AGNT-004 | should have trader agent available | ✅ |
| ULC-ANL-WEB-AGNT-005 | should have picker agent available | ✅ |
| ULC-ANL-WEB-AGNT-006 | should have miniproduct agent available | ✅ |
| ULC-ANL-WEB-AGNT-007 | should have ai-engineer agent available | ✅ |
| ULC-ANL-WEB-AGNT-008 | should select decision agent | ✅ |

#### Session Workflow (ULC-ANL-WEB-SESS)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-ANL-WEB-SESS-001 | should create analysis session | ✅ |
| ULC-ANL-WEB-SESS-002 | should support analysis prompts in Chinese | ✅ |

#### Analysis Workflows (ULC-ANL-WEB-ANLZ)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-ANL-WEB-ANLZ-001 | should support observer analysis request | ✅ |
| ULC-ANL-WEB-ANLZ-002 | should support CLOSE framework request | ✅ |
| ULC-ANL-WEB-ANLZ-003 | should support macro analysis request | ✅ |
| ULC-ANL-WEB-ANLZ-004 | should support trading analysis request | ✅ |
| ULC-ANL-WEB-ANLZ-005 | should support product selection request | ✅ |
| ULC-ANL-WEB-ANLZ-006 | should support miniproduct analysis request | ✅ |

#### Memory Panel (ULC-ANL-WEB-MEMO)
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-ANL-WEB-MEMO-001 | should display memory panel | ✅ |
| ULC-ANL-WEB-MEMO-002 | should browse daily notes by date | ✅ 新增 |
| ULC-ANL-WEB-MEMO-003 | should view long-term memory sections | ✅ 新增 |

#### Memory Consolidation (ULC-ANL-WEB-CONSO) - 新增
| 测试 ID | 测试名称 | 状态 |
|---------|----------|------|
| ULC-ANL-WEB-CONSO-001 | should display consolidation stats | ✅ 新增 |
| ULC-ANL-WEB-CONSO-002 | should trigger consolidation | ✅ 新增 |
| ULC-ANL-WEB-CONSO-003 | should show consolidation results | ✅ 新增 |

---

## 测试统计

### 按用户类型统计

| 用户类型 | 原有测试 | 新增测试 | 总计 |
|----------|:--------:|:--------:|:----:|
| Common | 24 | 15 | 39 |
| Developer | 17 | 10 | 27 |
| Creator | 14 | 5 | 19 |
| Analyst | 17 | 5 | 22 |
| **总计** | **72** | **35** | **107** |

### 按功能模块统计

| 功能模块 | 测试数量 |
|----------|:--------:|
| Dashboard & Navigation | 10 |
| Session Management | 4 |
| Theme & Settings | 6 |
| Agent Selection | 17 |
| Message Interaction | 3 |
| Command Palette | 3 |
| Error Handling | 2 |
| Permission Management | 3 |
| MCP Server Management | 4 |
| Hooks Management | 4 |
| File Operations | 3 |
| Code Interaction | 4 |
| Provider Configuration | 2 |
| Task Management | 5 |
| LSP Integration | 5 |
| Document Handling | 5 |
| Document Export | 2 |
| Writing Workflow | 6 |
| Analysis Workflows | 6 |
| Memory System | 6 |

---

## 运行测试

### 运行所有 E2E 测试

```bash
cd packages/web
SKIP_E2E=false bun run test:e2e
```

### 运行特定用户类型测试

```bash
# Developer 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-DEV-WEB"

# Creator 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-CRT-WEB"

# Analyst 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-ANL-WEB"

# Common 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-CMN"
```

### 运行特定功能模块测试

```bash
# Task Management 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-DEV-WEB-TASK"

# LSP Integration 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-DEV-WEB-LSP"

# Memory Consolidation 测试
SKIP_E2E=false bun run test:e2e -- --grep "ULC-ANL-WEB-CONSO"
```

---

## 测试文件位置

| 文件 | 路径 |
|------|------|
| Common 测试 | `packages/web/test/e2e/user-lifecycle/common.spec.ts` |
| Developer 测试 | `packages/web/test/e2e/user-lifecycle/developer.spec.ts` |
| Creator 测试 | `packages/web/test/e2e/user-lifecycle/creator.spec.ts` |
| Analyst 测试 | `packages/web/test/e2e/user-lifecycle/analyst.spec.ts` |

---

## 维护指南

### 添加新测试

1. 确定测试所属的用户类型和功能模块
2. 使用标准命名格式: `ULC-{USER}-{MODULE}-{NUMBER}`
3. 在对应的 `.spec.ts` 文件中添加测试
4. 更新本文档的测试用例清单

### 测试 ID 命名规范

- `ULC` = User Lifecycle (用户生命周期)
- `CMN` = Common (通用)
- `DEV` = Developer (开发者)
- `CRT` = Creator (创作者)
- `ANL` = Analyst (分析师)
- `WEB` = Web E2E (Web 端到端)

### 新增功能模块

1. 在功能覆盖矩阵中添加新行
2. 创建对应的测试组 (`test.describe`)
3. 确保至少一个用户类型覆盖该功能
4. 更新测试统计

---

## 更新日志

### 2026-02-17 v1.0.0

- 初始版本
- 新增 35 个测试用例
- 覆盖 Permission、MCP、Hooks、Task、LSP、Document Export、Memory Consolidation 功能
