# TypeScript 到 Rust 迁移 - Phase 8 阻塞报告

**日期**: 2026-03-06
**状态**: ❌ 阻塞
**阻塞原因**: Native Git Bindings 尚未创建

## 执行尝试摘要

### 目标
Phase 8.1 计划将 `git-ops.ts` 从 child_process.execSync 迁移到 Rust 原生实现，删除约 600 行 fallback 代码。

### 发现的问题

**关键发现：Native Git Bindings 不存在**

迁移计划假设 `@codecoder-ai/core` 已导出以下 git 函数：
- `openGitRepo()`
- `initGitRepo()`
- `cloneGitRepo()`
- `isGitRepo()`
- 以及 `NativeGitOpsHandle` 类

**实际情况**：
1. Rust 实现确实存在于 `services/zero-core/src/git/mod.rs` (~949 行)
2. 但是**没有** NAPI 绑定导出到 `@codecoder-ai/core`
3. `packages/core/src/binding.js` 和 `binding.d.ts` 中不包含任何 git 相关导出
4. 原始 `git-ops.ts` 不是"有 fallback 的 native 实现"，而是**纯 child_process.execSync 实现**

### 尝试的迁移

1. 删除 `fallbackGetStatus()` 和 `fallbackCreateCommit()` 函数
2. 将 `loadNativeBindings()` 改为同步 fail-fast 模式
3. 将 `getNativeHandle()` 改为直接返回非空 handle
4. 移除所有 GitOps 函数中的 fallback 分支
5. 更新调用方（checkpoint.ts, project-scaffolder.ts, project.ts）

### 结果

迁移导致测试大量失败：
- 原始: 603 pass, 40 fail
- 迁移后: 452 pass, 191 fail
- 净增失败: +151

根本原因：移除了唯一可用的 child_process.execSync 实现，而 native 绑定根本不存在。

### 回滚操作

已回滚以下文件到原始状态：
- `packages/ccode/src/autonomous/execution/git-ops.ts`
- `packages/ccode/src/autonomous/execution/checkpoint.ts`
- `packages/ccode/src/autonomous/execution/project-scaffolder.ts`
- `packages/ccode/src/tool/project.ts`
- `packages/ccode/test/preload.ts`

## 解除阻塞所需步骤

### 1. 创建 Git NAPI 绑定

需要在 `services/zero-core/src/napi/` 中添加 git 模块：

```rust
// services/zero-core/src/napi/git.rs
use napi_derive::napi;
use crate::git::{Repository, Status, CommitInfo};

#[napi]
pub struct GitOpsHandle {
    repo: Repository,
}

#[napi]
impl GitOpsHandle {
    #[napi]
    pub fn status(&self) -> napi::Result<JsGitStatus> { ... }

    #[napi]
    pub fn commit(&self, message: String, add_all: Option<bool>, allow_empty: Option<bool>)
        -> napi::Result<JsCommitResult> { ... }

    // ... 其他方法
}

#[napi]
pub fn open_git_repo(path: String) -> napi::Result<GitOpsHandle> {
    let repo = Repository::open(&path)?;
    Ok(GitOpsHandle { repo })
}

#[napi]
pub fn init_git_repo(path: String, options: Option<JsInitOptions>)
    -> napi::Result<GitOpsHandle> { ... }

#[napi]
pub fn clone_git_repo(url: String, path: String, options: Option<JsCloneOptions>)
    -> napi::Result<GitOpsHandle> { ... }

#[napi]
pub fn is_git_repo(path: String) -> bool { ... }
```

### 2. 导出到 packages/core

在 `packages/core/src/index.ts` 中添加：

```typescript
export const openGitRepo = nativeBindings?.openGitRepo
export const initGitRepo = nativeBindings?.initGitRepo
export const cloneGitRepo = nativeBindings?.cloneGitRepo
export const isGitRepo = nativeBindings?.isGitRepo
export const GitOpsHandle = nativeBindings?.GitOpsHandle
```

### 3. 重建 native 模块

```bash
cd services/zero-core
cargo build --release
cd ../../packages/core
bun run build
```

### 4. 重新执行 Phase 8.1

一旦 NAPI 绑定可用，可以继续执行原计划。

## 计划调整

| Phase | 原状态 | 新状态 | 说明 |
|-------|--------|--------|------|
| 8.1 Git Ops | P1 | P2 | 降级，需先创建 NAPI 绑定 |
| 8.2-8.4 | P2/P3 | P3 | 维持低优先级 |

## 下一步建议

1. **短期**: 跳过 Phase 8，继续 Phase 9 (Builder 模块) 或其他已有 NAPI 绑定的模块
2. **中期**: 为 git 操作创建 NAPI 绑定 (估计工作量: 1-2 天)
3. **长期**: 完成 Phase 8.1 的原计划迁移

## 经验教训

1. **验证假设**: 在开始迁移前，应验证 NAPI 绑定确实存在并可用
2. **测试优先**: 修改前运行完整测试套件，确认基线状态
3. **渐进式验证**: 每个小改动后运行测试，而不是一次性修改所有文件

---

**负责人**: Claude
**创建时间**: 2026-03-06 04:50
