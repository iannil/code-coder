# OpenFang vs Code-Coder 比较分析

**创建时间**: 2026-02-27
**文档类型**: 架构对比分析
**状态**: 已完成

---

## Executive Summary

| 维度 | OpenFang | Code-Coder |
|------|----------|------------|
| **核心定位** | Agent Operating System (自洽代理系统) | 个人工作台 (融合工程与决策) |
| **主要语言** | Rust (纯 Rust 实现) | TypeScript + Rust (混合架构) |
| **二进制大小** | ~32MB | 依赖 Bun 运行时 |
| **冷启动时间** | 180ms | ~2-5s (Bun 启动) |
| **内存占用** | 40MB idle | 100-200MB (Node.js 基础) |
| **Agent 数量** | 7 Hands (自主代理) + 60 Skills | 23 Agents (交互式为主) |
| **安全层级** | 16 层防御 | 基础 RBAC + 沙箱 |
| **发布版本** | v0.1.0 (2026.02) | 活跃开发中 |

---

## 架构对比

### OpenFang: 纯 Rust 单体内核

```
┌─────────────────────────────────────────────────────────────┐
│                    openfang-cli                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │   kernel    │ │   runtime   │ │    api      │            │
│  └─────────────┘ └─────────────┘ └─────────────┘            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │  channels   │ │   memory    │ │   skills    │            │
│  └─────────────┘ └─────────────┘ └─────────────┘            │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │    hands    │ │extensions   │ │    wire     │            │
│  └─────────────┘ └─────────────┘ └─────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

**特点**:
- 单一二进制文件发布
- WASM 沙箱执行工具
- SQLite 持久化 + 向量嵌入
- 14 个独立 crate 模块化设计

### Code-Coder: TypeScript + Rust 混合架构

```
┌─────────────────────────────────────────────────────────────┐
│                  TypeScript Layer (Bun)                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  ccode - Agent Engine (23 agents)                   │    │
│  │  Memory System (Markdown-based)                     │    │
│  │  AI Provider Integration (20+ providers)            │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              ↕ HTTP/Redis
┌─────────────────────────────────────────────────────────────┐
│                    Rust Services Layer                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ zero-gw  │ │ zero-ch  │ │ zero-wf  │ │ zero-tr  │        │
│  │  :4430   │ │  :4431   │ │  :4432   │ │  :4434   │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
└─────────────────────────────────────────────────────────────┘
```

**特点**:
- 确定性任务 (Rust) vs 不确定性任务 (TS/LLM) 分离
- 多服务架构需要进程编排
- 透明 Markdown 记忆系统
- 哲学框架集成 (祝融说)

---

## 核心差异分析

### 1. Hands vs Agents 模式

| OpenFang Hands | Code-Coder Agents |
|----------------|-------------------|
| **自主运行**: 按 schedule 工作，无需提示 | **交互式**: 等待用户输入 |
| **任务导向**: Clip, Lead, Predictor 等 | **能力导向**: build, plan, code-reviewer |
| **持久化状态**: SQLite 完整追踪 | **会话式**: 每次重新启动 |
| **7 个精心设计** | **23 个通用 agent** |

`★ Insight ─────────────────────────────────────`
OpenFang 的 Hands 模式更接近"自主员工"——你定义目标和约束，它们持续工作。Code-Coder 的 Agents 更像"专家顾问"——你提问，它们回答。

Hands 适合:
- 定期数据收集 (Collector, Lead)
- 内容处理流水线 (Clip)
- 监控与预警 (Predictor)

Agents 适合:
- 一次性复杂任务 (代码审查、架构设计)
- 需要人类判断的决策 (CLOSE 框架)
`─────────────────────────────────────────────────`

### 2. 安全架构

#### OpenFang: 16 层防御

1. WASM Dual-Metered Sandbox (工具执行)
2. Merkle Hash-Chain Audit Trail (审计)
3. Information Flow Taint Tracking (污点追踪)
4. Ed25519 Signed Agent Manifests (签名验证)
5. SSRF Protection
6. Secret Zeroization
7. OFP Mutual Authentication
8. Capability Gates
9. Security Headers
10. Health Endpoint Redaction
11. Subprocess Sandbox
12. Prompt Injection Scanner
13. Loop Guard
14. Session Repair
15. Path Traversal Prevention
16. GCRA Rate Limiter

#### Code-Coder: 分层安全

- Gateway RBAC (zero-gateway)
- JWT + Pairing Code 双模式认证
- 环境变量 + secrets.json 分离
- Rust 服务内存安全

`★ Insight ─────────────────────────────────────`
OpenFang 在安全上投入巨大——16 层防御覆盖了从沙箱执行到网络通信的全链路。Code-Coder 目前依赖基础认证和 Rust 的内存安全，但缺少:
- WASM 沙箱工具执行
- 污点追踪数据流
- Prompt 注入扫描

这在外部不可信环境部署时可能成为瓶颈。
`─────────────────────────────────────────────────`

### 3. 记忆系统

| OpenFang | Code-Coder |
|----------|------------|
| SQLite + 向量嵌入 | Markdown 文件 |
| 结构化会话 (Canonical Sessions) | 双层架构 (流/沉积) |
| 自动压缩 (Compaction) | 手动整理 |
| 语义检索 | 文本搜索 (grep) |
| 机器友好 | 人类友好 |

`★ Insight ─────────────────────────────────────`
Code-Coder 的 Markdown 记忆系统设计优雅且透明，但:
- 缺少语义检索能力
- 手动维护成本高
- 难以做大规模上下文压缩

OpenFang 的 SQLite + 向量方案更适合生产环境，但牺牲了可读性。
`─────────────────────────────────────────────────`

### 4. 性能对比

| 指标 | OpenFang | Code-Coder |
|------|----------|------------|
| 冷启动 | 180ms | 2-5s |
| 内存占用 | 40MB | 100-200MB |
| 磁盘占用 | 32MB | ~50MB (仅 Rust 部分) |
| 并发模型 | Tokio 异步 | Bun 异步 |

`★ Insight ─────────────────────────────────────`
OpenFang 的性能优势来自:
1. 纯 Rust 实现 (无 JIT 预热)
2. 单体架构 (无 IPC 开销)
3. 精心设计的内存管理

Code-Coder 的架构在灵活性上有优势:
- TS 层可热重载开发
- 服务独立扩展
- 多语言生态集成

这是"极致性能"与"开发效率"的典型权衡。
`─────────────────────────────────────────────────`

---

## 可借鉴的设计

### 从 OpenFang 到 Code-Coder

1. **Hands 模式**: 实现自主运行的定时任务
   - 集成到 zero-workflow
   - HAND.toml 风格的 manifest

2. **WASM 沙箱**: 工具执行安全隔离
   - 在 zero-gateway 中集成 wasmtime
   - MCP 工具在沙箱中运行

3. **审计追踪**: Merkle 链式日志
   - 追踪所有 Agent 决策
   - 支持事后问责

4. **技能市场**: FangHub 模式
   - SKILL.md 标准化
   - 社区贡献技能

### 从 Code-Coder 到 OpenFang

1. **哲学框架**: CLOSE 决策系统
   - 宏观经济分析能力
   - 祝融说观察者理论

2. **透明记忆**: 人类可读的 Markdown
   - 便于理解和调试
   - Git 版本控制

3. **混合架构**: TS/Rust 职责分离
   - LLM 用 TS (灵活性)
   - 确定性用 Rust (性能)

---

## 技术债务与改进建议

### Code-Coder 可考虑的改进

#### 短期 (1-2 月)
1. **迁移关键路径到 Rust**: ccode API 核心用 Rust 重写
2. **添加 WASM 沙箱**: MCP 工具执行隔离
3. **审计日志**: 追踪所有 Agent 操作

#### 中期 (3-6 月)
1. **Hands 模式**: 实现自主代理调度
2. **向量检索**: 记忆系统语义搜索
3. **Prompt 注入扫描**: 输入验证层

#### 长期 (6+ 月)
1. **单体 Rust 内核**: 考虑纯 Rust 方案
2. **技能市场**: 社区贡献生态
3. **A2A 通信**: Agent 间直接通信协议

---

## 结论

**OpenFang** 是一个工程奇迹——在 32MB 二进制中实现了完整的 Agent OS，16 层安全和 7 个自主 Hands。它的设计目标是生产级别的可靠性。

**Code-Coder** 是一个个人工作台——融合工程能力与决策智慧，强调人类可读性和哲学框架集成。它的设计目标是个人效能提升。

两者不是直接竞争关系，而是互补:
- OpenFang 适合作为**基础设施**部署
- Code-Coder 适合作为**个人助手**使用

**建议**: 考虑将 OpenFang 作为 Code-Coder 的"运行时后端"，保留 TS 层的灵活性和哲学框架，利用 Rust 层的性能和安全。

---

## 参考资料

- OpenFang GitHub: https://github.com/RightNow-AI/openfang
- Code-Coder 架构文档: `docs/architecture/`
- CCODE vs Zero 职责划分: `docs/architecture/CCODE_VS_ZERO.md`
