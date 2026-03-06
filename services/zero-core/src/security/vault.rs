//! Encrypted credential vault

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Path to the vault file
    pub path: PathBuf,
    /// Whether to use system keychain
    #[serde(default)]
    pub use_keychain: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codecoder")
                .join("vault.enc"),
            use_keychain: false,
        }
    }
}

/// A secret entry in the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    /// Secret name/key
    pub name: String,
    /// Secret value (encrypted at rest)
    pub value: String,
    /// Optional description
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl SecretEntry {
    /// Create a new secret entry
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            name: name.into(),
            value: value.into(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Encrypted vault for storing secrets
pub struct Vault {
    config: VaultConfig,
    key: [u8; 32],
    secrets: HashMap<String, SecretEntry>,
}

impl Vault {
    /// Create or open a vault with a master password
    pub fn open(config: VaultConfig, password: &str) -> Result<Self> {
        // Derive key from password using a simple hash (in production, use Argon2)
        let key = Self::derive_key(password);

        let mut vault = Self {
            config,
            key,
            secrets: HashMap::new(),
        };

        // Load existing vault if it exists
        if vault.config.path.exists() {
            vault.load()?;
        }

        Ok(vault)
    }

    /// Create a new in-memory vault (for testing)
    pub fn in_memory(password: &str) -> Self {
        Self {
            config: VaultConfig::default(),
            key: Self::derive_key(password),
            secrets: HashMap::new(),
        }
    }

    /// Derive encryption key from password
    fn derive_key(password: &str) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"codecoder-vault-salt-v1");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    /// Store a secret
    pub fn set(&mut self, entry: SecretEntry) {
        self.secrets.insert(entry.name.clone(), entry);
    }

    /// Get a secret by name
    pub fn get(&self, name: &str) -> Option<&SecretEntry> {
        self.secrets.get(name)
    }

    /// Get a secret value by name
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.secrets.get(name).map(|e| e.value.as_str())
    }

    /// Delete a secret
    pub fn delete(&mut self, name: &str) -> bool {
        self.secrets.remove(name).is_some()
    }

    /// List all secret names
    pub fn list(&self) -> Vec<&str> {
        self.secrets.keys().map(|s| s.as_str()).collect()
    }

    /// Save the vault to disk
    pub fn save(&self) -> Result<()> {
        // Serialize secrets
        let plaintext = serde_json::to_vec(&self.secrets)
            .context("Failed to serialize secrets")?;

        // Encrypt
        let cipher = ChaCha20Poly1305::new_from_slice(&self.key)
            .context("Failed to create cipher")?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Write to file: nonce || ciphertext
        let mut data = Vec::with_capacity(12 + ciphertext.len());
        data.extend_from_slice(&nonce_bytes);
        data.extend_from_slice(&ciphertext);

        // Ensure parent directory exists
        if let Some(parent) = self.config.path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create vault directory")?;
        }

        fs::write(&self.config.path, &data)
            .context("Failed to write vault file")?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.config.path, permissions)?;
        }

        Ok(())
    }

    /// Load the vault from disk
    fn load(&mut self) -> Result<()> {
        let data = fs::read(&self.config.path)
            .context("Failed to read vault file")?;

        if data.len() < 12 {
            anyhow::bail!("Invalid vault file: too short");
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = ChaCha20Poly1305::new_from_slice(&self.key)
            .context("Failed to create cipher")?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow::anyhow!("Decryption failed: wrong password or corrupted vault"))?;

        self.secrets = serde_json::from_slice(&plaintext)
            .context("Failed to deserialize secrets")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_basic() {
        let mut vault = Vault::in_memory("test-password");

        vault.set(SecretEntry::new("api_key", "sk-test-12345"));
        vault.set(SecretEntry::new("db_password", "supersecret").with_description("Database password"));

        assert_eq!(vault.get_value("api_key"), Some("sk-test-12345"));
        assert_eq!(vault.get_value("db_password"), Some("supersecret"));
        assert_eq!(vault.list().len(), 2);

        vault.delete("api_key");
        assert!(vault.get("api_key").is_none());
    }

    #[test]
    fn test_vault_persistence() {
        let dir = tempfile::TempDir::new().unwrap();
        let vault_path = dir.path().join("test-vault.enc");

        let config = VaultConfig {
            path: vault_path.clone(),
            use_keychain: false,
        };

        // Create and save
        {
            let mut vault = Vault::open(config.clone(), "password123").unwrap();
            vault.set(SecretEntry::new("secret", "value"));
            vault.save().unwrap();
        }

        // Load and verify
        {
            let vault = Vault::open(config, "password123").unwrap();
            assert_eq!(vault.get_value("secret"), Some("value"));
        }
    }

    #[test]
    fn test_vault_wrong_password() {
        let dir = tempfile::TempDir::new().unwrap();
        let vault_path = dir.path().join("test-vault.enc");

        let config = VaultConfig {
            path: vault_path.clone(),
            use_keychain: false,
        };

        // Create with one password
        {
            let mut vault = Vault::open(config.clone(), "correct-password").unwrap();
            vault.set(SecretEntry::new("secret", "value"));
            vault.save().unwrap();
        }

        // Try to open with wrong password
        let result = Vault::open(config, "wrong-password");
        assert!(result.is_err());
    }
}
