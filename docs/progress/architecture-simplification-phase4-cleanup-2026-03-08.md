# Phase 4: 清理旧服务 - 转换为纯库

## 完成时间
2026-03-08

## 概述
将 `zero-gateway`、`zero-channels`、`zero-workflow`、`zero-api` 从独立可执行服务转换为纯库，由 `zero-server` 统一调用。

## 实施内容

### Step 1: 移除 Binary 目标 ✅
修改 Cargo.toml 文件，删除 `[[bin]]` 段落，添加/保留 `[lib]` 段落：

| 文件 | 操作 |
|------|------|
| `services/zero-gateway/Cargo.toml` | 删除 `[[bin]]`，添加 `[lib]` |
| `services/zero-channels/Cargo.toml` | 删除 `[[bin]]`，添加 `[lib]` |
| `services/zero-workflow/Cargo.toml` | 删除 `[[bin]]`，添加 `[lib]` |
| `services/zero-api/Cargo.toml` | 删除 `[[bin]]`，保留 `[lib]` |

### Step 2: 删除 main.rs 文件 ✅
已删除以下文件：
- `services/zero-gateway/src/main.rs`
- `services/zero-channels/src/main.rs`
- `services/zero-workflow/src/main.rs`
- `services/zero-api/src/main.rs`

### Step 3: 更新 ops.sh ✅
更新了以下内容：

1. **架构说明注释** - 更新为 zero-server 统一服务
2. **RUST_MICROSERVICES** - 从 6 个服务减少为 3 个：
   - `zero-server` (统一服务)
   - `zero-browser`
   - `zero-trading`
3. **get_service_port()** - 移除旧服务端口
4. **get_service_name()** - 移除旧服务名称
5. **get_service_color()** - 移除旧服务颜色
6. **健康检查端点** - 更新为 zero-server
7. **METRICS_ENDPOINTS** - 简化为 ccode-api 和 zero-server
8. **服务状态面板** - 更新为新架构
9. **帮助文档** - 全面更新服务列表和说明

### Step 4: 验证 ✅

```bash
# 库编译成功
cargo build -p zero-gateway -p zero-channels -p zero-workflow -p zero-api
# Finished `dev` profile in 13.05s

# zero-server 依赖库编译成功
cargo build -p zero-server
# Finished `dev` profile in 6.69s
```

## 变更统计

| 指标 | 变更前 | 变更后 |
|------|--------|--------|
| Rust 可执行服务 | 6 | 3 |
| ops.sh 服务引用 | ~40 处 | ~20 处 |
| 端口占用 | 4430-4435 (6个) | 4430, 4433, 4434 (3个) |

## 保留的可执行服务
- `zero-cli` - CLI 主程序和 daemon
- `zero-server` - 统一服务 (Gateway+Channels+Workflow+API)
- `zero-browser` - 浏览器自动化
- `zero-trading` - 自动化交易

## 转换为库的服务
- `zero-gateway` → `libzero_gateway`
- `zero-channels` → `libzero_channels`
- `zero-workflow` → `libzero_workflow`
- `zero-api` → `libzero_api`

## 注意事项
- 这些目录不能删除，因为 `zero-server` 依赖它们的库代码
- 库名使用下划线（`zero_gateway`）而非连字符
- 编译时会有一些未使用变量的警告，不影响功能
