# 宏观经济分析智能体集成 - 实现进展

**日期**: 2026-02-25
**状态**: 已完成

## 概述

完成了 zero-trading 服务的宏观经济分析智能体（macro agent）集成，实现了混合模式的智能宏观判断。

## 实现架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      zero-trading 宏观集成                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────┐     ┌───────────────┐     ┌──────────────┐  │
│  │  MacroFilter  │     │ AgentBridge   │     │ MacroReport  │  │
│  │  (规则引擎)    │     │ (Agent调用)   │     │ (定期报告)    │  │
│  └───────┬───────┘     └───────┬───────┘     └──────┬───────┘  │
│          │                     │                     │          │
│          └──────────┬──────────┴─────────────────────┘          │
│                     ▼                                           │
│            ┌─────────────────┐                                  │
│            │ MacroOrchestrator│                                  │
│            │ (统一协调器)     │                                  │
│            └────────┬────────┘                                  │
│                     │                                           │
└─────────────────────┼───────────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │zero-     │  │CodeCoder │  │zero-     │
  │workflow  │  │API :4400 │  │channels  │
  │:4432     │  │(macro    │  │:4431     │
  │(数据源)   │  │agent)    │  │(Telegram)│
  └──────────┘  └──────────┘  └──────────┘
```

## 新增文件

```
services/zero-trading/src/macro_agent/
├── mod.rs              # 模块入口（含 create_* 工厂函数）
├── bridge.rs           # Agent 调用桥接（HTTP 客户端）
├── orchestrator.rs     # 混合模式协调器
├── report.rs           # 定期报告生成
└── types.rs            # Agent 相关类型定义
```

## 核心组件

### 1. AgentBridge (`bridge.rs`)

HTTP 客户端，用于调用 CodeCoder API 的 macro agent：

- `analyze()` - 发送宏观数据，获取智能分析
- `generate_report()` - 生成周度/月度报告
- 自动解析 JSON 响应（支持代码块格式）
- 重试机制和错误处理

### 2. MacroOrchestrator (`orchestrator.rs`)

混合模式协调器，核心逻辑：

1. **快速路径（规则引擎）**: 常规情况下使用 MacroFilter 快速判断
2. **慢速路径（Agent 分析）**: 异常情况触发深度分析

**异常触发条件**:

| 条件 | 阈值 | 说明 |
|------|------|------|
| 极端风险偏好 | < 30 或 > 70 | 市场情绪极端 |
| 避免交易信号 | trading_bias = AvoidTrading | 规则引擎建议观望 |
| 大幅降仓建议 | position_multiplier < 0.5 | 建议显著降低仓位 |
| PMI 极端值 | < 48 或 > 54 | 经济指标极端 |
| 指标背离 | 多指标方向冲突 | PMI 与 M2 背离等 |

### 3. MacroReportGenerator (`report.rs`)

定期报告生成器：

- **周报**: 每周一 9:00（北京时间）
- **月报**: 每月 1 日 9:00（北京时间）
- 自动发送到 Telegram
- 支持即时报告生成

### 4. 类型定义 (`types.rs`)

- `AgentRequest/AgentResponse` - API 请求/响应
- `MacroContext` - 宏观数据上下文
- `AgentAnalysis` - 解析后的分析结果
- `MacroDecision` - 最终决策（含来源、置信度等）
- `AnalysisTrigger` - 触发原因枚举
- `MacroReport` - 报告结构

## 配置项

在 `~/.codecoder/config.json` 中新增：

```json
{
  "trading": {
    "macro_agent": {
      "enabled": true,
      "codecoder_endpoint": "http://127.0.0.1:4400",
      "timeout_secs": 30,
      "cache_duration_secs": 3600,
      "weekly_report_enabled": true,
      "weekly_report_cron": "0 9 * * 1",
      "monthly_report_enabled": true,
      "monthly_report_cron": "0 9 1 * *"
    }
  }
}
```

## API 端点

新增 HTTP API 路由：

| 路径 | 方法 | 说明 |
|------|------|------|
| `/api/v1/macro/decision` | GET | 获取当前宏观决策（混合模式） |
| `/api/v1/macro/analyze` | POST | 强制触发 Agent 分析（绕过缓存） |
| `/api/v1/macro/report` | GET | 生成即时宏观报告 |
| `/api/v1/macro/report/send` | POST | 生成并发送报告到 Telegram |
| `/api/v1/macro/status` | GET | 检查 Agent 可用性 |

## 测试结果

```
running 109 tests
...
test result: ok. 109 passed; 0 failed; 0 ignored
```

新增 28 个 macro_agent 相关测试：
- `macro_agent::bridge::tests` - 7 个
- `macro_agent::orchestrator::tests` - 8 个
- `macro_agent::report::tests` - 5 个
- `macro_agent::types::tests` - 4 个
- `macro_agent::tests` - 4 个

## 修改文件

1. **`services/zero-common/src/config.rs`**
   - 新增 `MacroAgentConfig` 结构
   - 在 `TradingConfig` 中添加 `macro_agent` 字段

2. **`services/zero-trading/src/lib.rs`**
   - 添加 `macro_agent` 模块
   - 更新 `TradingState` 包含 `macro_orchestrator` 和 `report_generator`
   - 策略扫描器使用 orchestrator 检查是否建议交易
   - 启动报告调度器后台任务

3. **`services/zero-trading/src/routes.rs`**
   - 新增宏观相关 API 路由

## 使用示例

### 获取宏观决策

```bash
curl http://127.0.0.1:4434/api/v1/macro/decision
```

响应示例：
```json
{
  "source": "Merged",
  "cycle_phase": "EarlyRecovery",
  "position_multiplier": 0.85,
  "trading_bias": "Neutral",
  "risk_appetite": 45.0,
  "risk_warnings": ["触发条件: 指标背离"],
  "summary": "智能体分析判断...",
  "confidence": 0.75,
  "trading_recommended": true
}
```

### 强制分析

```bash
curl -X POST http://127.0.0.1:4434/api/v1/macro/analyze
```

### 生成报告

```bash
curl http://127.0.0.1:4434/api/v1/macro/report
```

## 后续工作

1. **集成测试**: 启动 CodeCoder API 进行端到端测试
2. **性能监控**: 添加 Agent 调用延迟和成功率指标
3. **缓存优化**: 考虑使用 Redis 进行分布式缓存
4. **更多触发条件**: 基于实际运行经验调整阈值
