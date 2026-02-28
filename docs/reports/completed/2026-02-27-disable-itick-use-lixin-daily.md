# 禁用 iTick 并使用 Lixin 日线数据

**日期**: 2026-02-27

## 背景

用户需要暂时禁用 iTick 数据源。Lixin 只支持日线（Daily）及以上的数据，不支持分钟级数据（H1/H4）。需要确保在 Lixin 支持的范围内正常完成业务目标。

### 数据源能力对比

| 功能 | iTick | Lixin |
|------|-------|-------|
| 分钟级数据 (M1/M5/M15/M30) | Yes | No |
| 小时级数据 (H1/H4) | Yes | No |
| 日线数据 (Daily) | Yes | Yes |
| 周线数据 (Weekly) | Yes | Yes |
| 基本面数据 | No | Yes |
| 估值指标 | No | Yes |

## 修改内容

### Phase 1: 配置更改

**文件**: `~/.codecoder/config.json`

1. 禁用 iTick 数据源：
```json
{
  "trading": {
    "data_sources": {
      "sources": [
        {
          "provider": "itick",
          "enabled": false,  // 从 true 改为 false
          "priority": 1
        },
        {
          "provider": "lixin",
          "enabled": true,
          "priority": 2
        }
      ]
    }
  }
}
```

2. 更新时间框架配置为仅日线：
```json
{
  "trading": {
    "timeframes": ["D"]  // 从 ["D", "H4", "H1"] 改为 ["D"]
  }
}
```

### Phase 2: 策略层降级

**文件**: `services/zero-trading/src/strategy/mod.rs`

1. `StrategyConfig::default()` 默认时间框架从 `[Daily, H4, H1]` 改为 `[Daily]`
2. `from_config()` 回退时间框架从 `[Daily, H4, H1]` 改为 `[Daily]`
3. 更新测试用例以匹配新的默认值

### Phase 3: 任务调度器优化

**文件**: `services/zero-trading/src/task_scheduler/mod.rs`

`preload_data()` 方法简化为仅加载 Daily 时间框架：
```rust
// 修改前
for tf in [Timeframe::Daily, Timeframe::H4, Timeframe::H1] {
    // ...
}

// 修改后
let tf = Timeframe::Daily;
// ...
```

### Phase 4: 数据聚合器优化

**文件**: `services/zero-trading/src/data/aggregator.rs`

1. `preload_historical_data()`:
   - 移除交易时段 H1/H4 加载逻辑
   - 简化为仅加载 Daily 时间框架

2. `start_updater()`:
   - 移除 H1/H4 提供者能力检查逻辑
   - 简化为仅更新 Daily 时间框架
   - SMT 配对数据也仅使用 Daily

3. `sync_symbol_data()`:
   - 移除 H4/H1 同步尝试
   - 添加注释说明原因

## 受影响的功能

| 功能 | 影响 | 降级方案 |
|------|------|----------|
| 多时间框架分析 | 默认 `[D, H4, H1]` 降为 `[D]` | 仅使用日线分析 |
| SMT 背离检测 | H4 时间框架不可用 | 使用日线检测 |
| 数据预加载 | 减少 API 调用 | 仅预加载日线 |
| 数据同步 | 减少同步数据量 | 仅同步日线 |

## 验证方法

```bash
# 1. 启动服务
./ops.sh start

# 2. 检查日志，确认 iTick 未注册
./ops.sh logs zero-trading | grep -i "itick\|lixin\|provider"

# 3. 触发数据同步
curl -X POST http://127.0.0.1:4434/api/v1/data/sync \
  -H "Content-Type: application/json" \
  -d '{"symbol": "600000.SH"}'

# 4. 验证存储统计
curl http://127.0.0.1:4434/api/v1/data/stats

# 5. 验证基本面数据获取
curl http://127.0.0.1:4434/api/v1/valuation/000001.SZ
```

## 回滚方案

如需重新启用 iTick 和多时间框架：

1. 配置文件：
   - `trading.data_sources.sources[0].enabled` 改为 `true`
   - `trading.timeframes` 改为 `["D", "H4", "H1"]`

2. 代码回滚（如需要）：
   - `git checkout HEAD~1 -- services/zero-trading/src/strategy/mod.rs`
   - `git checkout HEAD~1 -- services/zero-trading/src/task_scheduler/mod.rs`
   - `git checkout HEAD~1 -- services/zero-trading/src/data/aggregator.rs`

3. 重启服务：
   - `./ops.sh stop && ./ops.sh start`

## 相关文件

- `~/.codecoder/config.json`
- `services/zero-trading/src/strategy/mod.rs`
- `services/zero-trading/src/task_scheduler/mod.rs`
- `services/zero-trading/src/data/aggregator.rs`
