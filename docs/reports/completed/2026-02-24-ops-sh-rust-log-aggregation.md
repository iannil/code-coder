# ops.sh Rust 服务日志聚合实现报告

## 概述

**日期**: 2026-02-24
**任务**: 在 `ops.sh` 中聚合 Rust 微服务日志
**状态**: 已完成

## 问题背景

分布式追踪功能已实现，Rust 服务的 lifecycle events 正确输出到独立日志文件。但 `./ops.sh logs` 命令只显示核心服务（api, web, zero-daemon, whisper）的日志，没有聚合由 daemon spawn 的 Rust 微服务日志。

用户需要手动 `cat .logs/zero-channels.log` 查看微服务日志，体验不佳。

## 实现方案

### 修改文件

- `ops.sh`

### 新增功能

1. **Rust 微服务常量**
   - 新增 `RUST_MICROSERVICES` 变量，包含 `zero-gateway zero-channels zero-workflow`

2. **颜色映射扩展**
   - `zero-gateway`: 黄色 (`\033[0;33m`)
   - `zero-channels`: 亮红色 (`\033[0;91m`)
   - `zero-workflow`: 亮蓝色 (`\033[0;94m`)

3. **服务名称扩展**
   - `get_service_name()` 函数支持 Rust 微服务的友好名称

4. **`logs all` 聚合**
   - `show_all_logs()` 现在遍历 `RUST_MICROSERVICES`，显示其日志

5. **`tail all` 聚合**
   - `tail_all_logs()` 现在包含 Rust 微服务日志的实时监控

6. **`logs trace <id>` 命令**
   - 新增 `show_trace_logs()` 函数，按 `trace_id` 搜索并聚合所有服务日志
   - 支持分布式追踪分析

7. **直接查看微服务日志**
   - 支持 `./ops.sh logs zero-channels` 等直接查看 Rust 微服务日志

### 帮助文档更新

- 新增 `logs trace <id>` 命令说明
- 更新 `tail all` 说明，注明包含 Rust 微服务
- 新增示例：`./ops.sh logs zero-channels`、`./ops.sh logs trace <trace_id>`

## 验证结果

```bash
# 1. 查看所有服务日志（包括 Rust 微服务）
./ops.sh logs all
# ✓ 显示 api, web, zero-daemon, whisper, zero-gateway, zero-channels, zero-workflow

# 2. 按 trace_id 搜索日志
./ops.sh logs trace ea3e6821-ea34-4ad3-bfcf-8864c1e3535f
# ✓ 聚合显示 api 和 zero-channels 中相关的日志条目

# 3. 直接查看 Rust 微服务日志
./ops.sh logs zero-channels
# ✓ 显示最后 50 行日志

# 4. 帮助信息更新
./ops.sh help
# ✓ 包含新命令说明
```

## 技术细节

### 日志文件位置

| 服务 | 日志文件 |
|------|----------|
| api | `.logs/api.log` |
| web | `.logs/web.log` |
| zero-daemon | `.logs/zero-daemon.log` |
| whisper | Docker 日志 |
| zero-gateway | `.logs/zero-gateway.log` |
| zero-channels | `.logs/zero-channels.log` |
| zero-workflow | `.logs/zero-workflow.log` |

### trace_id 搜索原理

使用 `grep` 在所有 `.logs/*.log` 文件中搜索 `trace_id`，按服务分组显示匹配的日志行。支持 JSON 格式和 tracing 格式的日志。

## 后续建议

1. **日志轮转**: 考虑添加日志轮转配置，防止日志文件过大
2. **时间排序**: `logs trace` 命令可增强为按时间戳排序所有匹配条目
3. **JSON 解析**: 可考虑使用 `jq` 进行更精细的 JSON 日志过滤
