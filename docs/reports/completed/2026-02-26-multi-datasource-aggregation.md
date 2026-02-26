# 多数据源聚合架构实现报告

**日期**: 2026-02-26
**状态**: ✅ 已完成

## 概述

本报告记录了 zero-trading 多数据源聚合架构的实现过程。该架构实现了自动故障转移（Failover）机制，使用 **Ashare 作为主数据源**，**理杏仁 (Lixin) 作为备用数据源**，并提供后台健康检查功能。

> **重要更新**: Tushare 已被完全移除，替换为理杏仁 (Lixin) 作为备用数据源。

## 当前数据源架构

```
┌─────────────────────────────────────────────────────────────┐
│                   MarketDataAggregator                       │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                 DataProviderRouter                     │  │
│  │                                                        │  │
│  │   ┌──────────────┐      ┌──────────────┐              │  │
│  │   │ AshareAdapter│ ──▶  │ LixinAdapter │              │  │
│  │   │  优先级: 1   │      │   优先级: 2  │              │  │
│  │   │   (主要)     │      │    (备用)    │              │  │
│  │   └──────────────┘      └──────────────┘              │  │
│  │         │                      │                       │  │
│  │         ▼                      ▼                       │  │
│  │   eastmoney API           理杏仁 API                   │  │
│  │   (免费/无限制)           (需Token/高质量)             │  │
│  └───────────────────────────────────────────────────────┘  │
│                           │                                  │
│              ┌────────────┴────────────┐                    │
│              ▼                         ▼                    │
│       HealthMonitor              DataCache                  │
│       (30秒检查一次)              (TTL 60秒)                 │
└─────────────────────────────────────────────────────────────┘
```

## 数据源对比

| 属性 | Ashare (主要) | Lixin (备用) |
|------|--------------|--------------|
| 优先级 | 1 (最高) | 2 |
| API Key | 不需要 | 需要 `lixin_token` |
| 调用限制 | 无 | 有（按订阅级别） |
| 费用 | 免费 | 付费 |
| 数据类型 | 日线、分钟线、指数 | 日线、指数（无分钟线） |
| 数据质量 | 良好 | 高（特别是基本面数据） |
| 实时数据 | 支持 | 不支持 |

## 实现内容

### 新增文件

| 文件路径 | 描述 |
|---------|------|
| `data/provider.rs` | DataProvider trait 定义 |
| `data/health.rs` | HealthMonitor 实现 |
| `data/router.rs` | DataProviderRouter 实现 |
| `data/ashare.rs` | Ashare 适配器 (eastmoney API) |
| `data/lixin.rs` | 理杏仁适配器 |

### 修改文件

| 文件路径 | 描述 |
|---------|------|
| `data/mod.rs` | 更新模块导出，移除 tushare |
| `data/aggregator.rs` | 使用 Router，支持 Ashare + Lixin |
| `zero-common/config.rs` | `tushare_token` → `lixin_token` |

### 删除文件

| 文件路径 | 原因 |
|---------|------|
| `data/tushare.rs` | 替换为 Lixin |

## 配置方式

在 `~/.codecoder/config.json` 中：

```json
{
  "trading": {
    "lixin_token": "2b365f7e-9024-4450-ace4-4f037cc0f75c",
    "data_sources": {
      "sources": [
        {"provider": "ashare", "enabled": true, "priority": 1},
        {"provider": "lixin", "enabled": true, "priority": 2}
      ],
      "health_check_interval_secs": 30,
      "unhealthy_threshold": 3
    }
  }
}
```

如果不配置 `data_sources`，默认使用 Ashare 作为主数据源，如果配置了 `lixin_token` 则自动添加理杏仁作为备用。

## 故障转移机制

```
请求流程:
1. 检查 Ashare 是否健康 → 是 → 使用 Ashare
                        ↓ 否
2. 检查 Lixin 是否健康 → 是 → 使用 Lixin
                        ↓ 否
3. 尝试所有提供者（包括不健康的）作为最后手段
```

**健康检查规则**:
- 每 30 秒检查一次
- 连续 3 次失败 → 标记为 unhealthy
- 恢复后自动重新启用

## 测试结果

```
test result: ok. 38 passed; 0 failed; 4 ignored

测试覆盖:
- data::provider::tests - 5 tests
- data::health::tests - 7 tests
- data::router::tests - 5 tests
- data::ashare::tests - 6 tests
- data::lixin::tests - 5 tests (2 需要 API token)
- data::aggregator::tests - 3 tests
- 其他 data 模块测试 - 7 tests
```

## 使用示例

```rust
let config = Config::load()?;
let aggregator = MarketDataAggregator::new(&config);
aggregator.initialize(&config).await?;

// 获取数据（自动路由到健康的提供者）
let candles = aggregator.get_candles("000001.SZ", Timeframe::Daily, 100).await?;

// 查看提供者状态
let providers = aggregator.get_providers_info().await;
for p in providers {
    println!("{}: healthy={}, priority={}", p.name, p.healthy, p.priority);
}
```

## 理杏仁 API 说明

**基础 URL**: `https://open.lixinger.com/api`

**主要端点**:
- `/a/stock/fs/daily-candlestick` - 股票日线
- `/a/index/fs/daily-candlestick` - 指数日线

**请求格式**:
```json
{
  "token": "your-api-token",
  "stockCodes": ["000001"],
  "startDate": "2024-01-01",
  "endDate": "2024-12-31"
}
```

**注意事项**:
- 理杏仁不提供分钟级别数据，分钟线请求会返回 `DataNotAvailable` 错误
- 分钟线数据将自动 failover 到 Ashare

---

*实现完成于 2026-02-26*
