# 技术债务清理完成报告

**完成时间**: 2026-02-16
**相关任务**: P0-P4 技术债务清理

## 完成摘要

基于 `docs/DEBT.md` 技术债务清单，完成了以下清理工作：

| 优先级 | 任务 | 状态 |
|--------|------|------|
| P0 | BookExpander Zod 兼容性 | ✅ 完成 |
| P1 | 导入路径标准化 | ✅ 完成 |
| P2 | ACP 文档清理 | ✅ 完成 |
| P3 | ZeroBot 类型共享 | ✅ 完成 |
| P4 | 测试 skip 标记处理 | ✅ 完成 |

---

## P0: BookExpander Zod 兼容性修复

### 问题
Zod v4.1.8 + Bun 运行时存在 `escapeRegex` 函数错误，导致 `.default([])` 模式失败。

### 解决方案
创建辅助函数避免直接使用 `.default([])`：

```typescript
const defaultArray = <T extends z.ZodTypeAny>(schema: T) =>
  z.array(schema).optional().transform((v) => v ?? [])

const defaultRecord = <V extends z.ZodTypeAny>(valueSchema: V) =>
  z.record(z.string(), valueSchema).optional().transform((v) => v ?? {})
```

### 修改文件
- `packages/ccode/src/document/knowledge/schema.ts`
- `packages/ccode/src/document/context.ts`
- `packages/ccode/src/document/schema.ts`

### 验证
```bash
bun run tsc --noEmit  # 无 document 相关错误
```

---

## P1: 导入路径标准化

### 问题
项目混用三种导入方式：
- `../util/` 相对路径：59处
- `@/util/` 别名：151处
- `@codecoder-ai/util`：25处

### 解决方案
统一使用 `@/util/` 别名替换 `../util/` 相对路径。

### 修改范围
- 58 处 `../util/` 替换为 `@/util/`
- 1 处保留（TUI 本地工具 `createDebouncedSignal`）

### 验证
```bash
grep -r 'from "../util/' packages/ccode/src  # 仅剩 TUI 本地引用
```

---

## P2: ACP 文档清理

### 问题
ACP (Agent Client Protocol) 已移除，但文档中仍有引用。

### 解决方案
- 保留历史记录中的 ACP 提及（作为架构演进记录）
- 删除代码中无效的 ACP 引用
- 更新活跃文档中的 ACP 引用

### 修改文件
- `docs/zrs/products.md` - 移除 ACP 协议引用
- `packages/ccode/test/verify-coverage.ts` - 移除 AU (ACP User) 类型

---

## P3: ZeroBot 类型共享

### 问题
Rust 和 TypeScript 之间缺乏类型同步机制。

### 解决方案
1. 更新 TypeScript 类型匹配 Rust 类型（添加 `session_id` 字段）
2. 添加详细文档说明类型来源
3. 记录 ts-rs 自动生成的设置步骤（作为后续 TODO）

### 修改文件
- `packages/ccode/src/memory-zerobot/types.ts`

### 类型对照

| Rust 字段 | TypeScript 字段 | 状态 |
|-----------|----------------|------|
| id: String | id: string | ✅ |
| key: String | key: string | ✅ |
| content: String | content: string | ✅ |
| category: MemoryCategory | category: MemoryCategory | ✅ |
| timestamp: String | timestamp: string | ✅ |
| session_id: Option<String> | session_id?: string | ✅ 新增 |
| score: Option<f64> | score?: number | ✅ |

---

## P4: 测试 skip 标记处理

### 问题
14 个 skip 标记分布在 13 个测试文件中。

### 分类结果

**合理的条件跳过** (保留):
- `skipIf(!rgAvailable)` - 依赖 ripgrep 工具
- `skipIf(SKIP_E2E)` - E2E 测试默认跳过

**需要后续处理** (已文档化):
- `autonomous-mode.test.tsx` - 需要更新测试断言匹配当前 API

### 修改文件
- `packages/ccode/test/integration/autonomous-mode.test.tsx` - 更新文档说明

---

## 验证命令

```bash
# 类型检查
bun run tsc --noEmit

# 构建验证
bun run --cwd packages/ccode build
```

---

## 后续建议

1. **ts-rs 自动化**: 考虑在 CI 中添加类型同步检查
2. **autonomous-mode 测试**: 计划专门的测试重写任务
3. **Zod 升级**: 监控 Zod v4 的 Bun 兼容性修复
