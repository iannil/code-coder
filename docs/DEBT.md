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

**状态**: ✅ 已完成 (2026-03-03)

**已修复**:

- `test/helpers/visual-helper.ts` - tolerance undefined 问题
- `test/helpers/e2e-helper.ts` - getOutput() this 类型问题
- `test/helpers/fixtures/messages.ts` - 添加 Message 接口定义
- `test/helpers/fixtures/sessions.ts` - 添加 Session 和 Message 接口定义
- `test/integration/tui/edge-cases.test.tsx` - Map.keys() undefined 和 isEmpty 问题

**验证**: `bun turbo typecheck` 0 errors (2026-03-03)

## 8. 新增技术债务 (2026-02-16)

### 8.1 BookExpander Zod 兼容性问题

- **状态**: ✅ 已完成
- **问题**: Zod v4 + Bun 的 escapeRegex 错误
- **解决方案**: 使用 `defaultArray` helper 替代 `.default([])` 模式
- **修复文件**: 7 个文件已统一应用 workaround
  - `src/cli/cmd/document/schema.ts`
  - `src/cli/cmd/document/context.ts`
  - `src/cli/cmd/document/knowledge/schema.ts`
  - `src/prompt-templates/index.ts`
  - `src/agent/registry.ts`
  - `src/memory/tools/types.ts`
  - `src/provider/routing-rules.ts`
- **文档**: `docs/progress/2026-02-13-bookexpander-implementation.md`

### 8.2 CodeCoder + Zero-* 类型共享

- **状态**: ✅ 已自动化 (ts-rs)
- **问题**: Rust 和 TypeScript 之间缺乏类型同步机制
- **解决方案**: 使用 `ts-rs` 从 Rust 自动生成 TypeScript 类型
- **生成命令**: `./script/generate-ts-bindings.sh`
- **输出位置**: `packages/ccode/src/generated/`
- **文档**: [SHARED_TYPES.md](architecture/SHARED_TYPES.md)

### 8.3 存储路径迁移后清理

- **状态**: ✅ 已完成
- **内容**: 路径从 `~/.zero-bot` 迁移到 `~/.codecoder`
- **后续**: 用户需手动执行 `mv ~/.zero-bot/* ~/.codecoder/`
- **文档**: `docs/reports/completed/2026-02-16-storage-path-migration.md`

### 8.4 Zero-CLI 重构遗留

- **状态**: ✅ 已完成
- **问题**: zero-cli 本地模块与 zero-* 服务存在重复
- **解决**: Phase 15 四个阶段全部完成，HTTP 客户端已创建，服务依赖已迁移
- **文档**: `docs/reports/completed/2026-02-22-phase15-cli-client.md`

## 9. 代码级技术债务标记

本节记录代码中的 TODO/FIXME/WORKAROUND 等标记，便于追踪管理。

### 9.1 功能待实现

| 文件 | 行号 | 描述 | 优先级 |
|------|------|------|--------|
| `services/zero-cli/src/tools/auto_login.rs` | 93 | 2FA 交互式审批系统 | ✅ 已完成 |

### 9.2 外部依赖问题

| 文件 | 行号 | 问题 | 上游 Issue |
|------|------|------|------------|
| `packages/ccode/src/bun/index.ts` | 92 | Bun 代理缓存 | [#19936](https://github.com/oven-sh/bun/issues/19936) |
| `packages/ccode/parsers-config.ts` | 145 | HTML 注入不生效 | tree-sitter-html |
| `packages/ccode/parsers-config.ts` | 244 | Nix WASM 未发布 | [#66](https://github.com/nix-community/tree-sitter-nix/issues/66) |
| `packages/ccode/src/provider/sdk/openai-compatible/...` | 1690 | AI SDK 6 类型变更 | [详细文档](progress/ai-sdk-migration-tech-debt.md) |

### 9.3 类型共享债务

| 文件 | 行号 | 描述 | 关联章节 |
|------|------|------|----------|
| `packages/ccode/src/memory-zerobot/types.ts` | 13 | Memory 类型 (手动同步) | 8.2 |

**注意**: Guardrails、HitL、Events 类型已通过 ts-rs 自动化，详见 [SHARED_TYPES.md](architecture/SHARED_TYPES.md)。

### 9.4 扫描命令

定期扫描新增标记：

```bash
grep -rn "TODO\|FIXME\|HACK\|WORKAROUND\|KNOWN ISSUE\|IMPLEMENTATION NEEDED" \
  packages/ccode/src services --include="*.ts" --include="*.rs" | \
  grep -v "node_modules\|bench/fixture"
```

## 优先级

| 优先级 | 任务                          | 状态            | 预计工作量 |
| ------ | ----------------------------- | --------------- | ---------- |
| 高     | 统一工具函数到 packages/util  | ✓ 已完成        | 2小时      |
| 高     | 标准化导入路径                | ✅ 已完成       | 3小时      |
| 高     | BookExpander Zod 兼容性       | ✅ 已完成       | 2小时      |
| 高     | Zero-CLI 重构完成 (Phase 15)  | ✅ 已完成       | 4小时      |
| 高     | AI SDK 迁移技术债务           | ✅ 已完成       | 2小时      |
| 中     | 清理过时测试                  | ✓ 已验证有效    | 1小时      |
| 中     | 重写 Skills 文档              | ✓ 已完成        | 1小时      |
| 中     | Zero-* 类型共享               | ✅ 已自动化     | 4小时      |
| 中     | 2FA 审批系统实现              | ✅ 已完成       | 3小时      |
| 低     | 审查依赖                      | ✓ 已完成        | 1小时      |
| 低     | 修复 TS 类型错误              | ✅ 已完成       | 4小时      |
| 低     | 外部依赖追踪                  | 🆕 等待上游     | -          |

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
- 2026-02-16: 添加新技术债务 - BookExpander Zod 兼容性问题、ZeroBot 类型共享；文档归档整理（11个文档从 progress 移至 completed）
- 2026-02-16: 完成存储路径迁移；更新技术债务列表；规范化报告文件命名
- 2026-02-22: 更新类型共享问题描述（zero-bot → zero-*）；添加 Zero-CLI 重构遗留项；移动 13 个已完成文档到 completed/
- 2026-03-03: 新增代码级技术债务标记章节 (§9)；完成全量扫描 (6 个有效标记)；更新优先级表
- 2026-03-03: 更新 §8.4 Zero-CLI 重构状态为已完成 (Phase 15 四阶段全部完成)
- 2026-03-03: 完成 AI SDK v6 迁移 (ai@6.0.105, @ai-sdk/*@3-4.x); 更新 tool factory API; 修复类型兼容性问题
- 2026-03-03: 完成导入路径标准化 (23 个文件更新为 @/ 和 @tui/ 别名)
- 2026-03-03: 完成 Zod `.default([])` workaround 统一 (7 个文件); TypeScript typecheck 0 errors
- 2026-03-03: 创建 SHARED_TYPES.md 文档; 完成 2FA 审批系统实现 (zero-cli/auto_login.rs + HitLClient 集成)
- 2026-03-03: 完成 ts-rs 类型自动化; 生成脚本 script/generate-ts-bindings.sh; 更新 §8.2 状态为已自动化
- 2026-03-11: 文档归档整理 (14 个文档从 plans+reports 移至 completed/); 删除空目录 (guides/, plans/); 创建 PROJECT_STATUS.md
