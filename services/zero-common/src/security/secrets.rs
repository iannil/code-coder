//! Secret storage with encrypted persistence.
//!
//! Uses ChaCha20-Poly1305 for authenticated encryption.

use anyhow::{Context, Result};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroizing;

/// Encrypted secret store.
pub struct SecretStore {
    path: PathBuf,
    cipher: ChaCha20Poly1305,
    secrets: HashMap<String, Zeroizing<String>>,
}

/// Serialized format for the secret store file.
#[derive(Serialize, Deserialize)]
struct EncryptedStore {
    /// Base64-encoded nonce
    nonce: String,
    /// Base64-encoded ciphertext
    ciphertext: String,
}

impl SecretStore {
    /// Create or open a secret store at the given path with the given key.
    pub fn open(path: PathBuf, key: &[u8; 32]) -> Result<Self> {
        let cipher = ChaCha20Poly1305::new(key.into());

        let secrets = if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read secret store from {}", path.display()))?;
            let store: EncryptedStore = serde_json::from_str(&content)
                .with_context(|| "Failed to parse secret store")?;

            let nonce_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &store.nonce,
            )
            .with_context(|| "Failed to decode nonce")?;
            let ciphertext = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &store.ciphertext,
            )
            .with_context(|| "Failed to decode ciphertext")?;

            let nonce = Nonce::from_slice(&nonce_bytes);
            let plaintext = cipher
                .decrypt(nonce, ciphertext.as_ref())
                .map_err(|_| anyhow::anyhow!("Failed to decrypt secret store - wrong key?"))?;

            let data: HashMap<String, String> = serde_json::from_slice(&plaintext)
                .with_context(|| "Failed to parse decrypted secrets")?;

            data.into_iter()
                .map(|(k, v)| (k, Zeroizing::new(v)))
                .collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            cipher,
            secrets,
        })
    }

    /// Get a secret by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.secrets.get(key).map(|s| s.as_str())
    }

    /// Set a secret.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.secrets.insert(key.into(), Zeroizing::new(value.into()));
    }

    /// Remove a secret.
    pub fn remove(&mut self, key: &str) -> Option<Zeroizing<String>> {
        self.secrets.remove(key)
    }

    /// List all secret keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.secrets.keys().map(String::as_str)
    }

    /// Save the secret store to disk.
    pub fn save(&self) -> Result<()> {
        let data: HashMap<&str, &str> = self
            .secrets
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let plaintext = serde_json::to_vec(&data)?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|_| anyhow::anyhow!("Failed to encrypt secrets"))?;

        let store = EncryptedStore {
            nonce: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, nonce),
            ciphertext: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                ciphertext,
            ),
        };

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&store)?;
        fs::write(&self.path, content)
            .with_context(|| format!("Failed to write secret store to {}", self.path.display()))
    }
}

/// Generate a random 32-byte key for the secret store.
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key);
    key
}

/// Derive a key from a password using a simple (but not ideal) method.
/// For production, use a proper KDF like Argon2.
pub fn derive_key_simple(password: &str, salt: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_secret_store_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("secrets.json");
        let key = generate_key();

        // Create and save
        {
            let mut store = SecretStore::open(path.clone(), &key).unwrap();
            store.set("api_key", "sk-test-12345");
            store.set("password", "secret123");
            store.save().unwrap();
        }

        // Load and verify
        {
            let store = SecretStore::open(path, &key).unwrap();
            assert_eq!(store.get("api_key"), Some("sk-test-12345"));
            assert_eq!(store.get("password"), Some("secret123"));
            assert_eq!(store.get("nonexistent"), None);
        }
    }

    #[test]
    fn test_wrong_key_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("secrets.json");
        let key1 = generate_key();
        let key2 = generate_key();

        // Create with key1
        {
            let mut store = SecretStore::open(path.clone(), &key1).unwrap();
            store.set("test", "value");
            store.save().unwrap();
        }

        // Try to open with key2
        let result = SecretStore::open(path, &key2);
        assert!(result.is_err());
    }
}
