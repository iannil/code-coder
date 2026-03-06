//! NAPI bindings for provider transform functions
//!
//! This module exposes provider transformation functions to Node.js.

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::provider::transform::{
    apply_caching as rust_apply_caching,
    normalize_messages as rust_normalize_messages,
    remap_provider_options as rust_remap_provider_options,
    get_sdk_key as rust_get_sdk_key,
    get_temperature as rust_get_temperature,
    get_top_p as rust_get_top_p,
    get_top_k as rust_get_top_k,
};

// ============================================================================
// Message Normalization
// ============================================================================

/// Result of message normalization
#[napi(object)]
pub struct NormalizeMessagesResult {
    /// Normalized messages as JSON string
    pub messages: String,
    /// Strategy used for normalization
    pub strategy: String,
    /// Number of modifications made
    pub modifications: u32,
}

/// Normalize messages for a specific provider
///
/// Takes messages and model info as JSON strings, returns normalized messages.
/// This function handles:
/// - Filtering empty content for Anthropic
/// - Tool call ID normalization for Claude (alphanumeric + underscore + hyphen)
/// - Tool call ID normalization for Mistral (exactly 9 alphanumeric chars)
/// - Message sequence fixing for Mistral (tool -> user requires assistant in between)
/// - Interleaved reasoning extraction for compatible providers
#[napi]
pub fn normalize_messages(
    messages_json: String,
    model_json: String,
) -> Result<NormalizeMessagesResult> {
    let result = rust_normalize_messages(&messages_json, &model_json)
        .map_err(|e| Error::from_reason(e))?;

    // Parse the result to extract fields
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .map_err(|e| Error::from_reason(format!("Failed to parse result: {}", e)))?;

    Ok(NormalizeMessagesResult {
        messages: serde_json::to_string(&parsed["messages"])
            .map_err(|e| Error::from_reason(format!("Failed to serialize messages: {}", e)))?,
        strategy: parsed["strategy"].as_str().unwrap_or("unknown").to_string(),
        modifications: parsed["modifications"].as_u64().unwrap_or(0) as u32,
    })
}

// ============================================================================
// Cache Application
// ============================================================================

/// Result of cache application
#[napi(object)]
pub struct ApplyCachingResult {
    /// Messages with caching hints as JSON string
    pub messages: String,
    /// Number of messages marked with cache hints
    pub marked_count: u32,
}

/// Apply caching hints to messages
///
/// Marks the first 2 system messages and last 2 messages with ephemeral cache hints.
/// The cache hints are applied to providerOptions for Anthropic, OpenRouter, Bedrock,
/// and openaiCompatible providers.
#[napi]
pub fn apply_caching(
    messages_json: String,
    provider_id: String,
) -> Result<ApplyCachingResult> {
    let result = rust_apply_caching(&messages_json, &provider_id)
        .map_err(|e| Error::from_reason(e))?;

    let parsed: serde_json::Value = serde_json::from_str(&result)
        .map_err(|e| Error::from_reason(format!("Failed to parse result: {}", e)))?;

    Ok(ApplyCachingResult {
        messages: serde_json::to_string(&parsed["messages"])
            .map_err(|e| Error::from_reason(format!("Failed to serialize messages: {}", e)))?,
        marked_count: parsed["markedCount"].as_u64().unwrap_or(0) as u32,
    })
}

// ============================================================================
// Provider Options Remapping
// ============================================================================

/// Remap provider options keys from stored providerID to SDK-expected key
///
/// Some AI SDK packages expect different keys in providerOptions than the
/// provider ID used in storage. This function remaps the keys.
///
/// Example: OpenAI SDK expects "openai" key, but provider might be stored as "azure"
#[napi]
pub fn remap_provider_options(
    messages_json: String,
    from_key: String,
    to_key: String,
) -> Result<String> {
    rust_remap_provider_options(&messages_json, &from_key, &to_key)
        .map_err(|e| Error::from_reason(e))
}

// ============================================================================
// SDK Key Mapping
// ============================================================================

/// Get the SDK key for provider options based on npm package
///
/// Returns the key that the AI SDK expects in providerOptions for the given
/// npm package. Returns null if no mapping exists.
#[napi]
pub fn get_sdk_key(npm: String) -> Option<String> {
    rust_get_sdk_key(&npm).map(|s| s.to_string())
}

// ============================================================================
// Sampling Parameters
// ============================================================================

/// Get recommended temperature for a model
///
/// Returns the optimal temperature setting for the given model ID,
/// or null if the model should use the default temperature.
#[napi]
pub fn get_temperature(model_id: String) -> Option<f64> {
    rust_get_temperature(&model_id)
}

/// Get recommended top_p for a model
///
/// Returns the optimal top_p (nucleus sampling) setting for the given model ID,
/// or null if the model should use the default.
#[napi]
pub fn get_top_p(model_id: String) -> Option<f64> {
    rust_get_top_p(&model_id)
}

/// Get recommended top_k for a model
///
/// Returns the optimal top_k setting for the given model ID,
/// or null if the model should use the default.
#[napi]
pub fn get_top_k(model_id: String) -> Option<u32> {
    rust_get_top_k(&model_id).map(|k| k)
}

// ============================================================================
// Combined Transform
// ============================================================================

/// Model info for combined transform
#[napi(object)]
pub struct TransformModelInfo {
    pub provider_id: String,
    pub api_id: String,
    pub api_npm: String,
    #[napi(js_name = "hasInterleavedField")]
    pub has_interleaved_field: Option<bool>,
    #[napi(js_name = "interleavedField")]
    pub interleaved_field: Option<String>,
}

/// Result of combined transform
#[napi(object)]
pub struct TransformMessagesResult {
    /// Transformed messages as JSON string
    pub messages: String,
    /// Whether caching was applied
    pub caching_applied: bool,
    /// Whether normalization was applied
    pub normalization_applied: bool,
    /// Strategy used for normalization
    pub strategy: String,
}

/// Combined message transformation (normalize + cache + remap)
///
/// This function performs all message transformations in a single call:
/// 1. Normalize messages for the provider
/// 2. Apply caching hints (if Anthropic/Claude provider)
/// 3. Remap provider options keys
#[napi]
pub fn transform_messages(
    messages_json: String,
    model: TransformModelInfo,
) -> Result<TransformMessagesResult> {
    // Build model JSON for normalization
    let model_json = serde_json::json!({
        "providerId": model.provider_id,
        "apiId": model.api_id,
        "apiNpm": model.api_npm,
        "hasInterleavedField": model.has_interleaved_field.unwrap_or(false),
        "interleavedField": model.interleaved_field
    });

    // Step 1: Normalize messages
    let norm_result = rust_normalize_messages(&messages_json, &model_json.to_string())
        .map_err(|e| Error::from_reason(e))?;

    let norm_parsed: serde_json::Value = serde_json::from_str(&norm_result)
        .map_err(|e| Error::from_reason(format!("Failed to parse normalize result: {}", e)))?;

    let mut messages = serde_json::to_string(&norm_parsed["messages"])
        .map_err(|e| Error::from_reason(format!("Failed to serialize messages: {}", e)))?;

    let strategy = norm_parsed["strategy"].as_str().unwrap_or("passthrough").to_string();
    let normalization_applied = norm_parsed["modifications"].as_u64().unwrap_or(0) > 0;

    // Step 2: Apply caching for Anthropic/Claude
    let should_cache = model.provider_id == "anthropic"
        || model.api_id.contains("anthropic")
        || model.api_id.contains("claude")
        || model.provider_id.contains("anthropic")
        || model.provider_id.contains("claude")
        || model.api_npm == "@ai-sdk/anthropic";

    let mut caching_applied = false;
    if should_cache {
        let cache_result = rust_apply_caching(&messages, &model.provider_id)
            .map_err(|e| Error::from_reason(e))?;

        let cache_parsed: serde_json::Value = serde_json::from_str(&cache_result)
            .map_err(|e| Error::from_reason(format!("Failed to parse cache result: {}", e)))?;

        messages = serde_json::to_string(&cache_parsed["messages"])
            .map_err(|e| Error::from_reason(format!("Failed to serialize cached messages: {}", e)))?;

        caching_applied = cache_parsed["markedCount"].as_u64().unwrap_or(0) > 0;
    }

    // Step 3: Remap provider options keys
    if let Some(sdk_key) = rust_get_sdk_key(&model.api_npm) {
        if sdk_key != model.provider_id && model.api_npm != "@ai-sdk/azure" {
            messages = rust_remap_provider_options(&messages, &model.provider_id, sdk_key)
                .map_err(|e| Error::from_reason(e))?;
        }
    }

    Ok(TransformMessagesResult {
        messages,
        caching_applied,
        normalization_applied,
        strategy,
    })
}
