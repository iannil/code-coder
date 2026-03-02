# Zero-Trading 完整工作流程文档

## Context

`zero-trading` 是一个 A股自动化交易系统，基于 PO3+SMT 策略，遵循 T+1 交易规则。

本文档描述从非交易时段到交易时段的完整工作流程，包括：选股 → 分析 → K线数据同步 → 宏观经济数据同步 → 评估交易信号 → IM推送消息。

---

## 完整工作流程图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            非交易时段 (15:00 - 09:25)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │ K线数据同步  │ ─→ │ 宏观数据同步  │ ─→ │ 选股筛选     │                  │
│  │ data/sync.rs │    │ data/sync.rs │    │ screener/*   │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│          ↓                   ↓                   ↓                         │
│  ┌──────────────────────────────────────────────────────────────┐          │
│  │              技术指标预计算 (MA20, MA50, ATR, SMT背离)         │          │
│  │                  loop/signal_detector.rs                     │          │
│  └──────────────────────────────────────────────────────────────┘          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│                       交易时段开始 (09:25 SessionStart)                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  scheduler.rs (cron: 0 25 9 * * 1-5)                                       │
│      ↓                                                                      │
│  TradingSessionManager::start_session()                                    │
│      ↓                                                                      │
│  SessionState: Idle → Starting → Running                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│                      交易循环 (09:30-11:30, 13:00-15:00)                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TradingLoop (loop/trading_loop.rs)                                        │
│  ├─ 价格检查 (1秒间隔) ─→ 止损/止盈触发                                      │
│  └─ 信号扫描 (5秒间隔) ─→ 新入场信号                                         │
│                              ↓                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                     信号检测流程                                  │       │
│  │  SignalDetector.scan()                                          │       │
│  │      ↓                                                          │       │
│  │  StrategyEngine.scan_for_signals()                              │       │
│  │      ├─ PO3Detector.detect() (多时间框架)                        │       │
│  │      └─ SmtDetector.detect_divergence()                         │       │
│  │      ↓                                                          │       │
│  │  信号过滤: min_strength, long_only(T+1), dedup_window           │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                              ↓                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                   宏观环境评估 (Hybrid Decision)                  │       │
│  │  MacroOrchestrator.evaluate()                                   │       │
│  │      ├─ 规则引擎评估 (快速)                                       │       │
│  │      └─ 触发条件检测 ─→ LLM Agent分析 (如需要)                    │       │
│  │          • ExtremeRiskAppetite (risk < 30 或 > 70)              │       │
│  │          • AvoidTradingSignal                                   │       │
│  │          • ExtremePmi (PMI < 48 或 > 54)                        │       │
│  │          • IndicatorDivergence (PMI vs M2 冲突)                 │       │
│  │      ↓                                                          │       │
│  │  决策合并: 70/30 权重 (LLM 优先)                                 │       │
│  │  保守偏向: 选择更保守的 TradingBias                              │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                              ↓                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                      订单执行                                    │       │
│  │  ExecutionEngine.execute_buy(signal)                            │       │
│  │      ├─ 仓位计算: (2% capital) / (entry - stop_loss)            │       │
│  │      └─ 数量取整: 100股整数倍 (A股要求)                           │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                              ↓                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │                      IM 推送通知                                 │       │
│  │  NotificationClient.notify_signal()                             │       │
│  │      ├─ 格式化: symbol, direction, entry/stop/target            │       │
│  │      ├─ 包含宏观上下文 (可选)                                     │       │
│  │      └─ 重试队列: 10次重试, 指数退避 (~1小时)                     │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│                         午休暂停 (11:30-13:00)                               │
├─────────────────────────────────────────────────────────────────────────────┤
│  scheduler.rs (cron: 0 30 11 * * 1-5) → SessionPause                       │
│  TradingLoop.state → Paused (价格监控继续, 信号扫描暂停)                      │
│  scheduler.rs (cron: 0 0 13 * * 1-5)  → SessionResume                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│                       交易时段结束 (15:00 SessionStop)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  scheduler.rs (cron: 0 0 15 * * 1-5)  → SessionStop                        │
│  TradingLoop.state → Stopped                                               │
│  会话状态持久化到 SQLite                                                     │
│                                                                             │
│  scheduler.rs (cron: 0 30 15 * * 1-5) → DailyReview                        │
│  计算: 胜率, 总盈亏, 交易统计                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. 非交易时段任务

### 1.1 K线数据同步 (`data/sync.rs`)

```rust
pub struct DataSynchronizer {
    config: SyncConfig,
    provider: Arc<dyn DataProvider>,
    storage: Arc<LocalStorage>,
}

impl DataSynchronizer {
    /// 增量同步K线数据
    pub async fn sync_candles(&self, symbols: &[String]) -> Result<SyncResult>;

    /// 同步宏观经济指标 (PMI, M2, 社融)
    pub async fn sync_macro_indicators(&self) -> Result<()>;
}
```

**同步时机**:
- 每日收盘后自动触发
- API 手动触发 (`POST /api/v1/data/sync`)

**数据来源**:
- iTick API (K线数据)
- 新浪财经 (宏观指标)
- 本地缓存 (故障转移)

### 1.2 选股筛选 (`screener/scheduler.rs`)

```rust
pub struct ScreenerScheduler {
    config: ScreenerConfig,
    engine: ScreenerEngine,
}

impl ScreenerScheduler {
    /// 全量扫描 (4000+ 股票)
    pub async fn trigger_full_scan(&self) -> Result<ScreenerResult>;

    /// 快速扫描 (自选股 + 关注列表)
    pub async fn trigger_quick_scan(&self) -> Result<ScreenerResult>;
}
```

**筛选条件** (`screener/config.rs`):
- 市值范围
- PE/PB 估值
- ROE 盈利能力
- 流动性要求
- 行业分类

**输出**:
- 本地报告 (Markdown/JSON)
- 候选股票列表

### 1.3 技术指标预计算 (`loop/signal_detector.rs`)

```rust
impl SignalDetector {
    /// 预加载跟踪标的数据
    pub async fn preload_data(&self, data: &Arc<MarketDataAggregator>) -> Result<()>;

    /// 预计算技术参数 (MA20, MA50, ATR)
    pub async fn precompute_parameters(&self, data: &Arc<MarketDataAggregator>) -> Result<()>;
}
```

**预计算内容**:
- MA20/MA50 移动平均线
- ATR (14日平均真实波幅)
- SMT 配对数据预加载

---

## 2. 交易时段调度 (`scheduler.rs`)

### 2.1 Cron 调度配置

```json
{
  "trading": {
    "schedule": {
      "enabled": true,
      "session_start": "0 25 9 * * 1-5",
      "session_pause": "0 30 11 * * 1-5",
      "session_resume": "0 0 13 * * 1-5",
      "session_stop": "0 0 15 * * 1-5",
      "daily_review": "0 30 15 * * 1-5"
    }
  }
}
```

### 2.2 调度器状态机

```
SchedulerState:
  Stopped ──start()─→ Running ──pause()─→ Paused
                         ↑                   │
                         └───resume()────────┘
```

### 2.3 任务执行 (带重试)

```rust
impl TradingScheduler {
    async fn execute_task(&self, task: ScheduledTask) {
        // 最多重试 3 次
        for attempt in 1..=self.max_retries {
            match self.handle_task(task).await {
                Ok(()) => return,
                Err(e) => {
                    // 指数退避: 1s, 2s, 4s
                    let backoff_ms = 1000 * (1 << (attempt - 1));
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
        // 连续失败超过阈值 (5次) 触发告警
        self.track_failure(task).await;
    }
}
```

---

## 3. 会话管理 (`session/manager.rs`)

### 3.1 会话状态机

```
SessionState:
  Idle ──start()─→ Starting ──init_complete()─→ Running
                                                   │
                       ┌───────────────────────────┤
                       ↓                           ↓
                   Paused ←──pause()──         Stopped
                       │                           ↑
                       └──resume()─→ Running ──stop()
                                                   │
                                                   ↓
                                                Failed
```

### 3.2 会话配置

```rust
pub struct SessionConfig {
    pub mode: TradingMode,       // Paper / Live
    pub auto_start: bool,        // 自动启动交易循环
    pub name: Option<String>,    // 会话名称
    pub watchlist: Vec<String>,  // 关注列表
}
```

### 3.3 崩溃恢复

- 会话状态持久化到 SQLite
- 重启后自动恢复 `Running` 状态的会话
- 恢复监控的持仓和未执行订单

---

## 4. 交易循环 (`loop/trading_loop.rs`)

### 4.1 双循环架构

```rust
pub struct TradingLoop {
    config: LoopConfig,
    data: Arc<MarketDataAggregator>,
    strategy: Arc<StrategyEngine>,
    execution: Arc<RwLock<ExecutionEngine>>,
    price_monitor: PriceMonitor,
    signal_detector: SignalDetector,
}

impl TradingLoop {
    pub async fn run(&self) -> Result<()> {
        let mut price_interval = interval(Duration::from_secs(1));   // 价格检查
        let mut signal_interval = interval(Duration::from_secs(5));  // 信号扫描

        loop {
            tokio::select! {
                _ = price_interval.tick() => {
                    self.check_prices().await?;  // 止损/止盈
                }
                _ = signal_interval.tick() => {
                    self.scan_signals().await?;  // 新信号
                }
            }
        }
    }
}
```

### 4.2 价格监控

```rust
async fn check_prices(&self) -> Result<()> {
    let positions = self.positions.read().await;

    for position in positions.iter() {
        let current_price = self.price_monitor.get_price(&position.symbol).await?;

        // 止损检查
        if current_price <= position.stop_loss {
            self.event_tx.send(LoopEvent::StopLossTriggered {...});
            if self.config.auto_execute {
                self.execute_close(&position.id, "stop_loss").await?;
            }
        }

        // 止盈检查
        if current_price >= position.take_profit {
            self.event_tx.send(LoopEvent::TakeProfitTriggered {...});
            if self.config.auto_execute {
                self.execute_close(&position.id, "take_profit").await?;
            }
        }
    }
}
```

---

## 5. 信号检测 (`loop/signal_detector.rs`)

### 5.1 过滤器配置

```rust
pub struct SignalFilter {
    pub min_strength: SignalStrength,    // Medium (默认)
    pub long_only: bool,                 // true (T+1 限制)
    pub max_signals_per_symbol: usize,   // 1
    pub dedup_window_secs: u64,          // 300 (5分钟)
    pub include_symbols: Vec<String>,
    pub exclude_symbols: Vec<String>,
}
```

### 5.2 信号检测流程

```
1. StrategyEngine.scan_for_signals()
   ├─ 遍历所有 SMT 配对
   └─ 对每个配对:
       ├─ 获取多时间框架 K线数据
       ├─ PO3Detector.detect() → Po3Structure
       └─ SmtDetector.detect_divergence() → SmtDivergence

2. evaluate_signals()
   ├─ 多时间框架对齐检查 (require_alignment)
   ├─ SMT 背离方向与 PO3 方向一致性检查
   └─ calculate_strength() → SignalStrength (Weak/Medium/Strong/VeryStrong)

3. SignalDetector 过滤
   ├─ strength >= min_strength
   ├─ direction == Long (long_only)
   ├─ !is_duplicate (dedup_window)
   └─ !exceeds_daily_limit
```

---

## 6. 策略引擎 (`strategy/mod.rs`)

### 6.1 PO3 (Power of 3) 检测

```rust
pub struct Po3Detector {
    min_accumulation_bars: usize,  // 5
    manipulation_threshold: f64,   // 1.5 ATR
}

pub struct Po3Structure {
    pub current_phase: Po3Phase,     // Accumulation/Manipulation/Distribution
    pub direction: SignalDirection,  // Long/Short
    pub accumulation_high: f64,
    pub accumulation_low: f64,
    pub manipulation_clear: bool,
    pub distribution_started: bool,
    pub ideal_entry: f64,
    pub stop_loss: f64,
    pub midpoint: f64,               // 50% 回撤目标
}
```

### 6.2 SMT 背离检测

```rust
pub struct SmtDetector;

pub struct SmtDivergence {
    pub divergence_type: DivergenceType,  // Bullish/Bearish
    pub primary_symbol: String,
    pub reference_symbol: String,
    pub strength: f64,  // 背离强度
}
```

**SMT 配对示例**:
- 上证50 vs 沪深300
- 中证500 vs 创业板指

### 6.3 信号强度计算

```rust
fn calculate_strength(po3_signals: &[(Timeframe, Po3Structure)], smt: &Option<SmtDivergence>) -> SignalStrength {
    let mut score = 0;

    score += po3_signals.len().min(3);  // 多时间框架 +1~3
    if smt.is_some() { score += 2; }     // SMT 背离 +2
    if po3.manipulation_clear { score += 1; }
    if po3.distribution_started { score += 1; }

    match score {
        0..=2 => Weak,
        3..=4 => Medium,
        5..=6 => Strong,
        _ => VeryStrong,
    }
}
```

---

## 7. 宏观环境评估 (`macro_agent/orchestrator.rs`)

### 7.1 混合决策架构

```
┌─────────────────────────────────────────────────────────────┐
│                     MacroOrchestrator                        │
│                                                             │
│  1. 规则引擎评估 (MacroFilter) ──────────────────────────┐   │
│     └─ 快速判断: 经济周期、PMI、M2、社融                 ↓   │
│                                                             │
│  2. 触发条件检测 ───────────────────────────────────────→   │
│     • ExtremeRiskAppetite (risk < 30 或 > 70)              │
│     • AvoidTradingSignal                                   │
│     • SignificantPositionReduction (< 50%)                 │
│     • ExtremePmi (< 48 或 > 54)                            │
│     • IndicatorDivergence (PMI vs M2)                      │
│                          ↓                                  │
│  3. 如果有触发 → AgentBridge 调用 LLM 分析                 │
│                          ↓                                  │
│  4. 决策合并                                                │
│     • 仓位建议: 规则 30% + Agent 70%                       │
│     • 交易偏向: 选择更保守的方向                           │
│     • 缓存: 3600秒有效期                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 交易偏向保守选择

```rust
fn more_conservative_bias(a: TradingBias, b: TradingBias) -> TradingBias {
    // 保守程度排序: AvoidTrading > Bearish > Neutral > Bullish
    let score = |bias| match bias {
        AvoidTrading => 0,
        Bearish => 1,
        Neutral => 2,
        Bullish => 3,
    };

    if score(a) <= score(b) { a } else { b }
}
```

---

## 8. IM 推送通知 (`notification.rs`)

### 8.1 信号通知格式

```rust
pub struct NotificationClient {
    telegram_bot_token: String,
    telegram_chat_id: String,
    retry_queue: Arc<RwLock<VecDeque<PendingNotification>>>,
}

impl NotificationClient {
    pub async fn notify_signal(&self, signal: &TradingSignal, macro_context: Option<&MacroDecision>) {
        let message = format!(
            "📊 *{}* {}\n\n\
             方向: {}\n\
             入场价: {:.2}\n\
             止损: {:.2}\n\
             目标: {:.2}\n\
             强度: {:?}\n\
             {}\
             时间: {}",
            signal.symbol,
            if signal.direction == Long { "🟢 做多" } else { "🔴 做空" },
            signal.direction,
            signal.entry_price,
            signal.stop_loss,
            signal.take_profit,
            signal.strength,
            macro_context.map(|m| format!("宏观: {}\n", m.summary)).unwrap_or_default(),
            signal.timestamp.format("%Y-%m-%d %H:%M:%S")
        );

        self.send_telegram(&message).await;
    }
}
```

### 8.2 重试队列

```rust
pub struct PendingNotification {
    message: String,
    attempts: u32,
    next_retry: DateTime<Utc>,
}

impl NotificationClient {
    async fn process_retry_queue(&self) {
        loop {
            let pending = self.retry_queue.write().await.pop_front();
            if let Some(mut notif) = pending {
                if notif.attempts < 10 {
                    match self.send_telegram(&notif.message).await {
                        Ok(_) => continue,
                        Err(_) => {
                            notif.attempts += 1;
                            // 指数退避: 1m, 2m, 4m, 8m, 16m, 32m (~1小时内)
                            notif.next_retry = Utc::now() + Duration::minutes(1 << notif.attempts);
                            self.retry_queue.write().await.push_back(notif);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
```

---

## 9. T+1 规则执行 (`execution/t1_risk.rs`)

### 9.1 当日卖出限制

```rust
impl Position {
    pub fn can_sell_today(&self, today: NaiveDate) -> bool {
        // 关键: 使用 > 而非 >=
        today > self.entry_date
    }
}
```

### 9.2 次日决策

```rust
impl T1RiskManager {
    pub async fn next_day_decision(&self, positions: &[Position]) -> Vec<T1Action> {
        let today = Local::now().date_naive();
        let mut actions = Vec::new();

        for position in positions {
            if position.can_sell_today(today) {
                // 检查是否触发止损/止盈
                if position.should_close() {
                    actions.push(T1Action::Close(position.id.clone()));
                }
            }
        }

        actions
    }
}
```

---

## 10. 关键文件路径

| 模块 | 文件路径 | 功能 |
|------|---------|------|
| 调度器 | `src/scheduler.rs` | Cron 调度会话生命周期 |
| 会话管理 | `src/session/manager.rs` | 会话状态机、持久化 |
| 交易循环 | `src/loop/trading_loop.rs` | 价格监控、信号扫描 |
| 信号检测 | `src/loop/signal_detector.rs` | 信号过滤、去重 |
| 策略引擎 | `src/strategy/mod.rs` | PO3+SMT 策略 |
| PO3 检测 | `src/strategy/po3.rs` | PO3 结构检测 |
| SMT 背离 | `src/strategy/smt.rs` | SMT 背离检测 |
| 宏观决策 | `src/macro_agent/orchestrator.rs` | 混合决策 |
| 数据同步 | `src/data/sync.rs` | K线、宏观数据同步 |
| 选股筛选 | `src/screener/scheduler.rs` | 定时选股 |
| IM 通知 | `src/notification.rs` | Telegram 推送 |
| T+1 风控 | `src/execution/t1_risk.rs` | T+1 规则 |

---

## 11. 验证方法

```bash
# 运行所有测试
cd services/zero-trading
cargo test

# 特定模块测试
cargo test scheduler:: --lib
cargo test session:: --lib
cargo test strategy:: --lib
cargo test macro_agent:: --lib

# 集成测试
cargo test --test e2e_signal_flow
cargo test --test macro_agent_integration
```

---

## 12. 实现状态

| 模块 | 状态 | 测试覆盖 |
|------|------|---------|
| 调度器 (scheduler.rs) | ✅ 完成 | 12 tests |
| 会话管理 (session/) | ✅ 完成 | 16 tests |
| 交易循环 (loop/) | ✅ 完成 | 多项测试 |
| 策略引擎 (strategy/) | ✅ 完成 | 35+ tests |
| 宏观决策 (macro_agent/) | ✅ 完成 | 11 tests |
| 数据同步 (data/sync.rs) | ✅ 完成 | 单元测试 |
| 选股筛选 (screener/) | ✅ 完成 | 集成测试 |
| IM 通知 (notification.rs) | ✅ 完成 | 20+ tests |
| T+1 风控 (execution/) | ✅ 完成 | 7 tests |

**总测试数**: 416 单元测试 + 99 集成测试 = 515 tests
**测试状态**: 全部通过
