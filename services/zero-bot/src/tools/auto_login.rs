//! Auto-Login Tool - Automated login using credential vault
//!
//! This tool provides automated login capabilities using credentials stored in the vault.
//! It supports:
//! - Username/password form filling
//! - TOTP 2FA (automatic via stored secret)
//! - Interactive 2FA (via Telegram/Discord prompt)
//! - Session persistence (cookies)

use super::traits::{Tool, ToolResult};
use crate::agent::confirmation;
use crate::security::{CredentialEntry, CredentialType, CredentialVault, SecurityPolicy};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use totp_rs::{Algorithm, TOTP};

/// Auto-login tool for automated authentication
pub struct AutoLoginTool {
    #[allow(dead_code)]
    security: Arc<SecurityPolicy>,
    vault_path: std::path::PathBuf,
}

/// Response from agent-browser --json commands
#[derive(Debug, Deserialize)]
struct AgentBrowserResponse {
    success: bool,
    data: Option<Value>,
    #[allow(dead_code)]
    error: Option<String>,
}

/// Login page element patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPattern {
    /// CSS selector or semantic locator for username field
    pub username_selector: String,
    /// CSS selector or semantic locator for password field
    pub password_selector: String,
    /// CSS selector or semantic locator for submit button
    pub submit_selector: String,
    /// CSS selector or semantic locator for 2FA code input (optional)
    pub totp_selector: Option<String>,
    /// CSS selector for "remember me" checkbox (optional)
    pub remember_selector: Option<String>,
}

impl Default for LoginPattern {
    fn default() -> Self {
        Self {
            // Common patterns for login forms
            username_selector: "[type='email'], [type='text'][name*='user'], [name='username'], [name='email'], #username, #email".into(),
            password_selector: "[type='password'], [name='password'], #password".into(),
            submit_selector: "[type='submit'], button[type='submit'], button:contains('Login'), button:contains('Sign in')".into(),
            totp_selector: Some("[name='totp'], [name='code'], [name='otp'], [autocomplete='one-time-code']".into()),
            remember_selector: Some("[type='checkbox'][name*='remember']".into()),
        }
    }
}

impl AutoLoginTool {
    pub fn new(security: Arc<SecurityPolicy>, vault_path: std::path::PathBuf) -> Self {
        Self { security, vault_path }
    }

    /// Generate TOTP code from secret
    fn generate_totp(secret: &str) -> anyhow::Result<String> {
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.as_bytes().to_vec(),
        ).map_err(|e| anyhow::anyhow!("Invalid TOTP secret: {e}"))?;

        let code = totp.generate_current()
            .map_err(|e| anyhow::anyhow!("Failed to generate TOTP: {e}"))?;

        Ok(code)
    }

    /// Request 2FA code from user via messaging channel
    async fn request_2fa_code(
        &self,
        platform: &str,
        user_id: &str,
        service: &str,
        timeout_secs: u64,
    ) -> anyhow::Result<String> {
        let msg = format!(
            "🔐 *{service} 需要验证码*\n\n请输入您的2FA验证码：\n────────────────\n⏱️ {timeout_secs}秒内有效\n\n回复数字验证码即可"
        );

        // Use confirmation system to request user input
        let response = confirmation::request_text_input(
            platform,
            user_id,
            &msg,
            timeout_secs,
        ).await?;

        // Extract numeric code from response
        let code: String = response.chars().filter(char::is_ascii_digit).collect();

        if code.len() < 4 || code.len() > 8 {
            return Err(anyhow::anyhow!("Invalid verification code format"));
        }

        Ok(code)
    }

    /// Execute agent-browser command
    async fn run_browser_command(&self, action: &str, args: &[(&str, &str)]) -> anyhow::Result<AgentBrowserResponse> {
        let mut cmd = Command::new("agent-browser");
        cmd.arg(action);
        cmd.arg("--json");

        for (key, value) in args {
            cmd.arg(format!("--{key}"));
            cmd.arg(value);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| anyhow::anyhow!("Failed to execute agent-browser: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("agent-browser failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: AgentBrowserResponse = serde_json::from_str(&stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse browser response: {e}"))?;

        Ok(response)
    }

    /// Perform automated login
    async fn perform_login(
        &self,
        credential: &CredentialEntry,
        login_url: &str,
        pattern: &LoginPattern,
        platform: &str,
        user_id: &str,
    ) -> anyhow::Result<String> {
        let login = credential.login.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Credential has no login data"))?;

        // Navigate to login page
        self.run_browser_command("open", &[("url", login_url)]).await?;

        // Wait for page load
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Fill username
        self.run_browser_command("fill", &[
            ("selector", &pattern.username_selector),
            ("value", &login.username),
        ]).await?;

        // Fill password
        self.run_browser_command("fill", &[
            ("selector", &pattern.password_selector),
            ("value", &login.password),
        ]).await?;

        // Click remember me if available
        if let Some(ref remember) = pattern.remember_selector {
            let _ = self.run_browser_command("click", &[
                ("selector", remember),
            ]).await;
        }

        // Submit form
        self.run_browser_command("click", &[
            ("selector", &pattern.submit_selector),
        ]).await?;

        // Wait for navigation
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Check if 2FA is required
        if let Some(ref totp_selector) = pattern.totp_selector {
            let visible_check = self.run_browser_command("is-visible", &[
                ("selector", totp_selector),
            ]).await;

            if visible_check.is_ok() && visible_check.unwrap().success {
                // Handle 2FA
                let code = if let Some(ref totp_secret) = login.totp_secret {
                    // Auto-generate TOTP
                    Self::generate_totp(totp_secret)?
                } else {
                    // Request from user
                    self.request_2fa_code(platform, user_id, &credential.service, 120).await?
                };

                self.run_browser_command("fill", &[
                    ("selector", totp_selector),
                    ("value", &code),
                ]).await?;

                // Submit 2FA form
                self.run_browser_command("press", &[("key", "Enter")]).await?;

                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }

        // Get final URL to confirm login success
        let url_response = self.run_browser_command("get-url", &[]).await?;
        let final_url = url_response.data
            .and_then(|d| d.as_str().map(String::from))
            .unwrap_or_else(|| login_url.to_string());

        Ok(format!(
            "Login completed for {}.\nFinal URL: {}",
            credential.service,
            final_url
        ))
    }
}

#[async_trait]
impl Tool for AutoLoginTool {
    fn name(&self) -> &str {
        "auto_login"
    }

    fn description(&self) -> &str {
        "Automatically log into websites using stored credentials. \
        Supports username/password forms and 2FA (TOTP or interactive). \
        Credentials are retrieved from the secure vault."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "service": {
                    "type": "string",
                    "description": "Service name to look up in credential vault (e.g., 'github', 'google')"
                },
                "url": {
                    "type": "string",
                    "description": "Login page URL. If not provided, will attempt to auto-detect"
                },
                "pattern": {
                    "type": "object",
                    "description": "Custom login form pattern (optional)",
                    "properties": {
                        "username_selector": { "type": "string" },
                        "password_selector": { "type": "string" },
                        "submit_selector": { "type": "string" },
                        "totp_selector": { "type": "string" },
                        "remember_selector": { "type": "string" }
                    }
                }
            },
            "required": ["service"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let service = args.get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'service' parameter"))?;

        let url = args.get("url")
            .and_then(|v| v.as_str());

        // Load vault and find credential
        let vault = CredentialVault::load(&self.vault_path)
            .map_err(|e| anyhow::anyhow!("Failed to load credential vault: {e}"))?;

        let credential = vault.get_by_service(service)
            .ok_or_else(|| anyhow::anyhow!("No credential found for service: {service}"))?;

        // Verify it's a login credential
        if credential.credential_type != CredentialType::Login {
            return Err(anyhow::anyhow!(
                "Credential for '{service}' is not a login type"
            ));
        }

        // Get login URL
        let login_url = url
            .map(String::from)
            .or_else(|| credential.patterns.first().cloned())
            .ok_or_else(|| anyhow::anyhow!("No login URL provided or configured"))?;

        // Parse custom pattern or use default
        let pattern: LoginPattern = if let Some(pattern_obj) = args.get("pattern") {
            serde_json::from_value(pattern_obj.clone())
                .unwrap_or_default()
        } else {
            LoginPattern::default()
        };

        // Extract context info
        let (platform, user_id) = if let Some(ctx) = args.get("_context") {
            let channel = ctx.get("channel").and_then(|v| v.as_str()).unwrap_or("zerobot");
            let sender = ctx.get("sender_id").and_then(|v| v.as_str()).unwrap_or("unknown");
            (channel.to_string(), sender.to_string())
        } else {
            ("zerobot".to_string(), "unknown".to_string())
        };

        // Perform login
        match self.perform_login(credential, &login_url, &pattern, &platform, &user_id).await {
            Ok(result) => Ok(ToolResult {
                success: true,
                output: result,
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_login_tool_name() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = AutoLoginTool::new(security, std::path::PathBuf::from("."));
        assert_eq!(tool.name(), "auto_login");
    }

    #[test]
    fn auto_login_tool_description() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = AutoLoginTool::new(security, std::path::PathBuf::from("."));
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("credential"));
    }

    #[test]
    fn auto_login_tool_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = AutoLoginTool::new(security, std::path::PathBuf::from("."));
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["service"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("service")));
    }

    #[test]
    fn login_pattern_default() {
        let pattern = LoginPattern::default();
        assert!(!pattern.username_selector.is_empty());
        assert!(!pattern.password_selector.is_empty());
        assert!(!pattern.submit_selector.is_empty());
    }
}
