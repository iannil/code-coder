# 项目状态快照 (2026-03-01)

**生成时间**: 2026-03-01
**目的**: 为 LLM Agent 提供项目当前状态的快速概览

---

## 完成度概览

| 维度 | 状态 | 详情 |
|------|------|------|
| **核心功能** | 95%+ | 所有主要功能就绪，部分集成测试待完善 |
| **测试覆盖** | ✅ | packages/web 74.93%, Rust 145 tests |
| **技术债务** | ✅ | P0-P1 全部清零 |
| **文档** | ✅ | 178 份完成报告，记忆系统更新 |
| **CI/CD** | ✅ | GitHub Actions 工作流就绪 |

---

## 近期完成项 (最近 7 天)

### 2026-03-01

| 项目 | 描述 |
|------|------|
| 技术债务 P0-P1 | 覆盖率阈值、CI/CD、代码质量改进全部完成 |
| Web 测试基础设施 | 10 个 store 模块测试，覆盖率 43.91% → 74.93% |
| AI SDK 升级 | @ai-sdk/* 17 个包升级至 v3/v4 |
| document.ts 拆分 | 2858 行 → 80 行入口 + 13 模块 |
| IM 事件溯源 | Redis Streams 实现 Phase 1-4 |
| Trace ID 首条回复 | IM 消息追踪能力增强 |

### 2026-02-28

| 项目 | 描述 |
|------|------|
| Hands + HITL Phase 4 | Prompt 注入扫描、Agent 签名验证、沙箱集成 |
| IM Agent 自动路由 | 智能推荐 agent 功能 |
| 文档整理 | 14 个 progress 文档归档 |

### 2026-02-27

| 项目 | 描述 |
|------|------|
| IM Agent 自动路由实现 | bridge.rs 增强，200ms 超时保护 |

---

## 活跃进行项

| 项目 | 状态 | 下一步 |
|------|------|--------|
| 端到端验证 | 95% | 补充集成测试 |
| 大文件拆分 | 延迟 | prompt.ts, config.ts, server.ts 待重构 |
| Type Safety | 延迟 | 95+ `any` 用法待清理（多数合理） |

---

## 关键文件索引

### 入口点

| 文件 | 用途 |
|------|------|
| `packages/ccode/src/index.ts` | CLI 主入口 |
| `packages/ccode/src/api/server/index.ts` | API 服务器入口 |
| `packages/web/src/main.tsx` | Web 前端入口 |
| `services/zero-cli/src/main.rs` | Rust daemon 入口 |

### 核心模块

| 模块 | 路径 | 职责 |
|------|------|------|
| Agent 系统 | `packages/ccode/src/agent/` | 30 个 Agent 定义和执行 |
| Provider SDK | `packages/ccode/src/provider/` | 20+ AI 提供商集成 |
| 权限系统 | `packages/ccode/src/permission/` | Auto-approve, 风险评估 |
| 工具系统 | `packages/ccode/src/tool/` | 工具定义和执行 |
| 存储层 | `packages/ccode/src/storage/` | 会话、消息持久化 |
| 基础设施 | `packages/ccode/src/infrastructure/` | Redis, 日志 |

### Rust 服务

| 服务 | 路径 | 职责 |
|------|------|------|
| zero-cli | `services/zero-cli/` | 进程编排器 |
| zero-gateway | `services/zero-gateway/` | 认证、路由、配额 |
| zero-channels | `services/zero-channels/` | IM 渠道集成 |
| zero-workflow | `services/zero-workflow/` | Hands、Cron、Webhook |
| zero-trading | `services/zero-trading/` | 交易信号生成 |
| zero-common | `services/zero-common/` | 共享库 |

### 配置文件

| 文件 | 用途 |
|------|------|
| `~/.codecoder/config.json` | 核心配置 |
| `~/.codecoder/secrets.json` | 凭证 (gitignored) |
| `~/.codecoder/workspace/` | 运行时数据目录 |

---

## Agent 概览

**总数**: 30 个

| 分类 | Agent | 数量 |
|------|-------|------|
| 主模式 | build, plan, autonomous, writer | 4 |
| 工程类 | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | 6 |
| 逆向工程 | code-reverse, jar-code-reverse | 2 |
| 内容创作 | expander, expander-fiction, expander-nonfiction, proofreader | 4 |
| 祝融说系列 | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | 8 |
| 产品运营 | verifier, prd-generator, feasibility-assess | 3 |
| 辅助 | synton-assistant | 1 |
| 系统隐藏 | compaction, title, summary | 3 |

---

## 测试状态

### TypeScript

| 包 | 状态 | 覆盖率 |
|----|------|--------|
| packages/ccode | ✅ | 阈值通过 |
| packages/web | ✅ | 74.93% |
| packages/util | ✅ | 基础覆盖 |

### Rust

| 服务 | 测试数 | 状态 |
|------|--------|------|
| zero-common | 22+ | ✅ |
| zero-gateway | 4+ | ✅ |
| zero-workflow | 119+ | ✅ |
| **总计** | 145 | ✅ |

---

## 端口配置

| 服务 | 端口 | 用途 |
|------|------|------|
| CodeCoder API | 4400 | 主 API |
| Web Frontend | 4401 | Vite 开发服务器 |
| Zero CLI Daemon | 4402 | 进程编排 |
| Faster Whisper | 4403 | 语音转文本 |
| Redis | 4410 | 会话存储 |
| MCP Server | 4420 | Model Context Protocol |
| Zero Gateway | 4430 | 网关服务 |
| Zero Channels | 4431 | IM 渠道 |
| Zero Workflow | 4432 | 工作流 |
| Zero Browser | 4433 | 浏览器自动化 |

---

## 文档结构

```
docs/
├── architecture/       # 架构文档
├── guides/            # 使用指南
├── progress/          # 进行中的工作 (当前为空)
├── reports/
│   └── completed/     # 已完成工作报告 (178 份)
├── standards/         # 文档标准
└── templates/         # 文档模板

memory/
├── daily/             # 每日笔记 (7 份)
└── MEMORY.md          # 长期记忆
```

---

## 快速命令

```bash
# 开发
bun dev                    # 启动 TUI
bun dev serve             # 启动 API 服务器

# 测试
cd packages/ccode && bun test
cd packages/web && bun test

# 构建
bun turbo typecheck       # 类型检查
bun run --cwd packages/ccode build  # 构建可执行文件

# 运维
./ops.sh start            # 启动服务
./ops.sh status           # 查看状态
./ops.sh logs <service>   # 查看日志
```

---

## 下一步优先级

1. **E2E 验证** - 补充端到端集成测试
2. **大文件重构** - prompt.ts, config.ts, server.ts (低优先级)
3. **Type Safety** - 清理 `any` 类型 (低优先级)
4. **文档同步** - 保持 CLAUDE.md 与实际状态一致
