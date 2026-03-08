# 架构简化评估报告

> 完成时间: 2026-03-08

## 概要

本次简化评估验证了代码库的实际状态，更新了过时的文档，并清理了少量死代码。原计划预估的大规模清理（5-10k行）实际上是对代码结构的误读——大部分被标记为"无外部导入"的模块实际上是良好封装的内部实现。

## 完成的工作

### Phase 1: 文档同步 ✓

**已更新的文档:**
1. `docs/architecture/ARCHITECTURE.md`
2. `docs/architecture/CCODE_VS_ZERO.md`
3. `CLAUDE.md`

**修正的信息:**

| 原文档描述 | 实际代码 |
|-----------|---------|
| 6 个独立 Rust 微服务 (gateway, channels, workflow, browser, trading, api) | 5 个 Rust crates (zero-cli, zero-core, zero-hub, zero-trading, zero-common) |
| 独立端口 4430-4435 | 统一通过 zero-cli daemon :4402 |
| zero-gateway, zero-channels, zero-workflow 独立服务 | 已整合为 `zero-hub` 的模块 |

**架构简化图:**
```
原设计:
  zero-cli → spawn → [gateway, channels, workflow, browser, trading, api]
                      (6个独立进程)

实际实现:
  zero-cli (daemon)
    ├── zero-core (工具库)
    ├── zero-hub (gateway + channels + workflow)
    ├── zero-trading
    └── zero-common
    (单进程，多模块)
```

### Phase 2: autonomous/ 子模块整合 ✓

**分析结果:**

原计划误判了 autonomous/ 的"无外部导入"模块为死代码。实际情况：

| 模块 | 状态 | 说明 |
|------|------|------|
| state/ | ✅ 通过 index.ts 导出 | 核心状态机 |
| decision/ | ✅ 通过 index.ts 导出 | CLOSE 决策框架 |
| orchestration/ | ✅ 通过 index.ts 导出 | 任务编排 |
| planning/ | ✅ 通过 index.ts 导出 | 需求跟踪 |
| safety/ | ✅ 通过 index.ts 导出 | 安全守护 |
| metrics/ | ✅ 通过 index.ts 导出 | 度量收集 |
| integration/ | ✅ 通过 index.ts 导出 | 集成钩子 |
| hands/ | ✅ 内部使用 | Rust Hands 桥接 |
| **agent/** | ❌ **已删除** | 未使用的死代码 |

**清理的代码:**
- `packages/ccode/src/autonomous/agent/autonomous-agent.ts` (166 行)

**验证:** `bun turbo typecheck` 通过

### Phase 3: NAPI fallback 完整性评估 ✓

**覆盖率分析:**

| 类别 | 函数数量 | 覆盖率 |
|------|---------|--------|
| 有 JS fallback | 6 | 3.6% |
| 无 fallback (NAPI-only) | 161 | 96.4% |

**有 fallback 的功能:**
- grep, glob, readFile, editFile
- version, init

**无 fallback 的主要功能:**
- 知识图谱 (Graph*, Causal*, Call*, Semantic*)
- 向量操作 (cosineSimilarity, normalizeVector, etc.)
- Shell 解析 (parseShellCommand, assessShellCommandsRisk)
- Git 操作 (GitOpsHandle)
- PTY 管理 (PtySessionHandle)
- 嵌入索引 (EmbeddingIndexHandle)
- 风险评估 (assessBashRisk, assessFileRisk)

**结论:**

扩展 fallback 覆盖不是优先事项。原因：
1. 基础 CLI 功能（文件操作）已有 fallback
2. 高级功能的 JS 实现会损失性能优势
3. 完整 fallback 估计需要 3000+ 行代码

## 实际节省 vs 预估

| 指标 | 原计划预估 | 实际结果 |
|------|-----------|---------|
| 删除代码量 | 5-10k 行 | 166 行 |
| 清理子目录数 | 5-8 个 | 1 个 |
| 文档更新 | 2 个文件 | 3 个文件 |

**差异原因:** 原计划将"无外部导入"误解为"未使用"。实际上这些模块通过 `autonomous/index.ts` 统一导出，是良好封装的体现。

## 后续建议

### 短期（不需要立即执行）
- 无

### 中期（可选优化）
1. **API handlers 合并**: 41 个 handlers 可以按功能域合并，减少文件数量
2. **NAPI fallback 按需扩展**: 如果特定环境需要无 Rust 运行，可针对性添加

### 长期（架构演进）
1. **zero-hub 进一步拆分评估**: 当前 56k 行，如果继续增长可考虑拆分
2. **zero-core 精简**: 73k 行中可能有可提取的独立模块

## 验证命令

```bash
# 类型检查通过
bun turbo typecheck

# Git status clean (仅文档变更)
git status
```

## 修改的文件

1. `docs/architecture/ARCHITECTURE.md` - 更新 Rust 服务架构描述
2. `docs/architecture/CCODE_VS_ZERO.md` - 更新依赖关系图
3. `CLAUDE.md` - 更新端口配置和服务架构
4. `packages/ccode/src/autonomous/agent/autonomous-agent.ts` - **已删除**
