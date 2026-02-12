# CodeCoder 用户全生命周期与测试用例文档

本文档定义了 CodeCoder 的不同用户类型、全生命周期流程，以及覆盖所有功能的测试用例。

---

## 目录

1. [用户类型定义](#用户类型定义)
2. [用户全生命周期](#用户全生命周期)
3. [功能覆盖矩阵](#功能覆盖矩阵)
4. [测试用例规范](#测试用例规范)
5. [完整测试用例](#完整测试用例)

---

## 用户类型定义

### 1. 新用户 (New User)

首次使用 CodeCoder 的用户，需要完成初始化设置。

**特征:**

- 无任何配置文件
- 未认证任何 AI 提供商
- 不熟悉命令和工作流

**关键需求:**

- 快速上手引导
- 默认配置可用
- 清晰的错误提示

### 2. CLI 开发者 (CLI Developer)

主要通过命令行界面使用 CodeCoder 的开发者。

**特征:**

- 熟悉终端操作
- 习惯键盘交互
- 需要 Git 集成

**关键需求:**

- 快速命令响应
- 脚本化工作流
- Git 集成

### 3. TUI 用户 (TUI User)

使用终端用户界面的交互式用户。

**特征:**

- 偏好视觉化界面
- 需要会话历史
- 使用快捷键

**关键需求:**

- 响应式界面
- 会话管理
- 快捷键绑定

### 4. Web 用户 (Web User)

通过浏览器使用 CodeCoder 的用户。

**特征:**

- 跨平台访问
- 需要 HTTP 服务器
- 依赖网络连接

**关键需求:**

- 服务器可用性
- 会话同步
- 安全认证

### 5. 远程用户 (Remote User)

连接到远程 CodeCoder 服务器的用户。

**特征:**

- 分布式团队协作
- 需要网络配置
- 多会话管理

**关键需求:**

- 网络稳定性
- 会话隔离
- 权限控制

### 6. MCP 用户 (MCP User)

使用 MCP (Model Context Protocol) 服务器扩展功能的用户。

**特征:**

- 需要外部工具集成
- 配置 MCP 服务器
- 管理 OAuth 认证

**关键需求:**

- MCP 服务器管理
- 认证流程
- 错误调试

### 7. ACP 用户 (ACP User)

使用 Agent Client Protocol 的开发者。

**特征:**

- 集成 ACP 客户端
- 标准化协议交互
- 流式数据处理

**关键需求:**

- ACP 协议兼容
- 流式通信
- 错误处理

### 8. 高级用户 (Power User)

深度使用所有功能的专业用户。

**特征:**

- 多提供商配置
- 自定义 Agent
- 高级工作流

**关键需求:**

- 灵活配置
- 性能优化
- 扩展能力

---

## 用户全生命周期

### 新用户生命周期 (New User Lifecycle)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        新用户生命周期                                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 安装阶段                                                         │
│     ├─ 下载安装 bun install -g codecoder                            │
│     ├─ 验证安装 codecoder --version                                 │
│     └─ 初始化配置 codecoder run                                     │
│                                                                     │
│  2. 认证阶段                                                         │
│     ├─ 选择提供商 codecoder auth login                              │
│     ├─ OAuth 流程 (浏览器认证)                                      │
│     └─ 验证认证 codecoder auth list                                 │
│                                                                     │
│  3. 首次使用                                                         │
│     ├─ 创建会话 codecoder run                                       │
│     ├─ 发送请求 "帮我理解这个项目"                                   │
│     └─ 查看响应                                                     │
│                                                                     │
│  4. 功能探索                                                         │
│     ├─ 查看模型 codecoder models                                    │
│     ├─ 切换 Agent /plan, /build, /explore                         │
│     └─ 会话管理 /help                                              │
│                                                                     │
│  5. 进阶使用                                                         │
│     ├─ 添加 MCP codecoder mcp add                                   │
│     ├─ Git 集成 codecoder pr create                                 │
│     └─ 导出会话 codecoder export                                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### CLI 开发者生命周期 (CLI Developer Lifecycle)

```
┌─────────────────────────────────────────────────────────────────────┐
│                      CLI 开发者生命周期                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 日常启动                                                         │
│     ├─ 进入项目目录 cd my-project                                  │
│     ├─ 启动会话 codecoder run                                       │
│     └─ 检查状态                                                     │
│                                                                     │
│  2. 开发工作流                                                       │
│     ├─ 规划任务 /plan "添加用户认证功能"                             │
│     ├─ 编写代码 /build "实现认证逻辑"                                │
│     ├─ 测试验证 /test                                              │
│     └─ 代码审查 /review                                            │
│                                                                     │
│  3. Git 集成                                                        │
│     ├─ 创建分支 git checkout -b feature/auth                       │
│     ├─ 提交代码 git commit -m "feat: add auth"                     │
│     ├─ 创建 PR codecoder pr create                                   │
│     └─ 审查 PR codecoder pr review <url>                             │
│                                                                     │
│  4. 会话管理                                                         │
│     ├─ 继续会话 codecoder run --continue                            │
│     ├─ 附加会话 codecoder run --attach <session-id>                 │
│     ├─ 列出会话 codecoder session list                               │
│     └─ 导出数据 codecoder export <session-id>                        │
│                                                                     │
│  5. 问题调试                                                         │
│     ├─ 查看日志 codecoder run --verbose                             │
│     ├─ MCP 调试 codecoder mcp debug <server>                        │
│     └─ 重置配置 rm -rf ~/.codecoder                                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### TUI 用户生命周期 (TUI User Lifecycle)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       TUI 用户生命周期                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 启动 TUI                                                         │
│     ├─ 自动启动 codecoder run (默认 TUI)                             │
│     ├─ 界面加载显示                                                 │
│     └─ 会话历史加载                                                 │
│                                                                     │
│  2. 交互操作                                                         │
│     ├─ 输入消息 (底部输入框)                                         │
│     ├─ 查看响应 (主消息区)                                           │
│     ├─ 快捷键操作                                                   │
│     │   ├─ Ctrl+C: 退出                                            │
│     │   ├─ Ctrl+D: 发送                                            │
│     │   ├─ Ctrl+N: 新会话                                          │
│     │   └─ Ctrl+P: 上一个会话                                      │
│     └─ 滚动浏览历史                                                 │
│                                                                     │
│  3. Agent 切换                                                      │
│     ├─ /plan: 规划模式                                             │
│     ├─ /build: 构建模式                                            │
│     ├─ /explore: 探索模式                                          │
│     └─ /general: 通用模式                                          │
│                                                                     │
│  4. 权限处理                                                         │
│     ├─ 读取文件请求 → 允许/拒绝                                      │
│     ├─ 编辑文件请求 → 允许/拒绝                                      │
│     ├─ Bash 命令 → 允许/拒绝                                         │
│     └─ 永久记住选择                                                 │
│                                                                     │
│  5. 会话结束                                                         │
│     ├─ 正常退出 Ctrl+C                                              │
│     ├─ 会话自动保存                                                 │
│     └─ 历史记录保留                                                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Web 用户生命周期 (Web User Lifecycle)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       Web 用户生命周期                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 服务器启动                                                       │
│     ├─ 启动服务器 codecoder serve                                    │
│     ├─ 默认端口 4096                                                 │
│     ├─ 自定义端口 codecoder serve --port 8080                        │
│     └─ 验证启动 curl http://localhost:4096/health                   │
│                                                                     │
│  2. Web 访问                                                        │
│     ├─ 打开浏览器 http://localhost:4096                             │
│     ├─ 或启动 Web 界面 codecoder web                                  │
│     └─ 自动打开浏览器                                               │
│                                                                     │
│  3. 会话交互                                                         │
│     ├─ 创建新会话                                                   │
│     ├─ 选择现有会话                                                   │
│     ├─ 发送消息                                                     │
│     └─ 查看响应                                                     │
│                                                                     │
│  4. 远程连接                                                         │
│     ├─ 配置远程地址 --baseUrl <url>                                 │
│     ├─ 认证连接                                                     │
│     └─ 使用远程会话                                                 │
│                                                                     │
│  5. 会话管理                                                         │
│     ├─ 切换项目                                                     │
│     ├─ 导出会话                                                     │
│     └─ 删除会话                                                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### MCP 用户生命周期 (MCP User Lifecycle)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       MCP 用户生命周期                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. MCP 服务器管理                                                   │
│     ├─ 列出可用服务器 codecoder mcp list                              │
│     ├─ 添加服务器 codecoder mcp add <name> <command>                 │
│     ├─ 删除服务器 codecoder mcp remove <name>                         │
│     └─ 验证配置                                                     │
│                                                                     │
│  2. OAuth 认证                                                       │
│     ├─ 发起认证 codecoder mcp auth <server>                          │
│     ├─ 浏览器授权                                                   │
│     ├─ Token 存储                                                   │
│     └─ 验证状态 codecoder mcp list                                    │
│                                                                     │
│  3. 使用 MCP 工具                                                    │
│     ├─ 在会话中调用 MCP 工具                                        │
│     ├─ 查看工具结果                                                 │
│     ├─ 处理认证过期                                                 │
│     └─ 重新认证                                                     │
│                                                                     │
│  4. 调试 MCP                                                         │
│     ├─ 查看服务器状态 codecoder mcp debug <server>                   │
│     ├─ 查看日志                                                     │
│     ├─ 测试连接                                                     │
│     └─ 重启服务器                                                   │
│                                                                     │
│  5. 注销管理                                                         │
│     ├─ 注销服务器 codecoder mcp logout <server>                       │
│     ├─ 清除 Token                                                   │
│     └─ 重新认证                                                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### ACP 用户生命周期 (ACP User Lifecycle)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       ACP 用户生命周期                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 启动 ACP 服务器                                                  │
│     ├─ 启动 CodeCoder 服务器 codecoder serve                          │
│     ├─ 启动 ACP 模式 codecoder acp                                    │
│     ├─ 配置工作目录 --cwd <path>                                    │
│     └─ 网络配置 --port, --host                                       │
│                                                                     │
│  2. 客户端连接                                                       │
│     ├─ 建立 ACP 连接                                               │
│     ├─ NDJSON 流通信                                                │
│     ├─ 消息解析                                                     │
│     └─ 错误处理                                                     │
│                                                                     │
│  3. 协议交互                                                         │
│     ├─ 发送任务请求                                                 │
│     ├─ 接收流式响应                                                 │
│     ├─ 订阅事件                                                     │
│     └─ 处理状态变更                                                 │
│                                                                     │
│  4. 集成开发                                                         │
│     ├─ 使用 SDK @codecoder-ai/sdk                                    │
│     ├─ 自定义客户端                                                 │
│     ├─ 事件处理                                                     │
│     └─ 错误重试                                                     │
│                                                                     │
│  5. 生产部署                                                         │
│     ├─ 服务化运行                                                   │
│     ├─ 监控日志                                                     │
│     ├─ 性能优化                                                     │
│     └─ 故障恢复                                                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 功能覆盖矩阵

### 命令覆盖表

| 功能模块   | 命令             | 新用户 | CLI | TUI | Web | 远程 | MCP | ACP | 高级 |
| ---------- | ---------------- | ------ | --- | --- | --- | ---- | --- | --- | ---- |
| **认证**   | auth login       | ✅     | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | auth logout      |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | auth list        |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
| **会话**   | run              | ✅     | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | run --continue   |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | run --attach     |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | session list     |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | session delete   |        |     | ✅  | ✅  | ✅   |     |     | ✅   |
|            | export           |        |     | ✅  | ✅  | ✅   |     |     | ✅   |
|            | import           |        |     | ✅  | ✅  | ✅   |     |     | ✅   |
| **服务器** | serve            |        | ✅  |     | ✅  | ✅   |     |     | ✅   |
|            | web              |        |     |     | ✅  |      |     |     | ✅   |
| **MCP**    | mcp add          |        |     | ✅  | ✅  |      | ✅  |     | ✅   |
|            | mcp list         |        | ✅  | ✅  | ✅  |      | ✅  |     | ✅   |
|            | mcp remove       |        |     | ✅  | ✅  |      | ✅  |     | ✅   |
|            | mcp auth         |        |     | ✅  | ✅  |      | ✅  |     | ✅   |
|            | mcp logout       |        |     | ✅  | ✅  |      | ✅  |     | ✅   |
|            | mcp debug        |        |     |     |     |      | ✅  |     | ✅   |
| **模型**   | models           | ✅     | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
|            | models --refresh |        |     |     |     |      |     |     | ✅   |
| **PR**     | pr create        |        | ✅  | ✅  | ✅  |      |     |     | ✅   |
|            | pr review        |        | ✅  | ✅  | ✅  |      |     |     | ✅   |
| **ACP**    | acp              |        |     |     |     |      |     | ✅  | ✅   |
| **其他**   | upgrade          |        | ✅  | ✅  | ✅  |      |     |     | ✅   |
|            | uninstall        |        | ✅  | ✅  | ✅  |      |     |     | ✅   |
|            | stats            |        |     |     |     |      |     |     | ✅   |
|            | generate         |        |     |     |     |      |     |     | ✅   |

### Agent 覆盖表

| Agent      | 用途     | 新用户 | CLI | TUI | Web | 远程 | MCP | ACP | 高级 |
| ---------- | -------- | ------ | --- | --- | --- | ---- | --- | --- | ---- |
| build      | 代码构建 |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
| plan       | 任务规划 |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
| general    | 通用对话 | ✅     | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
| explore    | 代码探索 |        | ✅  | ✅  | ✅  | ✅   |     |     | ✅   |
| compaction | 会话压缩 |        |     |     |     |      |     |     | ✅   |
| title      | 标题生成 |        |     |     |     |      |     |     | ✅   |
| summary    | 摘要生成 |        |     |     |     |      |     |     | ✅   |

---

## 测试用例规范

### 测试用例结构

每个测试用例包含以下字段:

```typescript
interface TestCase {
  id: string // 唯一标识符
  title: string // 测试标题
  userType: UserLifecycle // 用户类型
  lifecycleStage: string // 生命周期阶段
  feature: string // 被测功能
  priority: "critical" | "high" | "medium" | "low" // 优先级
  type: "e2e" | "integration" | "unit" // 测试类型
  preconditions: string[] // 前置条件
  steps: TestStep[] // 测试步骤
  expectedResults: string[] // 预期结果
  cleanup: string[] // 清理步骤
  tags: string[] // 标签
}

interface TestStep {
  step: number
  action: string // 操作描述
  command?: string // 执行的命令
  expected?: string // 预期输出
}
```

### 测试用例 ID 规范

```
ULC-<用户类型代码>-<功能代码>-<序号>

用户类型代码:
  NU = New User (新用户)
  CD = CLI Developer (CLI 开发者)
  TU = TUI User (TUI 用户)
  WU = Web User (Web 用户)
  RU = Remote User (远程用户)
  MU = MCP User (MCP 用户)
  AU = ACP User (ACP 用户)
  PU = Power User (高级用户)

功能代码:
  AUTH = Authentication (认证)
  SESS = Session (会话)
  SERV = Server (服务器)
  MCP  = MCP (MCP 协议)
  ACP  = ACP (ACP 协议)
  AGNT = Agent (智能体)
  PR   = Pull Request (PR)
  MDLS = Models (模型)
  EXP  = Export/Import (导入导出)
  UTIL = Utilities (工具)
```

---

## 完整测试用例

### 1. 新用户 (New User) 测试用例

#### ULC-NU-AUTH-001: 新用户首次登录

```yaml
title: "新用户完成 OAuth 登录流程"
userType: "New User"
lifecycleStage: "认证阶段"
feature: "auth login"
priority: "critical"
type: "e2e"
tags: ["auth", "onboarding", "oauth"]

preconditions:
  - CodeCoder 已安装
  - 未登录任何提供商
  - 无现有配置文件

steps:
  - step: 1
    action: "执行登录命令"
    command: "codecoder auth login"
    expected: "显示提供商选择提示"

  - step: 2
    action: "选择 Anthropic 提供商"
    input: "anthropic"
    expected: "浏览器打开 OAuth 授权页面"

  - step: 3
    action: "在浏览器完成授权"
    expected: "授权成功，显示成功消息"

  - step: 4
    action: "验证登录状态"
    command: "codecoder auth list"
    expected: "显示已登录的提供商信息"

expectedResults:
  - OAuth 流程成功完成
  - API Token 正确保存
  - auth list 显示提供商信息
  - 配置文件已创建

cleanup:
  - "codecoder auth logout"
  - "rm -rf ~/.codecoder"
```

#### ULC-NU-AUTH-002: 新用户使用 API Key 登录

```yaml
title: "新用户使用 API Key 登录"
userType: "New User"
lifecycleStage: "认证阶段"
feature: "auth login --api-key"
priority: "high"
type: "e2e"
tags: ["auth", "onboarding", "api-key"]

preconditions:
  - CodeCoder 已安装
  - 拥有有效的 API Key

steps:
  - step: 1
    action: "执行 API Key 登录"
    command: "codecoder auth login --api-key"
    expected: "提示输入 API Key"

  - step: 2
    action: "输入 API Key"
    input: "sk-ant-xxx..."
    expected: "验证 API Key 有效性"

  - step: 3
    action: "验证登录状态"
    command: "codecoder auth list"
    expected: "显示已登录的提供商"

expectedResults:
  - API Key 验证成功
  - Token 安全存储
  - 可以正常使用 AI 功能

cleanup:
  - "codecoder auth logout"
```

#### ULC-NU-SESS-001: 新用户首次创建会话

```yaml
title: "新用户创建首次会话"
userType: "New User"
lifecycleStage: "首次使用"
feature: "run"
priority: "critical"
type: "e2e"
tags: ["session", "onboarding"]

preconditions:
  - 已完成认证
  - 在项目目录中

steps:
  - step: 1
    action: "启动 CodeCoder"
    command: "codecoder run"
    expected: "TUI 界面启动"

  - step: 2
    action: "发送第一条消息"
    input: "你好，请介绍一下这个项目"
    expected: "AI 回复项目介绍"

  - step: 3
    action: "退出会话"
    input: "Ctrl+C"
    expected: "会话保存，程序退出"

expectedResults:
  - TUI 正常启动
  - AI 响应正确
  - 会话自动保存
  - 可以通过 --continue 恢复

cleanup:
  - "codecoder session list"
  - "codecoder session delete <session-id>"
```

#### ULC-NU-MDLS-001: 新用户查看可用模型

```yaml
title: "新用户列出所有可用模型"
userType: "New User"
lifecycleStage: "功能探索"
feature: "models"
priority: "medium"
type: "integration"
tags: ["models", "discovery"]

preconditions:
  - 已登录至少一个提供商
  - 已配置模型缓存

steps:
  - step: 1
    action: "列出所有模型"
    command: "codecoder models"
    expected: "显示所有提供商的模型列表"

  - step: 2
    action: "筛选特定提供商"
    command: "codecoder models anthropic"
    expected: "仅显示 Anthropic 模型"

  - step: 3
    action: "查看详细模型信息"
    command: "codecoder models --verbose"
    expected: "显示模型元数据（成本、上下文等）"

expectedResults:
  - 模型列表正确显示
  - 筛选功能正常
  - 详细信息准确

cleanup: []
```

---

### 2. CLI 开发者 (CLI Developer) 测试用例

#### ULC-CD-SESS-001: CLI 开发者日常会话管理

```yaml
title: "CLI 开发者完整会话工作流"
userType: "CLI Developer"
lifecycleStage: "日常使用"
feature: "session management"
priority: "critical"
type: "e2e"
tags: ["session", "workflow", "cli"]

preconditions:
  - 已认证
  - 存在项目目录
  - 有历史会话

steps:
  - step: 1
    action: "列出所有会话"
    command: "codecoder session list"
    expected: "显示会话列表（按时间排序）"

  - step: 2
    action: "继续上一个会话"
    command: "codecoder run --continue"
    expected: "加载最近会话，保持上下文"

  - step: 3
    action: "附加到特定会话"
    command: "codecoder run --attach <session-id>"
    expected: "加载指定会话"

  - step: 4
    action: "创建新会话"
    command: "codecoder run"
    input: "新的任务"
    expected: "创建新会话，独立上下文"

expectedResults:
  - 会话列表正确显示
  - 继续功能正常
  - 附加功能正常
  - 新会话创建成功

cleanup:
  - 清理测试会话
```

#### ULC-CD-AGNT-001: CLI 开发者使用 Agent 模式

```yaml
title: "CLI 开发者切换不同 Agent 模式"
userType: "CLI Developer"
lifecycleStage: "开发工作流"
feature: "agent modes"
priority: "high"
type: "e2e"
tags: ["agent", "workflow"]

preconditions:
  - 已认证
  - 在项目目录中

steps:
  - step: 1
    action: "使用 plan agent 规划任务"
    input: "/plan 添加用户认证功能"
    expected: "Agent 生成实施计划"

  - step: 2
    action: "使用 build agent 实现代码"
    input: "/build 实现登录接口"
    expected: "Agent 编写代码并测试"

  - step: 3
    action: "使用 explore agent 探索代码"
    input: "/explore 找到所有 API 路由"
    expected: "Agent 搜索并列出路由"

  - step: 4
    action: "使用 general agent 问答"
    input: "这个项目用什么框架？"
    expected: "Agent 回答问题"

expectedResults:
  - plan agent 生成结构化计划
  - build agent 生成可运行代码
  - explore agent 正确搜索代码
  - general agent 准确回答问题

cleanup: []
```

#### ULC-CD-PR-001: CLI 开发者创建和审查 PR

```yaml
title: "CLI 开发者完整的 PR 工作流"
userType: "CLI Developer"
lifecycleStage: "Git 集成"
feature: "pr commands"
priority: "high"
type: "e2e"
tags: ["pr", "git", "workflow"]

preconditions:
  - Git 仓库已初始化
  - 已配置 GitHub 认证
  - 有未推送的提交

steps:
  - step: 1
    action: "创建功能分支并提交"
    command: "git checkout -b feature/test && git commit -m 'feat: test'"

  - step: 2
    action: "创建 PR"
    command: "codecoder pr create"
    expected: "PR 创建成功，返回 URL"

  - step: 3
    action: "审查 PR"
    command: "codecoder pr review <pr-url>"
    expected: "生成审查报告"

  - step: 4
    action: "根据审查意见修改代码"
    input: "根据审查意见修复问题"
    expected: "代码修改完成"

expectedResults:
  - PR 成功创建
  - 审查报告准确
  - 代码修改正确应用

cleanup:
  - 关闭测试 PR
  - 删除测试分支
```

#### ULC-CD-EXP-001: CLI 开发者导出导入会话

```yaml
title: "CLI 开发者导出和导入会话数据"
userType: "CLI Developer"
lifecycleStage: "会话管理"
feature: "export/import"
priority: "medium"
type: "integration"
tags: ["session", "backup"]

preconditions:
  - 存在有效会话

steps:
  - step: 1
    action: "导出会话到 JSON"
    command: "codecoder export <session-id> > session-backup.json"
    expected: "JSON 格式会话数据输出"

  - step: 2
    action: "验证 JSON 格式"
    command: "jq '.' session-backup.json"
    expected: "JSON 格式正确"

  - step: 3
    action: "导入会话数据"
    command: "codecoder import session-backup.json"
    expected: "会话导入成功"

  - step: 4
    action: "从 URL 导入共享会话"
    command: "codecoder import https://opncd.ai/share/xxx"
    expected: "共享会话导入成功"

expectedResults:
  - 导出 JSON 格式正确
  - 导入数据完整
  - URL 导入正常工作

cleanup:
  - 删除导入的测试会话
  - 删除备份文件
```

---

### 3. TUI 用户 (TUI User) 测试用例

#### ULC-TU-SESS-001: TUI 界面交互

```yaml
title: "TUI 用户完整交互流程"
userType: "TUI User"
lifecycleStage: "交互操作"
feature: "TUI interface"
priority: "critical"
type: "e2e"
tags: ["tui", "interface"]

preconditions:
  - 已认证
  - 终端支持 TUI

steps:
  - step: 1
    action: "启动 TUI"
    command: "codecoder run"
    expected: "TUI 界面正常显示"

  - step: 2
    action: "输入消息"
    input: "测试消息"
    key: "Ctrl+D"
    expected: "消息发送，AI 响应显示"

  - step: 3
    action: "滚动历史消息"
    key: "PageUp/PageDown"
    expected: "消息滚动流畅"

  - step: 4
    action: "使用快捷键切换 Agent"
    input: "/plan"
    key: "Enter"
    expected: "Agent 切换到 plan 模式"

  - step: 5
    action: "退出 TUI"
    key: "Ctrl+C"
    expected: "会话保存，程序退出"

expectedResults:
  - TUI 界面渲染正确
  - 输入输出正常
  - 快捷键响应正确
  - 会话正确保存

cleanup: []
```

#### ULC-TU-SESS-002: TUI 权限处理流程

```yaml
title: "TUI 用户处理权限请求"
userType: "TUI User"
lifecycleStage: "权限管理"
feature: "permission system"
priority: "high"
type: "e2e"
tags: ["tui", "permissions"]

preconditions:
  - TUI 已启动
  - AI 请求执行敏感操作

steps:
  - step: 1
    action: "AI 请求读取文件"
    expected: "显示权限请求提示"

  - step: 2
    action: "允许一次"
    input: "y"
    expected: "操作执行，下次继续询问"

  - step: 3
    action: "AI 再次请求读取"
    expected: "再次显示权限请求"

  - step: 4
    action: "永久允许"
    input: "ya"
    expected: "操作执行，不再询问此文件"

  - step: 5
    action: "拒绝操作"
    input: "n"
    expected: "操作被拒绝，AI 继续其他方案"

expectedResults:
  - 权限提示清晰
  - 一次/永久选项正确
  - 拒绝后 AI 正常降级

cleanup: []
```

---

### 4. Web 用户 (Web User) 测试用例

#### ULC-WU-SERV-001: Web 用户服务器启动和访问

```yaml
title: "Web 用户启动服务器并访问"
userType: "Web User"
lifecycleStage: "服务器启动"
feature: "serve"
priority: "critical"
type: "e2e"
tags: ["web", "server"]

preconditions:
  - 已认证
  - 端口 4096 未被占用

steps:
  - step: 1
    action: "启动服务器"
    command: "codecoder serve"
    expected: "服务器在 4096 端口启动"

  - step: 2
    action: "验证健康检查"
    command: "curl http://localhost:4096/health"
    expected: "返回 200 OK"

  - step: 3
    action: "启动 Web 界面"
    command: "codecoder web"
    expected: "浏览器自动打开"

  - step: 4
    action: "在浏览器中创建会话"
    action: "点击新建会话，输入消息"
    expected: "会话创建成功，AI 响应"

expectedResults:
  - 服务器稳定运行
  - 健康检查通过
  - Web 界面正常
  - 会话功能正常

cleanup:
  - 停止服务器
```

#### ULC-WU-SESS-001: Web 用户远程会话管理

```yaml
title: "Web 用户连接远程服务器"
userType: "Web User"
lifecycleStage: "远程连接"
feature: "remote connection"
priority: "high"
type: "e2e"
tags: ["web", "remote"]

preconditions:
  - 远程 CodeCoder 服务器运行中
  - 网络连接正常

steps:
  - step: 1
    action: "配置远程服务器"
    command: "codecoder run --baseUrl https://remote-server.com"
    expected: "连接到远程服务器"

  - step: 2
    action: "创建远程会话"
    input: "远程测试消息"
    expected: "通过远程服务器处理"

  - step: 3
    action: "列出远程会话"
    command: "codecoder session list"
    expected: "显示远程会话列表"

expectedResults:
  - 远程连接成功
  - 会话数据存储在远程
  - 响应正常

cleanup: []
```

---

### 5. MCP 用户 (MCP User) 测试用例

#### ULC-MU-MCP-001: MCP 服务器添加和认证

```yaml
title: "MCP 用户添加服务器并完成 OAuth 认证"
userType: "MCP User"
lifecycleStage: "MCP 设置"
feature: "mcp add/auth"
priority: "critical"
type: "e2e"
tags: ["mcp", "oauth"]

preconditions:
  - 已登录 CodeCoder
  - MCP 服务器地址可用

steps:
  - step: 1
    action: "列出可用 MCP 服务器"
    command: "codecoder mcp list"
    expected: "显示内置 MCP 服务器列表"

  - step: 2
    action: "添加自定义 MCP 服务器"
    command: "codecoder mcp add my-server 'npx -y @my/mcp-server'"
    expected: "服务器添加成功"

  - step: 3
    action: "对需要认证的服务器进行认证"
    command: "codecoder mcp auth github"
    expected: "浏览器打开 GitHub OAuth"

  - step: 4
    action: "完成授权"
    action: "在浏览器中点击授权"
    expected: "Token 存储成功"

  - step: 5
    action: "验证认证状态"
    command: "codecoder mcp list"
    expected: "显示已认证状态"

expectedResults:
  - 服务器列表正确
  - 添加成功
  - OAuth 流程完成
  - 认证状态更新

cleanup:
  - "codecoder mcp logout github"
  - "codecoder mcp remove my-server"
```

#### ULC-MU-MCP-002: MCP 工具调用和调试

```yaml
title: "MCP 用户在会话中调用 MCP 工具"
userType: "MCP User"
lifecycleStage: "MCP 使用"
feature: "mcp tools"
priority: "high"
type: "integration"
tags: ["mcp", "tools"]

preconditions:
  - MCP 服务器已添加
  - 需要认证的已完成认证

steps:
  - step: 1
    action: "启动会话"
    command: "codecoder run"

  - step: 2
    action: "请求使用 MCP 工具"
    input: "使用 GitHub MCP 查看 pr review workflow 的文件列表"

  - step: 3
    action: "验证工具调用"
    expected: "AI 调用 GitHub MCP 工具"

  - step: 4
    action: "查看 MCP 调试信息"
    command: "codecoder mcp debug github"
    expected: "显示服务器状态和调用日志"

expectedResults:
  - MCP 工具成功调用
  - 结果正确返回
  - 调试信息准确

cleanup: []
```

#### ULC-MU-MCP-003: MCP Token 过期处理

```yaml
title: "MCP 用户处理 Token 过期"
userType: "MCP User"
lifecycleStage: "MCP 维护"
feature: "mcp token refresh"
priority: "medium"
type: "integration"
tags: ["mcp", "oauth", "error-handling"]

preconditions:
  - MCP 服务器已认证
  - Token 接近过期或已过期

steps:
  - step: 1
    action: "尝试使用已过期的 MCP 工具"
    input: "调用需要认证的 MCP 工具"
    expected: "显示认证过期错误"

  - step: 2
    action: "重新认证"
    command: "codecoder mcp auth <server>"
    expected: "重新打开 OAuth 流程"

  - step: 3
    action: "完成重新授权"
    expected: "新 Token 存储"

  - step: 4
    action: "验证工具可用"
    input: "再次调用 MCP 工具"
    expected: "工具调用成功"

expectedResults:
  - 过期检测正确
  - 重新认证流程正常
  - Token 自动刷新

cleanup: []
```

---

### 6. ACP 用户 (ACP User) 测试用例

#### ULC-AU-ACP-001: ACP 服务器启动和连接

```yaml
title: "ACP 用户启动 ACP 服务器并建立连接"
userType: "ACP User"
lifecycleStage: "ACP 设置"
feature: "acp command"
priority: "critical"
type: "e2e"
tags: ["acp", "protocol"]

preconditions:
  - CodeCoder 服务器已启动
  - ACP SDK 已安装

steps:
  - step: 1
    action: "启动 CodeCoder 服务器"
    command: "codecoder serve --port 4096"
    expected: "服务器启动"

  - step: 2
    action: "启动 ACP 模式"
    command: "codecoder acp --port 4096"
    expected: "ACP 服务器启动"

  - step: 3
    action: "使用 SDK 建立连接"
    code: |
      import { createCodecoderClient } from "@codecoder-ai/sdk/v2"
      const sdk = createCodecoderClient({ baseUrl: "http://localhost:4096" })
    expected: "连接建立成功"

  - step: 4
    action: "发送 ACP 请求"
    action: "通过 SDK 发送任务"
    expected: "接收流式响应"

expectedResults:
  - ACP 服务器启动
  - 连接成功
  - NDJSON 流通信正常
  - 响应流式返回

cleanup:
  - 停止服务器
```

#### ULC-AU-ACP-002: ACP 事件订阅

```yaml
title: "ACP 用户订阅事件并接收更新"
userType: "ACP User"
lifecycleStage: "ACP 交互"
feature: "acp events"
priority: "high"
type: "integration"
tags: ["acp", "events"]

preconditions:
  - ACP 连接已建立

steps:
  - step: 1
    action: "订阅消息事件"
    code: "conn.subscribe('message')"
    expected: "订阅成功"

  - step: 2
    action: "发送任务请求"
    action: "创建新任务"
    expected: "接收到消息事件"

  - step: 3
    action: "订阅状态更新"
    code: "conn.subscribe('status')"
    expected: "接收状态变化通知"

  - step: 4
    action: "取消订阅"
    code: "conn.unsubscribe('message')"
    expected: "停止接收事件"

expectedResults:
  - 订阅功能正常
  - 事件实时推送
  - 取消订阅生效

cleanup: []
```

---

### 7. 高级用户 (Power User) 测试用例

#### ULC-PU-MDLS-001: 高级用户刷新模型缓存

```yaml
title: "高级用户刷新模型缓存以获取最新模型"
userType: "Power User"
lifecycleStage: "高级配置"
feature: "models --refresh"
priority: "medium"
type: "integration"
tags: ["models", "cache"]

preconditions:
  - 网络连接正常
  - models.dev API 可访问

steps:
  - step: 1
    action: "刷新模型缓存"
    command: "codecoder models --refresh"
    expected: "从 models.dev 获取最新数据"

  - step: 2
    action: "验证新模型可用"
    command: "codecoder models"
    expected: "显示包含新模型的列表"

expectedResults:
  - 缓存成功刷新
  - 新模型正确显示
  - 元数据完整

cleanup: []
```

#### ULC-PU-UTIL-001: 高级用户查看统计信息

```yaml
title: "高级用户查看使用统计"
userType: "Power User"
lifecycleStage: "监控分析"
feature: "stats"
priority: "low"
type: "unit"
tags: ["stats", "monitoring"]

preconditions:
  - 有使用历史

steps:
  - step: 1
    action: "查看统计信息"
    command: "codecoder stats"
    expected: "显示使用统计"

expectedResults:
  - 显示会话数量
  - 显示消息数量
  - 显示 Token 使用
  - 显示成本估算

cleanup: []
```

#### ULC-PU-UTIL-002: 高级用户升级 CodeCoder

```yaml
title: "高级用户升级到最新版本"
userType: "Power User"
lifecycleStage: "维护"
feature: "upgrade"
priority: "medium"
type: "integration"
tags: ["upgrade", "maintenance"]

preconditions:
  - 网络连接正常
  - 有新版本可用

steps:
  - step: 1
    action: "检查更新"
    command: "codecoder upgrade"
    expected: "显示可用更新"

  - step: 2
    action: "确认升级"
    input: "y"
    expected: "下载并安装新版本"

  - step: 3
    action: "验证升级"
    command: "codecoder --version"
    expected: "显示新版本号"

expectedResults:
  - 更新检测正确
  - 升级过程成功
  - 版本号正确

cleanup: []
```

---

### 8. 跨用户类型测试用例

#### ULC-ALL-SESS-001: 多用户会话隔离

```yaml
title: "验证不同用户的会话完全隔离"
userType: "All"
lifecycleStage: "会话管理"
feature: "session isolation"
priority: "critical"
type: "integration"
tags: ["session", "security", "isolation"]

preconditions:
  - 多个用户账户

steps:
  - step: 1
    action: "用户 A 创建会话"
    user: "user-a"
    command: "codecoder run"
    input: "用户 A 的秘密信息"
    expected: "会话 A 创建"

  - step: 2
    action: "用户 B 列出会话"
    user: "user-b"
    command: "codecoder session list"
    expected: "不显示用户 A 的会话"

  - step: 3
    action: "用户 B 创建会话"
    user: "user-b"
    command: "codecoder run"
    input: "用户 B 的消息"
    expected: "会话 B 创建"

  - step: 4
    action: "用户 A 重新登录"
    user: "user-a"
    command: "codecoder session list"
    expected: "仅显示用户 A 的会话"

expectedResults:
  - 会话完全隔离
  - 无数据泄露
  - 权限正确

cleanup:
  - 删除所有测试会话
```

#### ULC-ALL-SESS-002: 会话导入导出兼容性

```yaml
title: "验证会话在不同用户间正确导入导出"
userType: "All"
lifecycleStage: "数据共享"
feature: "export/import sharing"
priority: "high"
type: "integration"
tags: ["session", "sharing"]

preconditions:
  - 用户 A 有有效会话

steps:
  - step: 1
    action: "用户 A 导出会话"
    user: "user-a"
    command: "codecoder export <session-id> > shared.json"
    expected: "JSON 文件生成"

  - step: 2
    action: "用户 B 导入会话"
    user: "user-b"
    command: "codecoder import shared.json"
    expected: "会话导入成功"

  - step: 3
    action: "用户 B 访问导入的会话"
    user: "user-b"
    command: "codecoder run --attach <imported-session-id>"
    expected: "可以查看完整历史"

expectedResults:
  - 导出格式正确
  - 导入数据完整
  - 消息历史保留
  - 附件/部件正确

cleanup:
  - 删除导入的会话
  - 删除共享文件
```

---

## 测试执行策略

### 测试优先级

| 优先级   | 执行频率 | 测试类型    | 示例                 |
| -------- | -------- | ----------- | -------------------- |
| Critical | 每次提交 | E2E         | 认证流程、会话创建   |
| High     | 每日     | Integration | Agent 切换、MCP 调用 |
| Medium   | 每周     | Integration | 导出导入、统计       |
| Low      | 每次发布 | Unit        | 统计信息、缓存刷新   |

### 测试环境

```yaml
environments:
  local:
    description: "本地开发环境"
    setup:
      - "bun install"
      - "codecoder auth login"

  ci:
    description: "CI/CD 环境"
    setup:
      - "使用测试 API Key"
      - "模拟 MCP 服务器"

  staging:
    description: "预发布环境"
    setup:
      - "连接 staging 服务器"
      - "使用测试账户"
```

### 测试数据

```yaml
testData:
  apiKeys:
    anthropic: "sk-ant-test-xxx"
    openai: "sk-test-xxx"

  mcpServers:
    - name: "test-server"
      command: "node /path/to/test-mcp.js"

  testProjects:
    - name: "empty-project"
      files: []
    - name: "ts-project"
      files: ["package.json", "tsconfig.json", "src/index.ts"]
```

---

## 测试报告模板

```markdown
# CodeCoder 测试报告

**执行时间**: 2024-XX-XX
**执行人**: XXX
**环境**: XXX

## 测试概要

| 指标     | 数值 |
| -------- | ---- |
| 总用例数 | XX   |
| 通过     | XX   |
| 失败     | XX   |
| 跳过     | XX   |
| 通过率   | XX%  |

## 失败用例

| ID          | 标题 | 错误 | 优先级 |
| ----------- | ---- | ---- | ------ |
| ULC-XXX-XXX | XXX  | XXX  | PXX    |

## 阻塞问题

- [ ] 问题描述
- [ ] 影响范围
- [ ] 修复计划

## 建议

- 功能改进建议
- 测试用例优化建议
```

---

## 附录

### A. 命令速查表

| 命令                             | 描述             | 用户类型    |
| -------------------------------- | ---------------- | ----------- |
| `codecoder auth login`           | 登录提供商       | All         |
| `codecoder auth logout`          | 登出提供商       | All         |
| `codecoder auth list`            | 列出已登录提供商 | All         |
| `codecoder run`                  | 启动会话 (TUI)   | All         |
| `codecoder run --continue`       | 继续上一个会话   | CLI, TUI    |
| `codecoder run --attach <id>`    | 附加到指定会话   | CLI, TUI    |
| `codecoder serve`                | 启动 HTTP 服务器 | Web, Remote |
| `codecoder web`                  | 启动 Web 界面    | Web         |
| `codecoder session list`         | 列出会话         | All         |
| `codecoder session delete <id>`  | 删除会话         | All         |
| `codecoder export <id>`          | 导出会话         | All         |
| `codecoder import <file>`        | 导入会话         | All         |
| `codecoder mcp add <name> <cmd>` | 添加 MCP 服务器  | MCP         |
| `codecoder mcp list`             | 列出 MCP 服务器  | MCP         |
| `codecoder mcp auth <server>`    | 认证 MCP 服务器  | MCP         |
| `codecoder mcp logout <server>`  | 注销 MCP 服务器  | MCP         |
| `codecoder mcp debug <server>`   | 调试 MCP 服务器  | MCP         |
| `codecoder models`               | 列出模型         | All         |
| `codecoder models --refresh`     | 刷新模型缓存     | Power       |
| `codecoder pr create`            | 创建 PR          | CLI         |
| `codecoder pr review <url>`      | 审查 PR          | CLI         |
| `codecoder acp`                  | 启动 ACP 服务器  | ACP         |
| `codecoder upgrade`              | 升级 CodeCoder   | Power       |
| `codecoder stats`                | 查看统计         | Power       |
| `codecoder uninstall`            | 卸载 CodeCoder   | All         |

### B. Agent 速查表

| Agent      | 命令       | 用途            |
| ---------- | ---------- | --------------- |
| Build      | `/build`   | 代码构建和实现  |
| Plan       | `/plan`    | 任务规划和分解  |
| General    | `/general` | 通用对话 (默认) |
| Explore    | `/explore` | 代码搜索和探索  |
| Compaction | -          | 会话压缩 (自动) |
| Title      | -          | 自动生成标题    |
| Summary    | -          | 自动生成摘要    |

### C. 环境变量

| 变量                 | 描述           | 默认值    |
| -------------------- | -------------- | --------- |
| `CCODE_BASE_URL` | 远程服务器地址 | localhost |
| `CCODE_PORT`     | 服务器端口     | 4096      |
| `CCODE_API_KEY`  | API Key        | -         |
| `CCODE_MODEL`    | 默认模型       | -         |

---

**文档版本**: 1.1.36
**最后更新**: 2025-01-28
**维护者**: CodeCoder Team
