# Zero-Trading 功能逻辑执行结果评估报告

**日期**: 2026-03-02
**评估人**: Claude Code
**评估范围**: services/zero-trading 全部功能模块

---

## 执行摘要

| 评估维度 | 评分 | 状态 |
|---------|------|------|
| T+1 合规性 | ⭐⭐⭐⭐⭐ | ✅ 完全正确 |
| 信号生成逻辑 | ⭐⭐⭐⭐ | ✅ 逻辑完整 |
| 风险管理 | ⭐⭐⭐⭐⭐ | ✅ 可配置化 |
| 混合决策 | ⭐⭐⭐⭐ | ✅ 架构合理 |
| API 实现 | ⭐⭐⭐ | ⚠️ 测试不足 |
| 会话管理 | ⭐⭐⭐⭐ | ✅ 持久化完整 |

**总体评价**: 核心业务逻辑实现正确，T+1 合规性严格，风险管理可配置。API 层测试覆盖不足是主要风险点。

---

## 1. T+1 合规性评估

### 1.1 核心实现 (`execution/position.rs`)

```rust
pub fn can_sell_today(&self) -> bool {
    let today = Local::now().date_naive();
    today > self.entry_date  // 关键：使用 > 而非 >=
}
```

**评估结果**: ✅ **完全正确**

| 验证项 | 预期行为 | 实际行为 | 结论 |
|-------|---------|---------|------|
| 当日买入 | 禁止卖出 | `today > entry_date` 为 false | ✅ 正确 |
| 次日卖出 | 允许卖出 | `today > entry_date` 为 true | ✅ 正确 |
| 边界条件 | 23:59买入,00:01卖出 | 本地日期比较，正确处理 | ✅ 正确 |

### 1.2 执行引擎强制检查 (`execution/mod.rs:292-297`)

```rust
pub async fn execute_sell(&mut self, symbol: &str, reason: &str) -> Result<Order> {
    let position = self.positions.get(symbol)...;

    if !position.can_sell_today() {
        anyhow::bail!("Cannot sell {} today (T+1 rule)", symbol);
    }
    // ... 继续执行
}
```

**评估结果**: ✅ 双重保护机制生效

---

## 2. 策略信号生成评估

### 2.1 PO3 + SMT 策略流程

```
┌─────────────────────────────────────────────────────────────┐
│                    Strategy Engine                           │
│                                                             │
│  scan_for_signals()                                         │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  SMT Pair   │───▶│  PO3 Check  │───▶│  SMT Check  │     │
│  │  Iteration  │    │  (多时间框架)│    │  (日线/4H)  │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│       │                   │                   │             │
│       ▼                   ▼                   ▼             │
│  ┌─────────────────────────────────────────────────┐       │
│  │           evaluate_signals()                     │       │
│  │  - 多时间框架对齐检查                            │       │
│  │  - PO3 与 SMT 方向一致性                         │       │
│  │  - 信号强度计算                                  │       │
│  └─────────────────────────────────────────────────┘       │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────┐                                           │
│  │  Trading    │  entry, stop_loss, take_profit            │
│  │  Signal     │  direction, strength, notes               │
│  └─────────────┘                                           │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 关键逻辑验证

| 功能 | 代码位置 | 执行结果 |
|-----|---------|---------|
| 多时间框架扫描 | `strategy/mod.rs:152-167` | ✅ 遍历所有配置的 timeframes |
| PO3 结构检测 | `po3.detect(&primary)` | ✅ 检测 Accumulation→Manipulation→Distribution |
| SMT 背离检测 | `smt.detect_divergence()` | ✅ 仅在 Daily/H4 执行 |
| 方向一致性 | `strategy/mod.rs:197-207` | ✅ SMT 必须与 PO3 方向一致 |
| 信号去重 | `strategy/mod.rs:130-133` | ✅ 同 symbol+direction 不重复添加 |

### 2.3 信号强度计算 (`strategy/mod.rs:242-272`)

```rust
fn calculate_strength(...) -> SignalStrength {
    let mut score = 0;
    score += po3_signals.len().min(3);     // 多时间框架: 0-3分
    if smt_divergence.is_some() { score += 2; }  // SMT确认: +2分
    if po3.manipulation_clear { score += 1; }     // 清晰操纵: +1分
    if po3.distribution_started { score += 1; }   // 分布开始: +1分

    match score {
        0..=2 => Weak,
        3..=4 => Medium,
        5..=6 => Strong,
        _ => VeryStrong,
    }
}
```

**评估结果**: ✅ 逻辑合理，分数边界清晰

---

## 3. 风险管理评估

### 3.1 T1RiskConfig 可配置性 (`execution/t1_risk.rs`)

| 参数 | 默认值 | 可配置 | 用途 |
|-----|-------|--------|-----|
| `stop_loss_pct` | 5.0% | ✅ | 止损百分比 |
| `take_profit_pct` | 10.0% | ✅ | 止盈百分比 |
| `max_loss_per_trade_pct` | 2.0% | ✅ | 单笔最大损失 |
| `gap_down_tolerance` | 2.0% | ✅ | 跳空下跌容忍度 |
| `gap_up_threshold` | 3.0% | ✅ | 跳空上涨阈值 |
| `limit_pct` | 10.0% | ✅ | 涨跌停限制 |
| `typical_overnight_gap_pct` | 2.0% | ✅ | 典型隔夜缺口 |

**评估结果**: ✅ **所有硬编码已参数化**，支持科创板/创业板 20% 涨跌停

### 3.2 仓位计算逻辑 (`execution/t1_risk.rs:141-158`)

```rust
pub fn calculate_position_size(&self, total_capital: f64, entry_price: f64, stop_loss: f64) -> f64 {
    let risk_per_trade = total_capital * (self.config.max_loss_per_trade_pct / 100.0);
    let risk_per_share = (entry_price - stop_loss).abs();

    if risk_per_share <= 0.0 { return 0.0; }  // 防止除零

    let shares = risk_per_trade / risk_per_share;
    (shares / 100.0).floor() * 100.0  // 向下取整到100股
}
```

**验证示例**:
- 资本: 100,000
- 入场价: 10.0, 止损: 9.5 (风险 0.5/股)
- 单笔最大损失 2% = 2,000
- 计算: 2000 / 0.5 = 4000 股 ✅

**评估结果**: ✅ 正确实现 A股手数规则 (100股整数倍)

### 3.3 次日决策逻辑 (`execution/t1_risk.rs:94-136`)

```
                ┌─────────────────────────────────────┐
                │  auction.expected_price 与 entry    │
                │  计算 expected_return               │
                └────────────────┬────────────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
        ▼                        ▼                        ▼
   < -stop_loss_pct     < -gap_down_tolerance    > gap_up_threshold
        │                        │                        │
        ▼                        ▼                        ▼
   SellAtOpen              SellAtOpen           HoldWithBreakeven
        │                        │                        │
        └────────────────────────┼────────────────────────┘
                                 │
                    其他情况: WaitAndSee
                    接近目标: HoldToTarget
```

**评估结果**: ✅ 决策树完整，边界条件处理正确

---

## 4. 混合决策系统评估

### 4.1 MacroOrchestrator 架构 (`macro_agent/orchestrator.rs`)

```
┌─────────────────────────────────────────────────────────────┐
│                   MacroOrchestrator                          │
├─────────────────────────────────────────────────────────────┤
│  1. Rule Engine (MacroFilter)                                │
│     └─ 快速返回: PMI, M2, 社融等指标                         │
│                                                             │
│  2. Trigger Check                                           │
│     ├─ ExtremeRiskAppetite (< 30 或 > 70)                   │
│     ├─ AvoidTradingSignal                                   │
│     ├─ SignificantPositionReduction (< 0.5x)                │
│     ├─ ExtremePmi (< 48 或 > 54)                            │
│     └─ IndicatorDivergence (PMI vs M2 冲突)                 │
│                                                             │
│  3. Agent Analysis (LLM)                                     │
│     └─ 仅在触发条件满足时调用                                │
│                                                             │
│  4. Decision Merge                                           │
│     └─ 70% Agent + 30% Rule, 取更保守的 TradingBias        │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 保守性原则验证 (`orchestrator.rs:299-314`)

```rust
fn more_conservative_bias(&self, a: TradingBias, b: TradingBias) -> TradingBias {
    // 保守程度: AvoidTrading > Bearish > Neutral > Bullish
    let score = |bias| match bias {
        AvoidTrading => 0,  // 最保守
        Bearish => 1,
        Neutral => 2,
        Bullish => 3,       // 最激进
    };
    if score(a) <= score(b) { a } else { b }
}
```

**评估结果**: ✅ 始终选择更保守的决策，符合风控要求

### 4.3 缓存机制

- 缓存时长: 3600秒 (1小时)
- 缓存条件: Agent 分析成功且 confidence >= min_agent_confidence
- 绕过缓存: `force_analyze()` 方法

**评估结果**: ✅ 避免频繁调用 LLM，减少成本

---

## 5. API 实现评估

### 5.1 路由清单 (`routes.rs`)

| 端点 | 方法 | 功能 | 测试覆盖 |
|-----|------|------|---------|
| `/health` | GET | 健康检查 | ⚠️ 无测试 |
| `/api/v1/signals` | GET | 获取信号 | ⚠️ 无测试 |
| `/api/v1/positions` | GET | 获取持仓 | ⚠️ 无测试 |
| `/api/v1/status` | GET | 服务状态 | ⚠️ 无测试 |
| `/api/v1/macro/decision` | GET | 宏观决策 | ⚠️ 无测试 |
| `/api/v1/macro/analyze` | POST | 强制分析 | ⚠️ 无测试 |
| `/api/v1/paper/start` | POST | 开始模拟 | ⚠️ 无测试 |
| `/api/v1/paper/stop` | POST | 停止模拟 | ⚠️ 无测试 |
| `/api/v1/paper/status` | GET | 模拟状态 | ⚠️ 无测试 |
| `/api/v1/value/analyze` | POST | 价值分析 | ⚠️ 无测试 |
| `/api/v1/screener/run` | POST | 触发扫描 | ⚠️ 无测试 |

**评估结果**: ⚠️ **API 层 0% 测试覆盖是重大风险**

### 5.2 错误处理评估

```rust
pub async fn get_macro_decision(...) -> Result<Json<MacroDecisionResponse>, StatusCode> {
    match state.macro_orchestrator.evaluate().await {
        Ok(decision) => Ok(Json(...)),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get macro decision");
            Err(StatusCode::INTERNAL_SERVER_ERROR)  // 仅返回 500
        }
    }
}
```

**问题**: 错误信息未返回给客户端，调试困难

**建议**: 添加结构化错误响应:
```rust
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}
```

---

## 6. 会话管理评估

### 6.1 状态机 (`session/mod.rs`)

```
     ┌─────────┐
     │  Idle   │
     └────┬────┘
          │ start()
          ▼
     ┌─────────┐
     │Starting │
     └────┬────┘
          │ initialized
          ▼
     ┌─────────┐    pause()    ┌─────────┐
     │ Running │──────────────▶│ Paused  │
     └────┬────┘◀──────────────┴────┬────┘
          │        resume()         │
          │                         │
          │ stop()                  │ stop()
          ▼                         ▼
     ┌─────────┐              ┌─────────┐
     │ Stopped │              │ Failed  │
     └─────────┘              └─────────┘
```

### 6.2 持久化评估 (`session/state.rs`)

| 功能 | 实现 | 评估 |
|-----|------|------|
| SQLite 存储 | `StateStore::open(path)` | ✅ 支持文件和内存 |
| 会话 CRUD | `create/get/update_session` | ✅ 完整 |
| 持仓跟踪 | `create/get/close_position` | ✅ 完整 |
| 活跃会话查询 | `get_active_sessions()` | ✅ 按状态过滤 |
| 历史清理 | `cleanup_old_sessions(days)` | ✅ 自动清理 |

**评估结果**: ✅ 持久化设计合理，支持崩溃恢复

---

## 7. 发现的问题和风险

### 7.1 高优先级

| 问题 | 模块 | 风险等级 | 建议 |
|-----|------|---------|-----|
| API 无测试 | routes.rs | 🔴 高 | 添加集成测试 |
| 调度器无测试 | scheduler.rs | 🔴 高 | 添加单元测试 |
| 会话管理器无测试 | session/manager.rs | 🔴 高 | 添加生命周期测试 |

### 7.2 中优先级

| 问题 | 模块 | 风险等级 | 建议 |
|-----|------|---------|-----|
| 策略覆盖率低 | strategy/po3.rs (17%) | 🟡 中 | 添加边界测试 |
| SMT 覆盖率低 | strategy/smt.rs (21%) | 🟡 中 | 添加背离类型测试 |
| 错误响应不详细 | routes.rs | 🟡 中 | 返回结构化错误 |

### 7.3 低优先级

| 问题 | 模块 | 风险等级 | 建议 |
|-----|------|---------|-----|
| 废弃字段警告 | data/aggregator.rs | 🟢 低 | 迁移到新配置 |
| 未使用导入 | lib.rs:645 | ✅ 已修复 | - |

---

## 8. 结论

### 8.1 正确执行的逻辑

1. **T+1 规则** - 使用 `today > entry_date` 严格阻止当日卖出
2. **仓位计算** - 向下取整到 100 股，符合 A股交易规则
3. **风险管理** - 所有参数可配置，支持不同板块涨跌停
4. **混合决策** - 规则引擎优先，异常时触发 LLM，总是取保守方向
5. **会话持久化** - SQLite 存储，支持崩溃恢复

### 8.2 需要关注的逻辑

1. **API 层** - 无测试覆盖，无法验证 HTTP 行为
2. **策略检测** - 覆盖率低，边界条件可能未充分测试
3. **调度器** - 定时任务逻辑未测试

### 8.3 建议优先级

1. **立即**: 为 API 层添加集成测试
2. **短期**: 提升策略模块测试覆盖率
3. **中期**: 添加端到端交易日模拟测试

---

## 附录: 测试运行结果

```
$ cargo test --lib
test result: ok. 377 passed; 0 failed; 31 ignored

$ cargo test --tests
test result: ok. 56 passed; 0 failed; 1 ignored

$ cargo tarpaulin --lib
31.02% coverage, 4160/13412 lines covered
```
