//! iMessage channel for zero-channels.
//!
//! Uses macOS AppleScript bridge for sending and SQLite database polling for receiving.
//! Only works on macOS with Full Disk Access permissions.

use crate::message::{ChannelMessage, ChannelType, MessageContent, OutgoingContent, OutgoingMessage};
use crate::traits::{Channel, ChannelError, ChannelResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// iMessage channel using macOS AppleScript bridge.
pub struct IMessageChannel {
    allowed_contacts: Vec<String>,
    poll_interval_secs: u64,
    connected: Arc<RwLock<bool>>,
}

impl IMessageChannel {
    /// Create a new iMessage channel.
    pub fn new(allowed_contacts: Vec<String>) -> Self {
        Self {
            allowed_contacts,
            poll_interval_secs: 3,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    fn is_contact_allowed(&self, sender: &str) -> bool {
        if self.allowed_contacts.iter().any(|u| u == "*") {
            return true;
        }
        self.allowed_contacts
            .iter()
            .any(|u| u.eq_ignore_ascii_case(sender))
    }

    fn get_messages_db_path() -> Option<std::path::PathBuf> {
        directories::UserDirs::new().map(|u| u.home_dir().join("Library/Messages/chat.db"))
    }
}

/// Escape a string for safe interpolation into AppleScript.
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Validate that a target looks like a valid phone number or email address.
fn is_valid_imessage_target(target: &str) -> bool {
    let target = target.trim();
    if target.is_empty() {
        return false;
    }

    // Phone number: +1234567890
    if target.starts_with('+') {
        let digits_only: String = target.chars().filter(char::is_ascii_digit).collect();
        return digits_only.len() >= 7 && digits_only.len() <= 15;
    }

    // Email: simple validation
    if let Some(at_pos) = target.find('@') {
        let local = &target[..at_pos];
        let domain = &target[at_pos + 1..];

        let local_valid = !local.is_empty()
            && local
                .chars()
                .all(|c| c.is_alphanumeric() || "._+-".contains(c));

        let domain_valid = !domain.is_empty()
            && domain.contains('.')
            && domain
                .chars()
                .all(|c| c.is_alphanumeric() || ".-".contains(c));

        return local_valid && domain_valid;
    }

    false
}

/// Get the current max ROWID from the messages table.
async fn get_max_rowid(db_path: &Path) -> anyhow::Result<i64> {
    let path = db_path.to_path_buf();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<i64> {
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let mut stmt = conn.prepare("SELECT MAX(ROWID) FROM message WHERE is_from_me = 0")?;
        let rowid: Option<i64> = stmt.query_row([], |row| row.get(0))?;
        Ok(rowid.unwrap_or(0))
    })
    .await??;
    Ok(result)
}

/// Fetch messages newer than `since_rowid`.
async fn fetch_new_messages(
    db_path: &Path,
    since_rowid: i64,
) -> anyhow::Result<Vec<(i64, String, String)>> {
    let path = db_path.to_path_buf();
    let results =
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(i64, String, String)>> {
            let conn = rusqlite::Connection::open_with_flags(
                &path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            let mut stmt = conn.prepare(
                "SELECT m.ROWID, h.id, m.text \
                 FROM message m \
                 JOIN handle h ON m.handle_id = h.ROWID \
                 WHERE m.ROWID > ?1 \
                 AND m.is_from_me = 0 \
                 AND m.text IS NOT NULL \
                 ORDER BY m.ROWID ASC \
                 LIMIT 20",
            )?;
            let rows = stmt.query_map([since_rowid], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        })
        .await??;
    Ok(results)
}

#[async_trait]
impl Channel for IMessageChannel {
    fn name(&self) -> &'static str {
        "imessage"
    }

    async fn init(&mut self) -> ChannelResult<()> {
        if !cfg!(target_os = "macos") {
            return Err(ChannelError::NotReady);
        }

        let db_path = Self::get_messages_db_path()
            .ok_or_else(|| ChannelError::Internal("Cannot find home directory".into()))?;

        if !db_path.exists() {
            return Err(ChannelError::Internal(format!(
                "Messages database not found at {}. Ensure Messages.app is set up and Full Disk Access is granted.",
                db_path.display()
            )));
        }

        *self.connected.write().await = true;
        tracing::info!("iMessage channel initialized");
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
        if !cfg!(target_os = "macos") {
            return Err(ChannelError::NotReady);
        }

        let text = match &message.content {
            OutgoingContent::Text { text } => text.clone(),
            OutgoingContent::Markdown { text } => text.clone(),
            _ => {
                return Err(ChannelError::InvalidMessage(
                    "iMessage only supports text messages".into(),
                ))
            }
        };

        let target = &message.channel_id;

        if !is_valid_imessage_target(target) {
            return Err(ChannelError::InvalidMessage(
                "Invalid iMessage target: must be a phone number (+1234567890) or email".into(),
            ));
        }

        let escaped_msg = escape_applescript(&text);
        let escaped_target = escape_applescript(target);

        let script = format!(
            r#"tell application "Messages"
    set targetService to 1st account whose service type = iMessage
    set targetBuddy to participant "{escaped_target}" of targetService
    send "{escaped_msg}" to targetBuddy
end tell"#
        );

        let output = tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Failed to run osascript: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ChannelError::SendFailed(format!(
                "iMessage send failed: {stderr}"
            )));
        }

        Ok(format!("imsg_{}", chrono::Utc::now().timestamp_millis()))
    }

    async fn listen<F>(&self, callback: F) -> ChannelResult<()>
    where
        F: Fn(ChannelMessage) + Send + Sync + 'static,
    {
        if !cfg!(target_os = "macos") {
            return Err(ChannelError::NotReady);
        }

        tracing::info!("iMessage channel listening (AppleScript bridge)...");

        let db_path = Self::get_messages_db_path()
            .ok_or_else(|| ChannelError::Internal("Cannot find home directory".into()))?;

        if !db_path.exists() {
            return Err(ChannelError::Internal(format!(
                "Messages database not found at {}",
                db_path.display()
            )));
        }

        let mut last_rowid = get_max_rowid(&db_path)
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to get max rowid: {e}")))?;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(self.poll_interval_secs)).await;

            match fetch_new_messages(&db_path, last_rowid).await {
                Ok(messages) => {
                    for (rowid, sender, text) in messages {
                        if rowid > last_rowid {
                            last_rowid = rowid;
                        }

                        if !self.is_contact_allowed(&sender) {
                            continue;
                        }

                        if text.trim().is_empty() {
                            continue;
                        }

                        let msg = ChannelMessage {
                            id: rowid.to_string(),
                            channel_type: ChannelType::IMessage,
                            channel_id: sender.clone(),
                            user_id: sender,
                            content: MessageContent::Text { text },
                            attachments: vec![],
                            metadata: HashMap::new(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            trace_id: zero_common::logging::generate_trace_id(),
                            span_id: zero_common::logging::generate_span_id(),
                            parent_span_id: None,
                        };

                        callback(msg);
                    }
                }
                Err(e) => {
                    tracing::warn!("iMessage poll error: {e}");
                }
            }
        }
    }

    async fn health_check(&self) -> ChannelResult<()> {
        if !cfg!(target_os = "macos") {
            return Err(ChannelError::NotReady);
        }

        let db_path = Self::get_messages_db_path()
            .ok_or_else(|| ChannelError::Internal("Cannot find home directory".into()))?;

        if db_path.exists() {
            Ok(())
        } else {
            Err(ChannelError::Internal("Messages database not found".into()))
        }
    }

    async fn shutdown(&self) -> ChannelResult<()> {
        *self.connected.write().await = false;
        tracing::info!("iMessage channel shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_contacts() {
        let ch = IMessageChannel::new(vec!["+1234567890".into()]);
        assert_eq!(ch.allowed_contacts.len(), 1);
    }

    #[test]
    fn wildcard_allows_anyone() {
        let ch = IMessageChannel::new(vec!["*".into()]);
        assert!(ch.is_contact_allowed("+1234567890"));
        assert!(ch.is_contact_allowed("random@icloud.com"));
    }

    #[test]
    fn specific_contact_allowed() {
        let ch = IMessageChannel::new(vec!["+1234567890".into(), "user@icloud.com".into()]);
        assert!(ch.is_contact_allowed("+1234567890"));
        assert!(ch.is_contact_allowed("user@icloud.com"));
    }

    #[test]
    fn unknown_contact_denied() {
        let ch = IMessageChannel::new(vec!["+1234567890".into()]);
        assert!(!ch.is_contact_allowed("+9999999999"));
    }

    #[test]
    fn contact_case_insensitive() {
        let ch = IMessageChannel::new(vec!["User@iCloud.com".into()]);
        assert!(ch.is_contact_allowed("user@icloud.com"));
    }

    #[test]
    fn name_returns_imessage() {
        let ch = IMessageChannel::new(vec![]);
        assert_eq!(ch.name(), "imessage");
    }

    #[test]
    fn escape_applescript_double_quotes() {
        assert_eq!(escape_applescript(r#"hello "world""#), r#"hello \"world\""#);
    }

    #[test]
    fn escape_applescript_backslashes() {
        assert_eq!(escape_applescript(r"path\to\file"), r"path\\to\\file");
    }

    #[test]
    fn valid_phone_number() {
        assert!(is_valid_imessage_target("+1234567890"));
        assert!(is_valid_imessage_target("+14155551234"));
    }

    #[test]
    fn valid_email() {
        assert!(is_valid_imessage_target("user@example.com"));
        assert!(is_valid_imessage_target("user@icloud.com"));
    }

    #[test]
    fn invalid_target_empty() {
        assert!(!is_valid_imessage_target(""));
        assert!(!is_valid_imessage_target("   "));
    }

    #[test]
    fn invalid_target_no_plus_prefix() {
        assert!(!is_valid_imessage_target("1234567890"));
    }

    #[test]
    fn invalid_target_injection_attempt() {
        assert!(!is_valid_imessage_target(r#"" & do shell script "id" & ""#));
    }
}
