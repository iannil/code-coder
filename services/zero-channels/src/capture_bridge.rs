//! Cross-platform asset capture bridge.
//!
//! Captures content from IM channels (Telegram, WeChat, etc.), extracts and summarizes
//! using LLM, and saves to knowledge bases (Feishu Docs, Notion).
//!
//! ## Workflow
//!
//! ```text
//! User forwards message / sends link
//!     │
//!     ▼
//! CaptureBridge.is_capturable() ──► Check if content should be captured
//!     │
//!     ▼
//! CaptureBridge.capture()
//!     │
//!     ├── extract_content() ◄─── Extract link content if URL present
//!     │
//!     ├── summarize_and_tag() ◄─ Call LLM for summary and tags
//!     │
//!     ├── save_to_feishu_docs() / save_to_notion()
//!     │
//!     ▼
//! Return CapturedAsset with confirmation
//! ```

use crate::message::{ChannelMessage, ChannelType, MessageContent};
use anyhow::Result;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use zero_common::config::CaptureConfig;

// ============================================================================
// Constants
// ============================================================================

const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";
const NOTION_API_BASE: &str = "https://api.notion.com/v1";
const TOKEN_REFRESH_MARGIN_SECS: u64 = 300;
const MAX_CONTENT_LENGTH: usize = 4000;

// ============================================================================
// Asset Types
// ============================================================================

/// Content type of the captured asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetContentType {
    /// Web article
    Article,
    /// Social media post (tweet, weibo, etc.)
    Tweet,
    /// Image with description
    Image,
    /// PDF or document
    Document,
    /// Pure link
    Link,
    /// Plain text
    RawText,
}

impl AssetContentType {
    /// Get the content type as a string.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Tweet => "tweet",
            Self::Image => "image",
            Self::Document => "document",
            Self::Link => "link",
            Self::RawText => "raw_text",
        }
    }

    /// Detect content type from URL.
    pub fn from_url(url: &str) -> Self {
        let lower = url.to_lowercase();
        if lower.contains("twitter.com") || lower.contains("x.com") || lower.contains("weibo.com")
        {
            Self::Tweet
        } else if lower.ends_with(".pdf") {
            Self::Document
        } else if lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".gif")
            || lower.ends_with(".webp")
        {
            Self::Image
        } else {
            Self::Article
        }
    }
}

/// Location where the asset was saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLocation {
    /// Platform name (feishu_docs, notion)
    pub platform: String,
    /// URL to the saved document/page
    pub url: String,
    /// Title of the document
    pub title: String,
}

/// A captured asset with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedAsset {
    /// Unique asset ID
    pub id: String,
    /// Source channel type
    pub source_channel: ChannelType,
    /// Source user ID
    pub source_user: String,
    /// Original URL (if any)
    pub original_url: Option<String>,
    /// Content type
    pub content_type: AssetContentType,
    /// Raw content (extracted text)
    pub raw_content: String,
    /// LLM-generated summary
    pub summary: String,
    /// LLM-generated tags
    pub tags: Vec<String>,
    /// Category
    pub category: Option<String>,
    /// Key points
    pub key_points: Vec<String>,
    /// Capture timestamp
    pub captured_at: DateTime<Utc>,
    /// Where the asset was saved
    pub saved_to: Vec<SavedLocation>,
}

/// Result from LLM summarization.
#[derive(Debug, Clone, Deserialize)]
pub struct SummaryResult {
    pub summary: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub key_points: Vec<String>,
    #[serde(default)]
    pub action_items: Vec<String>,
}

/// Extracted content from a URL.
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    pub title: String,
    pub content: String,
    pub images: Vec<String>,
    pub url: String,
}

// ============================================================================
// Token Cache
// ============================================================================

struct TokenCache {
    token: String,
    expires_at: Instant,
}

// ============================================================================
// Feishu Docs Client
// ============================================================================

/// Client for Feishu Docs API.
pub struct FeishuDocsClient {
    app_id: String,
    app_secret: String,
    folder_token: String,
    #[allow(dead_code)]
    template_id: Option<String>,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl FeishuDocsClient {
    /// Create a new Feishu Docs client.
    pub fn new(
        app_id: String,
        app_secret: String,
        folder_token: String,
        template_id: Option<String>,
    ) -> Self {
        Self {
            app_id,
            app_secret,
            folder_token,
            template_id,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or refresh the tenant access token.
    async fn get_access_token(&self) -> Result<String> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                let now = Instant::now();
                if cached.expires_at > now + Duration::from_secs(TOKEN_REFRESH_MARGIN_SECS) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Refresh token
        let url = format!("{}/auth/v3/tenant_access_token/internal", FEISHU_API_BASE);
        let body = serde_json::json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret
        });

        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get Feishu access token ({status}): {text}");
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            code: i32,
            msg: String,
            tenant_access_token: Option<String>,
            expire: Option<u64>,
        }

        let data: TokenResponse = resp.json().await?;

        if data.code != 0 {
            anyhow::bail!("Feishu API error ({}): {}", data.code, data.msg);
        }

        let token = data
            .tenant_access_token
            .ok_or_else(|| anyhow::anyhow!("Missing tenant_access_token in response"))?;
        let expire = data.expire.unwrap_or(7200);

        // Update cache
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(TokenCache {
                token: token.clone(),
                expires_at: Instant::now() + Duration::from_secs(expire),
            });
        }

        tracing::debug!(
            "Feishu Docs access token refreshed, expires in {} seconds",
            expire
        );
        Ok(token)
    }

    /// Create a document and return the URL.
    ///
    /// API: https://open.feishu.cn/document/server-docs/docs/docs/docx-v1/document/create
    pub async fn create_document(&self, title: &str, content: &str) -> Result<SavedLocation> {
        let token = self.get_access_token().await?;

        // Step 1: Create the document
        let url = format!("{}/docx/v1/documents", FEISHU_API_BASE);

        let body = serde_json::json!({
            "folder_token": self.folder_token,
            "title": title
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create Feishu document ({status}): {text}");
        }

        #[derive(Deserialize)]
        struct CreateDocResponse {
            code: i32,
            msg: String,
            data: Option<CreateDocData>,
        }

        #[derive(Deserialize)]
        struct CreateDocData {
            document: DocumentInfo,
        }

        #[derive(Deserialize)]
        struct DocumentInfo {
            document_id: String,
        }

        let create_resp: CreateDocResponse = resp.json().await?;

        if create_resp.code != 0 {
            anyhow::bail!(
                "Feishu create document error ({}): {}",
                create_resp.code,
                create_resp.msg
            );
        }

        let document_id = create_resp
            .data
            .ok_or_else(|| anyhow::anyhow!("Missing data in create document response"))?
            .document
            .document_id;

        // Step 2: Insert content blocks
        self.insert_content(&token, &document_id, content).await?;

        let doc_url = format!("https://bytedance.feishu.cn/docx/{}", document_id);

        tracing::info!("Created Feishu document: {}", doc_url);

        Ok(SavedLocation {
            platform: "feishu_docs".to_string(),
            url: doc_url,
            title: title.to_string(),
        })
    }

    /// Insert content blocks into a document.
    async fn insert_content(
        &self,
        token: &str,
        document_id: &str,
        content: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/docx/v1/documents/{}/blocks/{}/children",
            FEISHU_API_BASE, document_id, document_id
        );

        // Build text blocks
        let blocks = self.build_document_blocks(content);

        let body = serde_json::json!({
            "children": blocks,
            "index": 0
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!(
                "Failed to insert content into Feishu document ({status}): {text}"
            );
            // Don't fail - document was created successfully
        }

        Ok(())
    }

    /// Build document blocks from content.
    fn build_document_blocks(&self, content: &str) -> Vec<serde_json::Value> {
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let mut blocks = Vec::new();

        for para in paragraphs {
            if para.trim().is_empty() {
                continue;
            }

            let block = serde_json::json!({
                "block_type": 2,  // Text block
                "text": {
                    "elements": [{
                        "text_run": {
                            "content": para.trim()
                        }
                    }],
                    "style": {}
                }
            });
            blocks.push(block);
        }

        blocks
    }
}

// ============================================================================
// Notion Client
// ============================================================================

/// Client for Notion API.
pub struct NotionClient {
    token: String,
    database_id: String,
    client: reqwest::Client,
}

impl NotionClient {
    /// Create a new Notion client.
    pub fn new(token: String, database_id: String) -> Self {
        Self {
            token,
            database_id,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Create a Notion page.
    ///
    /// API: https://developers.notion.com/reference/post-page
    pub async fn create_page(&self, asset: &CapturedAsset) -> Result<SavedLocation> {
        let url = format!("{}/pages", NOTION_API_BASE);

        // Build page properties
        let title = if asset.summary.len() > 50 {
            format!("{}...", &asset.summary[..47])
        } else {
            asset.summary.clone()
        };

        let tags: Vec<serde_json::Value> = asset
            .tags
            .iter()
            .map(|t| serde_json::json!({ "name": t }))
            .collect();

        let body = serde_json::json!({
            "parent": { "database_id": self.database_id },
            "properties": {
                "Name": {
                    "title": [{ "text": { "content": title } }]
                },
                "Tags": {
                    "multi_select": tags
                },
                "Source": {
                    "url": asset.original_url.clone()
                },
                "Captured": {
                    "date": { "start": asset.captured_at.to_rfc3339() }
                }
            },
            "children": self.build_page_content(asset)
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create Notion page ({status}): {text}");
        }

        #[derive(Deserialize)]
        struct PageResponse {
            url: String,
        }

        let page: PageResponse = resp.json().await?;

        tracing::info!("Created Notion page: {}", page.url);

        Ok(SavedLocation {
            platform: "notion".to_string(),
            url: page.url,
            title,
        })
    }

    /// Build Notion page content blocks.
    fn build_page_content(&self, asset: &CapturedAsset) -> Vec<serde_json::Value> {
        let mut blocks = Vec::new();

        // Summary heading
        blocks.push(serde_json::json!({
            "object": "block",
            "type": "heading_2",
            "heading_2": {
                "rich_text": [{ "type": "text", "text": { "content": "Summary" } }]
            }
        }));

        // Summary paragraph
        blocks.push(serde_json::json!({
            "object": "block",
            "type": "paragraph",
            "paragraph": {
                "rich_text": [{ "type": "text", "text": { "content": asset.summary.clone() } }]
            }
        }));

        // Key points if any
        if !asset.key_points.is_empty() {
            blocks.push(serde_json::json!({
                "object": "block",
                "type": "heading_2",
                "heading_2": {
                    "rich_text": [{ "type": "text", "text": { "content": "Key Points" } }]
                }
            }));

            for point in &asset.key_points {
                blocks.push(serde_json::json!({
                    "object": "block",
                    "type": "bulleted_list_item",
                    "bulleted_list_item": {
                        "rich_text": [{ "type": "text", "text": { "content": point } }]
                    }
                }));
            }
        }

        // Original content (truncated)
        blocks.push(serde_json::json!({
            "object": "block",
            "type": "heading_2",
            "heading_2": {
                "rich_text": [{ "type": "text", "text": { "content": "Original Content" } }]
            }
        }));

        let truncated_content = if asset.raw_content.len() > 2000 {
            format!("{}...\n\n[Content truncated]", &asset.raw_content[..2000])
        } else {
            asset.raw_content.clone()
        };

        blocks.push(serde_json::json!({
            "object": "block",
            "type": "paragraph",
            "paragraph": {
                "rich_text": [{ "type": "text", "text": { "content": truncated_content } }]
            }
        }));

        blocks
    }
}

// ============================================================================
// Capture Bridge
// ============================================================================

/// Bridge for capturing content from IM channels.
pub struct CaptureBridge {
    config: CaptureConfig,
    codecoder_endpoint: String,
    client: reqwest::Client,
    feishu_docs: Option<FeishuDocsClient>,
    notion: Option<NotionClient>,
    /// In-memory cache of captured assets (for history)
    captured_assets: Arc<RwLock<Vec<CapturedAsset>>>,
}

impl CaptureBridge {
    /// Create a new capture bridge.
    pub fn new(config: CaptureConfig, codecoder_endpoint: String) -> Self {
        let feishu_docs = config.feishu_docs.as_ref().map(|f| {
            FeishuDocsClient::new(
                f.app_id.clone(),
                f.app_secret.clone(),
                f.folder_token.clone(),
                f.template_id.clone(),
            )
        });

        let notion = config
            .notion
            .as_ref()
            .map(|n| NotionClient::new(n.token.clone(), n.database_id.clone()));

        Self {
            config,
            codecoder_endpoint,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            feishu_docs,
            notion,
            captured_assets: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if capture is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a message is capturable (forwarded, contains link, or trigger prefix).
    pub fn is_capturable(&self, message: &ChannelMessage) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Check if forwarded message
        if self.config.auto_capture.capture_forwarded
            && (message.metadata.contains_key("forward_from")
                || message.metadata.contains_key("forward_from_chat"))
            {
                return true;
            }

        // Check message content
        if let MessageContent::Text { text } = &message.content {
            // Check if contains URL and auto-capture links is enabled
            if self.config.auto_capture.capture_links && self.contains_url(text) {
                return true;
            }

            // Check trigger prefixes
            for prefix in &self.config.auto_capture.trigger_prefixes {
                if text.starts_with(prefix) {
                    return true;
                }
            }
        }

        // Check if has document/image attachments
        !message.attachments.is_empty()
    }

    /// Check if the message is a capture request (contains capture keywords).
    pub fn is_capture_request(&self, message: &ChannelMessage) -> bool {
        if let MessageContent::Text { text } = &message.content {
            let lower = text.to_lowercase();
            lower.contains("收藏")
                || lower.contains("保存")
                || lower.contains("capture")
                || lower.starts_with("#save")
                || lower.starts_with("@save")
                || lower.starts_with("#收藏")
        } else {
            false
        }
    }

    /// Check if text contains a URL.
    fn contains_url(&self, text: &str) -> bool {
        if let Ok(regex) = Regex::new(r"https?://[^\s]+") {
            regex.is_match(text)
        } else {
            text.contains("http://") || text.contains("https://")
        }
    }

    /// Extract URL from text.
    fn extract_url(&self, text: &str) -> Option<String> {
        if let Ok(regex) = Regex::new(r"https?://[^\s]+") {
            regex.find(text).map(|m| m.as_str().to_string())
        } else {
            None
        }
    }

    /// Capture content from a message.
    pub async fn capture(&self, message: &ChannelMessage) -> Result<CapturedAsset> {
        let text = match &message.content {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Image { caption, .. } => {
                caption.clone().unwrap_or_else(|| "[Image]".to_string())
            }
            MessageContent::File { filename, .. } => {
                format!("[Document: {}]", filename)
            }
            _ => "[Content]".to_string(),
        };

        // Extract URL if present
        let url = self.extract_url(&text);

        // Determine content type
        let content_type = if let Some(ref u) = url {
            AssetContentType::from_url(u)
        } else if !message.attachments.is_empty() {
            AssetContentType::Document
        } else {
            AssetContentType::RawText
        };

        // Extract content
        let raw_content = if let Some(ref u) = url {
            match self.extract_link_content(u).await {
                Ok(extracted) => {
                    format!("# {}\n\n{}", extracted.title, extracted.content)
                }
                Err(e) => {
                    tracing::warn!("Failed to extract content from URL: {}", e);
                    text.clone()
                }
            }
        } else {
            text.clone()
        };

        // Generate summary and tags using LLM
        let summary_result = self.summarize_and_tag(&raw_content, &content_type).await?;

        // Create asset
        let mut asset = CapturedAsset {
            id: uuid::Uuid::new_v4().to_string(),
            source_channel: message.channel_type,
            source_user: message.user_id.clone(),
            original_url: url,
            content_type,
            raw_content,
            summary: summary_result.summary,
            tags: summary_result.tags,
            category: summary_result.category,
            key_points: summary_result.key_points,
            captured_at: Utc::now(),
            saved_to: Vec::new(),
        };

        // Save to configured destinations
        if let Some(ref feishu_docs) = self.feishu_docs {
            match feishu_docs
                .create_document(&asset.summary, &self.format_document_content(&asset))
                .await
            {
                Ok(location) => {
                    asset.saved_to.push(location);
                }
                Err(e) => {
                    tracing::error!("Failed to save to Feishu Docs: {}", e);
                }
            }
        }

        if let Some(ref notion) = self.notion {
            match notion.create_page(&asset).await {
                Ok(location) => {
                    asset.saved_to.push(location);
                }
                Err(e) => {
                    tracing::error!("Failed to save to Notion: {}", e);
                }
            }
        }

        // Cache the asset
        {
            let mut assets = self.captured_assets.write().await;
            assets.push(asset.clone());
            // Keep only last 100 assets
            if assets.len() > 100 {
                assets.remove(0);
            }
        }

        Ok(asset)
    }

    /// Format document content for saving.
    fn format_document_content(&self, asset: &CapturedAsset) -> String {
        let mut content = String::new();

        content.push_str(&format!("## Summary\n\n{}\n\n", asset.summary));

        if !asset.tags.is_empty() {
            content.push_str(&format!("## Tags\n\n{}\n\n", asset.tags.join(", ")));
        }

        if !asset.key_points.is_empty() {
            content.push_str("## Key Points\n\n");
            for point in &asset.key_points {
                content.push_str(&format!("- {}\n", point));
            }
            content.push('\n');
        }

        if let Some(ref url) = asset.original_url {
            content.push_str(&format!("## Source\n\n{}\n\n", url));
        }

        content.push_str("## Original Content\n\n");
        content.push_str(&asset.raw_content);

        content
    }

    /// Extract content from a URL.
    async fn extract_link_content(&self, url: &str) -> Result<ExtractedContent> {
        let response = self
            .client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 ZeroBot/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch URL: {}", response.status());
        }

        let html = response.text().await?;

        // Simple content extraction
        let title = self.extract_title(&html);
        let content = self.extract_main_content(&html);
        let images = self.extract_images(&html);

        Ok(ExtractedContent {
            title,
            content,
            images,
            url: url.to_string(),
        })
    }

    /// Extract title from HTML.
    fn extract_title(&self, html: &str) -> String {
        // Try <title> tag
        if let Ok(regex) = Regex::new(r"<title[^>]*>([^<]+)</title>") {
            if let Some(caps) = regex.captures(html) {
                if let Some(title) = caps.get(1) {
                    return html_escape::decode_html_entities(title.as_str())
                        .to_string()
                        .trim()
                        .to_string();
                }
            }
        }

        // Try og:title
        if let Ok(regex) = Regex::new(r#"<meta[^>]*property="og:title"[^>]*content="([^"]+)""#) {
            if let Some(caps) = regex.captures(html) {
                if let Some(title) = caps.get(1) {
                    return html_escape::decode_html_entities(title.as_str())
                        .to_string()
                        .trim()
                        .to_string();
                }
            }
        }

        "Untitled".to_string()
    }

    /// Extract main content from HTML.
    fn extract_main_content(&self, html: &str) -> String {
        // Remove script and style tags
        let mut content = html.to_string();

        if let Ok(regex) = Regex::new(r"(?is)<script[^>]*>.*?</script>") {
            content = regex.replace_all(&content, "").to_string();
        }

        if let Ok(regex) = Regex::new(r"(?is)<style[^>]*>.*?</style>") {
            content = regex.replace_all(&content, "").to_string();
        }

        // Try to find article or main content
        let content_patterns = [
            r"(?is)<article[^>]*>(.*?)</article>",
            r"(?is)<main[^>]*>(.*?)</main>",
            r#"(?is)<div[^>]*class="[^"]*content[^"]*"[^>]*>(.*?)</div>"#,
            r"(?is)<body[^>]*>(.*?)</body>",
        ];

        for pattern in &content_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(caps) = regex.captures(&content) {
                    if let Some(matched) = caps.get(1) {
                        content = matched.as_str().to_string();
                        break;
                    }
                }
            }
        }

        // Remove all HTML tags
        if let Ok(regex) = Regex::new(r"<[^>]+>") {
            content = regex.replace_all(&content, " ").to_string();
        }

        // Clean up whitespace
        if let Ok(regex) = Regex::new(r"\s+") {
            content = regex.replace_all(&content, " ").to_string();
        }

        // Decode HTML entities and trim
        html_escape::decode_html_entities(&content)
            .to_string()
            .trim()
            .to_string()
    }

    /// Extract images from HTML.
    fn extract_images(&self, html: &str) -> Vec<String> {
        let mut images = Vec::new();

        if let Ok(regex) = Regex::new(r#"<img[^>]*src="([^"]+)""#) {
            for caps in regex.captures_iter(html) {
                if let Some(src) = caps.get(1) {
                    images.push(src.as_str().to_string());
                }
            }
        }

        images
    }

    /// Use LLM to generate summary and tags.
    async fn summarize_and_tag(
        &self,
        content: &str,
        content_type: &AssetContentType,
    ) -> Result<SummaryResult> {
        let url = format!("{}/api/v1/chat", self.codecoder_endpoint);

        // Truncate content for LLM
        let truncated = if content.len() > MAX_CONTENT_LENGTH {
            format!("{}...[truncated]", &content[..MAX_CONTENT_LENGTH])
        } else {
            content.to_string()
        };

        let prompt = format!(
            r#"你是一个内容分析助手。请分析以下内容并生成摘要和标签。

内容类型: {}
原始内容:
{}

请按以下 JSON 格式输出（直接输出 JSON，不要有其他文字）:
{{
  "summary": "100字以内的核心摘要",
  "tags": ["标签1", "标签2", "标签3"],
  "category": "分类（如：技术、产品、设计、商业、生活）",
  "key_points": ["要点1", "要点2"],
  "action_items": []
}}"#,
            content_type.as_str(),
            truncated
        );

        let request_body = serde_json::json!({
            "message": prompt,
            "agent": "general",
            "user_id": "capture_bridge",
            "channel": "internal"
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .timeout(Duration::from_secs(120))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API failed ({status}): {text}");
        }

        #[derive(Deserialize)]
        struct ChatResponse {
            message: String,
        }

        let chat_resp: ChatResponse = response.json().await?;

        // Extract JSON from response
        self.extract_summary_result(&chat_resp.message)
    }

    /// Extract SummaryResult from LLM response.
    fn extract_summary_result(&self, response: &str) -> Result<SummaryResult> {
        // Try to find JSON in response
        if let Ok(regex) = Regex::new(r"\{[\s\S]*\}") {
            if let Some(m) = regex.find(response) {
                if let Ok(result) = serde_json::from_str::<SummaryResult>(m.as_str()) {
                    return Ok(result);
                }
            }
        }

        // Fallback: try parsing the whole response
        if let Ok(result) = serde_json::from_str::<SummaryResult>(response) {
            return Ok(result);
        }

        // Final fallback: create a basic summary
        Ok(SummaryResult {
            summary: response.chars().take(100).collect::<String>(),
            tags: vec!["auto-captured".to_string()],
            category: None,
            key_points: vec![],
            action_items: vec![],
        })
    }

    /// Get captured assets history.
    pub async fn get_history(&self, limit: usize, offset: usize) -> Vec<CapturedAsset> {
        let assets = self.captured_assets.read().await;
        assets
            .iter()
            .rev()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get a captured asset by ID.
    pub async fn get_asset(&self, asset_id: &str) -> Option<CapturedAsset> {
        let assets = self.captured_assets.read().await;
        assets.iter().find(|a| a.id == asset_id).cloned()
    }

    /// Save an existing asset to a new destination.
    pub async fn save_to_destination(
        &self,
        asset_id: &str,
        destination: &str,
    ) -> Result<SavedLocation> {
        let asset = self
            .get_asset(asset_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

        let location = match destination {
            "feishu_docs" => {
                let client = self
                    .feishu_docs
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Feishu Docs not configured"))?;

                client
                    .create_document(&asset.summary, &self.format_document_content(&asset))
                    .await?
            }
            "notion" => {
                let client = self
                    .notion
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Notion not configured"))?;

                client.create_page(&asset).await?
            }
            _ => {
                anyhow::bail!("Unknown destination: {}", destination);
            }
        };

        // Update asset in cache
        {
            let mut assets = self.captured_assets.write().await;
            if let Some(a) = assets.iter_mut().find(|a| a.id == asset_id) {
                a.saved_to.push(location.clone());
            }
        }

        Ok(location)
    }

    /// Format capture response for IM display.
    pub fn format_capture_response(&self, asset: &CapturedAsset) -> String {
        let mut lines = vec![
            "📥 **已捕获内容**".to_string(),
            String::new(),
            format!("📝 **摘要**: {}", asset.summary),
            String::new(),
        ];

        if !asset.tags.is_empty() {
            lines.push(format!("🏷️ **标签**: {}", asset.tags.join(", ")));
            lines.push(String::new());
        }

        if !asset.key_points.is_empty() {
            lines.push("📌 **要点**:".to_string());
            for point in &asset.key_points {
                lines.push(format!("  • {}", point));
            }
            lines.push(String::new());
        }

        if !asset.saved_to.is_empty() {
            lines.push("💾 **已保存到**:".to_string());
            for loc in &asset.saved_to {
                lines.push(format!("  • [{}]({})", loc.platform, loc.url));
            }
        }

        lines.join("\n")
    }

    /// Capture content from a URL directly (for API calls).
    pub async fn capture_url(
        &self,
        url: &str,
        tags: Option<Vec<String>>,
        destination: Option<&str>,
    ) -> Result<CapturedAsset> {
        // Determine content type
        let content_type = AssetContentType::from_url(url);

        // Extract content
        let extracted = self.extract_link_content(url).await?;
        let raw_content = format!("# {}\n\n{}", extracted.title, extracted.content);

        // Generate summary and tags using LLM
        let mut summary_result = self.summarize_and_tag(&raw_content, &content_type).await?;

        // Merge provided tags
        if let Some(extra_tags) = tags {
            summary_result.tags.extend(extra_tags);
        }

        // Create asset
        let mut asset = CapturedAsset {
            id: uuid::Uuid::new_v4().to_string(),
            source_channel: ChannelType::Cli,
            source_user: "api".to_string(),
            original_url: Some(url.to_string()),
            content_type,
            raw_content,
            summary: summary_result.summary,
            tags: summary_result.tags,
            category: summary_result.category,
            key_points: summary_result.key_points,
            captured_at: Utc::now(),
            saved_to: Vec::new(),
        };

        // Save to specified destination or all configured
        let destinations = destination
            .map(|d| vec![d])
            .unwrap_or_else(|| {
                let mut dests = Vec::new();
                if self.feishu_docs.is_some() {
                    dests.push("feishu_docs");
                }
                if self.notion.is_some() {
                    dests.push("notion");
                }
                dests
            });

        for dest in destinations {
            match dest {
                "feishu_docs" => {
                    if let Some(ref client) = self.feishu_docs {
                        match client
                            .create_document(&asset.summary, &self.format_document_content(&asset))
                            .await
                        {
                            Ok(location) => {
                                asset.saved_to.push(location);
                            }
                            Err(e) => {
                                tracing::error!("Failed to save to Feishu Docs: {}", e);
                            }
                        }
                    }
                }
                "notion" => {
                    if let Some(ref client) = self.notion {
                        match client.create_page(&asset).await {
                            Ok(location) => {
                                asset.saved_to.push(location);
                            }
                            Err(e) => {
                                tracing::error!("Failed to save to Notion: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Cache the asset
        {
            let mut assets = self.captured_assets.write().await;
            assets.push(asset.clone());
            if assets.len() > 100 {
                assets.remove(0);
            }
        }

        Ok(asset)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> CaptureConfig {
        CaptureConfig {
            enabled: true,
            feishu_docs: None,
            notion: None,
            auto_capture: zero_common::config::AutoCaptureConfig {
                capture_forwarded: true,
                capture_links: true,
                trigger_prefixes: vec!["#收藏".to_string(), "#save".to_string()],
            },
        }
    }

    fn create_test_message(text: &str) -> ChannelMessage {
        ChannelMessage {
            id: "test-msg-1".into(),
            channel_type: ChannelType::Telegram,
            channel_id: "123456".into(),
            user_id: "user1".into(),
            content: MessageContent::Text { text: text.into() },
            attachments: vec![],
            metadata: HashMap::new(),
            timestamp: 1234567890000,
            trace_id: "test-trace-123".into(),
            span_id: "test-span".into(),
            parent_span_id: None,
        }
    }

    fn create_forwarded_message(text: &str) -> ChannelMessage {
        let mut msg = create_test_message(text);
        msg.metadata.insert("forward_from".into(), "user2".into());
        msg
    }

    #[test]
    fn test_asset_content_type_from_url() {
        assert_eq!(
            AssetContentType::from_url("https://twitter.com/user/status/123"),
            AssetContentType::Tweet
        );
        assert_eq!(
            AssetContentType::from_url("https://x.com/user/status/123"),
            AssetContentType::Tweet
        );
        assert_eq!(
            AssetContentType::from_url("https://example.com/doc.pdf"),
            AssetContentType::Document
        );
        assert_eq!(
            AssetContentType::from_url("https://example.com/image.png"),
            AssetContentType::Image
        );
        assert_eq!(
            AssetContentType::from_url("https://example.com/article"),
            AssetContentType::Article
        );
    }

    #[test]
    fn test_is_capturable_forwarded() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let msg = create_forwarded_message("Some forwarded content");
        assert!(bridge.is_capturable(&msg));
    }

    #[test]
    fn test_is_capturable_with_link() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let msg = create_test_message("Check this out: https://example.com/article");
        assert!(bridge.is_capturable(&msg));
    }

    #[test]
    fn test_is_capturable_with_trigger_prefix() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let msg = create_test_message("#收藏 这篇文章很好");
        assert!(bridge.is_capturable(&msg));

        let msg2 = create_test_message("#save this article");
        assert!(bridge.is_capturable(&msg2));
    }

    #[test]
    fn test_is_capturable_regular_message() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let msg = create_test_message("Hello, how are you?");
        assert!(!bridge.is_capturable(&msg));
    }

    #[test]
    fn test_is_capture_request() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        assert!(bridge.is_capture_request(&create_test_message("请收藏这篇文章")));
        assert!(bridge.is_capture_request(&create_test_message("保存这个链接")));
        assert!(bridge.is_capture_request(&create_test_message("#save")));
        assert!(bridge.is_capture_request(&create_test_message("@save this")));
        assert!(bridge.is_capture_request(&create_test_message("#收藏 好文")));

        assert!(!bridge.is_capture_request(&create_test_message("Hello world")));
    }

    #[test]
    fn test_extract_url() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        assert_eq!(
            bridge.extract_url("Check this out: https://example.com/article"),
            Some("https://example.com/article".to_string())
        );
        assert_eq!(
            bridge.extract_url("Visit http://test.com for more"),
            Some("http://test.com".to_string())
        );
        assert_eq!(bridge.extract_url("No URL here"), None);
    }

    #[test]
    fn test_extract_title() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let html = r#"<html><head><title>Test Title</title></head><body></body></html>"#;
        assert_eq!(bridge.extract_title(html), "Test Title");

        let html_og = r#"<html><head><meta property="og:title" content="OG Title"/></head></html>"#;
        assert_eq!(bridge.extract_title(html_og), "OG Title");

        let html_empty = r#"<html><body>No title</body></html>"#;
        assert_eq!(bridge.extract_title(html_empty), "Untitled");
    }

    #[test]
    fn test_extract_main_content() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let html = r#"
            <html>
            <head><script>var x = 1;</script><style>body{}</style></head>
            <body>
                <article>
                    <p>Main content here</p>
                </article>
            </body>
            </html>
        "#;

        let content = bridge.extract_main_content(html);
        assert!(content.contains("Main content here"));
        assert!(!content.contains("var x"));
        assert!(!content.contains("body{}"));
    }

    #[test]
    fn test_extract_summary_result() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let response = r#"
        Here is the analysis:
        {
            "summary": "This is a test summary",
            "tags": ["test", "example"],
            "category": "技术",
            "key_points": ["Point 1", "Point 2"],
            "action_items": []
        }
        "#;

        let result = bridge.extract_summary_result(response).unwrap();
        assert_eq!(result.summary, "This is a test summary");
        assert_eq!(result.tags, vec!["test", "example"]);
        assert_eq!(result.category, Some("技术".to_string()));
        assert_eq!(result.key_points.len(), 2);
    }

    #[test]
    fn test_format_capture_response() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        let asset = CapturedAsset {
            id: "test-id".into(),
            source_channel: ChannelType::Telegram,
            source_user: "user1".into(),
            original_url: Some("https://example.com".into()),
            content_type: AssetContentType::Article,
            raw_content: "Test content".into(),
            summary: "Test summary".into(),
            tags: vec!["tag1".into(), "tag2".into()],
            category: Some("技术".into()),
            key_points: vec!["Point 1".into()],
            captured_at: Utc::now(),
            saved_to: vec![SavedLocation {
                platform: "feishu_docs".into(),
                url: "https://feishu.cn/doc/xxx".into(),
                title: "Test Doc".into(),
            }],
        };

        let formatted = bridge.format_capture_response(&asset);

        assert!(formatted.contains("已捕获内容"));
        assert!(formatted.contains("Test summary"));
        assert!(formatted.contains("tag1, tag2"));
        assert!(formatted.contains("Point 1"));
        assert!(formatted.contains("feishu_docs"));
    }

    #[test]
    fn test_disabled_capture() {
        let mut config = create_test_config();
        config.enabled = false;
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        assert!(!bridge.is_enabled());
        assert!(!bridge.is_capturable(&create_forwarded_message("Test")));
    }

    #[tokio::test]
    async fn test_get_history() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        // Add some test assets
        {
            let mut assets = bridge.captured_assets.write().await;
            for i in 0..5 {
                assets.push(CapturedAsset {
                    id: format!("asset-{}", i),
                    source_channel: ChannelType::Telegram,
                    source_user: "user1".into(),
                    original_url: None,
                    content_type: AssetContentType::RawText,
                    raw_content: format!("Content {}", i),
                    summary: format!("Summary {}", i),
                    tags: vec![],
                    category: None,
                    key_points: vec![],
                    captured_at: Utc::now(),
                    saved_to: vec![],
                });
            }
        }

        let history = bridge.get_history(3, 0).await;
        assert_eq!(history.len(), 3);
        // Should be in reverse order (newest first)
        assert_eq!(history[0].id, "asset-4");
        assert_eq!(history[2].id, "asset-2");

        let history_offset = bridge.get_history(2, 2).await;
        assert_eq!(history_offset.len(), 2);
        assert_eq!(history_offset[0].id, "asset-2");
    }

    #[tokio::test]
    async fn test_get_asset() {
        let config = create_test_config();
        let bridge = CaptureBridge::new(config, "http://localhost:4400".to_string());

        {
            let mut assets = bridge.captured_assets.write().await;
            assets.push(CapturedAsset {
                id: "test-asset-123".into(),
                source_channel: ChannelType::Telegram,
                source_user: "user1".into(),
                original_url: None,
                content_type: AssetContentType::RawText,
                raw_content: "Test content".into(),
                summary: "Test summary".into(),
                tags: vec![],
                category: None,
                key_points: vec![],
                captured_at: Utc::now(),
                saved_to: vec![],
            });
        }

        let asset = bridge.get_asset("test-asset-123").await;
        assert!(asset.is_some());
        assert_eq!(asset.unwrap().summary, "Test summary");

        let not_found = bridge.get_asset("non-existent").await;
        assert!(not_found.is_none());
    }
}
