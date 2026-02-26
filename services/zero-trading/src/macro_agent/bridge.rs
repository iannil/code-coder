//! Agent bridge for communicating with the CodeCoder API.
//!
//! This module provides HTTP client functionality to call the macro agent
//! in CodeCoder for deep analysis of macroeconomic conditions.

use anyhow::{Context, Result};
use std::time::Duration;
use tracing::{debug, info, warn};

use super::types::{AgentAnalysis, AgentRequest, AgentResponse, MacroContext, ReportType};
use crate::macro_filter::{EconomicCyclePhase, TradingBias};

/// Configuration for the agent bridge.
#[derive(Debug, Clone)]
pub struct AgentBridgeConfig {
    /// CodeCoder API endpoint
    pub codecoder_endpoint: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Backoff duration between retries
    pub retry_backoff: Duration,
}

impl Default for AgentBridgeConfig {
    fn default() -> Self {
        Self {
            codecoder_endpoint: "http://127.0.0.1:4400".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 2,
            retry_backoff: Duration::from_secs(1),
        }
    }
}

/// Bridge for calling the macro agent via CodeCoder API.
pub struct AgentBridge {
    /// Configuration
    config: AgentBridgeConfig,
    /// HTTP client
    client: reqwest::Client,
}

impl AgentBridge {
    /// Create a new agent bridge with the given configuration.
    pub fn new(config: AgentBridgeConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, client }
    }

    /// Analyze macro conditions using the macro agent.
    ///
    /// This sends the current macro context to the CodeCoder API's macro agent
    /// and parses the response into a structured analysis.
    pub async fn analyze(&self, context: &MacroContext) -> Result<AgentAnalysis> {
        let prompt = self.build_analysis_prompt(context);

        let request = AgentRequest {
            user_id: "zero-trading".to_string(),
            agent: "macro".to_string(),
            message: prompt,
            stream: false,
        };

        let response = self.send_request(&request).await?;
        self.parse_analysis_response(&response.content)
    }

    /// Generate a periodic macro report.
    pub async fn generate_report(&self, report_type: ReportType) -> Result<String> {
        let prompt = self.build_report_prompt(report_type);

        let request = AgentRequest {
            user_id: "zero-trading".to_string(),
            agent: "macro".to_string(),
            message: prompt,
            stream: false,
        };

        let response = self.send_request(&request).await?;
        Ok(response.content)
    }

    /// Send a request to the CodeCoder API with retry logic.
    async fn send_request(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let url = format!("{}/api/v1/chat", self.config.codecoder_endpoint);

        let mut last_error = None;

        for attempt in 1..=self.config.max_retries + 1 {
            match self.try_send(&url, request).await {
                Ok(response) => {
                    info!(
                        agent = %request.agent,
                        attempt,
                        "Agent request successful"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        agent = %request.agent,
                        attempt,
                        max_attempts = self.config.max_retries + 1,
                        error = %e,
                        "Agent request failed, retrying..."
                    );
                    last_error = Some(e);

                    if attempt <= self.config.max_retries {
                        tokio::time::sleep(self.config.retry_backoff).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
    }

    /// Try to send a single request.
    async fn try_send(&self, url: &str, request: &AgentRequest) -> Result<AgentResponse> {
        debug!(url, agent = %request.agent, "Sending request to CodeCoder API");

        let response = self
            .client
            .post(url)
            .json(request)
            .send()
            .await
            .context("Failed to send request to CodeCoder API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("CodeCoder API error: HTTP {} - {}", status, error_text);
        }

        let agent_response: AgentResponse = response
            .json()
            .await
            .context("Failed to parse CodeCoder API response")?;

        Ok(agent_response)
    }

    /// Build the analysis prompt for the macro agent.
    fn build_analysis_prompt(&self, context: &MacroContext) -> String {
        format!(
            r#"分析当前宏观环境对A股交易的影响。

## 当前数据
- PMI: {}
- M2同比: {}
- 社融: {}
- 风险偏好指数: {:.1}
- 当前仓位建议: {:.2}x
- 规则引擎判断: {:?}
- 备注: {}

## 请提供以下分析
1. **周期判断**: 当前经济处于什么周期阶段（扩张/早期复苏/放缓/收缩）
2. **仓位建议**: 建议的仓位调整系数（0.3-1.5，1.0为正常）
3. **交易偏向**: 应该偏向做多、做空、中性还是避免交易
4. **风险提示**: 当前需要关注的主要风险点
5. **置信度**: 对以上判断的置信度（0.0-1.0）

请用JSON格式返回结果，格式如下：
```json
{{
  "cycle_phase": "Expansion|EarlyRecovery|Slowdown|Contraction",
  "position_advice": 0.8,
  "trading_bias": "Bullish|Neutral|Bearish|AvoidTrading",
  "risk_warnings": ["风险1", "风险2"],
  "reasoning": "详细分析说明",
  "confidence": 0.75
}}
```"#,
            context.pmi.map(|p| format!("{:.1}", p)).unwrap_or_else(|| "未知".to_string()),
            context.m2_growth.map(|m| format!("{:.1}%", m)).unwrap_or_else(|| "未知".to_string()),
            context.social_financing.map(|s| format!("{:.2}万亿", s)).unwrap_or_else(|| "未知".to_string()),
            context.risk_appetite,
            context.position_multiplier,
            context.trading_bias,
            context.notes
        )
    }

    /// Build the prompt for periodic reports.
    fn build_report_prompt(&self, report_type: ReportType) -> String {
        match report_type {
            ReportType::Weekly => r#"生成本周宏观经济简报，包括：

1. **核心数据回顾**（PMI、工业增加值、固定资产投资）
2. **周期位置判断**（当前经济处于什么阶段）
3. **下周交易建议**（仓位、方向、关注点）
4. **风险提示**（需要警惕的风险因素）

请用Markdown格式，适合发送到Telegram。"#.to_string(),

            ReportType::Monthly => r#"生成月度宏观经济分析报告，包括：

1. **宏观经济总览**
   - 主要经济指标汇总
   - 与上月/去年同期对比

2. **货币政策分析**
   - M2增速变化
   - 社融数据解读
   - 利率环境判断

3. **周期定位**
   - 当前经济周期阶段
   - 与历史周期对比

4. **A股市场影响**
   - 对不同行业的影响
   - 建议关注的板块

5. **下月展望**
   - 关键数据发布时间
   - 建议的交易策略

请用Markdown格式，适合发送到Telegram。"#.to_string(),

            ReportType::DailyMorning => r#"生成早间宏观经济简报（开盘前参考）：

1. **市场概览**
   - 隔夜外盘走势（美股、欧股、亚太）
   - A股期货（IF/IH/IC）夜盘表现
   - 北向资金预估

2. **宏观动态**
   - 近期重要数据回顾（PMI/CPI/M2等）
   - 今日数据发布预告
   - 政策信号解读

3. **交易建议**
   - 当日仓位建议（0.3-1.5）
   - 关注板块/回避板块
   - 风险提示

请简洁明了，适合开盘前快速阅读。用Markdown格式，适合发送到Telegram。"#.to_string(),

            ReportType::DailyAfternoon => r#"生成收盘后市场总结：

1. **今日市场**
   - 主要指数表现
   - 成交额变化
   - 涨跌家数/涨停跌停

2. **资金动向**
   - 北向资金净流入
   - 行业资金流向
   - 主力动向

3. **明日展望**
   - 技术面分析
   - 关键支撑/压力位
   - 明日策略建议

请简洁明了，重点突出。用Markdown格式，适合发送到Telegram。"#.to_string(),

            ReportType::AdHoc => r#"分析当前宏观环境，提供即时交易建议：

1. 当前经济周期判断
2. 仓位建议（0.3-1.5）
3. 交易方向建议
4. 主要风险提示

请简洁明了，适合快速决策参考。"#.to_string(),

            ReportType::Quarterly => r#"生成季度宏观经济总览报告，包括：

1. **宏观经济全景**
   - GDP增速：名义与实际增速对比，GDP平减指数分析
   - 三驾马车分解：消费、投资、净出口各自贡献率
   - 两年平均增速（如有基数扰动）

2. **供给端分析**
   - 工业增加值：分行业拆解（采矿、制造、电力）
   - 工业企业利润："量价分离"分析（营收=量×价）
   - 产能利用率变化

3. **需求端分析（三驾马车详解）**
   - 投资：基建/房地产/制造业三分法
     - 房地产传导链：销售→资金→拿地→新开工→投资
     - 基建资金来源：专项债、政策性金融、预算内
   - 消费：社零增速、限额以上/以下分化
   - 净出口：全球需求、汇率影响、贸易条件

4. **经济周期定位**
   - 当前阶段：扩张/早期复苏/放缓/收缩
   - 库存周期：被动去库/主动补库/被动补库/主动去库
   - 与历史周期对比（2015-16、2018-19等）

5. **政策环境评估**
   - 货币政策：宽松/中性/紧缩，关注M2、社融、LPR
   - 财政政策：积极/稳健/收紧，关注赤字率、专项债
   - 重大政策信号解读（政治局会议、国常会表态）

6. **价格与货币**
   - CPI/PPI走势："剪刀差"分析及产业链利润含义
   - M2与社融：流动性环境判断
   - 实际利率水平

7. **因果链图谱**（使用Mermaid格式）
   - 当前经济的主要传导链条
   - 示例格式：
   ```mermaid
   flowchart TD
       A[政策信号] --> B[资金来源]
       B --> C[投资需求]
       C --> D[工业生产]
       D --> E[GDP增长]
   ```

8. **下季度展望**
   - 关键变量预判（PMI、社融、投资增速）
   - 上行风险与下行风险
   - 需要跟踪验证的高频指标

请用Markdown格式，包含必要的数据支撑和图表。"#.to_string(),

            ReportType::DataRelease => r#"解读最新发布的宏观经济数据：

1. **数据概览**
   - 发布数据项及数值
   - 与市场预期对比（超预期/符合预期/不及预期）
   - 与前值对比（环比改善/恶化）
   - 两年平均增速（如存在基数效应）

2. **数据本源解读**
   - 名义vs实际：剔除价格因素后的真实变化
   - 累计vs当月：计算单月增速
   - 同比vs环比：边际变化判断
   - 识别统计口径变化或季节性因素

3. **数据联动分析**
   - 与其他相关数据的交叉验证
   - 传导链分析：
     - 社融 → 投资 → 工业增加值
     - PPI → 工业利润 → 制造业投资
     - 房地产销售 → 土地出让 → 基建空间
   - "量价分离"分析（如适用）

4. **周期定位影响**
   - 对经济周期判断的影响
   - 对库存周期判断的影响
   - 是否触发周期阶段切换信号

5. **市场影响判断**
   - 对A股市场的直接影响
   - 受影响的行业板块（结合PPI-CPI剪刀差）
   - 短期交易机会或风险
   - 对债券市场的影响

6. **政策含义**
   - 数据是否会触发政策调整
   - 下一步政策走向预判

7. **后续关注**
   - 需要继续跟踪的验证指标（高频数据）
   - 下一个重要数据发布时间
   - 证据链构建建议

请简洁明了，重点突出数据解读和市场影响。使用"假设-验证-结论"的分析逻辑。"#.to_string(),
        }
    }

    /// Parse the agent's response into a structured analysis.
    fn parse_analysis_response(&self, content: &str) -> Result<AgentAnalysis> {
        // Try to extract JSON from the response
        let json_str = self.extract_json(content)?;

        // Parse the JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .context("Failed to parse agent response as JSON")?;

        // Extract fields with fallbacks
        let cycle_phase = self.parse_cycle_phase(
            parsed.get("cycle_phase")
                .and_then(|v| v.as_str())
                .unwrap_or("EarlyRecovery")
        );

        let position_advice = parsed.get("position_advice")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.3, 1.5);

        let trading_bias = self.parse_trading_bias(
            parsed.get("trading_bias")
                .and_then(|v| v.as_str())
                .unwrap_or("Neutral")
        );

        let risk_warnings: Vec<String> = parsed.get("risk_warnings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let reasoning = parsed.get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let confidence = parsed.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        Ok(AgentAnalysis {
            cycle_phase,
            position_advice,
            risk_warnings,
            trading_bias,
            reasoning,
            confidence,
        })
    }

    /// Extract JSON from a response that may contain markdown code blocks.
    fn extract_json(&self, content: &str) -> Result<String> {
        // Try to find JSON in code blocks first
        if let Some(start) = content.find("```json") {
            let start = start + 7;
            if let Some(end) = content[start..].find("```") {
                return Ok(content[start..start + end].trim().to_string());
            }
        }

        // Try to find raw JSON
        if let Some(start) = content.find('{') {
            // Find matching closing brace
            let mut depth = 0;
            let mut end = start;
            for (i, c) in content[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth == 0 {
                return Ok(content[start..end].to_string());
            }
        }

        anyhow::bail!("Could not find JSON in response")
    }

    /// Parse cycle phase from string.
    fn parse_cycle_phase(&self, s: &str) -> EconomicCyclePhase {
        match s.to_lowercase().as_str() {
            "expansion" | "扩张" => EconomicCyclePhase::Expansion,
            "earlyrecovery" | "early_recovery" | "早期复苏" | "复苏" => EconomicCyclePhase::EarlyRecovery,
            "slowdown" | "放缓" | "减速" => EconomicCyclePhase::Slowdown,
            "contraction" | "收缩" | "衰退" => EconomicCyclePhase::Contraction,
            _ => EconomicCyclePhase::EarlyRecovery,
        }
    }

    /// Parse trading bias from string.
    fn parse_trading_bias(&self, s: &str) -> TradingBias {
        match s.to_lowercase().as_str() {
            "bullish" | "多头" | "看多" => TradingBias::Bullish,
            "neutral" | "中性" | "观望" => TradingBias::Neutral,
            "bearish" | "空头" | "看空" => TradingBias::Bearish,
            "avoidtrading" | "avoid_trading" | "避免交易" | "空仓" => TradingBias::AvoidTrading,
            _ => TradingBias::Neutral,
        }
    }

    /// Check if the CodeCoder API is available.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.config.codecoder_endpoint);
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_bridge_config_default() {
        let config = AgentBridgeConfig::default();
        assert_eq!(config.codecoder_endpoint, "http://127.0.0.1:4400");
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_parse_cycle_phase() {
        let bridge = AgentBridge::new(AgentBridgeConfig::default());

        assert_eq!(bridge.parse_cycle_phase("Expansion"), EconomicCyclePhase::Expansion);
        assert_eq!(bridge.parse_cycle_phase("扩张"), EconomicCyclePhase::Expansion);
        assert_eq!(bridge.parse_cycle_phase("Contraction"), EconomicCyclePhase::Contraction);
        assert_eq!(bridge.parse_cycle_phase("收缩"), EconomicCyclePhase::Contraction);
        assert_eq!(bridge.parse_cycle_phase("unknown"), EconomicCyclePhase::EarlyRecovery);
    }

    #[test]
    fn test_parse_trading_bias() {
        let bridge = AgentBridge::new(AgentBridgeConfig::default());

        assert_eq!(bridge.parse_trading_bias("Bullish"), TradingBias::Bullish);
        assert_eq!(bridge.parse_trading_bias("看多"), TradingBias::Bullish);
        assert_eq!(bridge.parse_trading_bias("AvoidTrading"), TradingBias::AvoidTrading);
        assert_eq!(bridge.parse_trading_bias("避免交易"), TradingBias::AvoidTrading);
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let bridge = AgentBridge::new(AgentBridgeConfig::default());

        let content = r#"
Here is the analysis:

```json
{"cycle_phase": "Expansion", "position_advice": 1.2}
```

That's my recommendation.
"#;

        let json = bridge.extract_json(content).unwrap();
        assert!(json.contains("Expansion"));
    }

    #[test]
    fn test_extract_json_raw() {
        let bridge = AgentBridge::new(AgentBridgeConfig::default());

        let content = r#"Based on analysis: {"cycle_phase": "Slowdown", "position_advice": 0.7} is the result."#;

        let json = bridge.extract_json(content).unwrap();
        assert!(json.contains("Slowdown"));
    }

    #[test]
    fn test_parse_analysis_response() {
        let bridge = AgentBridge::new(AgentBridgeConfig::default());

        let content = r#"```json
{
  "cycle_phase": "Expansion",
  "position_advice": 1.2,
  "trading_bias": "Bullish",
  "risk_warnings": ["通胀压力", "外部风险"],
  "reasoning": "经济扩张期，适合积极配置",
  "confidence": 0.8
}
```"#;

        let analysis = bridge.parse_analysis_response(content).unwrap();
        assert_eq!(analysis.cycle_phase, EconomicCyclePhase::Expansion);
        assert!((analysis.position_advice - 1.2).abs() < 0.001);
        assert_eq!(analysis.trading_bias, TradingBias::Bullish);
        assert_eq!(analysis.risk_warnings.len(), 2);
        assert!((analysis.confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_build_analysis_prompt() {
        let bridge = AgentBridge::new(AgentBridgeConfig::default());

        let context = MacroContext {
            pmi: Some(51.5),
            m2_growth: Some(9.2),
            social_financing: Some(3.5),
            risk_appetite: 55.0,
            position_multiplier: 1.0,
            trading_bias: TradingBias::Neutral,
            notes: "规则引擎正常运行".to_string(),
        };

        let prompt = bridge.build_analysis_prompt(&context);
        assert!(prompt.contains("51.5"));
        assert!(prompt.contains("9.2%"));
        assert!(prompt.contains("周期判断"));
    }
}
