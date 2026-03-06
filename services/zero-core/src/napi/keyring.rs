//! NAPI bindings for keyring module
//!
//! Exposes credential management and MCP authentication to Node.js/TypeScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::PathBuf;
use std::sync::Arc;

use crate::security::keyring::{
    Credential as RustCredential, CredentialManager as RustCredentialManager,
    KeyringBackend as RustKeyringBackend, KeyringManager as RustKeyringManager,
    McpAuthEntry as RustMcpAuthEntry, McpAuthStore as RustMcpAuthStore,
};

// ============================================================================
// Keyring Types (NAPI)
// ============================================================================

/// Keyring backend type
#[napi(string_enum)]
pub enum KeyringBackend {
    System,
    File,
}

impl From<RustKeyringBackend> for KeyringBackend {
    fn from(b: RustKeyringBackend) -> Self {
        match b {
            RustKeyringBackend::System => KeyringBackend::System,
            RustKeyringBackend::File => KeyringBackend::File,
        }
    }
}

/// Credential type tag
#[napi(string_enum)]
pub enum CredentialType {
    ApiKey,
    OAuth,
    Login,
}

/// API Key credential
#[napi(object)]
pub struct NapiApiKeyCredential {
    pub service: String,
    pub key: String,
    pub patterns: Vec<String>,
}

/// OAuth credential
#[napi(object)]
pub struct NapiOAuthCredential {
    pub service: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    pub patterns: Vec<String>,
}

/// Login credential
#[napi(object)]
pub struct NapiLoginCredential {
    pub service: String,
    pub username: String,
    pub password: String,
    pub totp_secret: Option<String>,
    pub patterns: Vec<String>,
}

/// Unified credential representation for NAPI
#[napi(object)]
pub struct NapiCredential {
    pub credential_type: String,
    pub service: String,
    pub patterns: Vec<String>,
    // API Key fields
    pub key: Option<String>,
    // OAuth fields
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    // Login fields
    pub username: Option<String>,
    pub password: Option<String>,
    pub totp_secret: Option<String>,
}

impl From<RustCredential> for NapiCredential {
    fn from(c: RustCredential) -> Self {
        match c {
            RustCredential::ApiKey {
                service,
                key,
                patterns,
            } => Self {
                credential_type: "api_key".into(),
                service,
                patterns,
                key: Some(key),
                client_id: None,
                client_secret: None,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                scope: None,
                username: None,
                password: None,
                totp_secret: None,
            },
            RustCredential::OAuth {
                service,
                client_id,
                client_secret,
                access_token,
                refresh_token,
                expires_at,
                scope,
                patterns,
            } => Self {
                credential_type: "oauth".into(),
                service,
                patterns,
                key: None,
                client_id: Some(client_id),
                client_secret,
                access_token,
                refresh_token,
                expires_at,
                scope,
                username: None,
                password: None,
                totp_secret: None,
            },
            RustCredential::Login {
                service,
                username,
                password,
                totp_secret,
                patterns,
            } => Self {
                credential_type: "login".into(),
                service,
                patterns,
                key: None,
                client_id: None,
                client_secret: None,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                scope: None,
                username: Some(username),
                password: Some(password),
                totp_secret,
            },
        }
    }
}

/// MCP auth entry for NAPI
#[napi(object)]
pub struct NapiMcpAuthEntry {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    pub server_url: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
    pub oauth_state: Option<String>,
}

impl From<RustMcpAuthEntry> for NapiMcpAuthEntry {
    fn from(e: RustMcpAuthEntry) -> Self {
        Self {
            access_token: e.access_token,
            refresh_token: e.refresh_token,
            expires_at: e.expires_at,
            scope: e.scope,
            server_url: e.server_url,
            client_id: e.client_id,
            client_secret: e.client_secret,
            code_verifier: e.code_verifier,
            oauth_state: e.oauth_state,
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if system keyring is available
#[napi]
pub fn is_keyring_available() -> bool {
    RustKeyringManager::is_keyring_available()
}

// ============================================================================
// Keyring Manager Handle
// ============================================================================

/// Handle to a keyring manager
#[napi]
pub struct KeyringManagerHandle {
    inner: Arc<RustKeyringManager>,
}

/// Create a new keyring manager
///
/// Automatically selects system keyring if available, otherwise falls back to file.
#[napi]
pub fn create_keyring_manager(file_fallback_path: Option<String>) -> KeyringManagerHandle {
    let path = file_fallback_path.map(PathBuf::from);
    KeyringManagerHandle {
        inner: Arc::new(RustKeyringManager::new(path)),
    }
}

/// Create a keyring manager that only uses file storage
#[napi]
pub fn create_file_keyring_manager(path: String) -> KeyringManagerHandle {
    KeyringManagerHandle {
        inner: Arc::new(RustKeyringManager::file_only(PathBuf::from(path))),
    }
}

#[napi]
impl KeyringManagerHandle {
    /// Get the backend being used
    #[napi]
    pub fn backend(&self) -> KeyringBackend {
        self.inner.backend().into()
    }

    /// Store a secret
    #[napi]
    pub fn set(&self, key: String, value: String) -> Result<()> {
        self.inner
            .set(&key, &value)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get a secret
    #[napi]
    pub fn get(&self, key: String) -> Result<Option<String>> {
        self.inner
            .get(&key)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete a secret
    #[napi]
    pub fn delete(&self, key: String) -> Result<bool> {
        self.inner
            .delete(&key)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if a secret exists
    #[napi]
    pub fn exists(&self, key: String) -> Result<bool> {
        self.inner
            .exists(&key)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// Credential Manager Handle
// ============================================================================

/// Handle to a credential manager
#[napi]
pub struct CredentialManagerHandle {
    inner: Arc<RustCredentialManager>,
}

/// Create a new credential manager
#[napi]
pub fn create_credential_manager(file_fallback_path: Option<String>) -> CredentialManagerHandle {
    let path = file_fallback_path.map(PathBuf::from);
    CredentialManagerHandle {
        inner: Arc::new(RustCredentialManager::new(path)),
    }
}

/// Create a credential manager that only uses file storage (for testing)
#[napi]
pub fn create_file_credential_manager(path: String) -> CredentialManagerHandle {
    CredentialManagerHandle {
        inner: Arc::new(RustCredentialManager::file_only(PathBuf::from(path))),
    }
}

#[napi]
impl CredentialManagerHandle {
    /// Get the backend being used
    #[napi]
    pub fn backend(&self) -> KeyringBackend {
        self.inner.backend().into()
    }

    /// Store an API key credential
    #[napi]
    pub fn store_api_key(
        &self,
        id: String,
        service: String,
        key: String,
        patterns: Vec<String>,
    ) -> Result<()> {
        let credential = RustCredential::ApiKey {
            service,
            key,
            patterns,
        };
        self.inner
            .store_indexed(&id, &credential)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Store an OAuth credential
    #[napi]
    pub fn store_oauth(
        &self,
        id: String,
        service: String,
        client_id: String,
        client_secret: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        expires_at: Option<i64>,
        scope: Option<String>,
        patterns: Vec<String>,
    ) -> Result<()> {
        let credential = RustCredential::OAuth {
            service,
            client_id,
            client_secret,
            access_token,
            refresh_token,
            expires_at,
            scope,
            patterns,
        };
        self.inner
            .store_indexed(&id, &credential)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Store a login credential
    #[napi]
    pub fn store_login(
        &self,
        id: String,
        service: String,
        username: String,
        password: String,
        totp_secret: Option<String>,
        patterns: Vec<String>,
    ) -> Result<()> {
        let credential = RustCredential::Login {
            service,
            username,
            password,
            totp_secret,
            patterns,
        };
        self.inner
            .store_indexed(&id, &credential)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get a credential by ID
    #[napi]
    pub fn get(&self, id: String) -> Result<Option<NapiCredential>> {
        self.inner
            .get(&id)
            .map(|opt| opt.map(Into::into))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete a credential
    #[napi]
    pub fn delete(&self, id: String) -> Result<bool> {
        self.inner
            .delete(&id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find credential by service name
    #[napi]
    pub fn find_by_service(&self, service: String) -> Result<Option<NapiCredential>> {
        self.inner
            .find_by_service(&service)
            .map(|opt| opt.map(|(_, cred)| cred.into()))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Find credential matching a URL
    #[napi]
    pub fn find_for_url(&self, url: String) -> Result<Option<NapiCredential>> {
        self.inner
            .find_for_url(&url)
            .map(|opt| opt.map(|(_, cred)| cred.into()))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List all credential IDs
    #[napi]
    pub fn list_ids(&self) -> Result<Vec<String>> {
        self.inner
            .list_ids()
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// MCP Auth Store Handle
// ============================================================================

/// Handle to an MCP auth store
#[napi]
pub struct McpAuthStoreHandle {
    inner: Arc<RustMcpAuthStore>,
}

/// Create a new MCP auth store
#[napi]
pub fn create_mcp_auth_store(file_fallback_path: Option<String>) -> McpAuthStoreHandle {
    let path = file_fallback_path.map(PathBuf::from);
    McpAuthStoreHandle {
        inner: Arc::new(RustMcpAuthStore::new(path)),
    }
}

/// Create an MCP auth store that only uses file storage (for testing)
#[napi]
pub fn create_file_mcp_auth_store(path: String) -> McpAuthStoreHandle {
    McpAuthStoreHandle {
        inner: Arc::new(RustMcpAuthStore::file_only(PathBuf::from(path))),
    }
}

#[napi]
impl McpAuthStoreHandle {
    /// Store MCP OAuth tokens
    #[napi]
    pub fn store_tokens(
        &self,
        mcp_name: String,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<i64>,
        scope: Option<String>,
        server_url: String,
    ) -> Result<()> {
        self.inner
            .store_tokens(
                &mcp_name,
                &access_token,
                refresh_token.as_deref(),
                expires_at,
                scope.as_deref(),
                &server_url,
            )
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get MCP auth entry
    #[napi]
    pub fn get(&self, mcp_name: String) -> Result<Option<NapiMcpAuthEntry>> {
        self.inner
            .get(&mcp_name)
            .map(|opt| opt.map(Into::into))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get auth for a specific URL (validates URL matches)
    #[napi]
    pub fn get_for_url(
        &self,
        mcp_name: String,
        server_url: String,
    ) -> Result<Option<NapiMcpAuthEntry>> {
        self.inner
            .get_for_url(&mcp_name, &server_url)
            .map(|opt| opt.map(Into::into))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if tokens are expired
    #[napi]
    pub fn is_expired(&self, mcp_name: String) -> Result<Option<bool>> {
        self.inner
            .is_expired(&mcp_name)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Update tokens
    #[napi]
    pub fn update_tokens(
        &self,
        mcp_name: String,
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<i64>,
    ) -> Result<()> {
        self.inner
            .update_tokens(&mcp_name, &access_token, refresh_token.as_deref(), expires_at)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Store PKCE code verifier
    #[napi]
    pub fn store_code_verifier(&self, mcp_name: String, code_verifier: String) -> Result<()> {
        self.inner
            .store_code_verifier(&mcp_name, &code_verifier)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Store OAuth state
    #[napi]
    pub fn store_oauth_state(&self, mcp_name: String, state: String) -> Result<()> {
        self.inner
            .store_oauth_state(&mcp_name, &state)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete MCP auth entry
    #[napi]
    pub fn delete(&self, mcp_name: String) -> Result<bool> {
        self.inner
            .delete(&mcp_name)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_keyring_manager_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secrets.json").to_string_lossy().to_string();

        let manager = create_file_keyring_manager(path);
        assert!(matches!(manager.backend(), KeyringBackend::File));

        manager.set("test_key".into(), "test_value".into()).unwrap();
        assert_eq!(
            manager.get("test_key".into()).unwrap(),
            Some("test_value".into())
        );

        assert!(manager.exists("test_key".into()).unwrap());
        assert!(manager.delete("test_key".into()).unwrap());
        assert!(!manager.exists("test_key".into()).unwrap());
    }

    #[test]
    fn test_credential_manager() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("creds.json").to_string_lossy().to_string();

        let manager = create_file_credential_manager(path);

        manager
            .store_api_key(
                "github".into(),
                "github".into(),
                "ghp_xxxx".into(),
                vec!["*.github.com".into()],
            )
            .unwrap();

        let cred = manager.get("github".into()).unwrap().unwrap();
        assert_eq!(cred.credential_type, "api_key");
        assert_eq!(cred.service, "github");
        assert_eq!(cred.key, Some("ghp_xxxx".into()));

        let ids = manager.list_ids().unwrap();
        assert!(ids.contains(&"github".into()));
    }

    #[test]
    fn test_mcp_auth_store() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mcp_auth.json").to_string_lossy().to_string();

        let store = create_file_mcp_auth_store(path);

        store
            .store_tokens(
                "test_mcp".into(),
                "access_123".into(),
                Some("refresh_456".into()),
                Some(chrono::Utc::now().timestamp() + 3600),
                Some("read write".into()),
                "https://mcp.example.com".into(),
            )
            .unwrap();

        let entry = store.get("test_mcp".into()).unwrap().unwrap();
        assert_eq!(entry.access_token, "access_123");
        assert_eq!(entry.refresh_token, Some("refresh_456".into()));
        assert_eq!(entry.server_url, "https://mcp.example.com");

        // Test URL validation
        let entry_for_url = store
            .get_for_url("test_mcp".into(), "https://mcp.example.com".into())
            .unwrap();
        assert!(entry_for_url.is_some());

        let entry_wrong_url = store
            .get_for_url("test_mcp".into(), "https://other.com".into())
            .unwrap();
        assert!(entry_wrong_url.is_none());
    }
}
