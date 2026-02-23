# CodeCoder + ZeroBot 集成实现进度

**开始时间**: 2026-02-23
**完成时间**: 2026-02-23
**状态**: ✅ 已完成

## 实现进度

### Phase 1: IM Agent 深度集成 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**修改文件**:
- `services/zero-channels/src/bridge.rs`

**实现内容**:

1. **Agent 命令解析器** (`parse_agent_command`)
   - 解析 `@agent_name prompt` 模式
   - 支持 23 个 Agent（macro, decision, trader, observer, picker 等）
   - 支持中英文冒号分隔符（`:` 和 `：`）
   - 大小写不敏感

2. **帮助命令** (`is_agent_help_request`, `format_agent_help`)
   - 支持 `@help`, `@?`, `@帮助`, `@agents`, `help agents`, `list agents`
   - 返回格式化的 Agent 列表和使用说明

3. **路由集成**
   - Agent 命令优先于普通聊天处理
   - 通过 `process_chat_with_agent` 传递 agent 参数到 CodeCoder API

**测试**:
- `test_agent_command_parsing_zhurong_agents` - 祝融说系列 Agent 测试
- `test_agent_command_parsing_engineering_agents` - 工程类 Agent 测试
- `test_agent_command_parsing_with_chinese_colon` - 中文冒号测试
- `test_agent_command_parsing_case_insensitive` - 大小写测试
- `test_agent_command_parsing_negative_cases` - 负面用例测试
- `test_agent_help_request_detection` - 帮助请求测试
- `test_agent_help_format` - 帮助格式测试

**验证命令**:
```bash
cargo test agent_ -p zero-channels
```

**使用示例**:
```
# Telegram/飞书/钉钉发送
@macro 解读本月PMI数据
@decision 用CLOSE框架分析这个职业选择
@trader 分析今日情绪周期
@help  # 查看可用Agent列表
```

---

### Phase 2: Web 管理台完善 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**现有功能分析**:
Admin.tsx 页面已包含完整的管理功能：
- Users Tab: 用户管理（搜索、角色、状态、Token 使用）
- Roles Tab: 角色定义与权限
- Quotas Tab: Token 配额管理
- Budgets Tab: 部门预算管理与告警
- DLP Tab: 数据泄露防护规则与事件
- Executive Tab: Executive Dashboard（成本趋势、团队用量、项目活动）

**新增 API Handlers**:

1. **Budget Handler** (`handlers/budget.ts`)
   - `GET /api/v1/budgets/summary` - 预算汇总
   - `GET /api/v1/budgets` - 列出预算
   - `GET /api/v1/budgets/:id` - 获取单个预算
   - `POST /api/v1/budgets` - 创建预算
   - `PUT /api/v1/budgets/:id` - 更新预算
   - `DELETE /api/v1/budgets/:id` - 删除预算
   - `GET /api/v1/budgets/alerts` - 预算告警列表
   - `POST /api/v1/budgets/alerts/:id/acknowledge` - 确认告警
   - `POST /api/v1/budgets/:id/record` - 记录消费（内部API）

2. **DLP Handler** (`handlers/dlp.ts`)
   - `GET /api/v1/dlp/summary` - DLP 统计摘要
   - `GET /api/v1/dlp/config` - DLP 配置
   - `PUT /api/v1/dlp/config` - 更新 DLP 配置
   - `GET /api/v1/dlp/rules` - 规则列表
   - `POST /api/v1/dlp/rules` - 创建规则
   - `PUT /api/v1/dlp/rules/:id` - 更新规则
   - `DELETE /api/v1/dlp/rules/:id` - 删除规则
   - `GET /api/v1/dlp/whitelist` - 白名单列表
   - `POST /api/v1/dlp/whitelist` - 添加白名单
   - `DELETE /api/v1/dlp/whitelist/:id` - 删除白名单
   - `GET /api/v1/dlp/incidents` - 事件列表
   - `POST /api/v1/dlp/scan` - 扫描内容（内部API）

**默认 DLP 规则**:
- API Keys (sk-xxx, pk-xxx)
- AWS Access Keys (AKIA...)
- Credit Card Numbers
- Social Security Numbers
- Email Addresses (默认禁用)

**存储**:
- Budget: `~/.codecoder/budgets/budgets.json`, `alerts.json`
- DLP: `~/.codecoder/dlp/rules.json`, `incidents.json`, `whitelist.json`, `config.json`

---

### Phase 3: 自主求解完整闭环 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**新增文件**:
- `packages/ccode/src/autonomous/execution/sandbox.ts`
- `packages/ccode/src/autonomous/execution/enhanced-web-search.ts`
- `packages/ccode/src/autonomous/execution/knowledge-sedimentation.ts`
- `packages/ccode/src/autonomous/execution/evolution-loop.ts`

**实现内容**:

1. **Sandbox Executor** (`sandbox.ts`)
   - 支持 Python/Node.js/Shell 执行
   - 代码安全验证（危险模式检测）
   - 资源限制（内存、超时）
   - 环境变量过滤（移除敏感信息）
   - 自动反思与重试机制

2. **Enhanced Web Search** (`enhanced-web-search.ts`)
   - 实际 URL 获取（使用 Bun fetch）
   - HTML 内容解析与提取
   - 代码块提取
   - 文档置信度计算
   - 技术特定文档 URL 生成

3. **Knowledge Sedimentation** (`knowledge-sedimentation.ts`)
   - 9 种知识分类（error_solution, api_pattern, code_snippet 等）
   - 自动标签提取
   - 相似知识去重与合并
   - 成功次数追踪
   - 置信度累积

4. **Evolution Loop** (`evolution-loop.ts`)
   - 完整 4 步进化循环实现：
     - Step 1: 主动资源检索（Web Search）
     - Step 2: 查询已有知识库
     - Step 3: 代码生成与反思重试
     - Step 4: 知识沉淀
   - 可配置最大重试次数
   - 详细执行追踪

**4 步进化循环流程**:
```
问题输入 → 置信度评估 → 需要搜索？
    ↓                        ↓ 是
  查询知识库              联网搜索文档
    ↓                        ↓
  找到？→ 是 → 直接使用  ← 提取代码示例
    ↓ 否                     ↓
  生成解决代码 ← ─ ─ ─ ─ ─ ─┘
    ↓
  沙箱执行 → 成功？→ 是 → 知识沉淀 → 返回结果
    ↑         ↓ 否
    └─ 反思修正 ←┘
```

**使用示例**:
```typescript
import { evolveProblem } from "@/autonomous"

const result = await evolveProblem({
  sessionId: "session-123",
  description: "接入 Stripe v4 API",
  technology: "typescript",
  enableWebSearch: true,
  enableCodeExecution: true,
})

if (result.solved) {
  console.log("Solution:", result.solution)
  console.log("Knowledge ID:", result.knowledgeId)
}
```

---

### Phase 4: 全局上下文枢纽 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**新增文件**:
- `packages/ccode/src/api/server/handlers/context-hub.ts`

**修改文件**:
- `packages/ccode/src/api/server/router.ts` - 注册 Context Hub 路由

**实现内容**:

1. **Context Hub Handler** (`context-hub.ts`)
   - 跨用户/跨部门知识共享系统
   - 权限控制（private/department/global）
   - 10 种知识分类

2. **API 端点**:
   - `GET /api/v1/hub/stats` - 获取 Hub 统计信息
   - `GET /api/v1/hub/entries` - 列出知识条目
   - `GET /api/v1/hub/entries/:id` - 获取单个条目
   - `POST /api/v1/hub/entries` - 创建知识条目
   - `PUT /api/v1/hub/entries/:id` - 更新条目
   - `DELETE /api/v1/hub/entries/:id` - 删除条目
   - `POST /api/v1/hub/search` - 搜索知识库
   - `POST /api/v1/hub/entries/:id/helpful` - 标记有用
   - `GET /api/v1/hub/tags` - 获取标签列表
   - `GET /api/v1/hub/categories` - 获取分类列表
   - `POST /api/v1/hub/agent-context` - 获取 Agent 上下文

3. **知识分类体系**:
   - `prd` - 产品需求文档
   - `meeting_notes` - 会议纪要
   - `lessons_learned` - 经验教训
   - `risk_log` - 风控日志
   - `architecture` - 架构设计
   - `runbook` - 运维手册
   - `faq` - 常见问题
   - `onboarding` - 入职指南
   - `decision` - 决策记录
   - `custom` - 自定义

4. **可见性控制**:
   - `private` - 仅创建者可见
   - `department` - 部门内可见
   - `global` - 全局可见

5. **Agent 上下文集成**:
   - 根据查询自动检索相关上下文
   - 按相关性排序返回
   - 支持 Agent 在处理请求时增强上下文

**存储**:
- 数据: `~/.codecoder/context-hub/entries.json`
- 索引: `~/.codecoder/context-hub/index.json`

**使用示例**:
```typescript
// 创建知识条目
POST /api/v1/hub/entries
{
  "title": "支付系统架构设计",
  "category": "architecture",
  "content": "...",
  "tags": ["payment", "microservices"],
  "visibility": "department",
  "department": "engineering"
}

// Agent 获取相关上下文
POST /api/v1/hub/agent-context
{
  "query": "支付系统",
  "agentId": "architect",
  "userId": "user-123"
}
```

---

### Phase 5: 安全沙箱完善 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**修改文件**:
- `services/zero-gateway/src/sandbox.rs`

**已完成功能**（Phase 2 + Phase 5）:

**TypeScript DLP Handler** (`packages/ccode/src/api/server/handlers/dlp.ts`):
- 完整的 DLP 规则 CRUD API
- 事件记录与查询
- 白名单管理
- 内容扫描 API

**Rust Gateway Sandbox** (`services/zero-gateway/src/sandbox.rs`):
- 敏感数据检测与脱敏
- 请求过滤（路径遍历、XSS、SQL 注入）
- 审计日志

**新增敏感数据模式**:

| 类别 | 模式名称 | 示例 |
|-----|---------|-----|
| API Keys | google_api_key | AIza... |
| API Keys | github_token | ghp_... |
| API Keys | github_oauth | gho_... |
| API Keys | stripe_key | sk_live_... |
| API Keys | slack_token | xoxb-... |
| Credentials | secret_field | "secret": "..." |
| PII | ssn_us | 123-45-6789 |
| PII | phone_us | (555) 123-4567 |
| PII | phone_cn | 13800138000 |
| PII | id_cn | 身份证号 |
| PII | bank_account_json | "account": "..." |
| PII | iban | DE89... |
| Database | postgres_url | postgres://... |
| Database | mysql_url | mysql://... |
| Database | redis_url | redis://... |

**双层安全架构**:
```
用户请求 → Zero-Gateway (Rust)
              ↓
         敏感数据检测
         请求过滤
         审计日志
              ↓
         CodeCoder API (TypeScript)
              ↓
         DLP 扫描
         规则匹配
         事件记录
              ↓
         Agent 执行
```

---

### Phase 6: 统一 Token 网关 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**新增文件**:
- `packages/ccode/src/api/server/handlers/token-gateway.ts`

**修改文件**:
- `packages/ccode/src/api/server/router.ts` - 注册 Token Gateway 路由

**实现内容**:

1. **部门级 Token 池管理**
   - 创建/更新/删除部门 Token 池
   - 设置月度预算
   - 配置告警阈值
   - 配置超额行为（block/downgrade/notify）

2. **用户配额分配**
   - 从部门池分配配额给用户
   - 设置日/月限额
   - 支持动态调整
   - 自动超额处理

3. **告警系统**
   - 阈值触发告警
   - 三级告警（info/warning/critical）
   - 告警确认机制
   - 可配置 Webhook 通知

4. **模型分层**
   - Premium 层：claude-3-opus, gpt-4
   - Standard 层：claude-3-sonnet, gpt-4-turbo
   - Budget 层：claude-3-haiku, gpt-3.5-turbo
   - 超额自动降级

5. **API 端点** (20 个):
   - `GET /api/v1/gateway/stats` - 网关统计
   - `GET /api/v1/gateway/config` - 获取配置
   - `PUT /api/v1/gateway/config` - 更新配置
   - `GET /api/v1/gateway/pools` - 列出部门池
   - `GET /api/v1/gateway/pools/:id` - 获取部门池
   - `POST /api/v1/gateway/pools` - 创建部门池
   - `PUT /api/v1/gateway/pools/:id` - 更新部门池
   - `DELETE /api/v1/gateway/pools/:id` - 删除部门池
   - `GET /api/v1/gateway/allocations` - 列出配额分配
   - `GET /api/v1/gateway/allocations/:userId` - 获取用户配额
   - `POST /api/v1/gateway/allocations` - 创建/更新配额
   - `DELETE /api/v1/gateway/allocations/:userId` - 删除配额
   - `POST /api/v1/gateway/record` - 记录使用量
   - `GET /api/v1/gateway/check/:userId` - 检查配额
   - `GET /api/v1/gateway/alerts` - 列出告警
   - `POST /api/v1/gateway/alerts/:id/acknowledge` - 确认告警
   - `POST /api/v1/gateway/reset-daily` - 重置日使用量
   - `POST /api/v1/gateway/reset-monthly` - 重置月使用量
   - `GET /api/v1/gateway/usage` - 使用历史
   - `GET /api/v1/gateway/health` - 健康检查

**存储**:
- 部门池: `~/.codecoder/token-gateway/pools.json`
- 用户配额: `~/.codecoder/token-gateway/allocations.json`
- 告警: `~/.codecoder/token-gateway/alerts.json`
- 使用记录: `~/.codecoder/token-gateway/usage.json`
- 配置: `~/.codecoder/token-gateway/config.json`

**使用示例**:
```typescript
// 创建部门 Token 池
POST /api/v1/gateway/pools
{
  "name": "Engineering",
  "monthlyBudget": 50000000,  // 50M tokens
  "alertThreshold": 80,
  "overageAction": "downgrade",
  "downgradeTier": "standard"
}

// 分配用户配额
POST /api/v1/gateway/allocations
{
  "userId": "user-123",
  "departmentId": "pool-id",
  "dailyLimit": 1000000,
  "monthlyLimit": 10000000
}

// 检查用户是否可以请求
GET /api/v1/gateway/check/user-123
// Response:
{
  "allowed": true,
  "tier": "premium",
  "allowedModels": ["claude-3-opus", "gpt-4"],
  "remaining": {
    "daily": 500000,
    "monthly": 8000000
  }
}
```

**与 Rust Gateway 的集成**:
- TypeScript API 提供管理界面
- Rust `metering.rs` 中间件调用 `/api/v1/gateway/record` 记录使用
- Rust `quota.rs` 可升级为调用 `/api/v1/gateway/check/:userId`

---

## 技术要点

### Agent 路由协议

```rust
// 支持的 Agent 列表
const AGENTS: &[&str] = &[
    // 主模式
    "build", "plan", "autonomous", "writer",
    // 工程类
    "code-reviewer", "security-reviewer", "tdd-guide", "architect", "explore", "general",
    // 内容类
    "proofreader", "expander", "expander-fiction", "expander-nonfiction",
    // 逆向工程
    "code-reverse", "jar-code-reverse",
    // 祝融说系列
    "observer", "decision", "macro", "trader", "picker", "miniproduct", "ai-engineer",
    // 工具
    "synton-assistant", "verifier",
];
```

### 消息流程

```
IM 用户 → Telegram/飞书 → Zero-Channels
    → parse_agent_command() → 检测 @agent 命令
    → process_chat_with_agent() → 传递 agent 参数
    → CodeCoder /api/v1/chat → SessionPrompt.prompt()
    → Agent 执行 → 响应返回 → IM 用户
```

---

### Phase 7: 系统架构演进 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

基于 `docs/standards/tech-structure.md` 中描述的"五层一环"架构，实施关键缺失组件。

**新增文件**:

1. **Event Bus** (`services/zero-common/src/bus.rs`)
   - 事件总线抽象层，支持 Rust ↔ TypeScript 通信
   - 支持多后端：InMemory（开发）、Redis（生产）
   - Topic 模式匹配（`agent.*`, `channel.#`）
   - 预定义事件类型和 Topic 常量

2. **Discord Format Module** (`services/zero-channels/src/discord/format.rs`)
   - 标准 Markdown → Discord Markdown 转换
   - 标题映射：`# Title` → `**__Title__**`
   - 多行引用：`> Quote` → `>>> Quote`
   - 消息分块（2000 字符限制）

3. **Slack Format Module** (`services/zero-channels/src/slack/format.rs`)
   - 标准 Markdown → Slack mrkdwn 转换
   - 语法映射：`**bold**` → `*bold*`, `*italic*` → `_italic_`
   - 链接格式：`[text](url)` → `<url|text>`
   - 用户/频道 mention 支持

4. **Docker Sandbox** (`packages/ccode/src/autonomous/execution/docker-sandbox.ts`)
   - 生产级容器隔离执行
   - 支持 Python/Node.js/Shell/Rust/Go
   - 安全特性：
     - `--read-only` 只读文件系统
     - `--network none` 网络隔离
     - `--cap-drop ALL` 移除所有 Linux capabilities
     - `--security-opt no-new-privileges` 禁止提权
     - CPU/内存/进程数限制

**修改文件**:

1. **services/zero-common/src/lib.rs**
   - 导出 bus 模块及类型

2. **services/zero-channels/src/discord/mod.rs**
   - 重构为目录模块，集成 format 模块
   - send() 方法自动转换 Markdown 格式

3. **services/zero-channels/src/slack/mod.rs**
   - 重构为目录模块，集成 format 模块
   - send() 方法自动转换 mrkdwn 格式

4. **packages/ccode/src/autonomous/execution/sandbox.ts**
   - 添加 Docker 后端支持
   - 新增 `SandboxBackend` 类型 ("process" | "docker")
   - 自动回退机制：Docker 不可用时使用进程执行
   - 新增 `createSandboxExecutorWithFallback()` 工厂函数

5. **services/zero-gateway/src/sandbox.rs**
   - 添加 SQLite 审计日志持久化
   - 新增 `AuditStorage` 枚举（Memory/Sqlite）
   - 新增 `audit_db_path` 配置项
   - 持久化 API：
     - `get_audit_count()` - 获取审计日志总数
     - `get_audit_by_action_type()` - 按类型查询

**架构覆盖率更新**:

| 层级 | 之前 | 现在 | 改进 |
|------|------|------|------|
| 触点层 | 90% | 95% | Discord/Slack 格式化 |
| 中枢调度层 | 80% | 90% | Event Bus 框架 |
| 深度执行层 | 100% | 100% | - |
| 自主保底层 | 70% | 90% | Docker 沙箱隔离 |
| 全局记忆层 | 80% | 85% | 审计日志持久化 |

**验证命令**:
```bash
# Rust 编译检查
cd services && cargo check -p zero-common -p zero-gateway -p zero-channels

# TypeScript 类型检查
bun turbo typecheck --filter=ccode
```

**Event Bus 使用示例**:
```rust
use zero_common::bus::{create_bus, BusBackend, Event, topics};

// 创建事件总线
let bus = create_bus(BusBackend::Memory, None);

// 订阅 agent 相关事件
let mut receiver = bus.subscribe("agent.*").await?;

// 发布事件
let event = Event::new(topics::agent::REQUEST, "invoke", "zero-channels")
    .with_payload(AgentRequestPayload {
        agent_name: "macro".to_string(),
        prompt: "分析 PMI 数据".to_string(),
        session_id: "session-123".to_string(),
        context: None,
    })?;

bus.publish(event).await?;

// 接收事件
if let Some(event) = receiver.recv().await {
    println!("Received: {:?}", event.event_type);
}
```

**Docker Sandbox 使用示例**:
```typescript
import { createSandboxExecutorWithFallback } from "@/autonomous/execution/sandbox"

// 自动选择最佳后端
const sandbox = await createSandboxExecutorWithFallback()
console.log(`Using backend: ${sandbox.getBackend()}`) // "docker" or "process"

// 执行代码
const result = await sandbox.execute({
  language: "python",
  code: `print("Hello from sandbox!")`,
  limits: {
    maxMemoryMb: 128,
    maxTimeMs: 10000,
    allowNetwork: false,
  },
})

if (result.exitCode === 0) {
  console.log(result.stdout)
}
```

---

## 下一步计划

### 短期 (1-2 周)
1. ~~实现 Redis Event Bus 后端~~ ✅ 已完成
2. ~~添加 WASM 轻量沙箱（用于简单脚本快速执行）~~ ✅ 已完成
3. 完善审计日志查询 API

### 中期 (1-2 月)
1. NATS Event Bus 后端
2. ~~代码依赖图（基于 LSP）~~ ✅ 已完成
3. RBAC 角色继承

---

### Phase 9: WASM 轻量沙箱 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**目标**: 为简单 JavaScript 脚本提供快速执行环境（比 Docker ~50x 更快）

**新增文件**:

1. **packages/ccode/src/autonomous/execution/wasm-sandbox.ts**
   - `WasmSandboxExecutor` 类：WASM 沙箱执行器
   - 使用 QuickJS 编译到 WebAssembly
   - 支持内存限制、CPU 时间限制
   - 控制台输出捕获
   - 全局变量注入

2. **packages/ccode/test/unit/autonomous/wasm-sandbox.test.ts**
   - 22 个测试用例，全部通过
   - 覆盖执行、错误处理、超时、验证等场景

**修改文件**:

1. **packages/ccode/package.json**
   - 添加 `quickjs-emscripten` 依赖

2. **packages/ccode/src/autonomous/execution/sandbox.ts**
   - 新增 `SandboxBackend` 类型: `"wasm"` | `"auto"`
   - 新增 `executeWithWasm()` 方法
   - 新增 `isWasmAvailable()` 方法
   - 新增 `createAutoSandboxExecutor()` 工厂函数
   - 新增 `createWasmSandboxExecutorWithFallback()` 工厂函数

**实现细节**:

```typescript
// WASM 沙箱配置
interface WasmSandboxConfig {
  maxMemoryBytes?: number  // 默认 128MB
  maxTimeMs?: number       // 默认 5000ms
  maxStackBytes?: number   // 默认 256KB
  captureConsole?: boolean // 默认 true
}

// 使用示例
const executor = await createWasmSandboxExecutor()

const result = await executor.execute({
  language: "javascript",
  code: `
    const sum = [1, 2, 3].reduce((a, b) => a + b, 0)
    console.log("Sum:", sum)
    sum
  `,
  globals: {
    userName: "Alice",
  },
  config: {
    maxTimeMs: 1000,
  },
})

console.log(result.stdout) // "Sum: 6"
console.log(result.returnValue) // "6"
```

**后端选择策略**:

| 后端 | 启动时间 | 隔离级别 | 支持语言 | 适用场景 |
|------|---------|---------|---------|---------|
| WASM | ~5ms | 高（内存隔离） | JavaScript | 简单脚本、快速计算 |
| Docker | ~500ms | 最高（容器隔离） | Python/JS/Shell/Rust/Go | 复杂脚本、生产环境 |
| Process | ~10ms | 低（代码验证） | Python/JS/Shell | 开发测试 |

**自动选择模式**:

```typescript
// 使用 "auto" 后端自动选择最佳执行方式
const executor = await createAutoSandboxExecutor()

// 简单 JS → WASM（最快）
await executor.execute({ language: "nodejs", code: "1 + 1" })

// 复杂 JS 或其他语言 → Docker
await executor.execute({ language: "python", code: "print(1)" })
```

**测试覆盖**:

| 测试类别 | 数量 | 状态 |
|---------|------|------|
| 执行测试 | 10 | ✅ |
| 错误处理 | 3 | ✅ |
| 超时测试 | 1 | ✅ |
| 验证函数 | 5 | ✅ |
| 后端推荐 | 3 | ✅ |
| **总计** | **22** | **全部通过** |

**架构覆盖率更新**:

| 层级 | 之前 | 现在 | 改进 |
|------|------|------|------|
| 自主保底层 | 90% | 95% | WASM 轻量沙箱 |

---

### Phase 8: Redis Event Bus 后端 ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**目标**: 实现生产级多进程通信的 Redis Pub/Sub 后端

**修改文件**:

1. **services/Cargo.toml**
   - 添加 `redis` 依赖（版本 0.27，带 `tokio-comp` 和 `connection-manager` 特性）

2. **services/zero-common/Cargo.toml**
   - 添加 `redis` 可选依赖
   - 添加 `futures-util` 可选依赖（用于 StreamExt）
   - 添加 `redis-backend` 特性标志

3. **services/zero-common/src/bus.rs**
   - 完整实现 `RedisBus` 结构体
   - 使用 `redis::aio::ConnectionManager` 处理发布
   - 使用 `redis::aio::PubSub` 处理订阅
   - 支持 PSUBSCRIBE 模式匹配
   - 自动重连逻辑
   - 健康检查（PING/PONG）
   - 添加 `channel_prefix` 配置用于命名空间隔离
   - 添加 `create_bus_async()` 异步工厂函数
   - 添加完整的单元测试和集成测试

4. **services/zero-common/src/lib.rs**
   - 导出 `create_bus_async` 函数

**实现细节**:

```rust
// RedisBus 结构体（feature = "redis-backend" 时）
pub struct RedisBus {
    config: RedisBusConfig,
    client: redis::Client,
    conn_manager: tokio::sync::RwLock<Option<redis::aio::ConnectionManager>>,
    local_senders: Arc<RwLock<HashMap<String, broadcast::Sender<Event>>>>,
    subscription_handles: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    healthy: Arc<std::sync::atomic::AtomicBool>,
}
```

**配置选项**:

```rust
pub struct RedisBusConfig {
    pub url: String,           // redis://127.0.0.1:6379
    pub timeout_secs: u64,     // 5
    pub max_reconnects: u32,   // 3
    pub channel_prefix: String, // "zero:"
}
```

**使用示例**:

```rust
use zero_common::bus::{create_bus_async, BusBackend, Event, RedisBusConfig};

// 使用默认配置
let bus = create_bus_async(BusBackend::Redis, None).await?;

// 使用自定义配置
let config = RedisBusConfig {
    url: "redis://redis.example.com:6379".to_string(),
    channel_prefix: "production:".to_string(),
    ..Default::default()
};
let bus = create_bus_async(BusBackend::Redis, Some(config)).await?;

// 订阅模式
let mut receiver = bus.subscribe("agent.*").await?;

// 发布事件
let event = Event::new("agent.request", "invoke", "zero-gateway");
bus.publish(event).await?;

// 接收事件
while let Some(event) = receiver.recv().await {
    println!("Received: {:?}", event);
}
```

**启用方式**:

```bash
# 编译时启用 Redis 后端
cargo build --package zero-common --features redis-backend

# 运行测试（需要 Redis 服务）
cargo test --package zero-common --features redis-backend -- redis
```

**测试覆盖**:

| 测试 | 描述 |
|------|------|
| `test_redis_config_default` | 默认配置验证 |
| `test_create_bus_async` | 异步工厂函数 |
| `test_redis_bus_health_check` | Redis 连接健康检查 |
| `test_redis_bus_publish_subscribe` | 基本发布/订阅 |
| `test_redis_bus_pattern_subscription` | 模式订阅 (`agent.*`) |
| `test_redis_bus_async_factory` | Redis 后端工厂 |

**架构覆盖率更新**:

| 层级 | 之前 | 现在 | 改进 |
|------|------|------|------|
| 中枢调度层 | 90% | 95% | Redis Event Bus 生产级实现 |

---

## 变更日志

| 日期 | 阶段 | 内容 |
|------|------|------|
| 2026-02-23 | Phase 1 | IM Agent 深度集成 |
| 2026-02-23 | Phase 2 | Web 管理台完善 |
| 2026-02-23 | Phase 3 | 自主求解完整闭环 |
| 2026-02-23 | Phase 4 | 全局上下文枢纽 |
| 2026-02-23 | Phase 5 | 安全沙箱完善 |
| 2026-02-23 | Phase 6 | 统一 Token 网关 |
| 2026-02-23 | Phase 7 | 系统架构演进 |
| 2026-02-23 | Phase 8 | Redis Event Bus 后端 |
| 2026-02-23 | Phase 9 | WASM 轻量沙箱 |
| 2026-02-23 | Phase 10 | 代码依赖图 (Call Graph) |

---

### Phase 10: 代码依赖图 (Call Graph) ✅ 已完成

**修改时间**: 2026-02-23 (当前会话)

**目标**: 利用 LSP Call Hierarchy API 构建真实的函数调用关系图

**新增文件**:

1. **packages/ccode/src/memory/knowledge/call-graph.ts**
   - `CallGraph` 命名空间：完整的调用图实现
   - 数据结构：
     - `CallNode` - 调用节点（函数/方法/构造函数）
     - `CallEdge` - 调用边（调用位置信息）
     - `CallChain` - 调用链分析结果
     - `RecursionInfo` - 递归检测结果
     - `Graph` - 完整调用图
   - 核心功能：
     - `build()` - 使用 LSP 扫描代码库构建调用图
     - `getCallers()` - 获取函数的调用者
     - `getCallees()` - 获取函数调用的其他函数
     - `analyzeCallChain()` - BFS 分析调用链深度
     - `findHotspots()` - 查找热点函数（被调用最多）
     - `detectRecursion()` - 检测直接/间接递归
   - 工具函数：
     - `toMermaid()` - 导出为 Mermaid 图表格式
     - `getStats()` - 获取图统计信息
     - `getNodeByName()` - 按名称查找节点
     - `getNodesByFile()` - 按文件查找节点
     - `getEdgesBetween()` - 查找节点间的边

2. **packages/ccode/test/unit/memory/call-graph.test.ts**
   - 39 个测试用例，全部通过
   - 覆盖：
     - Schema 验证（CallNode/CallEdge/Graph/CallChain/RecursionInfo）
     - 图数据结构构建
     - 热点检测算法
     - BFS 调用链分析
     - DFS 环检测算法
     - 统计计算
     - Mermaid 导出

**修改文件**:

1. **packages/ccode/src/memory/index.ts**
   - 导出 `CallGraph` 模块
   - 在 `invalidate()` 中添加 `CallGraph.invalidate()`
   - 在 `getMemoryStats()` 中添加 callGraph 统计

2. **packages/ccode/src/memory/knowledge/semantic-graph.ts**
   - 导入 `CallGraph` 模块
   - 新增 `populateCallEdges()` - 将 CallGraph 数据填充到 SemanticGraph 的 "calls" 边
   - 新增 `buildWithCalls()` - 一键构建带调用关系的完整语义图

**实现细节**:

```typescript
// CallGraph 数据结构
interface CallNode {
  id: string           // "call:funcName:src/file.ts:10"
  name: string         // "funcName"
  kind: "function" | "method" | "constructor"
  file: string         // "src/file.ts"
  line: number         // 10
  character: number    // 0
  detail?: string      // 签名信息
}

interface CallEdge {
  id: string           // "caller->callee"
  caller: string       // 调用者节点 ID
  callee: string       // 被调用者节点 ID
  locations: Array<{ line: number; character: number }>
}

// 使用示例
import { CallGraph } from "@/memory"

// 构建调用图
const graph = await CallGraph.build()

// 查找热点函数
const hotspots = await CallGraph.findHotspots(10)
console.log("Most called functions:", hotspots)

// 分析调用链
const chain = await CallGraph.analyzeCallChain("call:main:index.ts:1", 5)
console.log("Call depth:", chain.depth)

// 检测递归
const recursions = await CallGraph.detectRecursion()
for (const r of recursions) {
  console.log(`${r.type} recursion in ${r.node.name}:`, r.cycle)
}

// 导出 Mermaid 图表
const mermaid = await CallGraph.toMermaid({ direction: "LR", maxNodes: 30 })
console.log(mermaid)
// 输出:
// graph LR
//   call_main_index_ts_1(main)
//   call_helper_utils_ts_5(helper)
//   call_main_index_ts_1 --> call_helper_utils_ts_5
```

**LSP 集成**:

```typescript
// 使用 LSP Call Hierarchy API
const callers = await CallGraph.getCallers({
  file: "src/service.ts",
  line: 15,
  character: 0,
})

const callees = await CallGraph.getCallees({
  file: "src/service.ts",
  line: 15,
  character: 0,
})
```

**SemanticGraph 集成**:

```typescript
import { SemanticGraph } from "@/memory"

// 构建完整的带调用边的语义图
const graph = await SemanticGraph.buildWithCalls()

// 或者增量添加调用边
await SemanticGraph.populateCallEdges()
// { added: 150 }  // 添加了 150 条调用边
```

**算法说明**:

| 功能 | 算法 | 时间复杂度 |
|------|------|-----------|
| analyzeCallChain | BFS | O(V + E) |
| detectRecursion | DFS 环检测 | O(V + E) |
| findHotspots | 入度排序 | O(V log V) |

**测试覆盖**:

| 测试类别 | 数量 | 状态 |
|---------|------|------|
| Schema 验证 | 16 | ✅ |
| 图数据结构 | 5 | ✅ |
| 热点检测 | 2 | ✅ |
| BFS 调用链 | 4 | ✅ |
| DFS 环检测 | 5 | ✅ |
| 统计计算 | 2 | ✅ |
| Mermaid 导出 | 5 | ✅ |
| **总计** | **39** | **全部通过** |

**架构覆盖率更新**:

| 层级 | 之前 | 现在 | 改进 |
|------|------|------|------|
| 全局记忆层 | 85% | 90% | Call Graph 实现 |

**验证命令**:

```bash
# 运行测试
cd packages/ccode && bun test test/unit/memory/call-graph.test.ts

# 类型检查
bun turbo typecheck --filter=ccode
```

---

## 当前架构覆盖率

| 层级 | 覆盖率 | 核心组件 |
|------|--------|----------|
| 触点层 | 95% | Telegram ✅, Discord ✅, Slack ✅, Web ✅, TUI ✅ |
| 中枢调度层 | 95% | API网关 ✅, LLM路由 ✅, DLP ✅, 事件总线 ✅, Redis Bus ✅ |
| 深度执行层 | 100% | LSP ✅, 多Agent ✅, 上下文 ✅ |
| 自主保底层 | 95% | Web检索 ✅, Docker沙箱 ✅, WASM沙箱 ✅, REPL ✅ |
| 全局记忆层 | 90% | 向量SQLite ✅, Markdown记忆 ✅, 审计持久化 ✅, Call Graph ✅ |
