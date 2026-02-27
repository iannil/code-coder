# Screener 自动同步功能实现

## 完成时间

2026-02-27 15:00

## 问题背景

用户启动 zero-trading 服务后发现部分数据库表为空（financial_statements, valuations 等）。

### 根因分析

`ScreenerConfig` 中定义了 cron 配置字段：
- `schedule_cron: "0 18 * * 1-5"` (工作日18:00扫描)
- `data_sync_cron: "0 20 * * 0"` (周日20:00同步)

但在 `TradingService::start()` 中**未启动 screener 调度循环**，只创建了实例却没有后台任务监听 cron 表达式。

## 解决方案

### 修改内容

1. **`services/zero-trading/src/screener/scheduler.rs`**
   - 添加 `config()` getter 方法，暴露调度器配置

2. **`services/zero-trading/src/lib.rs`**
   - 在 `TradingService::start()` 中添加 screener 调度器启动逻辑
   - 新增 `run_screener_scheduler()` 函数，使用 `cron` crate 解析表达式
   - 新增 `should_run_now()` 辅助函数判断当前时间是否匹配调度

### 技术亮点

1. **使用 cron crate 进行标准解析**：不是硬编码时间匹配，而是使用项目已有的 `cron` 依赖进行标准 cron 表达式解析，支持用户自定义调度

2. **防重复执行机制**：使用 `(hour, minute, day)` 元组跟踪上次执行时间，避免同一分钟内重复触发

3. **30秒检查间隔**：使用 30 秒而非 60 秒检查间隔，减少因检查时机导致错过分钟边界的概率

### 代码示例

```rust
// 启动 screener 调度器
if let Some(screener) = &self.state.screener_scheduler {
    if screener.config().enabled {
        let screener_clone = Arc::clone(screener);
        tokio::spawn(async move {
            run_screener_scheduler(screener_clone).await;
        });
        tracing::info!(
            scan_cron = %screener.config().schedule_cron,
            sync_cron = %screener.config().data_sync_cron,
            "Screener scheduler started"
        );
    }
}
```

## 配置说明

默认配置已足够使用，无需额外修改：

```json
{
  "trading": {
    "screener": {
      "enabled": true,
      "schedule_cron": "0 18 * * 1-5",  // 工作日18:00执行快速扫描
      "data_sync_cron": "0 20 * * 0"    // 周日20:00同步财务数据
    }
  }
}
```

## 验证步骤

1. **重启服务**：
```bash
./ops.sh stop
./ops.sh start
```

2. **确认调度器启动**：
```bash
./ops.sh logs zero-trading | grep -i "screener scheduler"
# 预期输出: "Screener scheduler started" + cron 配置信息
```

3. **手动触发测试**（无需等到周日）：
```bash
curl -X POST http://localhost:4434/api/v1/screener/sync
```

4. **验证数据填充**：
```bash
sqlite3 ~/.codecoder/financial.db "SELECT COUNT(*) FROM financial_statements;"
# 预期: > 0
```

## 文件变更

| 文件 | 变更类型 | 内容 |
|------|----------|------|
| `services/zero-trading/src/screener/scheduler.rs` | 修改 | 添加 `config()` getter |
| `services/zero-trading/src/lib.rs` | 修改 | 添加调度器启动 + `run_screener_scheduler` 函数 |

## 状态

✅ 已完成
