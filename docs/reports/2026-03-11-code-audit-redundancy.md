# 冗余代码审计报告

> 审计日期: 2026-03-11
> 审计范围: packages/ccode/, services/zero-*
> 状态: 完成

---

## 执行摘要

本报告识别项目中的冗余、过期、废弃代码，为后续清理提供依据。

| 类别 | 数量 | 预估行数 | 优先级 |
|------|------|----------|--------|
| 废弃模块 (@deprecated) | 9 个文件 | ~4,500 行 | P1 |
| TODO/FIXME 标记 | 2 个文件 | - | P2 |
| console.log 语句 | 50 个文件 | 582 处 | P3 |
| 孤儿测试目录 | 0 | 0 | ✅ 已清理 |
| 备份文件 | 0 | 0 | ✅ 无 |

---

## 1. 废弃模块 (@deprecated)

以下模块已标记为废弃，功能已迁移到 Rust daemon:

### 1.1 高优先级 - 可安全删除

| 文件 | 行数 | Rust 替代 | 依赖检查 |
|------|------|-----------|----------|
| `agent/agent.ts` | ~800 | `zero-cli/src/agent/` | SDK 已适配 |
| `provider/provider.ts` | ~1,400 | `zero-cli/src/providers/` | SDK 已适配 |
| `security/index.ts` | ~200 | `zero-core/src/security/` | NAPI 已绑定 |

**删除条件**: SDK 模式 (`CODECODER_SDK_MODE=1`) 运行正常后可删除

### 1.2 中优先级 - 需进一步迁移

| 文件 | 行数 | Rust 替代 | 阻塞原因 |
|------|------|-----------|----------|
| `session/index.ts` | ~500 | `zero-cli/src/session/` | TUI 深度依赖 |
| `session/prompt.ts` | ~300 | `zero-cli/src/session/` | SystemPrompt 生成 |
| `session/message-v2.ts` | ~400 | `zero-cli/src/session/` | MessageV2 复杂类型 |
| `config/config.ts` | ~500 | `zero-cli/src/config/` | TUI 配置读取 |
| `autonomous/index.ts` | ~300 | `zero-cli/src/autonomous/` | 部分逻辑未迁移 |

**删除条件**: TUI 完全切换到 SDK 后可删除

### 1.3 保留 - 仅类型定义

| 文件 | 行数 | 说明 |
|------|------|------|
| `observer/types.ts` | ~618 | TUI 需要的类型定义，已从废弃实现中提取 |

---

## 2. TODO/FIXME 标记

### 2.1 待处理项

| 文件 | 标记 | 描述 |
|------|------|------|
| `autonomous/execution/evolution-loop.ts` | TODO | 进化循环优化 |
| `autonomous/execution/research-loop.ts` | TODO | 研究循环优化 |

**建议**: 保留，这些是功能增强点

---

## 3. console.log 分析

### 3.1 合理使用 (CLI 命令输出)

以下文件中的 console.log 是 CLI 命令的正常输出，应保留:

- `cli/cmd/debug/*.ts` - 调试命令
- `cli/cmd/document/*.ts` - 文档命令
- `cli/cmd/memory.ts` - 记忆命令
- `cli/cmd/reverse.ts` - 逆向命令
- `cli/cmd/jar-reverse.ts` - JAR 逆向命令
- `cli/cmd/session.ts` - 会话命令
- `cli/cmd/agent.ts` - Agent 命令

### 3.2 待审查

| 文件 | 数量 | 建议 |
|------|------|------|
| `api/server/*.ts` | 6 | 替换为结构化日志 |
| `autonomous/confidence/scorer.ts` | 3 | 替换为结构化日志 |
| `infrastructure/migration/*.ts` | 25 | 迁移脚本，可保留 |
| `ipc/index.ts` | 3 | 替换为结构化日志 |
| `observer/index.ts` | 2 | 替换为结构化日志 |

---

## 4. 已清理项 (Phase 1-3)

### 4.1 废弃代码清理完成

| 阶段 | 删除内容 | 行数 |
|------|----------|------|
| Phase 1 | Observer Network 实现 | ~14,800 |
| Phase 2 | Trace + Bootstrap | ~17,200 |
| Phase 2.5 | Observer 孤儿测试 | ~7,350 |
| Phase 3 | Trace 孤儿测试 | ~230 |
| **累计** | **~39,580 行** | ✅ |

### 4.2 目录清理完成

- ✅ `packages/ccode/memory/` - 已删除
- ✅ `packages/ccode/docs/` - 已删除
- ✅ `test/observer/` - 已删除 (Phase 2.5)
- ✅ `test/unit/trace/` - 已删除 (Phase 3)

---

## 5. 验证命令

```bash
# 检查废弃模块是否仍被引用
grep -r "from ['\"]@/agent/agent['\"]" packages/ccode/src --include="*.ts" | grep -v ".test.ts"
grep -r "from ['\"]@/provider/provider['\"]" packages/ccode/src --include="*.ts" | grep -v ".test.ts"

# 检查 console.log 使用
grep -rn "console.log" packages/ccode/src --include="*.ts" | grep -v "cli/cmd" | wc -l

# 验证 SDK 模式运行
CODECODER_SDK_MODE=1 bun dev --health

# 检查孤儿测试目录
find packages/ccode/test -type d -empty
```

---

## 6. 建议下一步

### 6.1 立即可执行 (P1)

1. 验证 SDK 模式全功能正常
2. 删除 `agent/agent.ts` (除 Agent 类型定义)
3. 删除 `provider/provider.ts` (除 Provider 类型定义)
4. 删除 `security/index.ts`

### 6.2 中期 (P2)

1. 完成 TUI 到 SDK 的全面切换
2. 删除 session/*.ts 废弃部分
3. 替换 api/server 中的 console.log 为结构化日志

### 6.3 持续 (P3)

1. 定期运行 `grep -r "@deprecated"` 检查新增废弃标记
2. 保持 DEBT.md 更新

---

## 更新记录

| 日期 | 变更 |
|------|------|
| 2026-03-11 | 初始审计报告 |
