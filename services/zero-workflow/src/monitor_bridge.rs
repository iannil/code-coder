//! Monitor bridge for competitive intelligence.
//!
//! Fetches content from multiple sources (websites, RSS feeds),
//! uses LLM to generate summaries, and sends reports to IM channels.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use zero_common::config::{MonitorNotificationConfig, MonitorSourceConfig, MonitorTask};

// ============================================================================
// Types
// ============================================================================

/// Source type for monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Website,
    Rss,
    Twitter,
}

impl SourceType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rss" => Self::Rss,
            "twitter" => Self::Twitter,
            _ => Self::Website,
        }
    }
}

/// Content fetched from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceContent {
    /// Source ID
    pub source_id: String,
    /// Source name
    pub source_name: String,
    /// Fetched content (text)
    pub content: String,
    /// Fetch timestamp
    pub fetched_at: DateTime<Utc>,
    /// Original URL
    pub url: String,
    /// Any error that occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Summary for a single source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSummary {
    /// Source ID
    pub source_id: String,
    /// Source name
    pub source_name: String,
    /// Summary text
    pub summary: String,
    /// Key points extracted
    #[serde(default)]
    pub key_points: Vec<String>,
}

/// A complete monitor report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorReport {
    /// Report ID
    pub id: String,
    /// Task ID this report belongs to
    pub task_id: String,
    /// Task name
    pub task_name: String,
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Individual source summaries
    pub sources: Vec<SourceSummary>,
    /// Top highlights across all sources
    pub highlights: Vec<String>,
    /// Full analysis text
    pub full_analysis: String,
    /// Suggested action items
    #[serde(default)]
    pub action_items: Vec<String>,
}

/// RSS feed item.
#[derive(Debug, Clone)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    pub pub_date: Option<String>,
}

/// Result of a monitor run.
#[derive(Debug)]
pub enum MonitorRunResult {
    /// Successfully generated report
    Success {
        report: MonitorReport,
        notification_sent: bool,
    },
    /// Partial success (some sources failed)
    Partial {
        report: MonitorReport,
        failed_sources: Vec<String>,
        notification_sent: bool,
    },
    /// Complete failure
    Failed { reason: String },
}

// ============================================================================
// IM Configuration
// ============================================================================

/// IM notification configuration for monitor reports.
#[derive(Debug, Clone)]
pub struct MonitorIMConfig {
    /// Enable IM notifications
    pub enabled: bool,
    /// Zero Channels endpoint
    pub channels_endpoint: Option<String>,
    /// Channel type (feishu, wecom, dingtalk)
    pub channel_type: String,
    /// Channel ID
    pub channel_id: Option<String>,
    /// Report template
    pub template: String,
}

impl Default for MonitorIMConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            channels_endpoint: None,
            channel_type: "feishu".to_string(),
            channel_id: None,
            template: "daily_brief".to_string(),
        }
    }
}

impl From<&MonitorNotificationConfig> for MonitorIMConfig {
    fn from(config: &MonitorNotificationConfig) -> Self {
        Self {
            enabled: true,
            channels_endpoint: None, // Set separately
            channel_type: config.channel_type.clone(),
            channel_id: Some(config.channel_id.clone()),
            template: config.template.clone(),
        }
    }
}

// ============================================================================
// Monitor Bridge
// ============================================================================

/// Bridge for competitive intelligence monitoring.
pub struct MonitorBridge {
    /// CodeCoder API endpoint
    codecoder_endpoint: String,
    /// HTTP client
    client: reqwest::Client,
    /// IM notification configuration
    im_config: MonitorIMConfig,
    /// Report history (in-memory, recent reports)
    reports: Arc<tokio::sync::RwLock<Vec<MonitorReport>>>,
}

impl MonitorBridge {
    /// Create a new monitor bridge.
    pub fn new(codecoder_endpoint: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0 ZeroBot Monitor/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            codecoder_endpoint: codecoder_endpoint.into(),
            client,
            im_config: MonitorIMConfig::default(),
            reports: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Set IM notification configuration.
    pub fn with_im_config(mut self, config: MonitorIMConfig) -> Self {
        self.im_config = config;
        self
    }

    /// Set channels endpoint.
    pub fn with_channels_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.im_config.channels_endpoint = Some(endpoint.into());
        self
    }

    /// Run a monitor task.
    pub async fn run_monitor(&self, task: &MonitorTask) -> Result<MonitorRunResult> {
        tracing::info!(
            task_id = %task.id,
            task_name = %task.name,
            sources_count = task.sources.len(),
            "Starting monitor task"
        );

        // Fetch all sources concurrently
        let fetch_futures: Vec<_> = task
            .sources
            .iter()
            .map(|source| self.fetch_source(source))
            .collect();

        let fetch_results = futures::future::join_all(fetch_futures).await;

        // Separate successful and failed fetches
        let mut contents = Vec::new();
        let mut failed_sources = Vec::new();

        for result in fetch_results {
            match result {
                Ok(content) => {
                    if content.error.is_some() {
                        failed_sources.push(content.source_id.clone());
                    }
                    contents.push(content);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Source fetch failed");
                }
            }
        }

        if contents.is_empty() {
            return Ok(MonitorRunResult::Failed {
                reason: "All sources failed to fetch".into(),
            });
        }

        // Generate report using LLM
        let report = self.generate_report(task, contents).await?;

        // Store report
        {
            let mut reports = self.reports.write().await;
            reports.push(report.clone());
            // Keep only last 100 reports
            if reports.len() > 100 {
                reports.remove(0);
            }
        }

        // Send IM notification
        let notification_sent = self.send_report(&task.notification, &report).await.is_ok();

        if failed_sources.is_empty() {
            Ok(MonitorRunResult::Success {
                report,
                notification_sent,
            })
        } else {
            Ok(MonitorRunResult::Partial {
                report,
                failed_sources,
                notification_sent,
            })
        }
    }

    /// Fetch content from a single source.
    async fn fetch_source(&self, source: &MonitorSourceConfig) -> Result<SourceContent> {
        let source_type = SourceType::from_str(&source.source_type);

        tracing::debug!(
            source_id = %source.id,
            source_type = ?source_type,
            url = %source.url,
            "Fetching source"
        );

        let content_result = match source_type {
            SourceType::Website => self.fetch_website(&source.url, source.selector.as_deref()).await,
            SourceType::Rss => self.fetch_rss(&source.url).await,
            SourceType::Twitter => {
                // Twitter requires API access, return placeholder
                Err(anyhow::anyhow!("Twitter source type not yet implemented"))
            }
        };

        match content_result {
            Ok(content) => Ok(SourceContent {
                source_id: source.id.clone(),
                source_name: source.name.clone(),
                content,
                fetched_at: Utc::now(),
                url: source.url.clone(),
                error: None,
            }),
            Err(e) => {
                tracing::warn!(
                    source_id = %source.id,
                    error = %e,
                    "Failed to fetch source"
                );
                Ok(SourceContent {
                    source_id: source.id.clone(),
                    source_name: source.name.clone(),
                    content: String::new(),
                    fetched_at: Utc::now(),
                    url: source.url.clone(),
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Fetch and extract content from a website.
    async fn fetch_website(&self, url: &str, selector: Option<&str>) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch website")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP error: {}", response.status());
        }

        let html = response.text().await.context("Failed to read response body")?;

        if let Some(sel) = selector {
            // Use scraper to extract content with CSS selector
            let document = scraper::Html::parse_document(&html);
            let selector = scraper::Selector::parse(sel)
                .map_err(|e| anyhow::anyhow!("Invalid CSS selector: {:?}", e))?;

            let content: String = document
                .select(&selector)
                .map(|el| el.text().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join("\n\n");

            if content.is_empty() {
                // Fall back to extracting all text
                Ok(extract_text_from_html(&html))
            } else {
                Ok(content)
            }
        } else {
            Ok(extract_text_from_html(&html))
        }
    }

    /// Fetch and parse RSS feed.
    async fn fetch_rss(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch RSS feed")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP error: {}", response.status());
        }

        let content = response.bytes().await.context("Failed to read RSS content")?;

        let channel = rss::Channel::read_from(&content[..]).context("Failed to parse RSS feed")?;

        // Extract recent items (last 10)
        let items: Vec<String> = channel
            .items()
            .iter()
            .take(10)
            .map(|item| {
                let title = item.title().unwrap_or("No title");
                let description = item
                    .description()
                    .map(|d| extract_text_from_html(d))
                    .unwrap_or_default();
                let pub_date = item.pub_date().unwrap_or("");

                format!("**{}** ({})\n{}", title, pub_date, description)
            })
            .collect();

        Ok(items.join("\n\n---\n\n"))
    }

    /// Generate a report using LLM.
    async fn generate_report(
        &self,
        task: &MonitorTask,
        contents: Vec<SourceContent>,
    ) -> Result<MonitorReport> {
        let url = format!("{}/api/v1/chat", self.codecoder_endpoint);

        // Format contents for LLM
        let formatted_contents = contents
            .iter()
            .filter(|c| c.error.is_none() && !c.content.is_empty())
            .map(|c| {
                format!(
                    "### {} ({})\nURL: {}\n\n{}",
                    c.source_name,
                    c.source_id,
                    c.url,
                    truncate_content(&c.content, 5000)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let prompt = format!(
            r#"你是一个竞品分析专家。请根据以下收集的竞品动态，生成一份简洁的早报。

## 收集的内容

{formatted_contents}

## 输出要求

请按以下 JSON 格式输出（确保是有效的 JSON）：

```json
{{
  "highlights": ["要点1", "要点2", "要点3"],
  "by_source": {{
    "source_id_1": {{
      "summary": "简要动态（50字以内）",
      "key_points": ["要点1", "要点2"]
    }},
    "source_id_2": {{
      "summary": "简要动态（50字以内）",
      "key_points": ["要点1", "要点2"]
    }}
  }},
  "analysis": "整体分析（100字以内）",
  "action_items": ["建议1", "建议2"]
}}
```

注意：
1. 如果某个来源没有有价值的内容，可以跳过
2. 重点关注产品更新、融资消息、市场策略变化
3. highlights 最多 5 条，按重要性排序"#
        );

        let request = CodeCoderRequest {
            message: prompt,
            agent: Some("general".into()),
            user_id: "zero-monitor".into(),
            channel: "monitor".into(),
        };

        let response = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(120))
            .json(&request)
            .send()
            .await
            .context("Failed to call CodeCoder API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CodeCoder API error {}: {}", status, body);
        }

        let codecoder_response: CodeCoderResponse = response
            .json()
            .await
            .context("Failed to parse CodeCoder response")?;

        // Parse LLM response
        let report = self.parse_llm_response(task, &contents, &codecoder_response.message)?;

        Ok(report)
    }

    /// Parse LLM response into a MonitorReport.
    fn parse_llm_response(
        &self,
        task: &MonitorTask,
        contents: &[SourceContent],
        response: &str,
    ) -> Result<MonitorReport> {
        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response);

        let parsed: LLMReportResponse = serde_json::from_str(&json_str).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to parse LLM JSON response, using fallback");
            LLMReportResponse {
                highlights: vec!["分析完成，请查看详情".into()],
                by_source: HashMap::new(),
                analysis: response.chars().take(500).collect(),
                action_items: vec![],
            }
        });

        // Build source summaries
        let sources: Vec<SourceSummary> = contents
            .iter()
            .filter(|c| c.error.is_none())
            .map(|c| {
                let llm_summary = parsed.by_source.get(&c.source_id);
                SourceSummary {
                    source_id: c.source_id.clone(),
                    source_name: c.source_name.clone(),
                    summary: llm_summary
                        .map(|s| s.summary.clone())
                        .unwrap_or_else(|| "暂无更新".into()),
                    key_points: llm_summary
                        .map(|s| s.key_points.clone())
                        .unwrap_or_default(),
                }
            })
            .collect();

        Ok(MonitorReport {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            generated_at: Utc::now(),
            sources,
            highlights: parsed.highlights,
            full_analysis: parsed.analysis,
            action_items: parsed.action_items,
        })
    }

    /// Send report to IM channel.
    async fn send_report(
        &self,
        notification: &MonitorNotificationConfig,
        report: &MonitorReport,
    ) -> Result<()> {
        let endpoint = self
            .im_config
            .channels_endpoint
            .as_ref()
            .context("Channels endpoint not configured")?;

        // Format the report as markdown
        let message = format_report_markdown(report, &notification.template);

        let url = format!("{}/api/v1/send", endpoint);
        let body = serde_json::json!({
            "channel_type": notification.channel_type,
            "channel_id": notification.channel_id,
            "content": {
                "type": "markdown",
                "text": message
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send IM notification")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("IM notification failed {}: {}", status, body);
        }

        tracing::info!(
            channel_type = %notification.channel_type,
            channel_id = %notification.channel_id,
            report_id = %report.id,
            "Sent monitor report to IM"
        );

        Ok(())
    }

    /// Get recent reports for a task.
    pub async fn get_reports(&self, task_id: &str, limit: usize) -> Vec<MonitorReport> {
        let reports = self.reports.read().await;
        reports
            .iter()
            .filter(|r| r.task_id == task_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get all recent reports.
    pub async fn get_all_reports(&self, limit: usize) -> Vec<MonitorReport> {
        let reports = self.reports.read().await;
        reports.iter().rev().take(limit).cloned().collect()
    }
}

// ============================================================================
// Helper Types
// ============================================================================

#[derive(Debug, Serialize)]
struct CodeCoderRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    user_id: String,
    channel: String,
}

#[derive(Debug, Deserialize)]
struct CodeCoderResponse {
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    agent: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct LLMReportResponse {
    #[serde(default)]
    highlights: Vec<String>,
    #[serde(default)]
    by_source: HashMap<String, SourceLLMSummary>,
    #[serde(default)]
    analysis: String,
    #[serde(default)]
    action_items: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct SourceLLMSummary {
    summary: String,
    #[serde(default)]
    key_points: Vec<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract text content from HTML.
fn extract_text_from_html(html: &str) -> String {
    let document = scraper::Html::parse_document(html);

    // Remove script and style elements
    let text: String = document
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    text
}

/// Truncate content to a maximum length.
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    // Try to truncate at a word boundary
    let truncated = &content[..max_len];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

/// Extract JSON from LLM response (handles markdown code blocks).
fn extract_json_from_response(response: &str) -> String {
    // Try to find JSON in code blocks
    if let Some(start) = response.find("```json") {
        let after_marker = &response[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }

    // Try to find raw JSON
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }

    response.to_string()
}

/// Format monitor report as markdown.
fn format_report_markdown(report: &MonitorReport, template: &str) -> String {
    let date = report.generated_at.format("%Y-%m-%d").to_string();

    match template {
        "detailed" => format_detailed_report(report, &date),
        "comparison" => format_comparison_report(report, &date),
        _ => format_daily_brief(report, &date), // default: daily_brief
    }
}

/// Format as daily brief (concise).
fn format_daily_brief(report: &MonitorReport, date: &str) -> String {
    let mut md = format!("📊 **{}** | {}\n\n", report.task_name, date);

    // Highlights
    if !report.highlights.is_empty() {
        md.push_str("🔥 **今日要点**\n");
        for highlight in &report.highlights {
            md.push_str(&format!("• {}\n", highlight));
        }
        md.push('\n');
    }

    // Source summaries
    md.push_str("📈 **各竞品动态**\n\n");
    for source in &report.sources {
        md.push_str(&format!("**{}**\n", source.source_name));
        md.push_str(&format!("{}\n\n", source.summary));
    }

    // Analysis
    if !report.full_analysis.is_empty() {
        md.push_str(&format!("💡 **分析与建议**\n{}\n\n", report.full_analysis));
    }

    // Action items
    if !report.action_items.is_empty() {
        md.push_str("📋 **行动建议**\n");
        for (i, item) in report.action_items.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, item));
        }
        md.push('\n');
    }

    md.push_str("---\n*由 ZeroBot 自动生成*");

    md
}

/// Format as detailed report.
fn format_detailed_report(report: &MonitorReport, date: &str) -> String {
    let mut md = format!("# {} | {}\n\n", report.task_name, date);

    // Executive summary
    md.push_str("## 执行摘要\n\n");
    if !report.highlights.is_empty() {
        for highlight in &report.highlights {
            md.push_str(&format!("- {}\n", highlight));
        }
        md.push('\n');
    }

    // Detailed source analysis
    md.push_str("## 详细分析\n\n");
    for source in &report.sources {
        md.push_str(&format!("### {}\n\n", source.source_name));
        md.push_str(&format!("{}\n\n", source.summary));

        if !source.key_points.is_empty() {
            md.push_str("**关键点：**\n");
            for point in &source.key_points {
                md.push_str(&format!("- {}\n", point));
            }
            md.push('\n');
        }
    }

    // Overall analysis
    md.push_str("## 整体分析\n\n");
    md.push_str(&format!("{}\n\n", report.full_analysis));

    // Recommendations
    if !report.action_items.is_empty() {
        md.push_str("## 建议行动\n\n");
        for (i, item) in report.action_items.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, item));
        }
    }

    md.push_str("\n---\n*Generated by ZeroBot*");

    md
}

/// Format as comparison report.
fn format_comparison_report(report: &MonitorReport, date: &str) -> String {
    let mut md = format!("# 竞品对比报告 | {}\n\n", date);

    // Comparison table header
    md.push_str("| 竞品 | 最新动态 | 要点 |\n");
    md.push_str("|------|----------|------|\n");

    for source in &report.sources {
        let key_points = source.key_points.join("; ");
        md.push_str(&format!(
            "| {} | {} | {} |\n",
            source.source_name,
            truncate_content(&source.summary, 100),
            if key_points.is_empty() {
                "-".to_string()
            } else {
                key_points
            }
        ));
    }

    md.push_str(&format!("\n## 分析\n\n{}\n", report.full_analysis));

    md.push_str("\n---\n*Generated by ZeroBot*");

    md
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_type_from_str() {
        assert_eq!(SourceType::from_str("website"), SourceType::Website);
        assert_eq!(SourceType::from_str("rss"), SourceType::Rss);
        assert_eq!(SourceType::from_str("RSS"), SourceType::Rss);
        assert_eq!(SourceType::from_str("twitter"), SourceType::Twitter);
        assert_eq!(SourceType::from_str("unknown"), SourceType::Website);
    }

    #[test]
    fn test_extract_json_from_response() {
        let response1 = r#"Here is the analysis:
```json
{"highlights": ["point1"]}
```"#;
        assert_eq!(
            extract_json_from_response(response1),
            r#"{"highlights": ["point1"]}"#
        );

        let response2 = r#"{"highlights": ["point1"]}"#;
        assert_eq!(
            extract_json_from_response(response2),
            r#"{"highlights": ["point1"]}"#
        );
    }

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("short", 100), "short");
        assert_eq!(truncate_content("hello world test", 11), "hello...");
        assert_eq!(truncate_content("hello world test", 14), "hello world...");
    }

    #[test]
    fn test_extract_text_from_html() {
        let html = "<html><body><p>Hello</p><p>World</p></body></html>";
        let text = extract_text_from_html(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_format_daily_brief() {
        let report = MonitorReport {
            id: "test-123".into(),
            task_id: "task-1".into(),
            task_name: "每日竞品早报".into(),
            generated_at: Utc::now(),
            sources: vec![SourceSummary {
                source_id: "src-1".into(),
                source_name: "竞品A".into(),
                summary: "发布了新版本".into(),
                key_points: vec!["新功能".into()],
            }],
            highlights: vec!["竞品A发布新版本".into()],
            full_analysis: "市场竞争加剧".into(),
            action_items: vec!["关注竞品动态".into()],
        };

        let md = format_daily_brief(&report, "2026-02-22");
        assert!(md.contains("每日竞品早报"));
        assert!(md.contains("今日要点"));
        assert!(md.contains("竞品A"));
        assert!(md.contains("ZeroBot"));
    }

    #[test]
    fn test_llm_response_parsing() {
        let json = r#"{
            "highlights": ["Point 1", "Point 2"],
            "by_source": {
                "src-1": {
                    "summary": "Summary text",
                    "key_points": ["Key 1"]
                }
            },
            "analysis": "Overall analysis",
            "action_items": ["Action 1"]
        }"#;

        let parsed: LLMReportResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.highlights.len(), 2);
        assert!(parsed.by_source.contains_key("src-1"));
        assert_eq!(parsed.action_items.len(), 1);
    }
}
