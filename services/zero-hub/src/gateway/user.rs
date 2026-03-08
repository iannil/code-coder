//! User management for Zero Gateway.
//!
//! Provides user storage, password hashing, and CRUD operations.

use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use zero_common::security::rbac::{check_permission, Permission};

/// User record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID
    pub id: String,
    /// Username for login
    pub username: String,
    /// Argon2 password hash (never exposed in API responses)
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Assigned roles
    pub roles: Vec<String>,
    /// Whether the user is enabled
    pub enabled: bool,
    /// Email address (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Display name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Last login timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Request to create a new user.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default = "default_roles")]
    pub roles: Vec<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Request to update an existing user.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub roles: Option<Vec<String>>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

fn default_roles() -> Vec<String> {
    vec!["user".to_string()]
}

/// User store backed by SQLite.
#[derive(Clone)]
pub struct UserStore {
    conn: Arc<Mutex<Connection>>,
}

impl UserStore {
    /// Create a new user store at the given database path.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create users table
        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                roles TEXT NOT NULL DEFAULT '[]',
                enabled INTEGER NOT NULL DEFAULT 1,
                email TEXT,
                display_name TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_login_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
            CREATE INDEX IF NOT EXISTS idx_users_enabled ON users(enabled);
            ",
        )?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Create default admin user if no users exist
        if store.count()? == 0 {
            tracing::info!("Creating default admin user");
            store.create(&CreateUserRequest {
                username: "admin".to_string(),
                password: "admin123".to_string(), // Should be changed immediately!
                roles: vec!["admin".to_string()],
                email: None,
                display_name: Some("Administrator".to_string()),
            })?;
        }

        Ok(store)
    }

    /// Create a new user.
    pub fn create(&self, request: &CreateUserRequest) -> Result<User> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        // Validate username
        if request.username.is_empty() {
            anyhow::bail!("Username cannot be empty");
        }
        if request.username.len() > 64 {
            anyhow::bail!("Username too long (max 64 characters)");
        }

        // Validate password
        if request.password.len() < 8 {
            anyhow::bail!("Password must be at least 8 characters");
        }

        // Hash password
        let password_hash = hash_password(&request.password)?;

        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let roles_json = serde_json::to_string(&request.roles)?;

        conn.execute(
            r"
            INSERT INTO users (id, username, password_hash, roles, enabled, email, display_name, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?7)
            ",
            params![
                id,
                request.username,
                password_hash,
                roles_json,
                request.email,
                request.display_name,
                now.to_rfc3339(),
            ],
        )
        .with_context(|| format!("Failed to create user '{}'", request.username))?;

        Ok(User {
            id,
            username: request.username.clone(),
            password_hash,
            roles: request.roles.clone(),
            enabled: true,
            email: request.email.clone(),
            display_name: request.display_name.clone(),
            created_at: now,
            updated_at: now,
            last_login_at: None,
        })
    }

    /// Get a user by ID.
    pub fn get(&self, id: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.get_internal(&conn, "id", id)
    }

    /// Get a user by username.
    pub fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.get_internal(&conn, "username", username)
    }

    fn get_internal(&self, conn: &Connection, field: &str, value: &str) -> Result<Option<User>> {
        let query = format!(
            "SELECT id, username, password_hash, roles, enabled, email, display_name, created_at, updated_at, last_login_at
             FROM users WHERE {} = ?1",
            field
        );

        conn.query_row(&query, params![value], |row| {
            let roles_json: String = row.get(3)?;
            let created_at: String = row.get(7)?;
            let updated_at: String = row.get(8)?;
            let last_login_at: Option<String> = row.get(9)?;

            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                roles: serde_json::from_str(&roles_json).unwrap_or_default(),
                enabled: row.get::<_, i64>(4)? != 0,
                email: row.get(5)?,
                display_name: row.get(6)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_login_at: last_login_at.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                }),
            })
        })
        .optional()
        .with_context(|| format!("Failed to get user by {} = {}", field, value))
    }

    /// Update a user.
    pub fn update(&self, id: &str, request: &UpdateUserRequest) -> Result<Option<User>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        // Check if user exists
        let exists: bool = conn
            .query_row("SELECT 1 FROM users WHERE id = ?1", params![id], |_| {
                Ok(true)
            })
            .optional()?
            .unwrap_or(false);

        if !exists {
            return Ok(None);
        }

        let now = Utc::now().to_rfc3339();

        // Update password if provided
        if let Some(password) = &request.password {
            if password.len() < 8 {
                anyhow::bail!("Password must be at least 8 characters");
            }
            let hash = hash_password(password)?;
            conn.execute(
                "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
                params![hash, now, id],
            )?;
        }

        // Update roles if provided
        if let Some(roles) = &request.roles {
            let roles_json = serde_json::to_string(roles)?;
            conn.execute(
                "UPDATE users SET roles = ?1, updated_at = ?2 WHERE id = ?3",
                params![roles_json, now, id],
            )?;
        }

        // Update enabled if provided
        if let Some(enabled) = request.enabled {
            conn.execute(
                "UPDATE users SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![if enabled { 1 } else { 0 }, now, id],
            )?;
        }

        // Update email if provided
        if let Some(email) = &request.email {
            conn.execute(
                "UPDATE users SET email = ?1, updated_at = ?2 WHERE id = ?3",
                params![email, now, id],
            )?;
        }

        // Update display_name if provided
        if let Some(display_name) = &request.display_name {
            conn.execute(
                "UPDATE users SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
                params![display_name, now, id],
            )?;
        }

        drop(conn);
        self.get(id)
    }

    /// Delete a user.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let rows = conn.execute("DELETE FROM users WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    /// List all users.
    pub fn list(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let limit = limit.unwrap_or(100).min(1000);
        let offset = offset.unwrap_or(0);

        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, roles, enabled, email, display_name, created_at, updated_at, last_login_at
             FROM users ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;

        let users = stmt
            .query_map(params![limit, offset], |row| {
                let roles_json: String = row.get(3)?;
                let created_at: String = row.get(7)?;
                let updated_at: String = row.get(8)?;
                let last_login_at: Option<String> = row.get(9)?;

                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    roles: serde_json::from_str(&roles_json).unwrap_or_default(),
                    enabled: row.get::<_, i64>(4)? != 0,
                    email: row.get(5)?,
                    display_name: row.get(6)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    last_login_at: last_login_at.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(users)
    }

    /// Count total users.
    pub fn count(&self) -> Result<u64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Verify a user's password and update last login time.
    pub fn verify_password(&self, username: &str, password: &str) -> Result<Option<User>> {
        let user = self.get_by_username(username)?;

        let Some(user) = user else {
            return Ok(None);
        };

        if !user.enabled {
            return Ok(None);
        }

        if !verify_password(password, &user.password_hash)? {
            return Ok(None);
        }

        // Update last login time
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE users SET last_login_at = ?1 WHERE id = ?2",
            params![now, user.id],
        )?;

        drop(conn);
        self.get(&user.id)
    }

    /// Check if a user has a specific permission.
    pub fn has_permission(&self, user_id: &str, permission: Permission) -> Result<bool> {
        let user = self.get(user_id)?;
        match user {
            Some(u) if u.enabled => Ok(check_permission(&u.roles, permission)),
            _ => Ok(false),
        }
    }
}

/// Hash a password using Argon2.
fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    Ok(hash.to_string())
}

/// Verify a password against a hash.
fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_store() -> (UserStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("users.db");
        let store = UserStore::new(&db_path).unwrap();
        (store, dir)
    }

    #[test]
    fn test_create_user() {
        let (store, _dir) = create_test_store();

        let user = store
            .create(&CreateUserRequest {
                username: "testuser".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: Some("test@example.com".to_string()),
                display_name: Some("Test User".to_string()),
            })
            .unwrap();

        assert_eq!(user.username, "testuser");
        assert_eq!(user.roles, vec!["user"]);
        assert!(user.enabled);
        assert_eq!(user.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_default_admin_created() {
        let (store, _dir) = create_test_store();

        let admin = store.get_by_username("admin").unwrap().unwrap();
        assert_eq!(admin.username, "admin");
        assert!(admin.roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_get_user() {
        let (store, _dir) = create_test_store();

        let created = store
            .create(&CreateUserRequest {
                username: "findme".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        let found = store.get(&created.id).unwrap().unwrap();
        assert_eq!(found.username, "findme");

        let by_name = store.get_by_username("findme").unwrap().unwrap();
        assert_eq!(by_name.id, created.id);
    }

    #[test]
    fn test_update_user() {
        let (store, _dir) = create_test_store();

        let created = store
            .create(&CreateUserRequest {
                username: "updateme".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        let updated = store
            .update(
                &created.id,
                &UpdateUserRequest {
                    password: None,
                    roles: Some(vec!["admin".to_string()]),
                    enabled: Some(false),
                    email: Some("new@example.com".to_string()),
                    display_name: Some("New Name".to_string()),
                },
            )
            .unwrap()
            .unwrap();

        assert_eq!(updated.roles, vec!["admin"]);
        assert!(!updated.enabled);
        assert_eq!(updated.email, Some("new@example.com".to_string()));
        assert_eq!(updated.display_name, Some("New Name".to_string()));
    }

    #[test]
    fn test_delete_user() {
        let (store, _dir) = create_test_store();

        let created = store
            .create(&CreateUserRequest {
                username: "deleteme".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        assert!(store.delete(&created.id).unwrap());
        assert!(store.get(&created.id).unwrap().is_none());
    }

    #[test]
    fn test_list_users() {
        let (store, _dir) = create_test_store();

        store
            .create(&CreateUserRequest {
                username: "user1".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        store
            .create(&CreateUserRequest {
                username: "user2".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        let users = store.list(None, None).unwrap();
        assert!(users.len() >= 3); // admin + user1 + user2
    }

    #[test]
    fn test_verify_password() {
        let (store, _dir) = create_test_store();

        store
            .create(&CreateUserRequest {
                username: "authtest".to_string(),
                password: "correctpassword".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        // Correct password
        let user = store
            .verify_password("authtest", "correctpassword")
            .unwrap();
        assert!(user.is_some());
        assert!(user.unwrap().last_login_at.is_some());

        // Wrong password
        let user = store.verify_password("authtest", "wrongpassword").unwrap();
        assert!(user.is_none());

        // Wrong username
        let user = store.verify_password("nonexistent", "password").unwrap();
        assert!(user.is_none());
    }

    #[test]
    fn test_disabled_user_cannot_login() {
        let (store, _dir) = create_test_store();

        let created = store
            .create(&CreateUserRequest {
                username: "disabled".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        store
            .update(
                &created.id,
                &UpdateUserRequest {
                    enabled: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();

        let user = store.verify_password("disabled", "password123").unwrap();
        assert!(user.is_none());
    }

    #[test]
    fn test_has_permission() {
        let (store, _dir) = create_test_store();

        let admin = store.get_by_username("admin").unwrap().unwrap();
        assert!(store
            .has_permission(&admin.id, Permission::AdminAccess)
            .unwrap());

        let user = store
            .create(&CreateUserRequest {
                username: "normaluser".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        assert!(!store
            .has_permission(&user.id, Permission::AdminAccess)
            .unwrap());
        assert!(store
            .has_permission(&user.id, Permission::ProxyAccess)
            .unwrap());
    }

    #[test]
    fn test_password_validation() {
        let (store, _dir) = create_test_store();

        // Short password should fail
        let result = store.create(&CreateUserRequest {
            username: "shortpw".to_string(),
            password: "short".to_string(),
            roles: vec!["user".to_string()],
            email: None,
            display_name: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_username_validation() {
        let (store, _dir) = create_test_store();

        // Empty username should fail
        let result = store.create(&CreateUserRequest {
            username: "".to_string(),
            password: "password123".to_string(),
            roles: vec!["user".to_string()],
            email: None,
            display_name: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_username_fails() {
        let (store, _dir) = create_test_store();

        store
            .create(&CreateUserRequest {
                username: "duplicate".to_string(),
                password: "password123".to_string(),
                roles: vec!["user".to_string()],
                email: None,
                display_name: None,
            })
            .unwrap();

        let result = store.create(&CreateUserRequest {
            username: "duplicate".to_string(),
            password: "password456".to_string(),
            roles: vec!["user".to_string()],
            email: None,
            display_name: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_password_hashing() {
        let password = "testpassword123";
        let hash = hash_password(password).unwrap();

        // Hash should not contain the original password
        assert!(!hash.contains(password));

        // Should verify correctly
        assert!(verify_password(password, &hash).unwrap());

        // Wrong password should not verify
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }
}
