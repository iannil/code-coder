# Phase 2: TypeScript to Rust 迁移 - 工具层集成测试与性能优化

## 状态: ✅ 已完成

完成时间: 2026-03-04

---

## 执行摘要

Phase 2 修复了 Phase 1 遗留的 18 个失败测试，实现 100% 测试通过率。

| 指标 | 修复前 | 修复后 |
|------|--------|--------|
| 总测试数 | 64 | 64 |
| 通过测试 | 46 (72%) | 64 (100%) |
| 失败测试 | 18 (28%) | 0 (0%) |

---

## 根本原因分析

### 核心问题: Serde Default vs Derive Default

Rust 的 `#[derive(Default)]` 与 `#[serde(default = "fn")]` 行为不同：

```rust
// 问题代码
#[derive(Default)]  // 使用 bool::default() = false
pub struct Options {
    #[serde(default = "default_true")]  // 只在反序列化时生效
    pub enabled: bool,
}

// 修复后
impl Default for Options {
    fn default() -> Self {
        Self { enabled: true }  // 显式设置正确默认值
    }
}
```

**受影响模块**: ShellOptions, WriteOptions, ReadOptions, GrepOptions, GlobOptions

### Shell 模块问题

1. **死锁风险**: 顺序读取 stdout/stderr 可能导致死锁
2. **超时实现**: `try_wait()` 轮询不可靠
3. **默认值**: `timeout_ms` 默认为 0 导致立即超时

**解决方案**: 使用 `wait_with_output()` + channel 超时模式

### Grep/Glob Arc 问题

`Arc::try_unwrap()` 失败因为 clone 的 Arc 在 `walk.run()` 后仍有引用。

**解决方案**: 使用作用域块限制 Arc clone 生命周期

---

## 修复详情

### 1. Shell 模块 (6 tests)

**文件**: `services/zero-core/src/tools/shell.rs`

**修复内容**:
- 实现 `ShellOptions` 手动 Default trait
- 使用 channel + `recv_timeout` 实现超时
- 使用 `wait_with_output()` 避免死锁

```rust
impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            timeout_ms: 120_000,  // 2 minutes
            max_output: 1024 * 1024,
            inherit_env: true,
            // ...
        }
    }
}
```

### 2. Write 模块 (3 tests)

**文件**: `services/zero-core/src/tools/write.rs`

**修复内容**:
- 实现 `WriteOptions` 手动 Default trait
- `create_parents: true` (默认创建父目录)
- `normalize_newlines: true` (默认 CRLF -> LF)

### 3. Read 模块 (1 test)

**文件**: `services/zero-core/src/tools/read.rs`

**修复内容**:
- 实现 `ReadOptions` 手动 Default trait
- `offset: 1` (1-indexed)
- `line_numbers: true`

### 4. Grep/Glob 模块 (7 tests)

**文件**:
- `services/zero-core/src/tools/grep.rs`
- `services/zero-core/src/tools/glob.rs`

**修复内容**:
- 实现 `GrepOptions` 和 `GlobOptions` 手动 Default trait
- 修复 Arc 作用域问题

```rust
// 修复前
let results_clone = Arc::clone(&results);
walk.run(|| { /* ... */ });
Arc::try_unwrap(results)  // FAILS: results_clone still alive

// 修复后
{
    let results_clone = Arc::clone(&results);
    walk.run(|| { /* ... */ });
}  // results_clone dropped here
Arc::try_unwrap(results)  // OK: only one reference
```

### 5. Edit 模块 (1 test)

**文件**: `services/zero-core/src/tools/edit.rs`

**修复内容**:
- 修正 `find_best_match` 测试期望值
- "apple" 比 "app" 与 "appl" 更相似 (0.889 vs 0.857)

---

## 验证命令

```bash
# 运行所有测试
cargo test -p zero-core

# 检查编译
cargo check -p zero-core
```

---

## 经验教训

1. **Rust Default 陷阱**: 当 serde 默认值与类型默认值不同时，必须手动实现 Default
2. **Arc 作用域**: 使用作用域块限制 Arc clone 生命周期
3. **进程 I/O**: 使用 `wait_with_output()` 而非手动管理 stdout/stderr

---

## 下一步

Phase 3: 会话层迁移到 zero-core
- session 模块与 TypeScript 集成
- NAPI 绑定实现
- 性能基准测试
