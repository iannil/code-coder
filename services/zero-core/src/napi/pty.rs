//! PTY NAPI bindings
//!
//! Provides JavaScript bindings for PTY session management.

use std::collections::HashMap;
use std::sync::Mutex;

use napi::bindgen_prelude::*;
use napi_derive::napi;

#[cfg(feature = "pty")]
use crate::tools::shell_pty::{PtyConfig, PtyInfo, PtyManager, PtySession, PtyState};

// ============================================================================
// NAPI Types
// ============================================================================

#[napi(object)]
pub struct NapiPtyConfig {
    /// Initial terminal width in columns
    pub cols: Option<u32>,
    /// Initial terminal height in rows
    pub rows: Option<u32>,
    /// Shell to use
    pub shell: Option<String>,
    /// Working directory
    pub cwd: Option<String>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Inherit environment from parent
    pub inherit_env: Option<bool>,
}

#[napi(string_enum)]
pub enum NapiPtyState {
    Running,
    Exited,
    Killed,
    Error,
}

#[napi(object)]
pub struct NapiPtyInfo {
    /// Session ID
    pub id: String,
    /// Current state
    pub state: NapiPtyState,
    /// Exit code (if exited)
    pub exit_code: Option<i32>,
    /// Terminal width
    pub cols: u32,
    /// Terminal height
    pub rows: u32,
    /// Shell being used
    pub shell: String,
    /// Working directory
    pub cwd: String,
}

// ============================================================================
// Conversion helpers
// ============================================================================

#[cfg(feature = "pty")]
fn to_rust_config(config: NapiPtyConfig) -> PtyConfig {
    PtyConfig {
        cols: config.cols.unwrap_or(80) as u16,
        rows: config.rows.unwrap_or(24) as u16,
        shell: config.shell,
        cwd: config.cwd,
        env: config.env.unwrap_or_default(),
        inherit_env: config.inherit_env.unwrap_or(true),
        read_timeout_ms: 0,
    }
}

#[cfg(feature = "pty")]
fn from_rust_state(state: PtyState) -> NapiPtyState {
    match state {
        PtyState::Running => NapiPtyState::Running,
        PtyState::Exited => NapiPtyState::Exited,
        PtyState::Killed => NapiPtyState::Killed,
        PtyState::Error => NapiPtyState::Error,
    }
}

#[cfg(feature = "pty")]
fn from_rust_info(info: PtyInfo) -> NapiPtyInfo {
    NapiPtyInfo {
        id: info.id,
        state: from_rust_state(info.state),
        exit_code: info.exit_code,
        cols: info.cols as u32,
        rows: info.rows as u32,
        shell: info.shell,
        cwd: info.cwd,
    }
}

// ============================================================================
// PTY Session Handle
// ============================================================================

#[cfg(feature = "pty")]
#[napi]
pub struct PtySessionHandle {
    inner: Mutex<PtySession>,
}

#[cfg(feature = "pty")]
#[napi]
impl PtySessionHandle {
    /// Get the session ID
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.inner.lock().unwrap().id().to_string()
    }

    /// Check if the session is running
    #[napi]
    pub fn is_running(&self) -> bool {
        self.inner.lock().unwrap().is_running()
    }

    /// Get session info
    #[napi]
    pub fn info(&self) -> NapiPtyInfo {
        from_rust_info(self.inner.lock().unwrap().info())
    }

    /// Read output from the PTY
    #[napi]
    pub fn read(&self) -> Result<Buffer> {
        self.inner
            .lock()
            .unwrap()
            .read()
            .map(|data| Buffer::from(data))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Read output with timeout (milliseconds)
    #[napi]
    pub fn read_with_timeout(&self, timeout_ms: u32) -> Result<Buffer> {
        let timeout = std::time::Duration::from_millis(timeout_ms as u64);
        self.inner
            .lock()
            .unwrap()
            .read_with_timeout(timeout)
            .map(|data| Buffer::from(data))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Write data to the PTY
    #[napi]
    pub fn write(&self, data: Buffer) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .write(&data)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Write a line (with newline) to the PTY
    #[napi]
    pub fn write_line(&self, line: String) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .write_line(&line)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Resize the terminal
    #[napi]
    pub fn resize(&self, cols: u32, rows: u32) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .resize(cols as u16, rows as u16)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Kill the process
    #[napi]
    pub fn kill(&self) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .kill()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for the process to exit
    #[napi]
    pub fn wait(&self) -> Result<i32> {
        self.inner
            .lock()
            .unwrap()
            .wait()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Wait for the process with timeout (milliseconds)
    #[napi]
    pub fn wait_with_timeout(&self, timeout_ms: u32) -> Result<Option<i32>> {
        let timeout = std::time::Duration::from_millis(timeout_ms as u64);
        self.inner
            .lock()
            .unwrap()
            .wait_with_timeout(timeout)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get exit code (if exited)
    #[napi]
    pub fn exit_code(&self) -> Option<i32> {
        self.inner.lock().unwrap().exit_code()
    }
}

// ============================================================================
// PTY Manager Handle
// ============================================================================

#[cfg(feature = "pty")]
#[napi]
pub struct PtyManagerHandle {
    inner: Mutex<PtyManager>,
}

#[cfg(feature = "pty")]
#[napi]
impl PtyManagerHandle {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PtyManager::new()),
        }
    }

    /// Create a new shell session
    #[napi]
    pub fn create(&self, config: NapiPtyConfig) -> Result<String> {
        self.inner
            .lock()
            .unwrap()
            .create(to_rust_config(config))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Create a new session with a specific command
    #[napi]
    pub fn create_command(
        &self,
        command: String,
        args: Vec<String>,
        config: NapiPtyConfig,
    ) -> Result<String> {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.inner
            .lock()
            .unwrap()
            .create_command(&command, &args_refs, to_rust_config(config))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List all session IDs
    #[napi]
    pub fn list(&self) -> Vec<String> {
        self.inner.lock().unwrap().list()
    }

    /// List all session info
    #[napi]
    pub fn list_info(&self) -> Vec<NapiPtyInfo> {
        self.inner
            .lock()
            .unwrap()
            .list_info()
            .into_iter()
            .map(from_rust_info)
            .collect()
    }

    /// Clean up exited sessions
    #[napi]
    pub fn cleanup(&self) {
        self.inner.lock().unwrap().cleanup()
    }

    /// Kill all sessions
    #[napi]
    pub fn kill_all(&self) {
        self.inner.lock().unwrap().kill_all()
    }
}

// ============================================================================
// Standalone functions
// ============================================================================

/// Spawn a new PTY session
#[cfg(feature = "pty")]
#[napi]
pub fn spawn_pty(config: NapiPtyConfig) -> Result<PtySessionHandle> {
    PtySession::spawn(to_rust_config(config))
        .map(|session| PtySessionHandle {
            inner: Mutex::new(session),
        })
        .map_err(|e| Error::from_reason(e.to_string()))
}

/// Spawn a new PTY session with a specific command
#[cfg(feature = "pty")]
#[napi]
pub fn spawn_pty_command(
    command: String,
    args: Vec<String>,
    config: NapiPtyConfig,
) -> Result<PtySessionHandle> {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    PtySession::spawn_command(&command, &args_refs, to_rust_config(config))
        .map(|session| PtySessionHandle {
            inner: Mutex::new(session),
        })
        .map_err(|e| Error::from_reason(e.to_string()))
}

// ============================================================================
// Stub implementations when PTY feature is disabled
// ============================================================================

#[cfg(not(feature = "pty"))]
#[napi]
pub fn spawn_pty(_config: NapiPtyConfig) -> Result<()> {
    Err(Error::from_reason("PTY support not enabled"))
}

#[cfg(not(feature = "pty"))]
#[napi]
pub fn spawn_pty_command(
    _command: String,
    _args: Vec<String>,
    _config: NapiPtyConfig,
) -> Result<()> {
    Err(Error::from_reason("PTY support not enabled"))
}
