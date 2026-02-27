//! Consensus Analyzer Implementation.
//!
//! Analyzes policy documents using LLM to extract national consensus signals.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::value::types::{ConsensusSignal, PolicyReference, SafetyTheme};

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the consensus analyzer.
#[derive(Debug, Clone)]
pub struct ConsensusAnalyzerConfig {
    /// CodeCoder API endpoint
    pub codecoder_endpoint: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Backoff duration between retries
    pub retry_backoff: Duration,
    /// Agent name to use for consensus analysis
    pub agent_name: String,
}

impl Default for ConsensusAnalyzerConfig {
    fn default() -> Self {
        Self {
            codecoder_endpoint: "http://127.0.0.1:4400".to_string(),
            timeout: Duration::from_secs(60), // Longer timeout for policy analysis
            max_retries: 2,
            retry_backoff: Duration::from_secs(2),
            agent_name: "macro".to_string(), // Use macro agent for policy analysis
        }
    }
}

// ============================================================================
// Input Types
// ============================================================================

/// A policy document to analyze.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDocument {
    /// Document title
    pub title: String,
    /// Document type (e.g., "政府工作报告", "中央经济工作会议")
    pub document_type: String,
    /// Publication date
    pub published_at: DateTime<Utc>,
    /// Full text content
    pub content: String,
    /// Optional source URL
    pub source_url: Option<String>,
}

/// Type of policy change detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyChangeType {
    /// New formulation/term introduced
    NewFormulation,
    /// Order/priority changed
    OrderingChange,
    /// Adjective/intensity changed
    AdjectiveChange,
    /// Topic removed
    TopicRemoved,
    /// Topic strengthened
    TopicStrengthened,
    /// No significant change
    NoChange,
}

impl std::fmt::Display for PolicyChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewFormulation => write!(f, "新增提法"),
            Self::OrderingChange => write!(f, "排序变化"),
            Self::AdjectiveChange => write!(f, "形容词变动"),
            Self::TopicRemoved => write!(f, "话题移除"),
            Self::TopicStrengthened => write!(f, "话题强化"),
            Self::NoChange => write!(f, "无显著变化"),
        }
    }
}

/// Theme strength assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeStrength {
    /// The safety theme
    pub theme: SafetyTheme,
    /// Strength score (0.0-1.0)
    pub strength: f64,
    /// Key phrases supporting this assessment
    pub key_phrases: Vec<String>,
    /// Change type compared to previous period
    pub change_type: PolicyChangeType,
    /// Reasoning for the assessment
    pub reasoning: String,
}

// ============================================================================
// Output Types
// ============================================================================

/// Complete consensus analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusAnalysis {
    /// Document analyzed
    pub document_title: String,
    /// When analysis was performed
    pub analyzed_at: DateTime<Utc>,
    /// Consensus signals for each theme
    pub signals: Vec<ConsensusSignal>,
    /// Theme strength assessments with detailed reasoning
    pub theme_strengths: Vec<ThemeStrength>,
    /// Priority ranking of themes (ordered by importance)
    pub priority_ranking: Vec<SafetyTheme>,
    /// Overall policy tone summary
    pub policy_tone: String,
    /// Key takeaways
    pub highlights: Vec<String>,
    /// LLM confidence in analysis
    pub confidence: f64,
}

impl ConsensusAnalysis {
    /// Get the top priority themes (strength >= 0.7).
    pub fn top_priority_themes(&self) -> Vec<SafetyTheme> {
        self.theme_strengths
            .iter()
            .filter(|ts| ts.strength >= 0.7)
            .map(|ts| ts.theme)
            .collect()
    }

    /// Get signals with significant changes.
    pub fn changed_signals(&self) -> Vec<&ThemeStrength> {
        self.theme_strengths
            .iter()
            .filter(|ts| ts.change_type != PolicyChangeType::NoChange)
            .collect()
    }

    /// Generate a markdown summary.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# 政策共识分析报告\n\n"));
        md.push_str(&format!("**文档**: {}\n", self.document_title));
        md.push_str(&format!("**分析时间**: {}\n\n", self.analyzed_at.format("%Y-%m-%d %H:%M")));

        // Policy tone
        md.push_str("## 政策基调\n\n");
        md.push_str(&format!("{}\n\n", self.policy_tone));

        // Priority ranking
        md.push_str("## 安全主题优先级排序\n\n");
        for (i, theme) in self.priority_ranking.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, theme));
        }
        md.push_str("\n");

        // Theme strengths
        md.push_str("## 主题强度分析\n\n");
        for ts in &self.theme_strengths {
            let change_marker = if ts.change_type != PolicyChangeType::NoChange {
                format!(" [{}]", ts.change_type)
            } else {
                String::new()
            };
            md.push_str(&format!("### {}{}\n\n", ts.theme, change_marker));
            md.push_str(&format!("**强度**: {:.0}%\n\n", ts.strength * 100.0));
            md.push_str(&format!("**关键表述**:\n"));
            for phrase in &ts.key_phrases {
                md.push_str(&format!("- \"{}\"\n", phrase));
            }
            md.push_str(&format!("\n**分析**: {}\n\n", ts.reasoning));
        }

        // Highlights
        if !self.highlights.is_empty() {
            md.push_str("## 核心要点\n\n");
            for h in &self.highlights {
                md.push_str(&format!("- {}\n", h));
            }
        }

        md
    }
}

// ============================================================================
// API Request/Response
// ============================================================================

#[derive(Debug, Serialize)]
struct AgentRequest {
    user_id: String,
    channel: String,
    agent: String,
    message: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct AgentResponse {
    content: String,
    #[allow(dead_code)]
    agent: String,
}

// ============================================================================
// Consensus Analyzer
// ============================================================================

/// Analyzer for national policy consensus using LLM.
pub struct ConsensusAnalyzer {
    config: ConsensusAnalyzerConfig,
    client: reqwest::Client,
    /// Cache of recent analyses
    cache: HashMap<String, ConsensusAnalysis>,
}

impl ConsensusAnalyzer {
    /// Create a new consensus analyzer.
    pub fn new(config: ConsensusAnalyzerConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            client,
            cache: HashMap::new(),
        }
    }

    /// Analyze a policy document and extract consensus signals.
    pub async fn analyze(&mut self, document: &PolicyDocument) -> Result<ConsensusAnalysis> {
        // Check cache
        let cache_key = format!("{}_{}", document.title, document.published_at.timestamp());
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!(title = %document.title, "Returning cached analysis");
            return Ok(cached.clone());
        }

        let prompt = self.build_analysis_prompt(document);

        let request = AgentRequest {
            user_id: "zero-trading".to_string(),
            channel: "zero-trading".to_string(),
            agent: self.config.agent_name.clone(),
            message: prompt,
            stream: false,
        };

        let response = self.send_request(&request).await?;
        let analysis = self.parse_analysis_response(&response.content, document)?;

        // Cache the result
        self.cache.insert(cache_key, analysis.clone());

        Ok(analysis)
    }

    /// Compare two documents to detect policy changes.
    pub async fn compare(
        &self,
        current: &PolicyDocument,
        previous: &PolicyDocument,
    ) -> Result<Vec<ThemeStrength>> {
        let prompt = self.build_comparison_prompt(current, previous);

        let request = AgentRequest {
            user_id: "zero-trading".to_string(),
            channel: "zero-trading".to_string(),
            agent: self.config.agent_name.clone(),
            message: prompt,
            stream: false,
        };

        let response = self.send_request(&request).await?;
        self.parse_comparison_response(&response.content)
    }

    /// Build the analysis prompt for policy document.
    fn build_analysis_prompt(&self, document: &PolicyDocument) -> String {
        format!(
            r#"分析以下政策文档，提取国家安全共识信号。

## 文档信息
- 标题: {}
- 类型: {}
- 发布日期: {}

## 文档内容
{}

## 分析要求

请识别以下安全主题的强度和关键表述：

1. **能源安全** (EnergySecurity): 煤炭、电力、油气、新能源
2. **粮食安全** (FoodSecurity): 种业、农业科技、耕地保护
3. **金融安全** (FinancialSecurity): 金融稳定、数字货币、外汇储备
4. **产业安全** (IndustrySecurity): 产业链、供应链、高端制造
5. **科技安全** (TechnologySecurity): 半导体、AI、关键技术自主
6. **国防安全** (MilitarySecurity): 国防工业、军民融合

对每个主题，请评估：
- 强度 (0.0-1.0): 政策文本中提及频率和强调程度
- 关键表述: 直接引用文档中的相关原文
- 变化类型: 与常规政策语言相比是否有新增提法、排序变化或形容词变动

请用JSON格式返回结果：
```json
{{
  "theme_strengths": [
    {{
      "theme": "EnergySecurity|FoodSecurity|FinancialSecurity|IndustrySecurity|TechnologySecurity|MilitarySecurity",
      "strength": 0.85,
      "key_phrases": ["关键表述1", "关键表述2"],
      "change_type": "NewFormulation|OrderingChange|AdjectiveChange|TopicStrengthened|NoChange",
      "reasoning": "评估理由"
    }}
  ],
  "priority_ranking": ["EnergySecurity", "IndustrySecurity", ...],
  "policy_tone": "整体政策基调描述",
  "highlights": ["核心要点1", "核心要点2"],
  "confidence": 0.8
}}
```"#,
            document.title,
            document.document_type,
            document.published_at.format("%Y-%m-%d"),
            document.content
        )
    }

    /// Build the comparison prompt for two documents.
    fn build_comparison_prompt(
        &self,
        current: &PolicyDocument,
        previous: &PolicyDocument,
    ) -> String {
        format!(
            r#"比较以下两份政策文档的安全主题变化。

## 当前文档
- 标题: {}
- 发布日期: {}

## 内容摘要
{}

---

## 对比文档
- 标题: {}
- 发布日期: {}

## 内容摘要
{}

---

## 分析要求

请对比两份文档在以下安全主题上的变化：
1. 能源安全 2. 粮食安全 3. 金融安全 4. 产业安全 5. 科技安全 6. 国防安全

重点关注：
- **新增提法**: 当前文档中出现但之前没有的新表述
- **排序变化**: 主题优先级顺序的变化
- **形容词变动**: 描述强度的变化（如"积极"→"大力"）

请用JSON格式返回每个主题的变化分析。"#,
            current.title,
            current.published_at.format("%Y-%m-%d"),
            &current.content[..current.content.len().min(2000)],
            previous.title,
            previous.published_at.format("%Y-%m-%d"),
            &previous.content[..previous.content.len().min(2000)],
        )
    }

    /// Send request to CodeCoder API with retry logic.
    async fn send_request(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let url = format!("{}/api/v1/chat", self.config.codecoder_endpoint);

        let mut last_error = None;

        for attempt in 1..=self.config.max_retries + 1 {
            match self.try_send(&url, request).await {
                Ok(response) => {
                    info!(
                        agent = %request.agent,
                        attempt,
                        "Consensus analysis request successful"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        agent = %request.agent,
                        attempt,
                        max_attempts = self.config.max_retries + 1,
                        error = %e,
                        "Consensus analysis request failed, retrying..."
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
        debug!(url, agent = %request.agent, "Sending consensus analysis request");

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

    /// Parse analysis response from LLM.
    fn parse_analysis_response(
        &self,
        content: &str,
        document: &PolicyDocument,
    ) -> Result<ConsensusAnalysis> {
        let json_str = self.extract_json(content)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .context("Failed to parse response as JSON")?;

        // Parse theme strengths
        let theme_strengths: Vec<ThemeStrength> = parsed
            .get("theme_strengths")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| self.parse_theme_strength(v))
                    .collect()
            })
            .unwrap_or_default();

        // Parse priority ranking
        let priority_ranking: Vec<SafetyTheme> = parsed
            .get("priority_ranking")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| self.parse_safety_theme(s)))
                    .collect()
            })
            .unwrap_or_default();

        // Parse other fields
        let policy_tone = parsed
            .get("policy_tone")
            .and_then(|v| v.as_str())
            .unwrap_or("政策基调平稳")
            .to_string();

        let highlights: Vec<String> = parsed
            .get("highlights")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let confidence = parsed
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        // Build consensus signals from theme strengths
        let signals: Vec<ConsensusSignal> = theme_strengths
            .iter()
            .map(|ts| ConsensusSignal {
                theme: ts.theme,
                strength: ts.strength,
                policy_sources: vec![PolicyReference {
                    title: document.title.clone(),
                    date: document.published_at,
                    authority: document.document_type.clone(),
                    excerpt: ts.key_phrases.first().cloned(),
                }],
                key_phrases: ts.key_phrases.clone(),
                updated_at: Utc::now(),
            })
            .collect();

        Ok(ConsensusAnalysis {
            document_title: document.title.clone(),
            analyzed_at: Utc::now(),
            signals,
            theme_strengths,
            priority_ranking,
            policy_tone,
            highlights,
            confidence,
        })
    }

    /// Parse comparison response from LLM.
    fn parse_comparison_response(&self, content: &str) -> Result<Vec<ThemeStrength>> {
        let json_str = self.extract_json(content)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .context("Failed to parse comparison response as JSON")?;

        let theme_strengths: Vec<ThemeStrength> = parsed
            .get("theme_strengths")
            .or_else(|| parsed.get("changes"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| self.parse_theme_strength(v))
                    .collect()
            })
            .unwrap_or_default();

        Ok(theme_strengths)
    }

    /// Parse a single theme strength from JSON value.
    fn parse_theme_strength(&self, value: &serde_json::Value) -> Option<ThemeStrength> {
        let theme_str = value.get("theme")?.as_str()?;
        let theme = self.parse_safety_theme(theme_str)?;

        let strength = value
            .get("strength")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let key_phrases: Vec<String> = value
            .get("key_phrases")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let change_type = value
            .get("change_type")
            .and_then(|v| v.as_str())
            .map(|s| self.parse_change_type(s))
            .unwrap_or(PolicyChangeType::NoChange);

        let reasoning = value
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(ThemeStrength {
            theme,
            strength,
            key_phrases,
            change_type,
            reasoning,
        })
    }

    /// Parse safety theme from string.
    fn parse_safety_theme(&self, s: &str) -> Option<SafetyTheme> {
        match s.to_lowercase().as_str() {
            "energysecurity" | "energy_security" | "能源安全" => Some(SafetyTheme::EnergySecurity),
            "foodsecurity" | "food_security" | "粮食安全" => Some(SafetyTheme::FoodSecurity),
            "financialsecurity" | "financial_security" | "金融安全" => {
                Some(SafetyTheme::FinancialSecurity)
            }
            "industrysecurity" | "industry_security" | "产业安全" => {
                Some(SafetyTheme::IndustrySecurity)
            }
            "technologysecurity" | "technology_security" | "科技安全" => {
                Some(SafetyTheme::TechnologySecurity)
            }
            "militarysecurity" | "military_security" | "国防安全" => {
                Some(SafetyTheme::MilitarySecurity)
            }
            _ => None,
        }
    }

    /// Parse change type from string.
    fn parse_change_type(&self, s: &str) -> PolicyChangeType {
        match s.to_lowercase().as_str() {
            "newformulation" | "new_formulation" | "新增提法" => PolicyChangeType::NewFormulation,
            "orderingchange" | "ordering_change" | "排序变化" => PolicyChangeType::OrderingChange,
            "adjectivechange" | "adjective_change" | "形容词变动" => {
                PolicyChangeType::AdjectiveChange
            }
            "topicremoved" | "topic_removed" | "话题移除" => PolicyChangeType::TopicRemoved,
            "topicstrengthened" | "topic_strengthened" | "话题强化" => {
                PolicyChangeType::TopicStrengthened
            }
            _ => PolicyChangeType::NoChange,
        }
    }

    /// Extract JSON from response content.
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

    /// Check if the CodeCoder API is available.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.config.codecoder_endpoint);
        match self.client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Clear the analysis cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for ConsensusAnalyzer {
    fn default() -> Self {
        Self::new(ConsensusAnalyzerConfig::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ConsensusAnalyzerConfig::default();
        assert_eq!(config.codecoder_endpoint, "http://127.0.0.1:4400");
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_parse_safety_theme() {
        let analyzer = ConsensusAnalyzer::default();

        assert_eq!(
            analyzer.parse_safety_theme("EnergySecurity"),
            Some(SafetyTheme::EnergySecurity)
        );
        assert_eq!(
            analyzer.parse_safety_theme("能源安全"),
            Some(SafetyTheme::EnergySecurity)
        );
        assert_eq!(
            analyzer.parse_safety_theme("food_security"),
            Some(SafetyTheme::FoodSecurity)
        );
        assert_eq!(analyzer.parse_safety_theme("unknown"), None);
    }

    #[test]
    fn test_parse_change_type() {
        let analyzer = ConsensusAnalyzer::default();

        assert_eq!(
            analyzer.parse_change_type("NewFormulation"),
            PolicyChangeType::NewFormulation
        );
        assert_eq!(
            analyzer.parse_change_type("新增提法"),
            PolicyChangeType::NewFormulation
        );
        assert_eq!(
            analyzer.parse_change_type("OrderingChange"),
            PolicyChangeType::OrderingChange
        );
        assert_eq!(
            analyzer.parse_change_type("unknown"),
            PolicyChangeType::NoChange
        );
    }

    #[test]
    fn test_policy_change_type_display() {
        assert_eq!(PolicyChangeType::NewFormulation.to_string(), "新增提法");
        assert_eq!(PolicyChangeType::OrderingChange.to_string(), "排序变化");
        assert_eq!(PolicyChangeType::AdjectiveChange.to_string(), "形容词变动");
    }

    #[test]
    fn test_extract_json() {
        let analyzer = ConsensusAnalyzer::default();

        // Test extraction from code block
        let content = r#"
Here is the analysis:

```json
{"theme": "EnergySecurity", "strength": 0.85}
```

That's my assessment.
"#;

        let json = analyzer.extract_json(content).unwrap();
        assert!(json.contains("EnergySecurity"));
        assert!(json.contains("0.85"));
    }

    #[test]
    fn test_extract_json_raw() {
        let analyzer = ConsensusAnalyzer::default();

        let content = r#"Based on analysis: {"theme": "FoodSecurity", "strength": 0.7} is the result."#;

        let json = analyzer.extract_json(content).unwrap();
        assert!(json.contains("FoodSecurity"));
    }

    #[test]
    fn test_theme_strength_parsing() {
        let analyzer = ConsensusAnalyzer::default();

        let json_value = serde_json::json!({
            "theme": "IndustrySecurity",
            "strength": 0.9,
            "key_phrases": ["产业链安全", "供应链自主"],
            "change_type": "TopicStrengthened",
            "reasoning": "产业安全话题明显强化"
        });

        let ts = analyzer.parse_theme_strength(&json_value).unwrap();
        assert_eq!(ts.theme, SafetyTheme::IndustrySecurity);
        assert!((ts.strength - 0.9).abs() < 0.001);
        assert_eq!(ts.key_phrases.len(), 2);
        assert_eq!(ts.change_type, PolicyChangeType::TopicStrengthened);
    }

    #[test]
    fn test_consensus_analysis_top_priority() {
        let analysis = ConsensusAnalysis {
            document_title: "测试文档".to_string(),
            analyzed_at: Utc::now(),
            signals: vec![],
            theme_strengths: vec![
                ThemeStrength {
                    theme: SafetyTheme::EnergySecurity,
                    strength: 0.85,
                    key_phrases: vec![],
                    change_type: PolicyChangeType::NoChange,
                    reasoning: String::new(),
                },
                ThemeStrength {
                    theme: SafetyTheme::FoodSecurity,
                    strength: 0.5,
                    key_phrases: vec![],
                    change_type: PolicyChangeType::NoChange,
                    reasoning: String::new(),
                },
                ThemeStrength {
                    theme: SafetyTheme::TechnologySecurity,
                    strength: 0.75,
                    key_phrases: vec![],
                    change_type: PolicyChangeType::NewFormulation,
                    reasoning: String::new(),
                },
            ],
            priority_ranking: vec![
                SafetyTheme::EnergySecurity,
                SafetyTheme::TechnologySecurity,
                SafetyTheme::FoodSecurity,
            ],
            policy_tone: "积极稳健".to_string(),
            highlights: vec![],
            confidence: 0.8,
        };

        let top = analysis.top_priority_themes();
        assert_eq!(top.len(), 2);
        assert!(top.contains(&SafetyTheme::EnergySecurity));
        assert!(top.contains(&SafetyTheme::TechnologySecurity));

        let changed = analysis.changed_signals();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].theme, SafetyTheme::TechnologySecurity);
    }

    #[test]
    fn test_consensus_analysis_to_markdown() {
        let analysis = ConsensusAnalysis {
            document_title: "2024年政府工作报告".to_string(),
            analyzed_at: Utc::now(),
            signals: vec![],
            theme_strengths: vec![ThemeStrength {
                theme: SafetyTheme::EnergySecurity,
                strength: 0.85,
                key_phrases: vec!["能源安全".to_string(), "新型能源体系".to_string()],
                change_type: PolicyChangeType::TopicStrengthened,
                reasoning: "能源安全话题明显强化".to_string(),
            }],
            priority_ranking: vec![SafetyTheme::EnergySecurity],
            policy_tone: "积极稳健".to_string(),
            highlights: vec!["能源安全成为重点".to_string()],
            confidence: 0.8,
        };

        let md = analysis.to_markdown();
        assert!(md.contains("政策共识分析报告"));
        assert!(md.contains("能源安全"));
        assert!(md.contains("85%"));
        assert!(md.contains("话题强化"));
    }
}
