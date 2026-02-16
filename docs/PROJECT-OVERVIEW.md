# CodeCoder 项目全景文档

> 最后更新: 2026-02-16
> 文档类型: LLM 友好的项目参考

## 一、项目定位

**CodeCoder** 是一个融合工程能力与决策智慧的个人工作台，核心特点：

| 层次 | 能力 |
|------|------|
| 工程层 | 代码审查、安全分析、TDD、架构设计、逆向工程 |
| 领域层 | 宏观经济、交易分析、选品策略、极小产品、AI 工程 |
| 思维层 | 祝融说哲学体系、CLOSE 决策框架、观察者理论 |

## 二、技术架构

### 2.1 Monorepo 结构

```
/
├── packages/
│   ├── ccode/           # TypeScript 核心 CLI（主包）
│   │   ├── src/
│   │   │   ├── agent/       # Agent 定义与 Prompt
│   │   │   ├── api/         # HTTP API 服务器
│   │   │   ├── autonomous/  # 自主执行框架（Crazy Mode）
│   │   │   ├── cli/         # CLI 命令与 TUI
│   │   │   ├── config/      # 配置管理
│   │   │   ├── document/    # 文档处理（BookExpander）
│   │   │   ├── mcp/         # MCP 协议支持
│   │   │   ├── memory-zerobot/  # ZeroBot 记忆集成
│   │   │   ├── provider/    # AI 提供商适配器（20+）
│   │   │   ├── security/    # 安全策略
│   │   │   ├── storage/     # 数据持久化
│   │   │   ├── tool/        # 工具定义
│   │   │   ├── util/        # 内部工具函数
│   │   │   └── verifier/    # 形式化验证
│   │   └── test/        # 测试文件
│   ├── util/            # 共享工具库
│   ├── memory/          # 独立记忆模块（可选）
│   └── web/             # Web UI（开发中）
├── services/
│   └── zero-bot/        # Rust ZeroBot 服务
├── memory/              # 项目记忆系统
│   ├── MEMORY.md        # 长期记忆
│   └── daily/           # 每日笔记
├── docs/                # 文档
│   ├── progress/        # 进行中的工作
│   ├── reports/         # 完成报告
│   ├── guides/          # 使用指南
│   ├── standards/       # 标准规范
│   └── templates/       # 文档模板
└── script/              # 构建脚本
```

### 2.2 技术栈

| 类别 | 技术选型 |
|------|----------|
| 运行时 | Bun 1.3+ |
| 构建系统 | Turborepo |
| 前端框架 | Solid.js + OpenTUI（终端） |
| HTTP 框架 | Hono |
| AI 集成 | 20+ 提供商 SDK、MCP 协议 |
| 验证库 | Zod |
| 存储路径 | `~/.codecoder/` |

### 2.3 关键配置文件

| 文件 | 用途 |
|------|------|
| `packages/ccode/src/agent/agent.ts` | Agent 定义 |
| `packages/ccode/src/config/config.ts` | 配置加载 |
| `packages/ccode/src/storage/storage.ts` | 数据存储 |
| `~/.codecoder/config.jsonc` | 用户配置 |

## 三、Agent 系统

### 3.1 全部 23 个 Agent

| 分类 | Agent 名称 | 用途 |
|------|------------|------|
| **主模式** | build | 默认开发模式 |
| | plan | 计划模式 |
| | crazy | 自主执行模式（实验性） |
| **逆向工程** | code-reverse | 网站逆向分析 |
| | jar-code-reverse | JAR 包逆向 |
| **工程质量** | code-reviewer | 代码审查 |
| | security-reviewer | 安全审查 |
| | tdd-guide | TDD 指导 |
| | architect | 架构设计 |
| | verifier | 形式化验证 |
| **内容创作** | writer | 长文写作 |
| | proofreader | 校对 |
| | expander | 内容扩展 |
| **祝融说系列** | observer | 观察者视角 |
| | decision | CLOSE 决策框架 |
| | macro | 宏观经济分析 |
| | trader | 交易分析 |
| | picker | 选品策略 |
| | miniproduct | 极小产品 |
| | ai-engineer | AI 工程 |
| **工具辅助** | explore | 代码探索 |
| | general | 通用助手 |
| | synton-assistant | 句元助手 |
| **系统隐藏** | compaction, title, summary | 内部使用 |

### 3.2 Agent 调用示例

```bash
# 指定 Agent 运行
codecoder --agent code-reviewer "Review src/api/server.ts"

# 使用 @ 语法
codecoder "@decision 分析这个技术选型"

# 使用专用命令
codecoder reverse analyze https://example.com
```

## 四、最近完成的功能（按时间线）

### 2026-02-16

| 功能 | 说明 | 关键文件 |
|------|------|----------|
| 存储路径迁移 | `~/.zero-bot` → `~/.codecoder` | `storage.ts`, `config.ts` |
| Task API 实现 | 异步任务流、SSE 推送 | `src/api/task/` |
| ZeroBot 整合 | 双向集成完成 | `services/zero-bot/` |
| Storage 数据完整性 | 原子写入、备份、健康检查 | `storage.ts` |
| Web UI 基础 | Shadcn 组件、Zustand 状态 | `packages/web/` |

### 2026-02-14

| 功能 | 说明 |
|------|------|
| Writer + Expander 集成 | 长篇内容系统化扩展 |
| WriterStatsMonitor | 写作进度追踪 |

### 2026-02-12-13

| 功能 | 说明 |
|------|------|
| Crazy Mode | CLOSE 决策框架、状态机、安全层 |
| Verifier Agent | 形式化验证、属性测试 |
| Write 工具截断修复 | 大文件处理指导 |
| BookExpander | 系统化扩展框架（有阻塞） |

## 五、进行中的工作

### 5.1 docs/progress/ 文档

| 文件 | 状态 | 说明 |
|------|------|------|
| `2026-02-05-code-cleanup.md` | 长期任务 | TypeScript 类型错误清理 |
| `2026-02-13-bookexpander-implementation.md` | 阻塞 | Zod v4 + Bun 兼容性问题 |

### 5.2 未提交的代码修改

参见 `git status` 输出。主要包括：
- ZeroBot 服务集成
- Task API 实现
- 存储路径迁移
- Web UI 组件

## 六、已知问题与技术债务

### 6.1 阻塞问题

| 问题 | 影响 | 优先级 |
|------|------|--------|
| Zod v4 + Bun escapeRegex 错误 | BookExpander 无法运行 | 中 |

### 6.2 技术债务

| 类型 | 状态 | 说明 |
|------|------|------|
| TypeScript 类型错误 | ~100+ 个 | 主要在 TUI 测试 |
| 导入路径不一致 | 部分完成 | 混用三种方式 |
| ZeroBot 类型共享 | 待规划 | Rust/TS 类型同步 |

**详细参见**: `docs/DEBT.md`

## 七、开发命令速查

```bash
# 安装依赖
bun install

# 运行 TUI
bun dev

# 启动 API 服务器
bun dev serve

# 类型检查
bun turbo typecheck

# 运行测试
cd packages/ccode && bun test

# 构建可执行文件
bun run --cwd packages/ccode build
```

## 八、文档结构

| 目录 | 用途 |
|------|------|
| `docs/progress/` | 进行中的工作（未完成） |
| `docs/reports/completed/` | 已完成的功能报告 |
| `docs/reports/*.md` | 项目状态评估 |
| `docs/guides/` | 使用指南 |
| `docs/standards/` | 标准规范 |
| `docs/templates/` | 文档模板 |
| `memory/MEMORY.md` | 长期记忆（关键决策） |
| `memory/daily/` | 每日工作笔记 |

## 九、记忆系统

采用双层 Markdown 记忆架构：

1. **流层（每日笔记）**: `memory/daily/{YYYY-MM-DD}.md`
   - 仅追加日志
   - 记录当日所有工作

2. **沉积层（长期记忆）**: `memory/MEMORY.md`
   - 结构化知识
   - 用户偏好、关键决策、经验教训

**操作原则**: 人类可读、Git 友好、无复杂嵌入检索

---

*本文档用于快速理解项目全貌，适合 LLM 和新开发者参考。*
