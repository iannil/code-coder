# 用户全生命周期测试用例实施报告

> 创建日期: 2026-02-16
> 状态: Completed
> 版本: 1.0.0

## 概述

本报告记录了用户全生命周期测试用例的设计和实施过程。

## 实施内容

### 文档

| 文件 | 描述 | 状态 |
|-----|------|:----:|
| `docs/reports/user-lifecycle-test-cases.md` | 完整测试用例文档 | ✓ |
| `docs/reports/test-coverage-matrix.md` | 功能覆盖矩阵 | ✓ |

### ccode 包 E2E 测试

| 文件 | 用户类型 | 测试数量 | 状态 |
|-----|---------|:--------:|:----:|
| `packages/ccode/test/e2e/user-lifecycle/developer.test.ts` | 软件开发者 | 25+ | ✓ |
| `packages/ccode/test/e2e/user-lifecycle/creator.test.ts` | 内容创作者 | 20+ | ✓ |
| `packages/ccode/test/e2e/user-lifecycle/analyst.test.ts` | 决策分析师 | 20+ | ✓ |
| `packages/ccode/test/e2e/user-lifecycle/reverse-engineer.test.ts` | 逆向工程师 | 15+ | ✓ |

### Web E2E 测试 (Playwright)

| 文件 | 描述 | 状态 |
|-----|------|:----:|
| `packages/web/playwright.config.ts` | Playwright 配置 | ✓ |
| `packages/web/test/e2e/user-lifecycle/common.spec.ts` | 通用功能测试 | ✓ |
| `packages/web/test/e2e/user-lifecycle/developer.spec.ts` | 开发者流程测试 | ✓ |
| `packages/web/test/e2e/user-lifecycle/creator.spec.ts` | 创作者流程测试 | ✓ |
| `packages/web/test/e2e/user-lifecycle/analyst.spec.ts` | 分析师流程测试 | ✓ |

## 测试覆盖

### 用户类型覆盖

| 用户类型 | Agent 覆盖 | 生命周期阶段 |
|---------|:----------:|:------------:|
| 软件开发者 (ULC-DEV) | 7 agents | INIT, SESS, AGNT, FILE, ADVN, ERR |
| 内容创作者 (ULC-CRT) | 5 agents | INIT, WRTR, SESS, DOCS, ADVN, ERR |
| 决策分析师 (ULC-ANL) | 7 agents | INIT, ANLZ, SESS, MEMO, ADVN, ERR |
| 逆向工程师 (ULC-REV) | 2 agents | INIT, REVS, SESS, FILE, ADVN, ERR |

### Agent 覆盖

| 类别 | Agent | 测试覆盖 |
|-----|-------|:--------:|
| 主模式 | build, plan, writer | ✓ |
| 工程质量 | code-reviewer, security-reviewer, tdd-guide, architect | ✓ |
| 内容创作 | proofreader, expander, expander-fiction, expander-nonfiction | ✓ |
| 祝融说系列 | observer, decision, macro, trader, picker, miniproduct, ai-engineer | ✓ |
| 逆向工程 | code-reverse, jar-code-reverse | ✓ |
| 工具辅助 | explore, verifier | ✓ |

### 测试阶段覆盖

| 阶段 | 描述 | 覆盖状态 |
|-----|------|:--------:|
| INIT | 初始化 (配置、API Key、模型) | ✓ |
| SESS | 会话管理 (创建、列表、切换、删除) | ✓ |
| CORE | 核心交互 (消息、工具调用、权限) | ✓ |
| AGNT | Agent 工作流 | ✓ |
| FILE | 文件操作 | ✓ |
| ADVN | 高级功能 (命令面板、主题、配置) | ✓ |
| ERR | 错误处理 | ✓ |

## 测试命名规范

### 测试 ID 格式

```
ULC-{用户类型}-{模块}-{序号}
```

| 前缀 | 用户类型 |
|-----|---------|
| ULC-DEV | 软件开发者 |
| ULC-CRT | 内容创作者 |
| ULC-ANL | 决策分析师 |
| ULC-REV | 逆向工程师 |
| ULC-CMN | 通用功能 |

### 模块代码

| 代码 | 模块 |
|-----|------|
| INIT | 初始化 |
| SESS | 会话管理 |
| CORE | 核心交互 |
| AGNT | Agent |
| FILE | 文件操作 |
| WRTR | 写作 |
| ANLZ | 分析 |
| REVS | 逆向 |
| DOCS | 文档 |
| MEMO | 记忆 |
| ADVN | 高级功能 |
| ERR | 错误处理 |

## 运行测试

### ccode 包测试

```bash
cd packages/ccode

# 运行所有用户生命周期测试 (默认跳过 E2E)
bun test test/e2e/user-lifecycle/

# 启用 E2E 测试
SKIP_E2E=false bun test test/e2e/user-lifecycle/

# 运行特定用户类型测试
SKIP_E2E=false bun test test/e2e/user-lifecycle/developer.test.ts
```

### Web E2E 测试

```bash
cd packages/web

# 安装 Playwright (首次)
npx playwright install

# 运行所有 E2E 测试
SKIP_E2E=false npx playwright test

# 运行特定测试文件
SKIP_E2E=false npx playwright test test/e2e/user-lifecycle/common.spec.ts

# 运行带 UI 的测试
SKIP_E2E=false npx playwright test --ui

# 生成测试报告
npx playwright show-report
```

## 测试基础设施

### 依赖项

**ccode 包:**
- `bun:test` - Bun 内置测试框架
- `tmpdir()` - 临时目录夹具
- `Instance.provide()` - 上下文隔离

**web 包:**
- `@playwright/test` - E2E 测试框架
- Playwright 浏览器 (Chromium, Firefox, WebKit)

### 测试夹具

```typescript
// ccode 包 - 使用 tmpdir 夹具
await using tmp = await tmpdir({
  init: async (dir) => {
    await Bun.write(
      path.join(dir, "codecoder.json"),
      JSON.stringify({ $schema: "https://code-coder.com/config.json" })
    )
  },
})

await Instance.provide({
  directory: tmp.path,
  init: async () => {
    Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
  },
  fn: async () => {
    // 测试代码
  },
})
```

### 数据测试属性

Web 测试使用 `data-testid` 属性定位元素：

| 元素 | data-testid |
|-----|-------------|
| 侧边栏 | `sidebar` |
| 主面板 | `main-panel` |
| 新会话按钮 | `new-session-btn` |
| 会话列表 | `session-list` |
| 会话项 | `session-item` |
| Agent 选择器 | `agent-selector` |
| Agent 选项 | `agent-option` |
| 消息输入 | `message-input` |
| 消息列表 | `message-list` |
| 发送按钮 | `send-btn` |
| 主题切换 | `theme-toggle` |
| 命令面板 | `command-palette` |

## 验证清单

- [x] 所有测试文件符合项目编码规范
- [x] 测试使用 `Instance.provide()` 模式确保正确的上下文隔离
- [x] 测试后清理临时资源（使用 `await using` 或 `Session.remove()`）
- [x] E2E 测试默认跳过，通过 `SKIP_E2E=false` 启用
- [x] Web 测试配置了 Playwright 多浏览器支持
- [x] 测试命名遵循 ULC 规范

## 后续工作

1. **添加 data-testid 属性**: 在 Web 组件中添加相应的 `data-testid` 属性以支持 E2E 测试
2. **集成 CI/CD**: 将测试集成到 CI/CD 流水线
3. **扩展测试覆盖**: 根据实际使用情况补充边缘场景测试
4. **性能测试**: 添加响应时间基准测试

## 参考资料

- 现有测试: `packages/ccode/test/lifecycle/new-user.test.ts`
- 测试夹具: `packages/ccode/test/fixture/fixture.ts`
- E2E 辅助: `packages/ccode/test/helpers/e2e-helper.ts`
- Playwright 文档: https://playwright.dev/docs/test-configuration
