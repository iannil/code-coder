# TypeScript / Rust 边界规范

> 生成时间: 2026-03-10
> 文档状态: 规范文档

---

## 概述

本文档定义了 CodeCoder 系统中 TypeScript (packages/ccode) 和 Rust (services/zero-*) 之间的职责边界。

**核心原则**: 高确定性任务用 Rust 保证效率；高不确定性任务用 LLM 保证正确反应。

---

## 架构边界图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TypeScript (packages/ccode)                     │
│                                                                         │
│  用户接口层:                                                             │
│  ├── TUI (Solid.js + OpenTUI)                                          │
│  ├── CLI 命令解析                                                       │
│  └── API Server (:4400)                                                │
│                                                                         │
│  不确定性逻辑 (需要 LLM 推理):                                            │
│  ├── Agent Engine (31 个 Agent)                                        │
│  ├── Observer Network (4 个观察者 + 共识引擎)                            │
│  ├── LLM 调用 (Vercel AI SDK)                                          │
│  └── Prompt 管理                                                       │
│                                                                         │
│  工具包装层 (调用 NAPI):                                                 │
│  └── src/tool/*.ts → @codecoder-ai/core (Rust NAPI)                    │
└─────────────────────────────────────────────────────────────────────────┘
                                │
                                │ NAPI + HTTP
                                ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       Rust (services/zero-*)                            │
│                                                                         │
│  确定性工具 (zero-core):                                                 │
│  ├── grep, glob, read, write, edit (tools/)                            │
│  ├── shell, codesearch, apply_patch                                    │
│  ├── 协议解析 (MCP, LSP)                                                │
│  └── 性能关键路径                                                       │
│                                                                         │
│  共享模块 (zero-core::common):                                           │
│  ├── config (统一配置)                                                  │
│  ├── logging (结构化日志)                                               │
│  ├── bus (事件总线)                                                     │
│  └── security (安全模块)                                                │
│                                                                         │
│  安全边界 (zero-hub::gateway):                                           │
│  ├── 认证/授权 (Pairing + JWT)                                          │
│  ├── RBAC 权限控制                                                      │
│  └── 加密/签名                                                         │
│                                                                         │
│  服务中枢 (zero-hub):                                                    │
│  ├── IM 渠道适配 (channels/)                                            │
│  ├── 调度系统 (workflow/)                                               │
│  └── 交易系统 (zero-trading)                                            │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 边界判定规则

### 放在 Rust 的任务

| 任务类型 | 示例 | 原因 |
|----------|------|------|
| **协议解析** | JSON, Markdown, TOML, Telegram API | 格式固定，性能敏感 |
| **加密/签名验证** | JWT, HMAC, 配对码 | 安全敏感，需要正确性保证 |
| **文件操作** | grep, glob, read, write, edit | 性能敏感，大量 I/O |
| **正则匹配** | 代码搜索、模式匹配 | CPU 密集，性能敏感 |
| **调度执行** | Cron 任务、定时器 | 规则明确，需要可靠性 |
| **消息路由** | 渠道分发、Webhook 处理 | 逻辑固定，需要高吞吐 |

### 放在 TypeScript/LLM 的任务

| 任务类型 | 示例 | 原因 |
|----------|------|------|
| **意图理解** | 解析用户想要什么 | 需要语义理解能力 |
| **Agent 选择与协调** | 选择合适的 Agent | 需要推理和上下文 |
| **用户交互** | TUI, CLI 响应 | 需要灵活的 UI 逻辑 |
| **代码生成与审查** | 生成代码、审查建议 | 需要代码理解能力 |
| **决策建议** | CLOSE 框架分析 | 需要领域知识 |
| **自然语言处理** | 多轮对话、上下文管理 | LLM 核心能力 |

---

## 决策流程图

```
          收到新任务
              │
              ▼
    ┌─────────────────┐
    │  输入是否固定?  │
    │  (协议/格式)    │
    └────────┬────────┘
             │
      ┌──────┴──────┐
      │             │
      ▼ Yes         ▼ No
  ┌────────┐    ┌────────┐
  │  Rust  │    │  继续  │
  │        │    │        │
  └────────┘    └────┬───┘
                     │
                     ▼
           ┌─────────────────┐
           │  需要语义理解?  │
           │  (意图/推理)    │
           └────────┬────────┘
                    │
             ┌──────┴──────┐
             │             │
             ▼ Yes         ▼ No
         ┌────────┐    ┌────────────┐
         │ TS/LLM │    │  继续      │
         │        │    │            │
         └────────┘    └─────┬──────┘
                             │
                             ▼
                  ┌─────────────────┐
                  │  性能关键路径?  │
                  │  (大量I/O/CPU)  │
                  └────────┬────────┘
                           │
                    ┌──────┴──────┐
                    │             │
                    ▼ Yes         ▼ No
                ┌────────┐    ┌────────┐
                │  Rust  │    │ TS/LLM │
                │        │    │        │
                └────────┘    └────────┘
```

---

## 实际调用流程示例

### 示例 1: Telegram 用户发送消息

```
1. zero-hub::channels  → 确定性地解析 Telegram API 消息格式  ← 协议固定
2. zero-hub::gateway   → 确定性地验证用户身份和配额         ← 规则明确
3. ccode API           → 不确定性地理解用户意图             ← 需要推理
4. code-reviewer Agent → 不确定性地生成审查建议             ← 需要智能
5. zero-hub::channels  → 确定性地格式化并发送回 Telegram    ← 格式固定
```

### 示例 2: 执行代码搜索

```
1. ccode               → 解析用户搜索意图                   ← 需要推理
2. zero-core::grep     → 执行正则搜索                       ← 性能敏感
3. ccode               → 组织搜索结果、生成摘要              ← 需要智能
```

### 示例 3: 定时任务执行

```
1. zero-hub::scheduler → Cron 表达式解析、定时触发          ← 规则明确
2. ccode API           → 执行 Agent 任务                    ← 需要 LLM
3. zero-hub::channels  → 推送结果到渠道                     ← 协议固定
```

---

## 4 Crate 架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Rust Workspace (4 Crates)                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  zero-cli (入口点)                                                       │
│  ├── CLI 命令                                                           │
│  ├── Daemon (:4402)                                                     │
│  └── 依赖: zero-core, zero-hub                                          │
│                                                                         │
│  zero-core (核心库)                                                      │
│  ├── tools/ (grep, glob, edit, shell...)                                │
│  ├── common/ (原 zero-common 整合)                                       │
│  │   ├── config, logging, bus, error                                    │
│  │   ├── security/, memory/                                             │
│  │   └── events, guardrails, metrics                                    │
│  ├── protocol/ (MCP, LSP)                                               │
│  └── napi/ (Node.js 绑定)                                               │
│                                                                         │
│  zero-hub (服务中枢)                                                     │
│  ├── gateway/ (认证, RBAC, 配额)                                        │
│  ├── channels/ (Telegram, Discord, Slack...)                            │
│  └── workflow/ (调度, Webhook, Hands)                                   │
│  └── 依赖: zero-core                                                    │
│                                                                         │
│  zero-trading (独立业务)                                                 │
│  ├── PO3+SMT 策略                                                       │
│  └── 依赖: zero-core                                                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**为什么 zero-trading 保持独立?**

| 因素 | 评估 |
|------|------|
| 领域独立性 | PO3+SMT 策略是独立业务领域 |
| 依赖特殊性 | 使用 `statrs` 进行技术分析，其他 crate 不需要 |
| 部署灵活性 | 可选择性部署交易功能 |
| 单一职责 | 与 workflow 的调度职责不同 |

---

## 开发者指南

### 添加新功能时

1. **确定任务类型**: 使用上面的决策流程图
2. **选择实现位置**: Rust (zero-*) 或 TypeScript (ccode)
3. **定义接口**:
   - Rust → TS: HTTP API 或 Event Bus
   - TS → Rust: NAPI 绑定或 HTTP 调用

### 代码组织

```bash
# 确定性工具 (Rust)
services/zero-core/src/tools/         # 文件操作工具
services/zero-core/src/common/        # 共享模块 (原 zero-common)
services/zero-hub/src/gateway/        # 安全边界

# 不确定性逻辑 (TypeScript)
packages/ccode/src/agent/             # Agent 定义
packages/ccode/src/provider/          # AI Provider
packages/ccode/src/tool/              # 工具包装 (调用 NAPI)
```

### 测试策略

| 层级 | 测试方式 |
|------|----------|
| Rust 工具 | 单元测试 + 属性测试 (proptest) |
| HTTP API | 集成测试 (wiremock) |
| Agent 逻辑 | E2E 测试 + LLM 评估 |

---

## 相关文档

- `ARCHITECTURE.md` - 系统架构总览
- `CCODE_VS_ZERO.md` - 详细职责划分
- `DESIGN_PHILOSOPHY.md` - 设计哲学
