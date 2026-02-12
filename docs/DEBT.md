# 技术债务清单

本文档记录项目中需要处理的冗余、过期代码和架构问题。

## 1. 冗余代码 (需要清理)

### 1.1 重复的工具函数 (✓ 已完成)

| 文件       | packages/util/ | packages/ccode/util/ | 状态               |
| ---------- | -------------- | -------------------- | ------------------ |
| `fn.ts`    | 存在           | Re-export            | ✓ 已统一           |
| `iife.ts`  | 存在           | Re-export            | ✓ 已统一           |
| `lazy.ts`  | 存在           | Re-export            | ✓ 已统一           |

**解决方案 (已实施)**:
1. `packages/ccode/src/util/fn.ts` - 改为从 `@codecoder-ai/util/fn` re-export
2. `packages/ccode/src/util/iife.ts` - 改为从 `@codecoder-ai/util/iife` re-export
3. `packages/ccode/src/util/lazy.ts` - 改为从 `@codecoder-ai/util/lazy` re-export
4. 测试文件已更新为直接使用 `@codecoder-ai/util/`

### 1.2 ccode/util 独有文件 (保留在 ccode)

以下文件仅存在于 `ccode/util`，是特定功能，应保留:

- `archive.ts` - 归档工具
- `color.ts` - 颜色处理
- `context.ts` - 上下文管理
- `defer.ts` - 延迟执行
- `eventloop.ts` - 事件循环工具
- `filesystem.ts` - 文件系统操作
- `format.ts` - 格式化工具
- `keybind.ts` - 键盘绑定
- `locale.ts` - 本地化
- `lock.ts` - 锁机制
- `log.ts` - 日志工具
- `queue.ts` - 队列实现
- `rpc.ts` - RPC 通信
- `scrap.ts` - 抓取工具
- `security.ts` - 安全工具
- `signal.ts` - 信号处理
- `timeout.ts` - 超时处理
- `token.ts` - Token 处理
- `wildcard.ts` - 通配符匹配

## 2. 过时的测试 (需要审查)

### 2.1 待清理的测试文件

- `packages/ccode/test/snapshot/snapshot.test.ts` - 快照测试需要审查
- `packages/ccode/test/skill/skill.test.ts` - Skill 系统测试需要更新

### 2.2 测试中的跳过标记

需要检查以下标记并处理:
```bash
grep -r "skip\|pending\|todo" packages/ccode/test/ --include="*.ts"
```

## 3. 导入路径不一致

### 3.1 问题

项目中混用三种导入路径方式:
1. `@codecoder-ai/util/` - workspace 包 (推荐)
2. `@/util/` - 别名 (内部使用)
3. `../util/` - 相对路径 (不推荐)

### 3.2 标准化目标

- 工具函数: `@codecoder-ai/util/xxx`
- 项目内部: `@/path/to/module`
- 避免多级相对路径 `../../util`

## 4. 架构简化遗留

### 4.1 ACP (Agent Client Protocol) 引用

- **状态**: 已移除 ACP，采用直接 API 架构
- **待办**: 检查文档中是否还有 ACP 相关引用并更新

### 4.2 packages/sdk/ 引用 (✓ 已完成)

- **状态**: SDK 已合并到主包，使用本地类型定义
- **解决方案 (已实施)**: 2026-02-05 已删除 `packages/sdk/` 目录和 `script/generate.ts` 脚本，更新 AGENTS.md 文档

## 5. 待清理的依赖

### 5.1 @ai-sdk/* 依赖审查 (✓ 已完成)

**结果**: 所有 19 个 @ai-sdk/* 包都在被使用。

### 5.2 未使用的 devDependencies (✓ 已完成)

**发现以下未使用的依赖**:
- `@octokit/webhooks-types` - 仅在 package.json 中，代码中未使用
- `@octokit/graphql` - 仅在 package.json 中，代码中未使用

**解决方案 (已实施)**: 2026-02-05 已从 package.json 中删除这两个依赖。

## 6. 文档问题

### 6.1 需要重写的文档 (✓ 已完成)

- `docs/Skills 进阶全景图.md` - ✓ 已重写为技术文档

### 6.2 缺失的文档 (✓ 已完成)

- `docs/guides/testing.md` - ✓ 测试指南已创建
- `docs/CODEBASE.md` - ✓ 代码导航文档已创建

## 7. 代码质量问题

### 7.1 TypeScript 严格模式

检查是否可以启用更严格的 TypeScript 配置:
- `strictNullChecks`
- `noImplicitAny`
- `strictFunctionTypes`

### 7.2 测试覆盖率

当前测试覆盖情况:
- E2E: 21 个文件
- Unit: 33 个文件
- Integration: 18 个文件
- Performance: 4 个文件
- A11Y: 2 个文件

**目标**: 保持 80%+ 覆盖率

### 7.3 TypeScript 类型错误

**状态**: 部分完成 (2026-02-04)

**已修复**:
- `test/helpers/visual-helper.ts` - tolerance undefined 问题
- `test/helpers/e2e-helper.ts` - getOutput() this 类型问题
- `test/helpers/fixtures/messages.ts` - 添加 Message 接口定义
- `test/helpers/fixtures/sessions.ts` - 添加 Session 和 Message 接口定义
- `test/integration/tui/edge-cases.test.tsx` - Map.keys() undefined 和 isEmpty 问题

**剩余**: 约 100+ 个测试文件中的类型错误（主要是 TUI 集成测试）

**建议**: 这些错误不影响核心功能和测试运行，可以后续逐步修复。

## 优先级

| 优先级 | 任务                          | 状态            | 预计工作量 |
| ------ | ----------------------------- | --------------- | ---------- |
| 高     | 统一工具函数到 packages/util  | ✓ 已完成        | 2小时      |
| 高     | 标准化导入路径                | 部分完成        | 3小时      |
| 中     | 清理过时测试                  | ✓ 已验证有效    | 1小时      |
| 中     | 重写 Skills 文档              | ✓ 已完成        | 1小时      |
| 低     | 审查依赖                      | ✓ 已完成        | 1小时      |
| 低     | 修复 TS 类型错误              | 部分完成        | 4小时      |

## 更新记录

- 2026-02-04: 初始版本，记录重复代码和架构问题
- 2026-02-04: 完成工具函数统一，完成文档更新 (progress, CODEBASE, DEBT, testing, Skills)
- 2026-02-04: 清理过时测试（验证有效），审查依赖（发现 2 个未使用），修复部分 TS 类型错误
- 2026-02-05: 文档结构规范化，删除废弃文件 (script/generate.ts, script/duplicate-pr.ts, packages/sdk/)，清理未使用依赖 (@octokit/*)
- 2026-02-05: 创建代码清理计划文档 ([2026-02-05-code-cleanup.md](progress/2026-02-05-code-cleanup.md))，记录测试错误和 console.log 清理任务
- 2026-02-05: ✅ 修复 TUI UI 测试类型错误 (dialog-alert, dialog-confirm, dialog-prompt, link, toast, transcript)
- 2026-02-05: ✅ 修复 5 个失败测试 (provider source 类型错误和 defaultAgent 测试)
- 2026-02-05: ✅ console.log 清理 - 确认所有使用都在适当的 CLI 命令中
- 2026-02-05: ✅ 文档整理完成 - 移动已完成文档，创建记忆系统架构文档，更新索引链接 ([2026-02-05-doc-reorg.md](progress/2026-02-05-doc-reorg.md))
