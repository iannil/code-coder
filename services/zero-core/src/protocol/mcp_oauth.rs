//! MCP OAuth 2.0 PKCE implementation
//!
//! This module provides OAuth 2.0 with PKCE (Proof Key for Code Exchange)
//! authentication support for MCP servers.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info};
use url::Url;

/// OAuth token response from the authorization server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    /// Access token for API requests
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Optional refresh token for getting new access tokens
    pub refresh_token: Option<String>,
    /// Token expiration time in seconds
    pub expires_in: Option<u64>,
    /// Scope of the access token
    pub scope: Option<String>,
    /// Timestamp when token was obtained (Unix epoch seconds)
    #[serde(default)]
    pub obtained_at: u64,
}

impl OAuthTokens {
    /// Check if the access token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // Consider expired 60 seconds before actual expiration
            now >= self.obtained_at + expires_in.saturating_sub(60)
        } else {
            false
        }
    }
}

/// OAuth server metadata (RFC 8414)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMetadata {
    /// Authorization endpoint URL
    pub authorization_endpoint: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// Revocation endpoint URL (optional)
    pub revocation_endpoint: Option<String>,
    /// Registration endpoint URL (optional)
    pub registration_endpoint: Option<String>,
    /// Supported response types
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    /// Supported grant types
    #[serde(default)]
    pub grant_types_supported: Vec<String>,
    /// Supported code challenge methods
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,
    /// Issuer URL
    pub issuer: Option<String>,
}

/// Client registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistration {
    /// Client ID
    pub client_id: String,
    /// Client secret (optional)
    pub client_secret: Option<String>,
    /// Client ID issued at timestamp
    pub client_id_issued_at: Option<u64>,
    /// Client secret expires at timestamp
    pub client_secret_expires_at: Option<u64>,
}

/// OAuth state for an ongoing authorization flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// Random state parameter for CSRF protection
    pub state: String,
    /// PKCE code verifier
    pub code_verifier: String,
    /// Server URL this auth flow is for
    pub server_url: String,
    /// Redirect URI used
    pub redirect_uri: String,
}

/// OAuth configuration for an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Pre-registered client ID (optional, for servers without dynamic registration)
    pub client_id: Option<String>,
    /// Pre-registered client secret (optional)
    pub client_secret: Option<String>,
    /// OAuth scopes to request
    pub scope: Option<String>,
}

/// Stored OAuth credentials for an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    /// Client registration (from dynamic registration or config)
    pub client: ClientRegistration,
    /// OAuth tokens (after successful auth)
    pub tokens: Option<OAuthTokens>,
    /// Server URL
    pub server_url: String,
    /// OAuth metadata (cached)
    pub metadata: Option<OAuthMetadata>,
}

/// OAuth manager for MCP servers
pub struct McpOAuthManager {
    /// Stored credentials by server name
    credentials: RwLock<HashMap<String, OAuthCredentials>>,
    /// Pending OAuth flows (state -> OAuthState)
    pending_flows: RwLock<HashMap<String, OAuthState>>,
    /// Storage path for persisting credentials
    storage_path: PathBuf,
    /// HTTP client for OAuth requests
    http_client: reqwest::Client,
}

impl McpOAuthManager {
    /// Create a new OAuth manager
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            credentials: RwLock::new(HashMap::new()),
            pending_flows: RwLock::new(HashMap::new()),
            storage_path,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Load credentials from storage
    pub async fn load(&self) -> Result<()> {
        let creds_path = self.storage_path.join("oauth_credentials.json");
        if !creds_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&creds_path)
            .await
            .context("Failed to read OAuth credentials file")?;

        let creds: HashMap<String, OAuthCredentials> = serde_json::from_str(&content)
            .context("Failed to parse OAuth credentials")?;

        let mut credentials = self.credentials.write().await;
        *credentials = creds;
        Ok(())
    }

    /// Save credentials to storage
    pub async fn save(&self) -> Result<()> {
        let creds_path = self.storage_path.join("oauth_credentials.json");

        // Ensure directory exists
        if let Some(parent) = creds_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let credentials = self.credentials.read().await;
        let content = serde_json::to_string_pretty(&*credentials)?;
        tokio::fs::write(&creds_path, content).await?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&creds_path, perms)?;
        }

        Ok(())
    }

    /// Generate a cryptographically secure random string
    fn generate_random_string(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Generate a PKCE code verifier (43-128 characters)
    pub fn generate_code_verifier() -> String {
        Self::generate_random_string(64)
    }

    /// Generate a PKCE code challenge from a code verifier (S256 method)
    pub fn generate_code_challenge(code_verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Generate a random state parameter for CSRF protection
    pub fn generate_state() -> String {
        Self::generate_random_string(32)
    }

    /// Discover OAuth metadata from a server URL
    pub async fn discover_metadata(&self, server_url: &str) -> Result<OAuthMetadata> {
        let base_url = Url::parse(server_url)?;

        // Try well-known OAuth metadata endpoint
        let well_known_url = format!(
            "{}/.well-known/oauth-authorization-server",
            base_url.origin().ascii_serialization()
        );

        debug!("Discovering OAuth metadata from {}", well_known_url);

        let response = self.http_client
            .get(&well_known_url)
            .send()
            .await?;

        if !response.status().is_success() {
            // Try OpenID Connect discovery as fallback
            let oidc_url = format!(
                "{}/.well-known/openid-configuration",
                base_url.origin().ascii_serialization()
            );

            debug!("Trying OIDC discovery at {}", oidc_url);

            let response = self.http_client
                .get(&oidc_url)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(anyhow!("OAuth metadata discovery failed"));
            }

            return response.json().await.context("Failed to parse OIDC metadata");
        }

        response.json().await.context("Failed to parse OAuth metadata")
    }

    /// Register a client dynamically (if server supports it)
    pub async fn register_client(
        &self,
        metadata: &OAuthMetadata,
        redirect_uri: &str,
    ) -> Result<ClientRegistration> {
        let registration_endpoint = metadata.registration_endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("Server does not support dynamic client registration"))?;

        let request_body = serde_json::json!({
            "redirect_uris": [redirect_uri],
            "token_endpoint_auth_method": "none",
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "client_name": "CodeCoder",
            "client_uri": "https://codecoder.ai"
        });

        debug!("Registering client at {}", registration_endpoint);

        let response = self.http_client
            .post(registration_endpoint)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Client registration failed: {} - {}", status, body));
        }

        response.json().await.context("Failed to parse client registration response")
    }

    /// Start an OAuth authorization flow
    pub async fn start_auth(
        &self,
        server_name: &str,
        server_url: &str,
        config: Option<&OAuthConfig>,
        redirect_uri: &str,
    ) -> Result<String> {
        info!("Starting OAuth flow for {}", server_name);

        // Discover OAuth metadata
        let metadata = self.discover_metadata(server_url).await?;

        // Get or register client
        let client = if let Some(cfg) = config {
            if let Some(client_id) = &cfg.client_id {
                ClientRegistration {
                    client_id: client_id.clone(),
                    client_secret: cfg.client_secret.clone(),
                    client_id_issued_at: None,
                    client_secret_expires_at: None,
                }
            } else {
                self.register_client(&metadata, redirect_uri).await?
            }
        } else {
            self.register_client(&metadata, redirect_uri).await?
        };

        // Generate PKCE values
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);
        let state = Self::generate_state();

        // Store pending flow
        let oauth_state = OAuthState {
            state: state.clone(),
            code_verifier,
            server_url: server_url.to_string(),
            redirect_uri: redirect_uri.to_string(),
        };

        {
            let mut pending = self.pending_flows.write().await;
            pending.insert(state.clone(), oauth_state);
        }

        // Store credentials (without tokens yet)
        {
            let mut credentials = self.credentials.write().await;
            credentials.insert(
                server_name.to_string(),
                OAuthCredentials {
                    client,
                    tokens: None,
                    server_url: server_url.to_string(),
                    metadata: Some(metadata.clone()),
                },
            );
        }

        // Build authorization URL
        let scope = config
            .and_then(|c| c.scope.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("openid profile");

        let mut auth_url = Url::parse(&metadata.authorization_endpoint)?;
        {
            let creds = self.credentials.read().await;
            let cred = creds.get(server_name).ok_or_else(|| anyhow!("Credentials not found"))?;

            auth_url.query_pairs_mut()
                .append_pair("response_type", "code")
                .append_pair("client_id", &cred.client.client_id)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("state", &state)
                .append_pair("code_challenge", &code_challenge)
                .append_pair("code_challenge_method", "S256")
                .append_pair("scope", scope);
        }

        info!("Generated authorization URL for {}", server_name);
        Ok(auth_url.to_string())
    }

    /// Complete an OAuth authorization flow with the authorization code
    pub async fn finish_auth(
        &self,
        server_name: &str,
        authorization_code: &str,
        state: &str,
    ) -> Result<()> {
        info!("Completing OAuth flow for {}", server_name);

        // Get and remove pending flow
        let oauth_state = {
            let mut pending = self.pending_flows.write().await;
            pending.remove(state)
                .ok_or_else(|| anyhow!("No pending OAuth flow found for state"))?
        };

        // Get stored credentials
        let (metadata, client) = {
            let credentials = self.credentials.read().await;
            let cred = credentials.get(server_name)
                .ok_or_else(|| anyhow!("No credentials found for server"))?;
            (
                cred.metadata.clone().ok_or_else(|| anyhow!("No metadata found"))?,
                cred.client.clone(),
            )
        };

        // Exchange authorization code for tokens
        let mut form_data = vec![
            ("grant_type", "authorization_code"),
            ("code", authorization_code),
            ("redirect_uri", &oauth_state.redirect_uri),
            ("code_verifier", &oauth_state.code_verifier),
            ("client_id", &client.client_id),
        ];

        let client_secret_ref;
        if let Some(ref secret) = client.client_secret {
            client_secret_ref = secret.clone();
            form_data.push(("client_secret", &client_secret_ref));
        }

        debug!("Exchanging authorization code at {}", metadata.token_endpoint);

        let response = self.http_client
            .post(&metadata.token_endpoint)
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Token exchange failed: {} - {}", status, body));
        }

        let mut tokens: OAuthTokens = response.json().await
            .context("Failed to parse token response")?;

        // Record when we obtained the token
        tokens.obtained_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Store tokens
        {
            let mut credentials = self.credentials.write().await;
            if let Some(cred) = credentials.get_mut(server_name) {
                cred.tokens = Some(tokens);
            }
        }

        // Persist credentials
        self.save().await?;

        info!("OAuth flow completed for {}", server_name);
        Ok(())
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_token(&self, server_name: &str) -> Result<()> {
        info!("Refreshing token for {}", server_name);

        let (metadata, client, refresh_token) = {
            let credentials = self.credentials.read().await;
            let cred = credentials.get(server_name)
                .ok_or_else(|| anyhow!("No credentials found for server"))?;
            let tokens = cred.tokens.as_ref()
                .ok_or_else(|| anyhow!("No tokens found for server"))?;
            let refresh_token = tokens.refresh_token.clone()
                .ok_or_else(|| anyhow!("No refresh token available"))?;
            (
                cred.metadata.clone().ok_or_else(|| anyhow!("No metadata found"))?,
                cred.client.clone(),
                refresh_token,
            )
        };

        let mut form_data = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token),
            ("client_id", client.client_id.clone()),
        ];

        if let Some(ref secret) = client.client_secret {
            form_data.push(("client_secret", secret.clone()));
        }

        let response = self.http_client
            .post(&metadata.token_endpoint)
            .form(&form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Token refresh failed: {} - {}", status, body));
        }

        let mut tokens: OAuthTokens = response.json().await
            .context("Failed to parse token response")?;

        tokens.obtained_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Store new tokens (preserve refresh_token if not returned)
        {
            let mut credentials = self.credentials.write().await;
            if let Some(cred) = credentials.get_mut(server_name) {
                if tokens.refresh_token.is_none() {
                    tokens.refresh_token = cred.tokens.as_ref()
                        .and_then(|t| t.refresh_token.clone());
                }
                cred.tokens = Some(tokens);
            }
        }

        self.save().await?;

        info!("Token refreshed for {}", server_name);
        Ok(())
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_access_token(&self, server_name: &str) -> Result<String> {
        let tokens = {
            let credentials = self.credentials.read().await;
            credentials.get(server_name)
                .and_then(|c| c.tokens.clone())
        };

        match tokens {
            Some(tokens) if !tokens.is_expired() => {
                Ok(tokens.access_token)
            }
            Some(tokens) if tokens.refresh_token.is_some() => {
                // Try to refresh
                self.refresh_token(server_name).await?;
                let credentials = self.credentials.read().await;
                credentials.get(server_name)
                    .and_then(|c| c.tokens.as_ref())
                    .map(|t| t.access_token.clone())
                    .ok_or_else(|| anyhow!("Failed to get access token after refresh"))
            }
            _ => {
                Err(anyhow!("No valid access token available - re-authentication required"))
            }
        }
    }

    /// Check if we have valid credentials for a server
    pub async fn has_credentials(&self, server_name: &str) -> bool {
        let credentials = self.credentials.read().await;
        credentials.get(server_name)
            .and_then(|c| c.tokens.as_ref())
            .is_some()
    }

    /// Get authentication status for a server
    pub async fn get_auth_status(&self, server_name: &str) -> AuthStatus {
        let credentials = self.credentials.read().await;
        match credentials.get(server_name) {
            None => AuthStatus::NotAuthenticated,
            Some(cred) => match &cred.tokens {
                None => AuthStatus::NotAuthenticated,
                Some(tokens) if tokens.is_expired() => {
                    if tokens.refresh_token.is_some() {
                        AuthStatus::Expired
                    } else {
                        AuthStatus::NotAuthenticated
                    }
                }
                Some(_) => AuthStatus::Authenticated,
            }
        }
    }

    /// Remove stored credentials for a server
    pub async fn remove_credentials(&self, server_name: &str) -> Result<()> {
        {
            let mut credentials = self.credentials.write().await;
            credentials.remove(server_name);
        }
        self.save().await?;
        info!("Removed OAuth credentials for {}", server_name);
        Ok(())
    }

    /// Clear any pending OAuth flow for a server
    pub async fn cancel_pending(&self, server_name: &str) {
        let mut pending = self.pending_flows.write().await;
        pending.retain(|_, v| v.server_url != server_name);
    }
}

/// Authentication status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    /// Not authenticated
    NotAuthenticated,
    /// Authenticated with valid tokens
    Authenticated,
    /// Token expired but can be refreshed
    Expired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier() {
        let verifier = McpOAuthManager::generate_code_verifier();
        assert_eq!(verifier.len(), 64);
        // Should only contain allowed characters
        assert!(verifier.chars().all(|c|
            c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~'
        ));
    }

    #[test]
    fn test_generate_code_challenge() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = McpOAuthManager::generate_code_challenge(verifier);
        // SHA256 of the verifier, base64url encoded
        assert!(!challenge.is_empty());
        // Should be base64url without padding
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
    }

    #[test]
    fn test_generate_state() {
        let state = McpOAuthManager::generate_state();
        assert_eq!(state.len(), 32);
    }

    #[test]
    fn test_token_expiration() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Not expired
        let tokens = OAuthTokens {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            scope: None,
            obtained_at: now,
        };
        assert!(!tokens.is_expired());

        // Expired
        let expired_tokens = OAuthTokens {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            expires_in: Some(60),
            scope: None,
            obtained_at: now - 120, // 2 minutes ago, expired
        };
        assert!(expired_tokens.is_expired());

        // No expiration set
        let no_expiry = OAuthTokens {
            access_token: "test".to_string(),
            token_type: "Bearer".to_string(),
            refresh_token: None,
            expires_in: None,
            scope: None,
            obtained_at: now - 10000,
        };
        assert!(!no_expiry.is_expired());
    }
}
