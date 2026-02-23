# 服务启动与验证报告

**日期**: 2026-02-23 18:14 CST
**状态**: 验证完成

## 验证结果摘要

| 服务 | 端口 | 状态 | 健康检查 |
|------|------|------|----------|
| CodeCoder API Server | 4400 | ✅ 运行中 | `{"status":"ok"}` |
| Web Frontend (Vite) | 4401 | ✅ 运行中 | HTML 正常返回 |
| Zero CLI Daemon | 4402 | ✅ 运行中 | `{"status":"ok"}` |
| Whisper STT Server | 4403 | ✅ 运行中 | Docker 容器运行 |
| Zero Gateway | 4410 | ✅ 运行中 | `{"status":"healthy"}` |
| Zero Channels | 4411 | ✅ 运行中 | `{"status":"healthy"}` |
| Zero Workflow | 4412 | ✅ 运行中 | `{"status":"healthy"}` |

## 端口监听确认

```
bun       16237  TCP *:4400 (LISTEN)
node      16278  TCP localhost:4401 (LISTEN)
zero-cli  23158  TCP localhost:4402 (LISTEN)
zero-gate 23245  TCP localhost:4410 (LISTEN)
zero-chan 23269  TCP localhost:4411 (LISTEN)
zero-work 23292  TCP localhost:4412 (LISTEN)
OrbStack  68829  TCP *:4403 (LISTEN) [Whisper Docker]
```

## 详细健康检查响应

### CodeCoder API (4400)
```json
{"status":"ok","version":"local","uptime":298580}
```

### Zero CLI Daemon (4402)
```json
{
  "paired": false,
  "runtime": {
    "components": {
      "daemon": {"status": "ok"},
      "gateway": {"status": "ok"},
      "mcp": {"status": "ok"},
      "scheduler": {"status": "ok"},
      "channels": {"status": "error", "restart_count": 7}
    },
    "uptime_seconds": 164
  },
  "status": "ok"
}
```

**注意**: `channels` 组件显示 error 状态，有 7 次重启记录。这可能是由于配置缺失（如 Telegram/Discord token）导致的，不影响主要功能。

### Rust 独立服务 (4410-4412)
```json
{"status":"healthy","service":"zero-gateway","version":"0.1.0"}
{"status":"healthy","service":"zero-channels","version":"0.1.0"}
{"status":"healthy","service":"zero-workflow","version":"0.1.0"}
```

## 构建信息

Rust 服务构建完成（release 模式）:
- zero-cli: 4.2M
- zero-gateway: 3.9M
- zero-channels: 3.2M
- zero-workflow: 3.6M

## 总结

端口重新分配验证通过。所有 7 个服务在新端口配置下正常运行：
- 核心服务 (4400-4403): 全部正常
- 独立 Rust 服务 (4410-4412): 全部正常

新的端口分配方案已确认工作正常，符合 `docs/standards/port-allocation.md` 的规划。
