# 统一可观测性方案实施报告

**实施日期**: 2026-03-01
**状态**: ✅ 已完成 (Phase 1-4)

## 快速入门

```bash
# 1. 启动服务
./ops.sh start

# 2. 查看实时仪表盘
./ops.sh dashboard

# 3. 查看服务指标
./ops.sh metrics
./ops.sh metrics --watch  # 实时刷新

# 4. 查看慢请求
./ops.sh slow 500         # 显示 > 500ms 的请求

# 5. 链路追踪
./ops.sh trace <trace_id>

# 6. 直接访问 metrics 端点
curl http://127.0.0.1:4400/metrics      # ccode-api
curl http://127.0.0.1:4430/metrics      # zero-gateway
curl http://127.0.0.1:4431/metrics      # zero-channels
```

## 实施概览

本次实施为 CodeCoder 项目添加了统一的可观测性层，包括 Metrics 端点、增强的 ops.sh 命令、告警系统和 TUI 仪表盘。

## Phase 1: Metrics 端点统一 ✅

### 新建文件

1. **`services/zero-common/src/metrics.rs`**
   - 统一的 Metrics 收集器
   - 支持 Counter、Gauge、Histogram 类型
   - 滑动窗口 percentile 计算 (p50/p95/p99)
   - Prometheus 兼容文本格式输出
   - 线程安全 (`RwLock`)

2. **`packages/ccode/src/api/server/handlers/metrics.ts`**
   - TypeScript API metrics 处理器
   - `/metrics` - Prometheus 文本格式
   - `/api/v1/metrics` - JSON 格式

### 修改文件

1. **`services/zero-common/src/lib.rs`**
   - 添加 `pub mod metrics;`
   - 导出 `MetricsRegistry`, `MetricsSnapshot`

2. **`services/zero-gateway/src/routes.rs`**
   - 添加 `MetricsRegistry` 到 `AppState`
   - 添加 `/metrics` 和 `/api/v1/metrics` 路由
   - 添加 `metrics_handler` 和 `metrics_json_handler`

3. **`services/zero-channels/src/routes.rs`**
   - 添加 `metrics` 字段到 `ChannelsState`
   - 添加 `/metrics` 和 `/api/v1/metrics` 路由
   - 添加 metrics 处理函数

4. **`services/zero-channels/src/lib.rs`**
   - 初始化 MetricsRegistry

5. **`packages/ccode/src/api/server/router.ts`**
   - 注册 metrics 路由

## Phase 2: ops.sh 增强 ✅

### 新增命令

1. **`./ops.sh metrics [service] [--watch]`**
   - 显示所有服务的实时指标表格
   - 支持单服务筛选
   - `--watch` 模式每 2 秒刷新

2. **`./ops.sh trace <trace_id>`**
   - 可视化显示完整调用链
   - 跨服务追踪
   - Timeline 格式输出

3. **`./ops.sh slow [threshold] [--live]`**
   - 显示慢请求 (默认 > 1000ms)
   - 支持自定义阈值
   - `--live` 实时监控模式

4. **`./ops.sh dashboard [refresh_secs]`**
   - 启动实时 TUI 仪表盘
   - 服务状态、指标、错误面板
   - 可配置刷新间隔

### 修改文件

- **`ops.sh`**
  - 添加 `show_metrics()` 函数
  - 添加 `show_trace()` 函数
  - 添加 `show_slow_requests()` 函数
  - 添加 `show_dashboard()` 函数
  - 更新帮助信息

## Phase 3: 告警系统 ✅

### 新建文件

1. **`services/zero-cli/src/alerts/mod.rs`**
   - 告警规则引擎
   - 支持多种条件类型:
     - `ErrorRate` - 错误率阈值
     - `P99Latency` / `P95Latency` - 延迟阈值
     - `ServiceDown` - 服务不可用
     - `MemoryUsage` - 内存使用阈值
   - 告警静默 (防止告警风暴)
   - 通过 zero-channels 发送通知

### 修改文件

1. **`services/zero-cli/src/lib.rs`**
   - 添加 `pub mod alerts;`

2. **`services/zero-cli/src/daemon/mod.rs`**
   - 添加 alert worker 到后台任务

### 配置文件格式

默认配置位于 `~/.codecoder/alerts.json`:

```json
{
  "default_channel": "telegram",
  "silence_duration_secs": 300,
  "channels_endpoint": "http://127.0.0.1:4431",
  "rules": [
    {
      "name": "high_error_rate",
      "condition": "error_rate",
      "threshold": 5.0,
      "duration_secs": 60,
      "severity": "critical",
      "services": [],
      "enabled": true
    },
    {
      "name": "high_latency",
      "condition": "p99_latency",
      "threshold": 2000.0,
      "duration_secs": 30,
      "severity": "warning",
      "services": [],
      "enabled": true
    },
    {
      "name": "service_down",
      "condition": "service_down",
      "threshold": 0.0,
      "duration_secs": 10,
      "severity": "critical",
      "services": [],
      "enabled": true
    }
  ]
}
```

## Phase 4: TUI Dashboard ✅

### 功能

- 实时服务状态面板 (绿点/红点)
- 实时指标表格 (请求数、错误率、p50/p95/p99、内存)
- 最近错误面板 (最后 5 条)
- 可配置刷新间隔
- 优雅退出 (Ctrl+C)

### 使用方式

```bash
# 默认每 2 秒刷新
./ops.sh dashboard

# 自定义刷新间隔 (5 秒)
./ops.sh dashboard 5
```

## 验证方法

### 1. Metrics 端点验证

```bash
# 检查各服务 metrics 端点
curl http://127.0.0.1:4400/metrics
curl http://127.0.0.1:4430/metrics
curl http://127.0.0.1:4431/metrics

# JSON 格式
curl http://127.0.0.1:4400/api/v1/metrics
```

### 2. ops.sh 命令验证

```bash
# 查看指标
./ops.sh metrics
./ops.sh metrics --watch

# 查看调用链
./ops.sh trace <trace_id>

# 查看慢请求
./ops.sh slow 500
./ops.sh slow --live

# 启动仪表盘
./ops.sh dashboard
```

### 3. 告警验证

```bash
# 启动 daemon (会自动启动 alert worker)
./ops.sh start zero-daemon

# 查看日志确认 alert worker 启动
./ops.sh logs zero-daemon | grep -i alert

# 人为制造错误触发告警
curl http://127.0.0.1:4430/nonexistent -X POST
```

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `services/zero-common/src/metrics.rs` | 新建 | 统一 Metrics 收集器 |
| `services/zero-common/src/lib.rs` | 修改 | 导出 metrics 模块 |
| `services/zero-gateway/src/routes.rs` | 修改 | 添加 /metrics 路由 |
| `services/zero-channels/src/routes.rs` | 修改 | 添加 /metrics 路由 |
| `services/zero-channels/src/lib.rs` | 修改 | 初始化 metrics |
| `packages/ccode/src/api/server/handlers/metrics.ts` | 新建 | TypeScript metrics |
| `packages/ccode/src/api/server/router.ts` | 修改 | 注册 metrics 路由 |
| `services/zero-cli/src/alerts/mod.rs` | 新建 | 告警系统 |
| `services/zero-cli/src/lib.rs` | 修改 | 导出 alerts 模块 |
| `services/zero-cli/src/daemon/mod.rs` | 修改 | 添加 alert worker |
| `ops.sh` | 修改 | 添加 metrics/trace/slow/dashboard 命令 |

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          观测层 (Observability Layer)                    │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │   Metrics    │  │   Logging    │  │   Tracing    │  │   Alerts    │ │
│  │  /metrics    │  │  结构化日志   │  │  trace_id    │  │   rules.json│ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────┘ │
│         │                 │                 │                 │        │
│         └─────────────────┼─────────────────┼─────────────────┘        │
│                           ▼                 ▼                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    ops.sh CLI 工具                               │   │
│  │  • metrics   - 指标查看                                         │   │
│  │  • trace     - 链路追踪                                         │   │
│  │  • slow      - 慢请求分析                                       │   │
│  │  • dashboard - 实时仪表盘                                       │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                                            │
│         ┌─────────────────┼─────────────────┐                         │
│         ▼                 ▼                 ▼                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                 │
│  │  ccode-api   │  │zero-gateway  │  │zero-channels │                 │
│  │    :4400     │  │    :4430     │  │    :4431     │                 │
│  └──────────────┘  └──────────────┘  └──────────────┘                 │
└─────────────────────────────────────────────────────────────────────────┘
```

## 后续优化建议

1. **Grafana 集成** (可选): 如需更丰富的可视化，可接入 Grafana
2. **告警升级**: 支持 PagerDuty/OpsGenie 等专业告警平台
3. **分布式追踪**: 集成 Jaeger/Zipkin 实现完整 APM
4. **指标持久化**: 使用 InfluxDB/TimescaleDB 存储历史指标
