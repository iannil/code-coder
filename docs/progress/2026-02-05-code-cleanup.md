# Code Cleanup Plan 2026-02-05

> 文档类型: progress
> 创建时间: 2026-02-05
> 更新时间: 2026-02-05
> 状态: active

## 目标

记录和跟踪代码清理任务，包括测试错误修复、TODO/FIXME 清理、重复代码消除。

## 测试错误

### 类型错误概览

当前约 **103** 个类型错误，主要集中在 TUI 集成测试。

| 测试类型 | 文件数 | 类型错误 |
|----------|--------|----------|
| E2E 测试 | 21 | ~30 |
| 单元测试 | 33 | ~10 |
| 集成测试 | 18 | ~60 |
| 性能测试 | 4 | 0 |
| 无障碍测试 | 2 | 0 |

### 主要错误模式

1. **OpenTUI 类型不完整**
   - `component(...)` 返回类型不匹配
   - 事件处理器类型定义缺失
   - `isEmpty` 等工具函数类型问题

2. **测试辅助函数类型**
   - `getOutput()` this 类型问题
   - `tolerance` 参数可选性问题

### 失败测试清单

需要进一步调查的 5 个失败测试：

```bash
# 运行测试并记录失败详情
bun test 2>&1 | grep -A 10 "FAIL"
```

## TODO/FIXME 清理

### 统计

| 标记 | 数量 | 主要分布 |
|------|------|----------|
| TODO | ~142 | 散布在各模块 |
| FIXME | ~16 | 主要在调试和反向工程模块 |

### 优先级分类

#### 高优先级 (影响功能)

- CLI 命令中的 TODO
- Agent 核心逻辑中的 FIXME
- 安全相关标记

#### 中优先级 (改进功能)

- 性能优化 TODO
- 错误处理改进 FIXME

#### 低优先级 (nice-to-have)

- 文档改进 TODO
- 重构建议 FIXME

## console.log 清理

### 源文件清单 (15 个文件)

| 文件 | console.log 数量 | 状态 |
|------|------------------|------|
| src/cli/cmd/reverse.ts | 20+ | ✅ 适当 - CLI 输出 |
| src/cli/cmd/document.ts | 15+ | ✅ 适当 - CLI 输出 |
| src/cli/cmd/agent.ts | 1 | ✅ 适当 - CLI 输出 |
| src/cli/cmd/session.ts | 1 | ✅ 适当 - CLI 输出 |
| src/cli/cmd/debug/*.ts | 5+ | ✅ 适当 - 调试输出 |

### 分析结果

所有 console.log 语句都在 CLI 命令文件中，这是适当的使用场景：

1. **CLI 命令输出** - `reverse.ts`, `document.ts`, `agent.ts`, `session.ts`
   - 这些是用户可见的命令输出
   - 使用 console.log 是 CLI 工具的标准做法

2. **调试命令** - `debug/index.ts`, `debug/file.ts`, `debug/snapshot.ts`
   - 调试命令需要输出详细信息
   - console.log 是调试工具的预期行为

### 结论

✅ **无需更改** - 所有 console.log 语句都在适当的位置

CLI 命令应该使用 console.log 进行输出，这是 Node.js CLI 工具的标准模式。

## 重复代码

### invalidate 函数 (16 处)

在多个组件中存在相似的 `invalidate` 模式：

```typescript
// 常见模式
const invalidate = () => {
  // 刷新逻辑
}
```

建议: 考虑创建自定义 hook 或工具函数

## 行动计划

### Phase 1: 快速修复 (1-2 天)

- [ ] 修复 5 个失败测试
- [ ] 清理高优先级 console.log (session, file 模块)

### Phase 2: 类型错误 (1 周)

- [ ] 修复测试辅助函数类型
- [ ] 为 OpenTUI 组件添加类型声明

### Phase 3: 系统清理 (2-4 周)

- [ ] 按优先级处理 TODO/FIXME
- [ ] 抽象重复代码模式
- [ ] 更新文档反映变更

## 相关文档

- [技术债务清单](../DEBT.md)

## 更新记录

- 2026-02-05: 初始版本，记录清理任务清单
- 2026-02-05: 添加 memory-markdown 模块扩展进度

## memory-markdown 模块扩展 (2026-02-05)

### 新增文件

| 文件 | 说明 |
|------|------|
| `config.ts` | 配置加载 |
| `project.ts` | 项目检测 |
| `storage.ts` | 存储抽象 |

### 模块状态

- ✅ 类型检查通过
- ✅ 构建验证通过
- ✅ 遵循 LLM 友好原则
