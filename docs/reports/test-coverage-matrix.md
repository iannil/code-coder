# 测试覆盖率矩阵

> 创建日期: 2026-02-16
> 状态: Active
> 版本: 1.0.0

## 功能模块 × 用户类型矩阵

| 功能模块 | 开发者 (DEV) | 创作者 (CRT) | 分析师 (ANL) | 逆向工程师 (REV) | 测试覆盖 |
|---------|:------------:|:------------:|:------------:|:---------------:|:--------:|
| **核心功能** |
| Dashboard | ✓ | ✓ | ✓ | ✓ | P0 |
| Session 管理 | ✓ | ✓ | ✓ | ✓ | P0 |
| 消息交互 | ✓ | ✓ | ✓ | ✓ | P0 |
| Agent 切换 | ✓ | ✓ | ✓ | ✓ | P0 |
| 设置配置 | ✓ | ✓ | ✓ | ✓ | P0 |
| 主题切换 | ✓ | ✓ | ✓ | ✓ | P2 |
| 命令面板 | ✓ | ✓ | ✓ | ✓ | P1 |
| **工程功能** |
| 文件操作 | ✓ | ✓ | - | ✓ | P1 |
| 代码审查 | ✓ | - | - | - | P1 |
| 安全分析 | ✓ | - | - | - | P1 |
| TDD 指导 | ✓ | - | - | - | P2 |
| 架构设计 | ✓ | - | - | - | P2 |
| 代码探索 | ✓ | - | - | ✓ | P1 |
| **内容功能** |
| 长文写作 | - | ✓ | - | - | P0 |
| 内容校对 | - | ✓ | - | - | P1 |
| 内容扩展 | - | ✓ | - | - | P1 |
| 小说扩展 | - | ✓ | - | - | P2 |
| 非虚构扩展 | - | ✓ | - | - | P2 |
| **决策功能** |
| 观察者分析 | - | - | ✓ | - | P1 |
| CLOSE 决策 | - | - | ✓ | - | P1 |
| 宏观分析 | - | - | ✓ | - | P2 |
| 交易分析 | - | - | ✓ | - | P2 |
| 选品策略 | - | - | ✓ | - | P2 |
| 极小产品 | - | - | ✓ | - | P2 |
| AI 工程 | - | - | ✓ | - | P2 |
| **逆向功能** |
| 网站逆向 | - | - | - | ✓ | P1 |
| JAR 逆向 | - | - | - | ✓ | P1 |
| **系统功能** |
| 记忆系统 | ✓ | ✓ | ✓ | ✓ | P2 |
| Hook 系统 | ✓ | - | - | - | P2 |
| MCP 集成 | ✓ | - | - | - | P2 |
| LSP 集成 | ✓ | - | - | - | P2 |

---

## Agent 覆盖率

### 主模式 Agent

| Agent | 模式 | 测试覆盖 | 测试 ID |
|-------|------|:--------:|---------|
| build | primary | ✓ | ULC-DEV-AGNT-001 |
| plan | primary | ✓ | ULC-DEV-AGNT-002 |
| writer | primary | ✓ | ULC-CRT-WRTR-001, ULC-CRT-WRTR-002 |
| autonomous | primary | - | (系统内部) |

### 子代理 Agent

| Agent | 类别 | 测试覆盖 | 测试 ID |
|-------|------|:--------:|---------|
| code-reviewer | 工程 | ✓ | ULC-DEV-AGNT-003 |
| security-reviewer | 工程 | ✓ | ULC-DEV-AGNT-004 |
| tdd-guide | 工程 | ✓ | ULC-DEV-AGNT-005 |
| architect | 工程 | ✓ | ULC-DEV-AGNT-006 |
| explore | 工程 | ✓ | ULC-DEV-AGNT-007 |
| general | 工程 | - | (通用) |
| proofreader | 内容 | ✓ | ULC-CRT-WRTR-003 |
| expander | 内容 | ✓ | ULC-CRT-WRTR-004 |
| expander-fiction | 内容 | ✓ | ULC-CRT-WRTR-005 |
| expander-nonfiction | 内容 | ✓ | ULC-CRT-WRTR-006 |
| observer | 祝融说 | ✓ | ULC-ANL-ANLZ-001 |
| decision | 祝融说 | ✓ | ULC-ANL-ANLZ-002 |
| macro | 祝融说 | ✓ | ULC-ANL-ANLZ-003 |
| trader | 祝融说 | ✓ | ULC-ANL-ANLZ-004 |
| picker | 祝融说 | ✓ | ULC-ANL-ANLZ-005 |
| miniproduct | 祝融说 | ✓ | ULC-ANL-ANLZ-006 |
| ai-engineer | 祝融说 | ✓ | ULC-ANL-ANLZ-007 |
| code-reverse | 逆向 | ✓ | ULC-REV-REVS-001 |
| jar-code-reverse | 逆向 | ✓ | ULC-REV-REVS-002 |
| verifier | 验证 | - | (系统内部) |
| synton-assistant | 辅助 | - | (专用) |

### 隐藏 Agent

| Agent | 用途 | 测试覆盖 |
|-------|------|:--------:|
| compaction | 上下文压缩 | 系统测试 |
| title | 标题生成 | 系统测试 |
| summary | 摘要生成 | 系统测试 |

---

## API 端点覆盖率

### Session API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/sessions` | GET | ✓ | ✓ | ✓ |
| `/api/sessions` | POST | ✓ | ✓ | ✓ |
| `/api/sessions/:id` | GET | ✓ | ✓ | ✓ |
| `/api/sessions/:id` | PUT | ✓ | ✓ | - |
| `/api/sessions/:id` | DELETE | ✓ | ✓ | ✓ |

### Agent API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/agents` | GET | ✓ | ✓ | ✓ |
| `/api/agents/:name` | GET | ✓ | ✓ | - |

### Config API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/config` | GET | ✓ | ✓ | ✓ |
| `/api/config` | PUT | ✓ | ✓ | ✓ |

### Provider API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/providers` | GET | ✓ | ✓ | ✓ |
| `/api/providers/:id` | GET | ✓ | ✓ | - |

### Memory API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/memory` | GET | ✓ | ✓ | - |
| `/api/memory` | POST | ✓ | ✓ | - |

### Document API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/documents` | GET | ✓ | ✓ | - |
| `/api/documents` | POST | ✓ | ✓ | - |
| `/api/documents/:id` | GET | ✓ | ✓ | - |
| `/api/documents/:id` | PUT | ✓ | ✓ | - |
| `/api/documents/:id` | DELETE | ✓ | ✓ | - |

### Task API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/tasks` | GET | ✓ | ✓ | - |
| `/api/tasks` | POST | ✓ | ✓ | - |
| `/api/tasks/:id` | PUT | ✓ | ✓ | - |

### Hook API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/hooks` | GET | ✓ | ✓ | - |

### MCP API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/mcp` | GET | ✓ | ✓ | - |

### LSP API

| 端点 | 方法 | 单元测试 | 集成测试 | E2E 测试 |
|-----|------|:--------:|:--------:|:--------:|
| `/api/lsp` | GET | ✓ | ✓ | - |

---

## 测试类型分布

### 按测试类型

| 类型 | 数量 | 覆盖率目标 |
|------|:----:|:----------:|
| 单元测试 | 150+ | 80% |
| 集成测试 | 50+ | 70% |
| E2E 测试 (TUI) | 20+ | 关键路径 |
| E2E 测试 (Web) | 30+ | 关键路径 |

### 按用户类型

| 用户类型 | 测试用例数 | 覆盖 Agent 数 |
|---------|:----------:|:------------:|
| 软件开发者 (DEV) | 25+ | 7 |
| 内容创作者 (CRT) | 15+ | 5 |
| 决策分析师 (ANL) | 15+ | 7 |
| 逆向工程师 (REV) | 8+ | 2 |
| 通用 (CMN) | 10+ | - |

---

## 测试执行策略

### CI/CD 集成

```yaml
# 测试执行顺序
stages:
  - lint        # 代码风格检查
  - typecheck   # 类型检查
  - unit        # 单元测试 (P0-P3)
  - integration # 集成测试 (P0-P2)
  - e2e         # E2E 测试 (P0-P1)
```

### 执行频率

| 触发条件 | 执行测试 |
|---------|---------|
| 每次提交 | Lint + Typecheck + P0 单元测试 |
| PR 创建 | 全部单元测试 + P0-P1 集成测试 |
| PR 合并 | 全部单元测试 + 全部集成测试 + P0 E2E |
| 发布前 | 全部测试 |

### 性能基准

| 测试类型 | 目标时间 |
|---------|:--------:|
| 单元测试 (全部) | < 30s |
| 集成测试 (全部) | < 2min |
| E2E 测试 (TUI) | < 5min |
| E2E 测试 (Web) | < 5min |

---

## 测试文件位置

### ccode 包

```
packages/ccode/test/
├── fixture/
│   └── fixture.ts           # 测试夹具
├── helpers/
│   └── e2e-helper.ts        # E2E 辅助函数
├── lifecycle/
│   └── new-user.test.ts     # 新用户生命周期测试
├── e2e/
│   └── user-lifecycle/
│       ├── developer.test.ts      # 开发者生命周期
│       ├── creator.test.ts        # 创作者生命周期
│       ├── analyst.test.ts        # 分析师生命周期
│       └── reverse-engineer.test.ts # 逆向工程师生命周期
└── unit/                    # 单元测试
```

### web 包

```
packages/web/
├── test/
│   └── e2e/
│       └── user-lifecycle/
│           ├── common.spec.ts     # 通用功能测试
│           ├── developer.spec.ts  # 开发者流程
│           ├── creator.spec.ts    # 创作者流程
│           └── analyst.spec.ts    # 分析师流程
└── playwright.config.ts     # Playwright 配置
```

---

## 覆盖率报告

### 当前状态

| 包 | 行覆盖率 | 分支覆盖率 | 函数覆盖率 |
|---|:--------:|:----------:|:----------:|
| ccode | 目标 80% | 目标 75% | 目标 85% |
| web | 目标 70% | 目标 65% | 目标 75% |
| util | 目标 90% | 目标 85% | 目标 95% |

### 覆盖率排除

```typescript
// 排除的文件模式
const coverageExclude = [
  '**/node_modules/**',
  '**/test/**',
  '**/*.d.ts',
  '**/index.ts',  // 纯导出文件
  '**/types.ts',  // 纯类型定义
]
```

---

## 风险与缺口

### 已识别的测试缺口

| 模块 | 缺口描述 | 优先级 | 计划 |
|-----|---------|:------:|------|
| autonomous Agent | 完全自主执行难以测试 | P2 | Mock 关键决策点 |
| verifier Agent | 验证流程复杂 | P2 | 分步骤单元测试 |
| MCP 集成 | 外部服务依赖 | P2 | Mock MCP 协议 |
| LSP 集成 | 语言服务器依赖 | P2 | Mock LSP 协议 |

### 风险缓解

1. **外部依赖**: 使用 mock/stub 隔离外部服务
2. **异步操作**: 使用适当的等待机制
3. **状态管理**: 每个测试独立初始化状态
4. **资源清理**: 使用 `await using` 确保资源释放
