# 日志目录统一：log → logs

**状态**: ✅ 已完成
**实施时间**: 2026-03-01

## 变更概述

统一项目中日志目录路径，从 `~/.codecoder/log`（单数）改为 `~/.codecoder/logs`（复数），解决 TypeScript 和可观测性子系统之间的路径不一致问题。

## 问题分析

项目中存在两处日志目录定义的不一致：

| 位置 | 旧路径 | 新路径 |
|------|--------|--------|
| TypeScript Global.Path | `log` | `logs` |
| Rust daemon | `log` | `logs` |
| 可观测性/追踪子系统 | `logs` | `logs` (已统一) |

## 实施内容

### 1. TypeScript 路径定义

**文件**: `packages/ccode/src/global/index.ts`

```typescript
// 修改前
get log() {
  return path.join(configDir(), "log")
}

// 修改后
get logs() {
  return path.join(configDir(), "logs")
}
```

同时更新目录创建代码使用 `Global.Path.logs`。

### 2. 日志工具引用

**文件**: `packages/ccode/src/util/log.ts`

所有 `Global.Path.log` 引用更新为 `Global.Path.logs`。

### 3. Rust daemon 路径

**文件**: `services/zero-cli/src/daemon/mod.rs`

```rust
// 修改前
let resolved_log_dir = log_dir.unwrap_or_else(||
    zero_common::config::config_dir().join("log"));

// 修改后
let resolved_log_dir = log_dir.unwrap_or_else(||
    zero_common::config::config_dir().join("logs"));
```

### 4. 迁移脚本更新

**文件**: `packages/ccode/src/migration/storage-unification.ts`

- 更新 XDG 迁移目标路径为 `logs`
- 新增内部迁移路径：`~/.codecoder/log` → `~/.codecoder/logs`

### 5. 文档更新

- `docs/RUNBOOK.md` - 清理重复路径、更新目录结构
- `docs/reports/completed/storage-unification-implementation.md` - 同步更新

## 验证结果

- ✅ Rust 编译检查通过
- ✅ TypeScript 文件无新增类型错误
- ✅ 所有 `Global.Path.log` 引用已更新为 `Global.Path.logs`
- ✅ Rust daemon 使用 `logs` 目录
- ✅ 迁移脚本支持旧路径迁移

## 修改文件列表

| 文件 | 变更类型 |
|------|---------|
| `packages/ccode/src/global/index.ts` | 修改 |
| `packages/ccode/src/util/log.ts` | 修改 |
| `services/zero-cli/src/daemon/mod.rs` | 修改 |
| `packages/ccode/src/migration/storage-unification.ts` | 修改 |
| `docs/RUNBOOK.md` | 修改 |
| `docs/reports/completed/storage-unification-implementation.md` | 修改 |

## 用户操作指南

如果用户已有 `~/.codecoder/log` 目录中的数据，可执行以下命令迁移：

```bash
# 预览迁移
bun run packages/ccode/src/migration/storage-unification.ts --migrate --dry-run

# 执行迁移
bun run packages/ccode/src/migration/storage-unification.ts --migrate

# 或手动迁移
mv ~/.codecoder/log/* ~/.codecoder/logs/
rmdir ~/.codecoder/log
```
