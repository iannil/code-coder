//! Provider-specific message transformation
//!
//! This module handles normalization and transformation of messages
//! for different AI providers (Anthropic, Mistral, OpenAI, etc.).

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

// ============================================================================
// Types
// ============================================================================

/// A message in the AI SDK format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMessage {
    pub role: String,
    pub content: ProviderMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<Value>,
}

/// Message content can be a string or array of parts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderMessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

/// A single content part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub part_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<Value>,
    #[serde(flatten)]
    pub extra: Value,
}

/// Model information for transformation decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub provider_id: String,
    pub api_id: String,
    pub api_npm: String,
    #[serde(default)]
    pub has_interleaved_field: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interleaved_field: Option<String>,
}

/// Result of message normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizeResult {
    pub messages: Vec<Value>,
    pub strategy: String,
    pub modifications: u32,
}

/// Result of cache application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheResult {
    pub messages: Vec<Value>,
    pub marked_count: u32,
}

// ============================================================================
// Regex patterns (compiled once)
// ============================================================================

/// Regex for Claude tool call ID normalization (keep only alphanumeric, underscore, hyphen)
fn claude_tool_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^a-zA-Z0-9_-]").unwrap())
}

/// Regex for Mistral tool call ID normalization (keep only alphanumeric)
fn mistral_tool_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^a-zA-Z0-9]").unwrap())
}

// ============================================================================
// Message Normalization
// ============================================================================

/// Normalize messages for a specific provider
///
/// # Arguments
/// * `messages_json` - JSON string of messages array
/// * `model_json` - JSON string of model info
///
/// # Returns
/// JSON string of NormalizeResult
pub fn normalize_messages(messages_json: &str, model_json: &str) -> Result<String, String> {
    let messages: Vec<Value> = serde_json::from_str(messages_json)
        .map_err(|e| format!("Failed to parse messages: {}", e))?;

    let model: ModelInfo = serde_json::from_str(model_json)
        .map_err(|e| format!("Failed to parse model info: {}", e))?;

    let (normalized, strategy, modifications) = normalize_messages_impl(messages, &model);

    let result = NormalizeResult {
        messages: normalized,
        strategy,
        modifications,
    };

    serde_json::to_string(&result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

fn normalize_messages_impl(
    messages: Vec<Value>,
    model: &ModelInfo,
) -> (Vec<Value>, String, u32) {
    let mut msgs = messages;
    let mut total_modifications = 0u32;
    let mut strategies = Vec::new();

    // Anthropic normalization: filter empty content
    if model.api_npm == "@ai-sdk/anthropic" {
        let (filtered, count) = filter_anthropic_messages(msgs);
        msgs = filtered;
        total_modifications += count;
        if count > 0 {
            strategies.push("anthropic_empty_filter");
        }
    }

    // Claude tool ID normalization (can happen with Anthropic SDK)
    if model.api_id.contains("claude") {
        let (normalized, count) = normalize_claude_tool_ids(msgs);
        msgs = normalized;
        total_modifications += count;
        if count > 0 {
            strategies.push("claude_tool_id");
        }
    }
    // Mistral normalization: tool IDs + message sequence fixing
    else if model.provider_id == "mistral" || model.api_id.to_lowercase().contains("mistral") {
        let (normalized, count) = normalize_mistral_messages(msgs);
        msgs = normalized;
        total_modifications += count;
        if count > 0 {
            strategies.push("mistral");
        }
    }
    // Interleaved reasoning field handling
    else if model.has_interleaved_field {
        if let Some(field) = &model.interleaved_field {
            let (normalized, count) = normalize_interleaved_reasoning(msgs, field);
            msgs = normalized;
            total_modifications += count;
            if count > 0 {
                strategies.push("interleaved_reasoning");
            }
        }
    }

    let strategy = if strategies.is_empty() {
        "passthrough".to_string()
    } else {
        strategies.join("+")
    };

    (msgs, strategy, total_modifications)
}

/// Filter out empty content for Anthropic
fn filter_anthropic_messages(messages: Vec<Value>) -> (Vec<Value>, u32) {
    let mut filtered = Vec::with_capacity(messages.len());
    let mut removed = 0u32;

    for msg in messages {
        if let Some(content) = msg.get("content") {
            // String content
            if let Some(s) = content.as_str() {
                if s.is_empty() {
                    removed += 1;
                    continue;
                }
                filtered.push(msg);
                continue;
            }

            // Array content - filter out empty text/reasoning parts
            if let Some(arr) = content.as_array() {
                let filtered_parts: Vec<Value> = arr.iter()
                    .filter(|part| {
                        let part_type = part.get("type").and_then(|t| t.as_str());
                        if matches!(part_type, Some("text") | Some("reasoning")) {
                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                return !text.is_empty();
                            }
                            return false;
                        }
                        true
                    })
                    .cloned()
                    .collect();

                if filtered_parts.is_empty() {
                    removed += 1;
                    continue;
                }

                if filtered_parts.len() != arr.len() {
                    removed += 1;
                }

                let mut new_msg = msg.clone();
                new_msg["content"] = Value::Array(filtered_parts);
                filtered.push(new_msg);
                continue;
            }
        }
        filtered.push(msg);
    }

    (filtered, removed)
}

/// Normalize tool call IDs for Claude
fn normalize_claude_tool_ids(messages: Vec<Value>) -> (Vec<Value>, u32) {
    let re = claude_tool_id_regex();
    let mut modified = 0u32;

    let normalized: Vec<Value> = messages.into_iter().map(|mut msg| {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");

        if matches!(role, "assistant" | "tool") {
            if let Some(content) = msg.get_mut("content") {
                if let Some(arr) = content.as_array_mut() {
                    for part in arr.iter_mut() {
                        let part_type = part.get("type").and_then(|t| t.as_str());
                        if matches!(part_type, Some("tool-call") | Some("tool-result")) {
                            if let Some(tool_call_id) = part.get("toolCallId").and_then(|t| t.as_str()) {
                                let normalized_id = re.replace_all(tool_call_id, "_").to_string();
                                if normalized_id != tool_call_id {
                                    part["toolCallId"] = Value::String(normalized_id);
                                    modified += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        msg
    }).collect();

    (normalized, modified)
}

/// Normalize messages for Mistral (tool IDs + sequence fixing)
fn normalize_mistral_messages(messages: Vec<Value>) -> (Vec<Value>, u32) {
    let re = mistral_tool_id_regex();
    let mut result = Vec::with_capacity(messages.len() + 10);
    let mut modified = 0u32;

    for mut msg in messages.into_iter() {
        // Get role and check if we need to normalize - avoid borrowing conflict
        let is_assistant_or_tool = msg.get("role")
            .and_then(|r| r.as_str())
            .map(|r| r == "assistant" || r == "tool")
            .unwrap_or(false);

        // Normalize tool call IDs
        if is_assistant_or_tool {
            if let Some(content) = msg.get_mut("content") {
                if let Some(arr) = content.as_array_mut() {
                    for part in arr.iter_mut() {
                        let part_type = part.get("type").and_then(|t| t.as_str());
                        if matches!(part_type, Some("tool-call") | Some("tool-result")) {
                            if let Some(tool_call_id) = part.get("toolCallId").and_then(|t| t.as_str()) {
                                // Mistral requires exactly 9 alphanumeric characters
                                let cleaned = re.replace_all(tool_call_id, "").to_string();
                                let normalized_id = if cleaned.len() >= 9 {
                                    cleaned[..9].to_string()
                                } else {
                                    format!("{:0<9}", cleaned)
                                };
                                if normalized_id != tool_call_id {
                                    part["toolCallId"] = Value::String(normalized_id);
                                    modified += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        result.push(msg);
    }

    // Second pass: fix tool -> user sequences
    let mut fixed_result = Vec::with_capacity(result.len() + 10);
    for (i, msg) in result.iter().enumerate() {
        fixed_result.push(msg.clone());

        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role == "tool" {
            if let Some(next_msg) = result.get(i + 1) {
                let next_role = next_msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
                if next_role == "user" {
                    // Insert assistant message
                    let assistant_msg = serde_json::json!({
                        "role": "assistant",
                        "content": [{
                            "type": "text",
                            "text": "Done."
                        }]
                    });
                    fixed_result.push(assistant_msg);
                    modified += 1;
                }
            }
        }
    }

    (fixed_result, modified)
}

/// Normalize interleaved reasoning for providers that need it in a special field
fn normalize_interleaved_reasoning(messages: Vec<Value>, field: &str) -> (Vec<Value>, u32) {
    let mut modified = 0u32;

    let normalized: Vec<Value> = messages.into_iter().map(|mut msg| {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");

        if role == "assistant" {
            if let Some(content) = msg.get("content") {
                if let Some(arr) = content.as_array() {
                    // Extract reasoning parts
                    let reasoning_text: String = arr.iter()
                        .filter(|part| part.get("type").and_then(|t| t.as_str()) == Some("reasoning"))
                        .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("");

                    if !reasoning_text.is_empty() {
                        // Filter out reasoning parts from content
                        let filtered_content: Vec<Value> = arr.iter()
                            .filter(|part| part.get("type").and_then(|t| t.as_str()) != Some("reasoning"))
                            .cloned()
                            .collect();

                        msg["content"] = Value::Array(filtered_content);

                        // Add reasoning to provider options
                        let provider_opts = msg.get("providerOptions")
                            .cloned()
                            .unwrap_or(Value::Object(serde_json::Map::new()));

                        let mut opts = provider_opts.as_object().cloned().unwrap_or_default();
                        let compat = opts.entry("openaiCompatible")
                            .or_insert(Value::Object(serde_json::Map::new()));

                        if let Some(compat_obj) = compat.as_object_mut() {
                            compat_obj.insert(field.to_string(), Value::String(reasoning_text));
                        }

                        msg["providerOptions"] = Value::Object(opts);
                        modified += 1;
                    }
                }
            }
        }
        msg
    }).collect();

    (normalized, modified)
}

// ============================================================================
// Cache Application
// ============================================================================

/// Apply caching hints to messages
///
/// # Arguments
/// * `messages_json` - JSON string of messages array
/// * `provider_id` - Provider identifier
///
/// # Returns
/// JSON string of CacheResult
pub fn apply_caching(messages_json: &str, provider_id: &str) -> Result<String, String> {
    let messages: Vec<Value> = serde_json::from_str(messages_json)
        .map_err(|e| format!("Failed to parse messages: {}", e))?;

    let (cached, count) = apply_caching_impl(messages, provider_id);

    let result = CacheResult {
        messages: cached,
        marked_count: count,
    };

    serde_json::to_string(&result)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

fn apply_caching_impl(mut messages: Vec<Value>, provider_id: &str) -> (Vec<Value>, u32) {
    if messages.is_empty() {
        return (messages, 0);
    }

    // Get indices of first 2 system messages and last 2 messages
    let system_indices: Vec<usize> = messages.iter()
        .enumerate()
        .filter(|(_, m)| m.get("role").and_then(|r| r.as_str()) == Some("system"))
        .take(2)
        .map(|(i, _)| i)
        .collect();

    let len = messages.len();
    let final_indices: Vec<usize> = if len >= 2 {
        vec![len - 2, len - 1]
    } else if len == 1 {
        vec![0]
    } else {
        vec![]
    };

    // Combine and deduplicate indices
    let mut cache_indices: Vec<usize> = system_indices.into_iter()
        .chain(final_indices.into_iter())
        .collect();
    cache_indices.sort();
    cache_indices.dedup();

    // Build cache provider options
    let cache_options = serde_json::json!({
        "anthropic": {
            "cacheControl": { "type": "ephemeral" }
        },
        "openrouter": {
            "cacheControl": { "type": "ephemeral" }
        },
        "bedrock": {
            "cachePoint": { "type": "ephemeral" }
        },
        "openaiCompatible": {
            "cache_control": { "type": "ephemeral" }
        }
    });

    let mut marked = 0u32;

    for idx in cache_indices {
        if let Some(msg) = messages.get_mut(idx) {
            // For non-anthropic providers with array content, apply to last content part
            let should_use_content_options = provider_id != "anthropic"
                && msg.get("content").and_then(|c| c.as_array()).map(|a| !a.is_empty()).unwrap_or(false);

            if should_use_content_options {
                if let Some(content) = msg.get_mut("content") {
                    if let Some(arr) = content.as_array_mut() {
                        if let Some(last) = arr.last_mut() {
                            let existing = last.get("providerOptions")
                                .cloned()
                                .unwrap_or(Value::Object(serde_json::Map::new()));
                            last["providerOptions"] = merge_deep(existing, cache_options.clone());
                            marked += 1;
                            continue;
                        }
                    }
                }
            }

            // Apply to message-level providerOptions
            let existing = msg.get("providerOptions")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));
            msg["providerOptions"] = merge_deep(existing, cache_options.clone());
            marked += 1;
        }
    }

    (messages, marked)
}

/// Deep merge two JSON values (second takes precedence)
fn merge_deep(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Object(mut base_obj), Value::Object(overlay_obj)) => {
            for (key, overlay_val) in overlay_obj {
                let merged = if let Some(base_val) = base_obj.remove(&key) {
                    merge_deep(base_val, overlay_val)
                } else {
                    overlay_val
                };
                base_obj.insert(key, merged);
            }
            Value::Object(base_obj)
        }
        (_, overlay) => overlay,
    }
}

// ============================================================================
// SDK Key Mapping
// ============================================================================

/// Get the SDK key for provider options based on npm package
pub fn get_sdk_key(npm: &str) -> Option<&'static str> {
    match npm {
        "@ai-sdk/github-copilot" | "@ai-sdk/openai" | "@ai-sdk/azure" => Some("openai"),
        "@ai-sdk/amazon-bedrock" => Some("bedrock"),
        "@ai-sdk/anthropic" | "@ai-sdk/google-vertex/anthropic" => Some("anthropic"),
        "@ai-sdk/google-vertex" | "@ai-sdk/google" => Some("google"),
        "@ai-sdk/gateway" => Some("gateway"),
        "@openrouter/ai-sdk-provider" => Some("openrouter"),
        _ => None,
    }
}

/// Remap provider options keys from stored providerID to SDK-expected key
pub fn remap_provider_options(messages_json: &str, from_key: &str, to_key: &str) -> Result<String, String> {
    if from_key == to_key {
        return Ok(messages_json.to_string());
    }

    let mut messages: Vec<Value> = serde_json::from_str(messages_json)
        .map_err(|e| format!("Failed to parse messages: {}", e))?;

    for msg in messages.iter_mut() {
        remap_opts(msg, from_key, to_key);

        if let Some(content) = msg.get_mut("content") {
            if let Some(arr) = content.as_array_mut() {
                for part in arr.iter_mut() {
                    remap_opts(part, from_key, to_key);
                }
            }
        }
    }

    serde_json::to_string(&messages)
        .map_err(|e| format!("Failed to serialize result: {}", e))
}

fn remap_opts(obj: &mut Value, from_key: &str, to_key: &str) {
    if let Some(opts) = obj.get_mut("providerOptions") {
        if let Some(opts_obj) = opts.as_object_mut() {
            if let Some(val) = opts_obj.remove(from_key) {
                opts_obj.insert(to_key.to_string(), val);
            }
        }
    }
}

// ============================================================================
// Temperature/Sampling Parameters
// ============================================================================

/// Get recommended temperature for a model
pub fn get_temperature(model_id: &str) -> Option<f64> {
    let id = model_id.to_lowercase();

    if id.contains("qwen") {
        return Some(0.55);
    }
    if id.contains("claude") {
        return None; // Claude uses default
    }
    if id.contains("gemini") {
        return Some(1.0);
    }
    if id.contains("glm-4.6") || id.contains("glm-4.7") {
        return Some(1.0);
    }
    if id.contains("minimax-m2") {
        return Some(1.0);
    }
    if id.contains("kimi-k2") {
        if id.contains("thinking") {
            return Some(1.0);
        }
        return Some(0.6);
    }

    None
}

/// Get recommended top_p for a model
pub fn get_top_p(model_id: &str) -> Option<f64> {
    let id = model_id.to_lowercase();

    if id.contains("qwen") {
        return Some(1.0);
    }
    if id.contains("minimax-m2") {
        return Some(0.95);
    }
    if id.contains("gemini") {
        return Some(0.95);
    }

    None
}

/// Get recommended top_k for a model
pub fn get_top_k(model_id: &str) -> Option<u32> {
    let id = model_id.to_lowercase();

    if id.contains("minimax-m2") {
        if id.contains("m2.1") {
            return Some(40);
        }
        return Some(20);
    }
    if id.contains("gemini") {
        return Some(64);
    }

    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_tool_id_normalization() {
        let messages = serde_json::json!([
            {
                "role": "assistant",
                "content": [{
                    "type": "tool-call",
                    "toolCallId": "call_123!@#$%abc"
                }]
            }
        ]);

        let model = serde_json::json!({
            "providerId": "anthropic",
            "apiId": "claude-3-opus",
            "apiNpm": "@ai-sdk/anthropic"
        });

        let result = normalize_messages(
            &serde_json::to_string(&messages).unwrap(),
            &serde_json::to_string(&model).unwrap(),
        ).unwrap();

        let parsed: NormalizeResult = serde_json::from_str(&result).unwrap();
        // Strategy includes claude_tool_id since we're normalizing tool IDs
        // It may also include anthropic_empty_filter if any empty messages were filtered
        assert!(parsed.strategy.contains("claude_tool_id"), "Expected strategy to contain 'claude_tool_id', got '{}'", parsed.strategy);
        assert!(parsed.modifications > 0);

        // Check that the tool call ID was normalized
        let first_msg = &parsed.messages[0];
        let content = first_msg.get("content").unwrap().as_array().unwrap();
        let tool_id = content[0].get("toolCallId").unwrap().as_str().unwrap();
        assert_eq!(tool_id, "call_123_____abc");
    }

    #[test]
    fn test_mistral_tool_id_normalization() {
        let messages = serde_json::json!([
            {
                "role": "assistant",
                "content": [{
                    "type": "tool-call",
                    "toolCallId": "abc"
                }]
            }
        ]);

        let model = serde_json::json!({
            "providerId": "mistral",
            "apiId": "mistral-large",
            "apiNpm": "@ai-sdk/mistral"
        });

        let result = normalize_messages(
            &serde_json::to_string(&messages).unwrap(),
            &serde_json::to_string(&model).unwrap(),
        ).unwrap();

        let parsed: NormalizeResult = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.strategy, "mistral");

        // Check that the tool call ID was padded to 9 characters
        let first_msg = &parsed.messages[0];
        let content = first_msg.get("content").unwrap().as_array().unwrap();
        let tool_id = content[0].get("toolCallId").unwrap().as_str().unwrap();
        assert_eq!(tool_id.len(), 9);
        assert!(tool_id.starts_with("abc"));
    }

    #[test]
    fn test_apply_caching() {
        let messages = serde_json::json!([
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"},
            {"role": "user", "content": "How are you?"}
        ]);

        let result = apply_caching(
            &serde_json::to_string(&messages).unwrap(),
            "anthropic",
        ).unwrap();

        let parsed: CacheResult = serde_json::from_str(&result).unwrap();
        assert!(parsed.marked_count >= 2); // At least system and last 2 messages

        // Check that caching hints were added
        for msg in &parsed.messages {
            if let Some(opts) = msg.get("providerOptions") {
                if let Some(anthropic) = opts.get("anthropic") {
                    assert!(anthropic.get("cacheControl").is_some());
                }
            }
        }
    }

    #[test]
    fn test_get_sdk_key() {
        assert_eq!(get_sdk_key("@ai-sdk/anthropic"), Some("anthropic"));
        assert_eq!(get_sdk_key("@ai-sdk/openai"), Some("openai"));
        assert_eq!(get_sdk_key("@ai-sdk/google"), Some("google"));
        assert_eq!(get_sdk_key("@ai-sdk/unknown"), None);
    }

    #[test]
    fn test_temperature() {
        assert_eq!(get_temperature("qwen-72b"), Some(0.55));
        assert_eq!(get_temperature("claude-3-opus"), None);
        assert_eq!(get_temperature("gemini-pro"), Some(1.0));
    }

    #[test]
    fn test_merge_deep() {
        let base = serde_json::json!({
            "a": 1,
            "b": { "c": 2, "d": 3 }
        });
        let overlay = serde_json::json!({
            "b": { "c": 4, "e": 5 },
            "f": 6
        });

        let merged = merge_deep(base, overlay);

        assert_eq!(merged.get("a").unwrap(), 1);
        assert_eq!(merged.get("b").unwrap().get("c").unwrap(), 4);
        assert_eq!(merged.get("b").unwrap().get("d").unwrap(), 3);
        assert_eq!(merged.get("b").unwrap().get("e").unwrap(), 5);
        assert_eq!(merged.get("f").unwrap(), 6);
    }
}
