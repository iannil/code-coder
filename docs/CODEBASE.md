# CodeCoder 代码库导航指南

## 给 LLM 的项目概览

### 项目本质

CodeCoder 是一个基于 Bun + SolidJS + OpenTUI 构建的 AI 编程代理 CLI 工具。

### 关键约定

| 约定项         | 值                                      |
| -------------- | --------------------------------------- |
| 默认分支       | `master` (非 `dev`)                     |
| 代码风格       | Prettier 120字符宽度, 无分号, 2空格缩进  |
| 测试框架       | Bun 内置测试                            |
| 包管理         | Bun workspace                           |
| 主包           | `packages/ccode/` - 包含所有核心功能    |
| 共享包         | `packages/util/` - 共享工具库           |

## 快速定位文件

### 核心目录结构

```
packages/ccode/src/
├── agent/            # Agent 定义与实现
│   ├── agent.ts      # 核心 Agent 类
│   └── *.ts          # 特定 Agent 类型
├── session/          # 会话管理
│   ├── session.ts    # 会话核心逻辑
│   └── *.ts          # 会话相关组件
├── tool/             # 工具系统
│   ├── bash.ts       # Bash 命令执行
│   ├── edit.ts       # 文件编辑
│   ├── glob.ts       # 文件搜索
│   └── grep.ts       # 内容搜索
├── permission/       # 权限控制
│   └── permission.ts # 权限检查逻辑
├── provider/         # AI 提供商抽象
│   ├── *.ts          # 各提供商适配器
│   └── provider.ts   # 提供商接口
├── mcp/              # MCP 协议
│   └── *.ts          # MCP 实现
├── cli/
│   └── cmd/tui/      # 终端 UI 组件 (SolidJS)
│       ├── *.tsx     # UI 组件
│       └── routes/   # 路由定义
├── project/          # 项目实例管理
│   └── instance.ts   # Instance Pattern 实现
├── config/           # 配置系统
│   └── config.ts     # 配置加载与验证
└── util/             # 内部工具函数
    └── *.ts          # 各类工具函数

packages/ccode/test/
├── e2e/              # 端到端测试
├── unit/             # 单元测试
├── integration/      # 集成测试
├── performance/      # 性能测试
└── a11y/             # 无障碍测试
```

## 重要模式

### 1. Instance Pattern

**位置**: `packages/ccode/src/project/instance.ts`

```typescript
// 项目实例管理，确保每个工作目录只有一个实例
export class Instance { ... }
```

### 2. 配置系统

**位置**: `packages/ccode/src/config/config.ts`

```typescript
// 使用 Zod 进行配置验证
export const ConfigSchema = z.object({ ... })
```

### 3. 事件总线

**位置**: `packages/ccode/src/bus.ts`

```typescript
// 全局事件发布/订阅
export const bus = new EventEmitter()
```

## 文件命名约定

| 模式              | 含义                    | 示例                     |
| ----------------- | ----------------------- | ------------------------ |
| `*.test.ts`       | 普通单元测试            | `agent.test.ts`          |
| `*.test.tsx`      | SolidJS 组件测试        | `dialog.test.tsx`        |
| `*.context.tsx`   | SolidJS Context Provider | `session.context.tsx`    |
| `createMock*.ts`  | 测试 Mock 工厂          | `createMockAgent.ts`     |
| `DialogX.tsx`     | 对话框组件              | `dialog-alert.tsx`       |
| `useX()`          | React/Solid Hooks       | `useSession()`           |

## 重要的类/函数命名

- **Agent 相关**: `Agent`, `createAgent()`, `AgentConfig`
- **会话相关**: `Session`, `createSession()`, `SessionManager`
- **工具相关**: `Tool`, `createTool()`, `ToolExecutor`
- **权限相关**: `Permission`, `askPermission()`, `PermissionLevel`
- **UI 组件**: `DialogX`, `Toast`, `Link`

## 导入路径约定

```typescript
// 工具函数 - 使用 workspace 包
import { lazy, fn } from "@codecoder-ai/util"

// 项目内部 - 使用别名 @/
import { Agent } from "@/agent"
import { Session } from "@/session"

// 避免 - 相对路径 (超过 1 级)
import { Agent } from "../../agent" // 不推荐
```

## 技术栈详情

### 运行时
- **Bun 1.3+**: 比 Node.js 更快的 JavaScript 运行时

### 前端
- **SolidJS 1.9+**: 响应式 UI 框架，性能优异
- **OpenTUI 0.1+**: 基于 Solid 的终端渲染库
- **TailwindCSS 4.1+**: 样式系统

### 后端
- **Hono 4.10+**: HTTP 服务器
- **Cloudflare Workers**: 部署平台

### AI/ML
- **@ai-sdk/*: 多提供商统一接口
- **@modelcontextprotocol/sdk**: MCP 协议支持

## 开发命令速查

```bash
# 安装依赖
bun install

# 运行 TUI
bun dev

# 启动 API 服务器
bun dev serve

# 类型检查
bun turbo typecheck

# 运行测试 (必须在特定包内)
cd packages/ccode && bun test

# 构建可执行文件
bun run --cwd packages/ccode build
```

## 常见问题

### Q: 默认分支是 dev 还是 master?
A: 当前是 `master`

### Q: 测试在哪里运行?
A: 必须在 `packages/ccode/` 目录下运行 `bun test`，不能从根目录运行

### Q: 如何添加新的 AI 提供商?
A: 在 `packages/ccode/src/provider/` 下创建新的适配器，实现统一接口

### Q: UI 组件如何测试?
A: 使用 Bun 内置测试 + SolidJS 测试工具，参考 `test/unit/tui/ui/` 下的示例
