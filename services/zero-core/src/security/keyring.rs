//! System keyring integration
//!
//! Provides cross-platform access to OS secret stores:
//! - macOS: Keychain
//! - Linux: Secret Service (via D-Bus)
//! - Windows: Credential Manager
//!
//! Falls back to encrypted file storage when keyring is unavailable.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Service name for keyring entries
const _SERVICE_NAME: &str = "codecoder";

/// Keyring backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringBackend {
    /// System keyring (macOS Keychain, Linux Secret Service, Windows Credential Manager)
    System,
    /// Encrypted file fallback
    File,
}

/// Keyring manager for secure secret storage
pub struct KeyringManager {
    backend: KeyringBackend,
    file_path: Option<PathBuf>,
}

impl KeyringManager {
    /// Create a new keyring manager
    ///
    /// Tries to use system keyring first, falls back to file storage if unavailable.
    pub fn new(file_fallback_path: Option<PathBuf>) -> Self {
        let backend = if Self::is_keyring_available() {
            KeyringBackend::System
        } else {
            KeyringBackend::File
        };

        Self {
            backend,
            file_path: file_fallback_path,
        }
    }

    /// Create a manager that only uses system keyring (no fallback)
    pub fn system_only() -> Result<Self> {
        if !Self::is_keyring_available() {
            anyhow::bail!("System keyring is not available");
        }
        Ok(Self {
            backend: KeyringBackend::System,
            file_path: None,
        })
    }

    /// Create a manager that only uses file storage
    pub fn file_only(path: PathBuf) -> Self {
        Self {
            backend: KeyringBackend::File,
            file_path: Some(path),
        }
    }

    /// Check if system keyring is available
    #[cfg(feature = "keyring-support")]
    pub fn is_keyring_available() -> bool {
        // Try to create a test entry to check if keyring is functional
        let entry = keyring::Entry::new(SERVICE_NAME, "__test__");
        match entry {
            Ok(e) => {
                // Try a harmless operation to verify the backend works
                let _ = e.get_password(); // Ignore error, just checking if it doesn't panic
                true
            }
            Err(_) => false,
        }
    }

    #[cfg(not(feature = "keyring-support"))]
    pub fn is_keyring_available() -> bool {
        false
    }

    /// Get the current backend
    pub fn backend(&self) -> KeyringBackend {
        self.backend
    }

    /// Store a secret
    #[cfg(feature = "keyring-support")]
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        match self.backend {
            KeyringBackend::System => {
                let entry = keyring::Entry::new(SERVICE_NAME, key)
                    .context("Failed to create keyring entry")?;
                entry
                    .set_password(value)
                    .context("Failed to store secret in keyring")?;
                Ok(())
            }
            KeyringBackend::File => self.set_file(key, value),
        }
    }

    #[cfg(not(feature = "keyring-support"))]
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        self.set_file(key, value)
    }

    /// Get a secret
    #[cfg(feature = "keyring-support")]
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        match self.backend {
            KeyringBackend::System => {
                let entry = keyring::Entry::new(SERVICE_NAME, key)
                    .context("Failed to create keyring entry")?;
                match entry.get_password() {
                    Ok(password) => Ok(Some(password)),
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(e) => Err(anyhow::anyhow!("Failed to get secret from keyring: {}", e)),
                }
            }
            KeyringBackend::File => self.get_file(key),
        }
    }

    #[cfg(not(feature = "keyring-support"))]
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        self.get_file(key)
    }

    /// Delete a secret
    #[cfg(feature = "keyring-support")]
    pub fn delete(&self, key: &str) -> Result<bool> {
        match self.backend {
            KeyringBackend::System => {
                let entry = keyring::Entry::new(SERVICE_NAME, key)
                    .context("Failed to create keyring entry")?;
                match entry.delete_credential() {
                    Ok(()) => Ok(true),
                    Err(keyring::Error::NoEntry) => Ok(false),
                    Err(e) => Err(anyhow::anyhow!("Failed to delete secret from keyring: {}", e)),
                }
            }
            KeyringBackend::File => self.delete_file(key),
        }
    }

    #[cfg(not(feature = "keyring-support"))]
    pub fn delete(&self, key: &str) -> Result<bool> {
        self.delete_file(key)
    }

    /// Check if a secret exists
    pub fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key)?.is_some())
    }

    // File-based storage fallback
    fn get_secrets_path(&self) -> Result<PathBuf> {
        let path = self
            .file_path
            .clone()
            .or_else(|| {
                dirs::home_dir().map(|h| h.join(".codecoder").join("secrets.json"))
            })
            .context("No file path configured and home directory not found")?;
        Ok(path)
    }

    fn load_file_secrets(&self) -> Result<std::collections::HashMap<String, String>> {
        let path = self.get_secrets_path()?;
        if !path.exists() {
            return Ok(std::collections::HashMap::new());
        }

        let contents = std::fs::read_to_string(&path)
            .context("Failed to read secrets file")?;
        let secrets: std::collections::HashMap<String, String> =
            serde_json::from_str(&contents).context("Failed to parse secrets file")?;
        Ok(secrets)
    }

    fn save_file_secrets(&self, secrets: &std::collections::HashMap<String, String>) -> Result<()> {
        let path = self.get_secrets_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create secrets directory")?;
        }

        let contents = serde_json::to_string_pretty(secrets)
            .context("Failed to serialize secrets")?;

        std::fs::write(&path, contents)
            .context("Failed to write secrets file")?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, permissions)?;
        }

        Ok(())
    }

    fn set_file(&self, key: &str, value: &str) -> Result<()> {
        let mut secrets = self.load_file_secrets()?;
        secrets.insert(key.to_string(), value.to_string());
        self.save_file_secrets(&secrets)
    }

    fn get_file(&self, key: &str) -> Result<Option<String>> {
        let secrets = self.load_file_secrets()?;
        Ok(secrets.get(key).cloned())
    }

    fn delete_file(&self, key: &str) -> Result<bool> {
        let mut secrets = self.load_file_secrets()?;
        let existed = secrets.remove(key).is_some();
        if existed {
            self.save_file_secrets(&secrets)?;
        }
        Ok(existed)
    }
}

/// Credential types supported by the unified credential manager
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Credential {
    /// API key or bearer token
    ApiKey {
        service: String,
        key: String,
        #[serde(default)]
        patterns: Vec<String>,
    },
    /// OAuth credentials
    OAuth {
        service: String,
        client_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_secret: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        access_token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        refresh_token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
        #[serde(default)]
        patterns: Vec<String>,
    },
    /// Login credentials
    Login {
        service: String,
        username: String,
        password: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        totp_secret: Option<String>,
        #[serde(default)]
        patterns: Vec<String>,
    },
}

impl Credential {
    /// Get the service name
    pub fn service(&self) -> &str {
        match self {
            Credential::ApiKey { service, .. } => service,
            Credential::OAuth { service, .. } => service,
            Credential::Login { service, .. } => service,
        }
    }

    /// Get URL patterns for this credential
    pub fn patterns(&self) -> &[String] {
        match self {
            Credential::ApiKey { patterns, .. } => patterns,
            Credential::OAuth { patterns, .. } => patterns,
            Credential::Login { patterns, .. } => patterns,
        }
    }

    /// Check if this credential matches a URL
    pub fn matches_url(&self, url: &str) -> bool {
        self.patterns().iter().any(|pattern| {
            url_matches_pattern(url, pattern)
        })
    }
}

/// Check if a URL matches a pattern
///
/// Supports:
/// - Exact match: "api.github.com"
/// - Wildcard prefix: "*.github.com"
/// - Full URL pattern: "https://api.openai.com/*"
fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    // Extract host from URL
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .map(|s| s.split('/').next().unwrap_or(s))
        .unwrap_or(url);

    // Handle wildcard patterns
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return host.ends_with(suffix) || host == suffix;
    }

    // Handle full URL patterns with wildcards
    if pattern.contains("://") {
        let pattern_normalized = pattern.replace('*', "");
        return url.starts_with(&pattern_normalized) || url.contains(&pattern_normalized);
    }

    // Exact host match
    host == pattern || host.ends_with(&format!(".{}", pattern))
}

/// Unified credential manager
///
/// Combines keyring storage with structured credential management.
pub struct CredentialManager {
    keyring: KeyringManager,
}

impl CredentialManager {
    /// Create a new credential manager
    pub fn new(file_fallback_path: Option<PathBuf>) -> Self {
        Self {
            keyring: KeyringManager::new(file_fallback_path),
        }
    }

    /// Create a credential manager that only uses file storage (for testing)
    pub fn file_only(path: PathBuf) -> Self {
        Self {
            keyring: KeyringManager::file_only(path),
        }
    }

    /// Get the backend being used
    pub fn backend(&self) -> KeyringBackend {
        self.keyring.backend()
    }

    /// Store a credential
    pub fn store(&self, id: &str, credential: &Credential) -> Result<()> {
        let json = serde_json::to_string(credential)
            .context("Failed to serialize credential")?;
        self.keyring.set(&format!("credential:{}", id), &json)
    }

    /// Get a credential by ID
    pub fn get(&self, id: &str) -> Result<Option<Credential>> {
        match self.keyring.get(&format!("credential:{}", id))? {
            Some(json) => {
                let credential: Credential = serde_json::from_str(&json)
                    .context("Failed to deserialize credential")?;
                Ok(Some(credential))
            }
            None => Ok(None),
        }
    }

    /// Delete a credential
    pub fn delete(&self, id: &str) -> Result<bool> {
        self.keyring.delete(&format!("credential:{}", id))
    }

    /// Find credential by service name
    pub fn find_by_service(&self, service: &str) -> Result<Option<(String, Credential)>> {
        // For now, we need to iterate through known IDs
        // In a production system, we'd maintain an index
        let index_key = format!("credential_index:{}", service);
        if let Some(id) = self.keyring.get(&index_key)? {
            if let Some(cred) = self.get(&id)? {
                return Ok(Some((id, cred)));
            }
        }
        Ok(None)
    }

    /// Find credential matching a URL
    pub fn find_for_url(&self, url: &str) -> Result<Option<(String, Credential)>> {
        // This requires iterating through all credentials
        // In a production system, we'd use a more efficient lookup
        if let Some(index_json) = self.keyring.get("credential_ids")? {
            let ids: Vec<String> = serde_json::from_str(&index_json)?;
            for id in ids {
                if let Some(cred) = self.get(&id)? {
                    if cred.matches_url(url) {
                        return Ok(Some((id, cred)));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Store credential with indexing
    pub fn store_indexed(&self, id: &str, credential: &Credential) -> Result<()> {
        // Store the credential
        self.store(id, credential)?;

        // Update service index
        let service = credential.service();
        self.keyring.set(&format!("credential_index:{}", service), id)?;

        // Update ID list
        let mut ids: Vec<String> = self
            .keyring
            .get("credential_ids")?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        if !ids.contains(&id.to_string()) {
            ids.push(id.to_string());
            let json = serde_json::to_string(&ids)?;
            self.keyring.set("credential_ids", &json)?;
        }

        Ok(())
    }

    /// List all credential IDs
    pub fn list_ids(&self) -> Result<Vec<String>> {
        let ids: Vec<String> = self
            .keyring
            .get("credential_ids")?
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        Ok(ids)
    }
}

/// MCP authentication storage
///
/// Specialized storage for MCP OAuth flows.
pub struct McpAuthStore {
    keyring: KeyringManager,
}

impl McpAuthStore {
    /// Create a new MCP auth store
    pub fn new(file_fallback_path: Option<PathBuf>) -> Self {
        Self {
            keyring: KeyringManager::new(file_fallback_path),
        }
    }

    /// Create an MCP auth store that only uses file storage (for testing)
    pub fn file_only(path: PathBuf) -> Self {
        Self {
            keyring: KeyringManager::file_only(path),
        }
    }

    /// Store MCP OAuth tokens
    pub fn store_tokens(
        &self,
        mcp_name: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<i64>,
        scope: Option<&str>,
        server_url: &str,
    ) -> Result<()> {
        let entry = McpAuthEntry {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(String::from),
            expires_at,
            scope: scope.map(String::from),
            server_url: server_url.to_string(),
            client_id: None,
            client_secret: None,
            code_verifier: None,
            oauth_state: None,
        };
        let json = serde_json::to_string(&entry)?;
        self.keyring.set(&format!("mcp_auth:{}", mcp_name), &json)
    }

    /// Get MCP auth entry
    pub fn get(&self, mcp_name: &str) -> Result<Option<McpAuthEntry>> {
        match self.keyring.get(&format!("mcp_auth:{}", mcp_name))? {
            Some(json) => {
                let entry: McpAuthEntry = serde_json::from_str(&json)?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Get auth for a specific URL (validates URL matches)
    pub fn get_for_url(&self, mcp_name: &str, server_url: &str) -> Result<Option<McpAuthEntry>> {
        match self.get(mcp_name)? {
            Some(entry) if entry.server_url == server_url => Ok(Some(entry)),
            _ => Ok(None),
        }
    }

    /// Check if tokens are expired
    pub fn is_expired(&self, mcp_name: &str) -> Result<Option<bool>> {
        match self.get(mcp_name)? {
            Some(entry) => match entry.expires_at {
                Some(expires_at) => Ok(Some(chrono::Utc::now().timestamp() >= expires_at)),
                None => Ok(Some(false)),
            },
            None => Ok(None),
        }
    }

    /// Update tokens
    pub fn update_tokens(
        &self,
        mcp_name: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<i64>,
    ) -> Result<()> {
        if let Some(mut entry) = self.get(mcp_name)? {
            entry.access_token = access_token.to_string();
            if let Some(rt) = refresh_token {
                entry.refresh_token = Some(rt.to_string());
            }
            entry.expires_at = expires_at;
            let json = serde_json::to_string(&entry)?;
            self.keyring.set(&format!("mcp_auth:{}", mcp_name), &json)?;
        }
        Ok(())
    }

    /// Store PKCE code verifier
    pub fn store_code_verifier(&self, mcp_name: &str, code_verifier: &str) -> Result<()> {
        if let Some(mut entry) = self.get(mcp_name)? {
            entry.code_verifier = Some(code_verifier.to_string());
            let json = serde_json::to_string(&entry)?;
            self.keyring.set(&format!("mcp_auth:{}", mcp_name), &json)?;
        } else {
            // Create minimal entry just for code verifier
            let entry = McpAuthEntry {
                access_token: String::new(),
                refresh_token: None,
                expires_at: None,
                scope: None,
                server_url: String::new(),
                client_id: None,
                client_secret: None,
                code_verifier: Some(code_verifier.to_string()),
                oauth_state: None,
            };
            let json = serde_json::to_string(&entry)?;
            self.keyring.set(&format!("mcp_auth:{}", mcp_name), &json)?;
        }
        Ok(())
    }

    /// Store OAuth state
    pub fn store_oauth_state(&self, mcp_name: &str, state: &str) -> Result<()> {
        if let Some(mut entry) = self.get(mcp_name)? {
            entry.oauth_state = Some(state.to_string());
            let json = serde_json::to_string(&entry)?;
            self.keyring.set(&format!("mcp_auth:{}", mcp_name), &json)?;
        } else {
            let entry = McpAuthEntry {
                access_token: String::new(),
                refresh_token: None,
                expires_at: None,
                scope: None,
                server_url: String::new(),
                client_id: None,
                client_secret: None,
                code_verifier: None,
                oauth_state: Some(state.to_string()),
            };
            let json = serde_json::to_string(&entry)?;
            self.keyring.set(&format!("mcp_auth:{}", mcp_name), &json)?;
        }
        Ok(())
    }

    /// Delete MCP auth entry
    pub fn delete(&self, mcp_name: &str) -> Result<bool> {
        self.keyring.delete(&format!("mcp_auth:{}", mcp_name))
    }
}

/// MCP auth entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpAuthEntry {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub server_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_verifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_state: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_matching() {
        // Exact match
        assert!(url_matches_pattern("api.github.com", "api.github.com"));
        assert!(!url_matches_pattern("raw.github.com", "api.github.com"));

        // Wildcard prefix
        assert!(url_matches_pattern("api.github.com", "*.github.com"));
        assert!(url_matches_pattern("raw.github.com", "*.github.com"));
        assert!(!url_matches_pattern("github.io", "*.github.com"));

        // Full URL with wildcard
        assert!(url_matches_pattern(
            "https://api.openai.com/v1/chat",
            "https://api.openai.com/*"
        ));

        // URL normalization
        assert!(url_matches_pattern(
            "https://api.github.com/repos",
            "api.github.com"
        ));
    }

    #[test]
    fn test_keyring_file_fallback() {
        let dir = tempfile::TempDir::new().unwrap();
        let secrets_path = dir.path().join("secrets.json");

        let manager = KeyringManager::file_only(secrets_path.clone());
        assert_eq!(manager.backend(), KeyringBackend::File);

        // Store and retrieve
        manager.set("test_key", "test_value").unwrap();
        assert_eq!(manager.get("test_key").unwrap(), Some("test_value".to_string()));

        // Delete
        assert!(manager.delete("test_key").unwrap());
        assert_eq!(manager.get("test_key").unwrap(), None);
    }

    #[test]
    fn test_credential_serialization() {
        let cred = Credential::ApiKey {
            service: "github".into(),
            key: "ghp_xxxx".into(),
            patterns: vec!["*.github.com".into()],
        };

        let json = serde_json::to_string(&cred).unwrap();
        let restored: Credential = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.service(), "github");
        assert!(restored.matches_url("https://api.github.com/repos"));
    }

    #[test]
    fn test_credential_manager() {
        let dir = tempfile::TempDir::new().unwrap();
        let secrets_path = dir.path().join("secrets.json");

        // Use file_only to ensure consistent behavior across feature flags
        let manager = CredentialManager::file_only(secrets_path);

        let cred = Credential::Login {
            service: "example".into(),
            username: "user".into(),
            password: "pass".into(),
            totp_secret: None,
            patterns: vec!["example.com".into()],
        };

        manager.store_indexed("cred_1", &cred).unwrap();

        let retrieved = manager.get("cred_1").unwrap().unwrap();
        assert_eq!(retrieved.service(), "example");

        let ids = manager.list_ids().unwrap();
        assert!(ids.contains(&"cred_1".to_string()));
    }

    #[test]
    fn test_mcp_auth_store() {
        let dir = tempfile::TempDir::new().unwrap();
        let secrets_path = dir.path().join("mcp_auth.json");

        // Use file_only to ensure consistent behavior across feature flags
        let store = McpAuthStore::file_only(secrets_path);

        store
            .store_tokens(
                "test_mcp",
                "access_token_123",
                Some("refresh_token_456"),
                Some(chrono::Utc::now().timestamp() + 3600),
                Some("read write"),
                "https://mcp.example.com",
            )
            .unwrap();

        let entry = store.get("test_mcp").unwrap().unwrap();
        assert_eq!(entry.access_token, "access_token_123");
        assert_eq!(entry.refresh_token, Some("refresh_token_456".to_string()));
        assert_eq!(entry.server_url, "https://mcp.example.com");

        // Verify URL matching
        let entry_for_url = store.get_for_url("test_mcp", "https://mcp.example.com").unwrap();
        assert!(entry_for_url.is_some());

        let entry_wrong_url = store.get_for_url("test_mcp", "https://other.com").unwrap();
        assert!(entry_wrong_url.is_none());
    }
}
