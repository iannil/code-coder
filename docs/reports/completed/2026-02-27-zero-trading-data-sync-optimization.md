# zero-trading 数据同步优化

**日期**: 2026-02-27

## 问题描述

zero-trading 服务的 SQLite 数据库中没有数据，从 iTick 和 Lixin 接口抓取的数据没有被保存。

## 根因分析

1. **API 限流问题**：服务启动时同时请求多个股票和时间框架，导致 iTick 和 Lixin API 返回 429 Rate Limited
2. **所有数据提供者被标记为 unhealthy**：由于限流，数据提供者被标记为不健康
3. **没有请求间延迟**：`start_updater()` 和 `preload_historical_data()` 方法没有在请求间添加延迟

## 修复内容

### 1. 优化 `aggregator.rs` 中的请求延迟

**文件**: `services/zero-trading/src/data/aggregator.rs`

- `start_updater()`: 添加启动延迟 (5秒) 和符号间延迟 (500ms)
- `preload_historical_data()`: 添加符号间延迟 (1秒) 和时间框架间延迟 (300ms)
- 非交易时段跳过 H1/H4 时间框架，节省 API 配额

### 2. 新增数据同步方法

**文件**: `services/zero-trading/src/data/aggregator.rs`

- `sync_symbol_data()`: 同步单个股票数据，返回同步的蜡烛数量
- `sync_all_symbols()`: 批量同步所有追踪的股票，返回详细报告
- `get_storage_stats()`: 获取存储统计信息

### 3. 新增 HTTP API 接口

**文件**: `services/zero-trading/src/routes.rs`

| 路由 | 方法 | 描述 |
|------|------|------|
| `/api/v1/data/sync` | POST | 触发数据同步 (可选指定 symbol) |
| `/api/v1/data/stats` | GET | 获取存储统计信息 |
| `/api/v1/data/symbols` | GET | 获取追踪的股票列表 |
| `/api/v1/data/symbols/add` | POST | 添加追踪的股票 |

### 4. 新增类型

**文件**: `services/zero-trading/src/data/mod.rs`

- `SyncSummary`: 批量同步结果摘要
- `StorageStats`: 存储统计信息

## 验证结果

```bash
# 单个股票同步
curl -X POST http://127.0.0.1:4434/api/v1/data/sync \
  -H "Content-Type: application/json" \
  -d '{"symbol": "600000.SH"}'

# 返回结果
{
  "total_symbols": 1,
  "successful": 1,
  "failed": 0,
  "total_candles": 921,  # 成功！
  "errors": []
}

# 存储统计
curl http://127.0.0.1:4434/api/v1/data/stats

# 返回结果
{
  "candle_count": 921,
  "financial_count": 0,
  "valuation_count": 0,
  "unique_symbols": 1,
  "db_size_mb": 0.39453125
}
```

## API 限流说明

### iTick API
- 免费层: 5 requests/second = 300 requests/minute
- 认证方式: `token: <api_key>` header
- 限流响应: `{"code":429,"msg":"request limit exceeded"}`

### Lixin API
- 需要等待 Retry-After 头指定的时间
- 限流响应会在响应体中返回

## 数据源支持情况

### iTick API 支持的代码
- **普通股票**: ✅ 支持 (如 600000.SH, 000001.SZ)
- **指数**: ❌ 不支持或代码格式不同
- **ETF**: ❌ 不支持或代码格式不同

### 默认 tracked_symbols 问题

默认追踪的符号主要是指数和 ETF：
```
000001.SH  # 上证指数 - 无数据
000300.SH  # 沪深300 - 无数据
000905.SH  # 中证500 - 无数据
512880.SH  # 券商ETF - 无数据
...
```

这些在 iTick API 中没有数据。需要改为使用普通股票代码。

## 后续建议

1. **调整默认追踪符号**: 使用有数据的普通股票替代指数
2. **增加指数数据源**: 考虑添加支持指数的数据源
3. **优化限流处理**: 实现更智能的退避策略
4. **监控 API 配额**: 添加配额监控和告警

---

## 2026-02-27 更新：修复 Lixin API 404 错误

### 问题描述

日志中显示 Lixin API 持续返回 404 错误：
```
error=Internal error: HTTP 404 Not Found: {"code":0,"error":{"name":"NotFoundError","message":"Api was not found."}}
```

### 根因分析

理杏仁 API 端点已更新，旧端点已失效：

| 类型 | 旧端点 (错误) | 新端点 (正确) |
|------|--------------|--------------|
| 股票日K线 | `/a/stock/fs/daily-candlestick` | `/cn/company/candlestick` |
| 指数日K线 | `/a/index/fs/daily-candlestick` | `/cn/index/candlestick` |

此外，请求参数格式也发生变化：
- `stockCodes` (数组) → `stockCode` (单个字符串)
- 新增必填参数 `type`（复权类型）
- 日期格式变为 ISO 8601（带时区）

### 修复内容

**文件**: `services/zero-trading/src/data/lixin.rs`

1. **更新端点路径**:
   ```rust
   const STOCK_DAILY_ENDPOINT: &str = "/cn/company/candlestick";
   const INDEX_DAILY_ENDPOINT: &str = "/cn/index/candlestick";
   ```

2. **新增 `LixinCandlestickRequest` 结构**:
   ```rust
   struct LixinCandlestickRequest {
       token: String,
       #[serde(rename = "type")]
       candlestick_type: String,  // "lxr_fc_rights" 或 "normal"
       #[serde(rename = "stockCode")]
       stock_code: String,        // 单个代码，非数组
       #[serde(rename = "startDate")]
       start_date: String,        // 必填
       end_date: Option<String>,
       limit: Option<i32>,
   }
   ```

3. **更新日期解析**:
   - 支持 ISO 8601 格式: `"2018-01-19T00:00:00+08:00"`
   - 向后兼容旧格式: `"2018-01-19"`

4. **复权类型设置**:
   - 股票: `"lxr_fc_rights"` (理杏仁前复权，推荐)
   - 指数: `"normal"` (普通指数)

### API 文档参考

- 股票K线: https://www.lixinger.com/open/api/doc?api-key=cn/company/candlestick
- 指数K线: https://www.lixinger.com/open/api/doc?api-key=cn/index/candlestick

### 验证结果

```bash
cargo build -p zero-trading  # 编译成功
cargo test -p zero-trading --lib -- lixin  # 4 passed, 6 ignored
```

---

## 2026-02-27 更新：完善 Lixin API 接口修复

### 问题描述

继上次 404 修复后，进一步验证发现更多问题：

1. **Stock List 端点解析失败**: `LixinStockItem` 结构体字段名不匹配
2. **Fundamental Data 端点返回 400**: metric 名称使用 camelCase 但 API 要求 snake_case
3. **日期参数问题**: API 要求 `date` 或 `startDate`，但代码在部分情况下未提供

### 修复内容

**文件**: `services/zero-trading/src/data/lixin.rs`

#### 1. 修复 `fetch_stock_list` 方法

- 移除不存在的 `delisted_date` 字段引用
- 使用 `listing_status` 字段判断退市状态
- 优先使用 API 返回的 `exchange` 字段，保留代码前缀检测作为 fallback

```rust
// Before (错误)
let is_delisted = item.delisted_date.is_some();

// After (正确)
let is_delisted = item.listing_status.as_deref() == Some("delisted");
```

#### 2. 修复 ValuationMetricName API 名称

从 camelCase 改为 snake_case：

| Rust Enum | 旧 API 名 (错误) | 新 API 名 (正确) |
|-----------|-----------------|-----------------|
| PeTtm | `peTtm` | `pe_ttm` |
| DPeTtm | `dPeTtm` | `d_pe_ttm` |
| PsTtm | `psTtm` | `ps_ttm` |
| HaSh | `haSh` | `ha_sh` |
| HaShm | `haShm` | `ha_shm` |

#### 3. 修复 LixinNonFinancialData 反序列化

更新 serde rename 以匹配 API 返回的 snake_case 字段名：

```rust
// Before
#[serde(rename = "peTtm", default)]
pe_ttm: Option<f64>,

// After
#[serde(rename = "pe_ttm", default)]
pe_ttm: Option<f64>,
```

#### 4. 修复日期参数逻辑

Non-financial API 需要 `date` 或 `startDate` 参数，但当天数据可能不可用。

更新逻辑：
- 如果指定了 `date` 参数，直接使用
- 如果未指定，查询最近 7 天范围以获取最新可用数据

```rust
// Query last 7 days to get most recent available data
let start = today - chrono::Duration::days(7);
LixinNonFinancialRequest {
    date: None,
    start_date: Some(start.format("%Y-%m-%d").to_string()),
    end_date: Some(today.format("%Y-%m-%d").to_string()),
    limit: Some(1),
}
```

### 验证结果

```
╔════════════════════════════════════════════════════════════════╗
║           Lixin API Endpoint Verification Report               ║
╚════════════════════════════════════════════════════════════════╝

[1/5] Health Check... ✅ PASS
[2/5] Stock Candlestick (/cn/company/candlestick)... ✅ PASS (5 candles)
[3/5] Index Candlestick (/cn/index/candlestick)... ✅ PASS (242 candles)
[4/5] Stock List (/cn/company)... ✅ PASS (5582 stocks)
[5/5] Fundamental Data (/cn/company/fundamental/non_financial)... ✅ PASS (PE:20.39 PB:8.09)

Summary:
  Passed: 5/5 ✅
```

### API 要点总结

| 端点 | 方法 | 必填参数 | 数据格式 |
|------|------|---------|---------|
| `/cn/company/candlestick` | POST | token, stockCode, type, startDate, endDate | ISO 8601 日期 |
| `/cn/index/candlestick` | POST | token, stockCode, type, startDate, endDate | ISO 8601 日期 |
| `/cn/company` | POST | token | snake_case 字段 |
| `/cn/company/fundamental/non_financial` | POST | token, stockCodes, date 或 startDate, metricsList | snake_case metric 名 |

## 相关文件

- `services/zero-trading/src/data/aggregator.rs`
- `services/zero-trading/src/data/mod.rs`
- `services/zero-trading/src/routes.rs`
- `services/zero-trading/src/lib.rs`
