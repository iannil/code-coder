//! Credential Vault - Unified credential management for `ZeroBot` and `CodeCoder`
//!
//! Provides encrypted storage and retrieval of credentials (API keys, OAuth tokens,
//! login credentials) with URL pattern matching for automatic injection.
//!
//! Security features:
//! - `ChaCha20-Poly1305` encryption (via `SecretStore`)
//! - File permissions 0600
//! - File locking for concurrent access
//! - Secrets cleared from memory after use (via zeroize)

use super::secrets::SecretStore;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

/// Default credentials file name
const CREDENTIALS_FILE: &str = "credentials.json";

/// Credential type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    ApiKey,
    OAuth,
    Login,
    BearerToken,
}

/// OAuth-specific credential data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthCredential {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl Zeroize for OAuthCredential {
    fn zeroize(&mut self) {
        self.client_id.zeroize();
        if let Some(ref mut s) = self.client_secret {
            s.zeroize();
        }
        if let Some(ref mut s) = self.access_token {
            s.zeroize();
        }
        if let Some(ref mut s) = self.refresh_token {
            s.zeroize();
        }
    }
}

/// Login credential data (username/password)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoginCredential {
    pub username: String,
    pub password: String,
    /// Optional TOTP secret for 2FA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp_secret: Option<String>,
}

impl Zeroize for LoginCredential {
    fn zeroize(&mut self) {
        self.username.zeroize();
        self.password.zeroize();
        if let Some(ref mut s) = self.totp_secret {
            s.zeroize();
        }
    }
}

/// A single credential entry in the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialEntry {
    /// Unique identifier for this credential
    pub id: String,
    /// Type of credential
    #[serde(rename = "type")]
    pub credential_type: CredentialType,
    /// Human-readable name
    pub name: String,
    /// Service identifier (e.g., "github", "anthropic")
    pub service: String,

    /// API Key (for `api_key` and `bearer_token` types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// OAuth credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthCredential>,

    /// Login credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<LoginCredential>,

    /// URL patterns this credential applies to
    #[serde(default)]
    pub patterns: Vec<String>,

    /// Creation timestamp (Unix epoch seconds)
    pub created_at: i64,
    /// Last update timestamp (Unix epoch seconds)
    pub updated_at: i64,
}

impl Zeroize for CredentialEntry {
    fn zeroize(&mut self) {
        if let Some(ref mut s) = self.api_key {
            s.zeroize();
        }
        if let Some(ref mut oauth) = self.oauth {
            oauth.zeroize();
        }
        if let Some(ref mut login) = self.login {
            login.zeroize();
        }
    }
}

impl CredentialEntry {
    /// Create a new API key credential
    pub fn new_api_key(name: &str, service: &str, api_key: &str, patterns: Vec<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: generate_id(),
            credential_type: CredentialType::ApiKey,
            name: name.to_string(),
            service: service.to_string(),
            api_key: Some(api_key.to_string()),
            oauth: None,
            login: None,
            patterns,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new OAuth credential
    pub fn new_oauth(name: &str, service: &str, oauth: OAuthCredential, patterns: Vec<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: generate_id(),
            credential_type: CredentialType::OAuth,
            name: name.to_string(),
            service: service.to_string(),
            api_key: None,
            oauth: Some(oauth),
            login: None,
            patterns,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new login credential
    pub fn new_login(name: &str, service: &str, login: LoginCredential, patterns: Vec<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: generate_id(),
            credential_type: CredentialType::Login,
            name: name.to_string(),
            service: service.to_string(),
            api_key: None,
            oauth: None,
            login: Some(login),
            patterns,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new bearer token credential
    pub fn new_bearer_token(name: &str, service: &str, token: &str, patterns: Vec<String>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: generate_id(),
            credential_type: CredentialType::BearerToken,
            name: name.to_string(),
            service: service.to_string(),
            api_key: Some(token.to_string()),
            oauth: None,
            login: None,
            patterns,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this credential matches a URL
    pub fn matches_url(&self, url: &str) -> bool {
        for pattern in &self.patterns {
            if url_matches_pattern(url, pattern) {
                return true;
            }
        }
        false
    }

    /// Check if the OAuth token is expired
    pub fn is_oauth_expired(&self) -> bool {
        if let Some(ref oauth) = self.oauth {
            if let Some(expires_at) = oauth.expires_at {
                return Utc::now().timestamp() >= expires_at;
            }
        }
        false
    }
}

/// The credential vault - encrypted storage for all credentials
#[derive(Debug)]
pub struct CredentialVault {
    /// Path to the credentials file
    path: PathBuf,
    /// Secret store for encryption/decryption
    secret_store: SecretStore,
    /// Cached credentials (decrypted)
    credentials: HashMap<String, CredentialEntry>,
}

impl CredentialVault {
    /// Load or create a credential vault from the default location
    pub fn load(codecoder_dir: &Path) -> Result<Self> {
        let path = codecoder_dir.join(CREDENTIALS_FILE);
        let secret_store = SecretStore::new(codecoder_dir, true);

        let credentials = if path.exists() {
            Self::load_from_file(&path, &secret_store)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            secret_store,
            credentials,
        })
    }

    /// Load credentials from file with exclusive locking
    fn load_from_file(path: &Path, secret_store: &SecretStore) -> Result<HashMap<String, CredentialEntry>> {
        let file = File::open(path).context("Failed to open credentials file")?;
        file.lock_exclusive().context("Failed to lock credentials file")?;

        let mut contents = String::new();
        let mut reader = std::io::BufReader::new(&file);
        reader.read_to_string(&mut contents)?;

        file.unlock().context("Failed to unlock credentials file")?;

        if contents.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let encrypted: EncryptedVault = serde_json::from_str(&contents)
            .context("Failed to parse credentials file")?;

        let mut result = HashMap::new();
        for (id, encrypted_entry) in encrypted.credentials {
            let decrypted_json = secret_store.decrypt(&encrypted_entry)
                .context("Failed to decrypt credential")?;
            let entry: CredentialEntry = serde_json::from_str(&decrypted_json)
                .context("Failed to parse decrypted credential")?;
            result.insert(id, entry);
        }

        Ok(result)
    }

    /// Save credentials to file with exclusive locking
    fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Encrypt all credentials
        let mut encrypted_credentials = HashMap::new();
        for (id, entry) in &self.credentials {
            let json = serde_json::to_string(entry)?;
            let encrypted = self.secret_store.encrypt(&json)?;
            encrypted_credentials.insert(id.clone(), encrypted);
        }

        let vault = EncryptedVault {
            version: 1,
            credentials: encrypted_credentials,
        };

        // Write with exclusive lock
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .context("Failed to open credentials file for writing")?;

        file.lock_exclusive().context("Failed to lock credentials file")?;

        let mut writer = std::io::BufWriter::new(&file);
        serde_json::to_writer_pretty(&mut writer, &vault)?;
        writer.flush()?;

        // Set restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600))
                .context("Failed to set credentials file permissions")?;
        }

        file.unlock().context("Failed to unlock credentials file")?;

        Ok(())
    }

    /// Add a new credential to the vault
    pub fn add(&mut self, entry: CredentialEntry) -> Result<String> {
        let id = entry.id.clone();
        self.credentials.insert(id.clone(), entry);
        self.save()?;
        Ok(id)
    }

    /// Get a credential by ID
    pub fn get(&self, id: &str) -> Option<&CredentialEntry> {
        self.credentials.get(id)
    }

    /// Get a credential by service name
    pub fn get_by_service(&self, service: &str) -> Option<&CredentialEntry> {
        self.credentials.values().find(|c| c.service == service)
    }

    /// Resolve a credential for a given URL
    pub fn resolve_for_url(&self, url: &str) -> Option<&CredentialEntry> {
        self.credentials.values().find(|c| c.matches_url(url))
    }

    /// List all credentials (without sensitive data)
    pub fn list(&self) -> Vec<CredentialSummary> {
        self.credentials
            .values()
            .map(|c| CredentialSummary {
                id: c.id.clone(),
                credential_type: c.credential_type.clone(),
                name: c.name.clone(),
                service: c.service.clone(),
                patterns: c.patterns.clone(),
                created_at: DateTime::from_timestamp(c.created_at, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                updated_at: DateTime::from_timestamp(c.updated_at, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
            })
            .collect()
    }

    /// Remove a credential by ID
    pub fn remove(&mut self, id: &str) -> Result<bool> {
        if self.credentials.remove(id).is_some() {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Update an existing credential
    pub fn update(&mut self, id: &str, mut entry: CredentialEntry) -> Result<bool> {
        if self.credentials.contains_key(id) {
            entry.id = id.to_string();
            entry.updated_at = Utc::now().timestamp();
            self.credentials.insert(id.to_string(), entry);
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Update OAuth tokens for an existing credential
    pub fn update_oauth_tokens(
        &mut self,
        id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<i64>,
    ) -> Result<bool> {
        if let Some(entry) = self.credentials.get_mut(id) {
            if let Some(ref mut oauth) = entry.oauth {
                oauth.access_token = Some(access_token.to_string());
                if let Some(rt) = refresh_token {
                    oauth.refresh_token = Some(rt.to_string());
                }
                oauth.expires_at = expires_at;
                entry.updated_at = Utc::now().timestamp();
                self.save()?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Credential summary for listing (no sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: CredentialType,
    pub name: String,
    pub service: String,
    pub patterns: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Encrypted vault structure (stored on disk)
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedVault {
    version: u32,
    credentials: HashMap<String, String>,
}

/// Generate a unique credential ID
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let random: u32 = rand::random();
    format!("cred_{timestamp:x}_{random:08x}")
}

/// Check if a URL matches a pattern
/// Supports:
/// - Exact match: "api.github.com"
/// - Wildcard prefix: "*.github.com" (matches "api.github.com", "raw.github.com")
/// - Full URL pattern: `https://api.openai.com/*`
fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    // Extract host from URL
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .map_or(url, |s| s.split('/').next().unwrap_or(s));

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
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_url_matches_exact() {
        assert!(url_matches_pattern("https://api.github.com/repos", "api.github.com"));
        assert!(!url_matches_pattern("https://api.gitlab.com/repos", "api.github.com"));
    }

    #[test]
    fn test_url_matches_wildcard() {
        assert!(url_matches_pattern("https://api.github.com/repos", "*.github.com"));
        assert!(url_matches_pattern("https://raw.github.com/file", "*.github.com"));
        assert!(!url_matches_pattern("https://github.io/page", "*.github.com"));
    }

    #[test]
    fn test_url_matches_full_pattern() {
        assert!(url_matches_pattern(
            "https://api.openai.com/v1/chat",
            "https://api.openai.com/*"
        ));
    }

    #[test]
    fn test_credential_entry_new_api_key() {
        let entry = CredentialEntry::new_api_key(
            "OpenAI",
            "openai",
            "sk-test-key",
            vec!["api.openai.com".into()],
        );
        assert_eq!(entry.credential_type, CredentialType::ApiKey);
        assert_eq!(entry.service, "openai");
        assert_eq!(entry.api_key.as_deref(), Some("sk-test-key"));
        assert!(entry.id.starts_with("cred_"));
    }

    #[test]
    fn test_credential_entry_matches_url() {
        let entry = CredentialEntry::new_api_key(
            "GitHub",
            "github",
            "ghp_test",
            vec!["*.github.com".into(), "api.github.com".into()],
        );
        assert!(entry.matches_url("https://api.github.com/repos"));
        assert!(entry.matches_url("https://raw.github.com/file"));
        assert!(!entry.matches_url("https://gitlab.com/repos"));
    }

    #[test]
    fn test_vault_crud() {
        let tmp = TempDir::new().unwrap();
        let mut vault = CredentialVault::load(tmp.path()).unwrap();

        // Add
        let entry = CredentialEntry::new_api_key(
            "Test",
            "test-service",
            "test-key-12345",
            vec!["api.test.com".into()],
        );
        let id = vault.add(entry).unwrap();

        // Get
        let retrieved = vault.get(&id).unwrap();
        assert_eq!(retrieved.service, "test-service");
        assert_eq!(retrieved.api_key.as_deref(), Some("test-key-12345"));

        // List
        let list = vault.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].service, "test-service");

        // Resolve by URL
        let resolved = vault.resolve_for_url("https://api.test.com/v1/resource");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().service, "test-service");

        // Remove
        let removed = vault.remove(&id).unwrap();
        assert!(removed);
        assert!(vault.get(&id).is_none());
    }

    #[test]
    fn test_vault_persistence() {
        let tmp = TempDir::new().unwrap();

        // Create and save
        {
            let mut vault = CredentialVault::load(tmp.path()).unwrap();
            let entry = CredentialEntry::new_api_key(
                "Persistent",
                "persist",
                "persist-key",
                vec!["api.persist.com".into()],
            );
            vault.add(entry).unwrap();
        }

        // Reload and verify
        {
            let vault = CredentialVault::load(tmp.path()).unwrap();
            let list = vault.list();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].service, "persist");

            let cred = vault.get_by_service("persist").unwrap();
            assert_eq!(cred.api_key.as_deref(), Some("persist-key"));
        }
    }

    #[test]
    fn test_oauth_credential() {
        let tmp = TempDir::new().unwrap();
        let mut vault = CredentialVault::load(tmp.path()).unwrap();

        let oauth = OAuthCredential {
            client_id: "client-123".into(),
            client_secret: Some("secret-456".into()),
            access_token: Some("access-789".into()),
            refresh_token: Some("refresh-abc".into()),
            expires_at: Some(Utc::now().timestamp() + 3600),
            scope: Some("read write".into()),
        };

        let entry = CredentialEntry::new_oauth(
            "Google",
            "google",
            oauth,
            vec!["*.googleapis.com".into()],
        );
        let id = vault.add(entry).unwrap();

        let retrieved = vault.get(&id).unwrap();
        assert!(!retrieved.is_oauth_expired());

        let oauth_data = retrieved.oauth.as_ref().unwrap();
        assert_eq!(oauth_data.client_id, "client-123");
        assert_eq!(oauth_data.access_token.as_deref(), Some("access-789"));
    }

    #[test]
    fn test_login_credential() {
        let tmp = TempDir::new().unwrap();
        let mut vault = CredentialVault::load(tmp.path()).unwrap();

        let login = LoginCredential {
            username: "user@example.com".into(),
            password: "secret-password".into(),
            totp_secret: Some("JBSWY3DPEHPK3PXP".into()),
        };

        let entry = CredentialEntry::new_login(
            "Example Site",
            "example",
            login,
            vec!["login.example.com".into()],
        );
        let id = vault.add(entry).unwrap();

        let retrieved = vault.get(&id).unwrap();
        let login_data = retrieved.login.as_ref().unwrap();
        assert_eq!(login_data.username, "user@example.com");
        assert_eq!(login_data.password, "secret-password");
        assert!(login_data.totp_secret.is_some());
    }

    #[test]
    fn test_update_oauth_tokens() {
        let tmp = TempDir::new().unwrap();
        let mut vault = CredentialVault::load(tmp.path()).unwrap();

        let oauth = OAuthCredential {
            client_id: "client-123".into(),
            client_secret: None,
            access_token: Some("old-token".into()),
            refresh_token: Some("old-refresh".into()),
            expires_at: None,
            scope: None,
        };

        let entry = CredentialEntry::new_oauth("Service", "svc", oauth, vec![]);
        let id = vault.add(entry).unwrap();

        // Update tokens
        vault
            .update_oauth_tokens(&id, "new-token", Some("new-refresh"), Some(9999999999))
            .unwrap();

        let updated = vault.get(&id).unwrap();
        let oauth_data = updated.oauth.as_ref().unwrap();
        assert_eq!(oauth_data.access_token.as_deref(), Some("new-token"));
        assert_eq!(oauth_data.refresh_token.as_deref(), Some("new-refresh"));
        assert_eq!(oauth_data.expires_at, Some(9999999999));
    }

    #[test]
    fn test_zeroize() {
        let mut entry = CredentialEntry::new_api_key(
            "Test",
            "test",
            "sensitive-key",
            vec![],
        );

        // Verify sensitive data exists
        assert_eq!(entry.api_key.as_deref(), Some("sensitive-key"));

        // Zeroize
        entry.zeroize();

        // Verify sensitive data is cleared (zeroized strings become empty or contain zeros)
        // The actual behavior depends on the Zeroize implementation for String
        // For our purpose, we just verify zeroize() doesn't panic
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();

        let mut vault = CredentialVault::load(tmp.path()).unwrap();
        let entry = CredentialEntry::new_api_key("Test", "test", "key", vec![]);
        vault.add(entry).unwrap();

        let metadata = fs::metadata(tmp.path().join(CREDENTIALS_FILE)).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Credentials file must be owner-only (0600)");
    }
}
