//! Credential CLI Command Handler
//!
//! Manages credentials in the vault via CLI commands.

use crate::config::Config;
use crate::security::{
    CredentialEntry, CredentialType, CredentialVault,
    LoginCredential, OAuthCredential,
};
use crate::CredentialCommands;
use anyhow::{bail, Result};
use std::io::{self, Write};

/// Handle credential subcommands
pub fn handle_command(cmd: CredentialCommands, config: &Config) -> Result<()> {
    let vault_path = config.config_path.parent()
        .map_or_else(|| config.workspace_dir.clone(), std::path::Path::to_path_buf);

    match cmd {
        CredentialCommands::List => list_credentials(&vault_path),
        CredentialCommands::Add {
            credential_type,
            service,
            name,
            key,
            username,
            password,
            client_id,
            client_secret,
            patterns,
        } => add_credential(
            &vault_path,
            &credential_type,
            &service,
            name.as_deref(),
            key.as_deref(),
            username.as_deref(),
            password.as_deref(),
            client_id.as_deref(),
            client_secret.as_deref(),
            patterns.as_deref(),
        ),
        CredentialCommands::Remove { id } => remove_credential(&vault_path, &id),
        CredentialCommands::Show { id } => show_credential(&vault_path, &id),
    }
}

fn list_credentials(vault_path: &std::path::Path) -> Result<()> {
    let vault = CredentialVault::load(vault_path)?;
    let credentials = vault.list();

    if credentials.is_empty() {
        println!("No credentials stored.");
        return Ok(());
    }

    println!("Stored credentials:");
    println!();
    println!("{:<20} {:<15} {:<15} PATTERNS", "ID", "TYPE", "SERVICE");
    println!("{}", "-".repeat(80));

    for cred in credentials {
        let type_str = match cred.credential_type {
            CredentialType::ApiKey => "api_key",
            CredentialType::OAuth => "oauth",
            CredentialType::Login => "login",
            CredentialType::BearerToken => "bearer_token",
        };
        let patterns_str = if cred.patterns.is_empty() {
            "(none)".to_string()
        } else {
            cred.patterns.join(", ")
        };
        println!(
            "{:<20} {:<15} {:<15} {}",
            truncate(&cred.id, 18),
            type_str,
            cred.service,
            truncate(&patterns_str, 30),
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_credential(
    vault_path: &std::path::Path,
    credential_type: &str,
    service: &str,
    name: Option<&str>,
    key: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    patterns: Option<&str>,
) -> Result<()> {
    let mut vault = CredentialVault::load(vault_path)?;

    let patterns_vec: Vec<String> = patterns
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let cred_type: CredentialType = match credential_type.to_lowercase().as_str() {
        "api_key" | "apikey" => CredentialType::ApiKey,
        "oauth" => CredentialType::OAuth,
        "login" => CredentialType::Login,
        "bearer_token" | "bearer" => CredentialType::BearerToken,
        _ => bail!("Invalid credential type. Use: api_key, oauth, login, bearer_token"),
    };

    let display_name = name.unwrap_or(service);

    let entry = match cred_type {
        CredentialType::ApiKey => {
            let api_key = key.ok_or_else(|| anyhow::anyhow!("--key is required for api_key type"))?;
            CredentialEntry::new_api_key(display_name, service, api_key, patterns_vec)
        }
        CredentialType::BearerToken => {
            let token = key.ok_or_else(|| anyhow::anyhow!("--key is required for bearer_token type"))?;
            CredentialEntry::new_bearer_token(display_name, service, token, patterns_vec)
        }
        CredentialType::Login => {
            let user = username.ok_or_else(|| anyhow::anyhow!("--username is required for login type"))?;
            let pass = if let Some(p) = password {
                p.to_string()
            } else {
                // Prompt for password
                print!("Password: ");
                io::stdout().flush()?;
                rpassword::read_password()?
            };
            let login = LoginCredential {
                username: user.to_string(),
                password: pass,
                totp_secret: None,
            };
            CredentialEntry::new_login(display_name, service, login, patterns_vec)
        }
        CredentialType::OAuth => {
            let cid = client_id.ok_or_else(|| anyhow::anyhow!("--client-id is required for oauth type"))?;
            let oauth = OAuthCredential {
                client_id: cid.to_string(),
                client_secret: client_secret.map(String::from),
                access_token: None,
                refresh_token: None,
                expires_at: None,
                scope: None,
            };
            CredentialEntry::new_oauth(display_name, service, oauth, patterns_vec)
        }
    };

    let id = vault.add(entry)?;
    println!("✅ Credential added: {id}");

    Ok(())
}

fn remove_credential(vault_path: &std::path::Path, id: &str) -> Result<()> {
    let mut vault = CredentialVault::load(vault_path)?;

    // Try by ID first, then by service name
    let removed = if vault.get(id).is_some() {
        vault.remove(id)?
    } else if let Some(cred) = vault.get_by_service(id) {
        let cred_id = cred.id.clone();
        vault.remove(&cred_id)?
    } else {
        false
    };

    if removed {
        println!("✅ Credential removed: {id}");
    } else {
        println!("❌ Credential not found: {id}");
    }

    Ok(())
}

fn show_credential(vault_path: &std::path::Path, id: &str) -> Result<()> {
    let vault = CredentialVault::load(vault_path)?;

    // Try by ID first, then by service name
    let cred = vault.get(id).or_else(|| vault.get_by_service(id));

    if let Some(cred) = cred {
        println!("Credential: {}", cred.name);
        println!();
        println!("  ID:       {}", cred.id);
        println!("  Type:     {:?}", cred.credential_type);
        println!("  Service:  {}", cred.service);
        println!("  Patterns: {:?}", cred.patterns);
        println!();

        match cred.credential_type {
            CredentialType::ApiKey | CredentialType::BearerToken => {
                if let Some(ref key) = cred.api_key {
                    println!("  Key:      {}...{}", &key[..4.min(key.len())], if key.len() > 8 { &key[key.len()-4..] } else { "" });
                }
            }
            CredentialType::OAuth => {
                if let Some(ref oauth) = cred.oauth {
                    println!("  Client ID:     {}", oauth.client_id);
                    println!("  Has Secret:    {}", oauth.client_secret.is_some());
                    println!("  Has Token:     {}", oauth.access_token.is_some());
                    println!("  Has Refresh:   {}", oauth.refresh_token.is_some());
                    if let Some(exp) = oauth.expires_at {
                        let dt = chrono::DateTime::from_timestamp(exp, 0);
                        if let Some(dt) = dt {
                            println!("  Expires:       {}", dt.to_rfc3339());
                        }
                    }
                }
            }
            CredentialType::Login => {
                if let Some(ref login) = cred.login {
                    println!("  Username:      {}", login.username);
                    println!("  Has Password:  yes");
                    println!("  Has TOTP:      {}", login.totp_secret.is_some());
                }
            }
        }

        println!();
        println!("  Created:  {}", chrono::DateTime::from_timestamp(cred.created_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default());
        println!("  Updated:  {}", chrono::DateTime::from_timestamp(cred.updated_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default());
    } else {
        println!("❌ Credential not found: {id}");
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_empty_vault() {
        let tmp = TempDir::new().unwrap();
        let result = list_credentials(tmp.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }
}
