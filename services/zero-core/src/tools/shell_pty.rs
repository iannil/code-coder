//! PTY (Pseudo-Terminal) shell management
//!
//! This module provides PTY-based interactive shell sessions with:
//! - Full PTY emulation for interactive commands
//! - Terminal resize support
//! - Input/output streaming
//! - Session lifecycle management

#[cfg(feature = "pty")]
mod pty_impl {
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use anyhow::{Context, Result};
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use serde::{Deserialize, Serialize};

    /// PTY session configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyConfig {
        /// Initial terminal width in columns
        #[serde(default = "default_cols")]
        pub cols: u16,

        /// Initial terminal height in rows
        #[serde(default = "default_rows")]
        pub rows: u16,

        /// Shell to use (default: detected from environment)
        pub shell: Option<String>,

        /// Working directory
        pub cwd: Option<String>,

        /// Environment variables
        #[serde(default)]
        pub env: HashMap<String, String>,

        /// Inherit environment from parent process
        #[serde(default = "default_true")]
        pub inherit_env: bool,

        /// Read timeout in milliseconds (0 = non-blocking)
        #[serde(default)]
        pub read_timeout_ms: u64,
    }

    fn default_cols() -> u16 {
        80
    }

    fn default_rows() -> u16 {
        24
    }

    fn default_true() -> bool {
        true
    }

    impl Default for PtyConfig {
        fn default() -> Self {
            Self {
                cols: 80,
                rows: 24,
                shell: None,
                cwd: None,
                env: HashMap::new(),
                inherit_env: true,
                read_timeout_ms: 0,
            }
        }
    }

    /// PTY session state
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PtyState {
        /// Session is running
        Running,
        /// Session has exited normally
        Exited,
        /// Session was killed
        Killed,
        /// Session encountered an error
        Error,
    }

    /// PTY session information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyInfo {
        /// Session ID
        pub id: String,

        /// Current state
        pub state: PtyState,

        /// Exit code (if exited)
        pub exit_code: Option<i32>,

        /// Terminal size
        pub cols: u16,
        pub rows: u16,

        /// Shell being used
        pub shell: String,

        /// Working directory
        pub cwd: String,
    }

    /// A PTY session for interactive shell commands
    pub struct PtySession {
        id: String,
        master: Box<dyn portable_pty::MasterPty + Send>,
        child: Box<dyn portable_pty::Child + Send + Sync>,
        reader: Arc<Mutex<Box<dyn Read + Send>>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        config: PtyConfig,
        running: Arc<AtomicBool>,
        exit_code: Arc<Mutex<Option<i32>>>,
    }

    impl PtySession {
        /// Spawn a new PTY session with the given configuration
        pub fn spawn(config: PtyConfig) -> Result<Self> {
            let id = uuid::Uuid::new_v4().to_string();
            let pty_system = native_pty_system();

            let size = PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            };

            let pair = pty_system
                .openpty(size)
                .context("Failed to open PTY pair")?;

            let shell = config.shell.clone().unwrap_or_else(|| {
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
            });

            let mut cmd = CommandBuilder::new(&shell);

            // Set working directory
            if let Some(cwd) = &config.cwd {
                cmd.cwd(cwd);
            }

            // Set environment
            if !config.inherit_env {
                cmd.env_clear();
            }
            for (key, value) in &config.env {
                cmd.env(key, value);
            }

            let child = pair
                .slave
                .spawn_command(cmd)
                .context("Failed to spawn shell process")?;

            let reader = pair
                .master
                .try_clone_reader()
                .context("Failed to clone PTY reader")?;
            let writer = pair
                .master
                .take_writer()
                .context("Failed to take PTY writer")?;

            Ok(Self {
                id,
                master: pair.master,
                child,
                reader: Arc::new(Mutex::new(reader)),
                writer: Arc::new(Mutex::new(writer)),
                config,
                running: Arc::new(AtomicBool::new(true)),
                exit_code: Arc::new(Mutex::new(None)),
            })
        }

        /// Spawn a new PTY session with a specific command
        pub fn spawn_command(command: &str, args: &[&str], config: PtyConfig) -> Result<Self> {
            let id = uuid::Uuid::new_v4().to_string();
            let pty_system = native_pty_system();

            let size = PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            };

            let pair = pty_system
                .openpty(size)
                .context("Failed to open PTY pair")?;

            let mut cmd = CommandBuilder::new(command);
            for arg in args {
                cmd.arg(*arg);
            }

            // Set working directory
            if let Some(cwd) = &config.cwd {
                cmd.cwd(cwd);
            }

            // Set environment
            if !config.inherit_env {
                cmd.env_clear();
            }
            for (key, value) in &config.env {
                cmd.env(key, value);
            }

            let child = pair
                .slave
                .spawn_command(cmd)
                .context("Failed to spawn command")?;

            let reader = pair
                .master
                .try_clone_reader()
                .context("Failed to clone PTY reader")?;
            let writer = pair
                .master
                .take_writer()
                .context("Failed to take PTY writer")?;

            Ok(Self {
                id,
                master: pair.master,
                child,
                reader: Arc::new(Mutex::new(reader)),
                writer: Arc::new(Mutex::new(writer)),
                config,
                running: Arc::new(AtomicBool::new(true)),
                exit_code: Arc::new(Mutex::new(None)),
            })
        }

        /// Get the session ID
        pub fn id(&self) -> &str {
            &self.id
        }

        /// Check if the session is still running
        pub fn is_running(&mut self) -> bool {
            if !self.running.load(Ordering::SeqCst) {
                return false;
            }

            // Try to poll the child process
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.running.store(false, Ordering::SeqCst);
                    let code = status.exit_code();
                    *self.exit_code.lock().unwrap() = Some(code as i32);
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.running.store(false, Ordering::SeqCst);
                    false
                }
            }
        }

        /// Get session information
        pub fn info(&mut self) -> PtyInfo {
            let state = if self.is_running() {
                PtyState::Running
            } else {
                let exit_code = *self.exit_code.lock().unwrap();
                match exit_code {
                    Some(_) => PtyState::Exited,
                    None => PtyState::Error,
                }
            };

            let shell = self.config.shell.clone().unwrap_or_else(|| {
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
            });

            let cwd = self.config.cwd.clone().unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default()
            });

            PtyInfo {
                id: self.id.clone(),
                state,
                exit_code: *self.exit_code.lock().unwrap(),
                cols: self.config.cols,
                rows: self.config.rows,
                shell,
                cwd,
            }
        }

        /// Read output from the PTY (non-blocking)
        pub fn read(&self) -> Result<Vec<u8>> {
            let mut reader = self.reader.lock().unwrap();
            let mut buffer = vec![0u8; 4096];

            // Non-blocking read
            match reader.read(&mut buffer) {
                Ok(0) => Ok(Vec::new()),
                Ok(n) => {
                    buffer.truncate(n);
                    Ok(buffer)
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(Vec::new()),
                Err(e) => Err(e.into()),
            }
        }

        /// Read output with timeout
        pub fn read_with_timeout(&mut self, timeout: Duration) -> Result<Vec<u8>> {
            let start = std::time::Instant::now();
            let mut all_data = Vec::new();

            while start.elapsed() < timeout {
                let data = self.read()?;
                if data.is_empty() {
                    std::thread::sleep(Duration::from_millis(10));
                } else {
                    all_data.extend(data);
                }

                // Check if process has exited
                if !self.is_running() {
                    // Read any remaining output
                    loop {
                        let data = self.read()?;
                        if data.is_empty() {
                            break;
                        }
                        all_data.extend(data);
                    }
                    break;
                }
            }

            Ok(all_data)
        }

        /// Write input to the PTY
        pub fn write(&self, data: &[u8]) -> Result<()> {
            let mut writer = self.writer.lock().unwrap();
            writer.write_all(data)?;
            writer.flush()?;
            Ok(())
        }

        /// Write a string and newline
        pub fn write_line(&self, line: &str) -> Result<()> {
            let mut data = line.as_bytes().to_vec();
            data.push(b'\n');
            self.write(&data)
        }

        /// Resize the terminal
        pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
            let size = PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            };
            self.master.resize(size)?;
            self.config.cols = cols;
            self.config.rows = rows;
            Ok(())
        }

        /// Send a signal to the child process
        pub fn kill(&mut self) -> Result<()> {
            self.child.kill()?;
            self.running.store(false, Ordering::SeqCst);
            Ok(())
        }

        /// Wait for the process to exit
        pub fn wait(&mut self) -> Result<i32> {
            let status = self.child.wait()?;
            self.running.store(false, Ordering::SeqCst);
            let code = status.exit_code() as i32;
            *self.exit_code.lock().unwrap() = Some(code);
            Ok(code)
        }

        /// Wait for the process with timeout
        pub fn wait_with_timeout(&mut self, timeout: Duration) -> Result<Option<i32>> {
            let start = std::time::Instant::now();

            while start.elapsed() < timeout {
                match self.child.try_wait() {
                    Ok(Some(status)) => {
                        self.running.store(false, Ordering::SeqCst);
                        let code = status.exit_code() as i32;
                        *self.exit_code.lock().unwrap() = Some(code);
                        return Ok(Some(code));
                    }
                    Ok(None) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            Ok(None)
        }

        /// Get the exit code (if exited)
        pub fn exit_code(&self) -> Option<i32> {
            *self.exit_code.lock().unwrap()
        }
    }

    /// PTY session manager for handling multiple sessions
    pub struct PtyManager {
        sessions: HashMap<String, PtySession>,
    }

    impl Default for PtyManager {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PtyManager {
        /// Create a new PTY manager
        pub fn new() -> Self {
            Self {
                sessions: HashMap::new(),
            }
        }

        /// Create a new PTY session
        pub fn create(&mut self, config: PtyConfig) -> Result<String> {
            let session = PtySession::spawn(config)?;
            let id = session.id().to_string();
            self.sessions.insert(id.clone(), session);
            Ok(id)
        }

        /// Create a new PTY session with a specific command
        pub fn create_command(
            &mut self,
            command: &str,
            args: &[&str],
            config: PtyConfig,
        ) -> Result<String> {
            let session = PtySession::spawn_command(command, args, config)?;
            let id = session.id().to_string();
            self.sessions.insert(id.clone(), session);
            Ok(id)
        }

        /// Get a session by ID
        pub fn get(&self, id: &str) -> Option<&PtySession> {
            self.sessions.get(id)
        }

        /// Get a mutable session by ID
        pub fn get_mut(&mut self, id: &str) -> Option<&mut PtySession> {
            self.sessions.get_mut(id)
        }

        /// Remove a session
        pub fn remove(&mut self, id: &str) -> Option<PtySession> {
            self.sessions.remove(id)
        }

        /// List all session IDs
        pub fn list(&self) -> Vec<String> {
            self.sessions.keys().cloned().collect()
        }

        /// List all session info
        pub fn list_info(&mut self) -> Vec<PtyInfo> {
            self.sessions.values_mut().map(|s| s.info()).collect()
        }

        /// Clean up exited sessions
        pub fn cleanup(&mut self) {
            // First pass: identify exited sessions
            let mut exited = Vec::new();
            for (id, session) in self.sessions.iter_mut() {
                if !session.is_running() {
                    exited.push(id.clone());
                }
            }

            // Second pass: remove exited sessions
            for id in exited {
                self.sessions.remove(&id);
            }
        }

        /// Kill all sessions
        pub fn kill_all(&mut self) {
            for session in self.sessions.values_mut() {
                let _ = session.kill();
            }
            self.sessions.clear();
        }
    }

    impl Drop for PtyManager {
        fn drop(&mut self) {
            self.kill_all();
        }
    }
}

#[cfg(feature = "pty")]
pub use pty_impl::*;

// Provide stub types when pty feature is disabled
#[cfg(not(feature = "pty"))]
mod stub {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};
    use anyhow::Result;

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct PtyConfig {
        pub cols: u16,
        pub rows: u16,
        pub shell: Option<String>,
        pub cwd: Option<String>,
        pub env: HashMap<String, String>,
        pub inherit_env: bool,
        pub read_timeout_ms: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PtyState {
        Running,
        Exited,
        Killed,
        Error,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyInfo {
        pub id: String,
        pub state: PtyState,
        pub exit_code: Option<i32>,
        pub cols: u16,
        pub rows: u16,
        pub shell: String,
        pub cwd: String,
    }

    /// Stub PTY session (PTY feature disabled)
    pub struct PtySession;

    impl PtySession {
        pub fn spawn(_config: PtyConfig) -> Result<Self> {
            anyhow::bail!("PTY support not enabled. Enable the 'pty' feature flag.")
        }

        pub fn spawn_command(_command: &str, _args: &[&str], _config: PtyConfig) -> Result<Self> {
            anyhow::bail!("PTY support not enabled. Enable the 'pty' feature flag.")
        }
    }

    /// Stub PTY manager (PTY feature disabled)
    pub struct PtyManager;

    impl PtyManager {
        pub fn new() -> Self {
            Self
        }

        pub fn create(&mut self, _config: PtyConfig) -> Result<String> {
            anyhow::bail!("PTY support not enabled. Enable the 'pty' feature flag.")
        }
    }

    impl Default for PtyManager {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(feature = "pty"))]
pub use stub::*;

#[cfg(all(test, feature = "pty"))]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_pty_session_spawn() {
        let session = PtySession::spawn(PtyConfig::default()).unwrap();
        assert!(session.is_running());

        // Write a simple command
        session.write_line("echo 'Hello PTY'").unwrap();
        session.write_line("exit").unwrap();

        // Wait for exit
        let code = session.wait().unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn test_pty_session_command() {
        let session = PtySession::spawn_command("echo", &["Hello", "World"], PtyConfig::default())
            .unwrap();

        // Read output with timeout
        let output = session.read_with_timeout(Duration::from_secs(2)).unwrap();
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("Hello World"));
    }

    #[test]
    fn test_pty_manager() {
        let mut manager = PtyManager::new();

        // Create a session
        let id = manager.create(PtyConfig::default()).unwrap();
        assert!(manager.get(&id).is_some());

        // List sessions
        let sessions = manager.list();
        assert_eq!(sessions.len(), 1);

        // Remove session
        manager.remove(&id);
        assert!(manager.get(&id).is_none());
    }

    #[test]
    fn test_pty_resize() {
        let mut session = PtySession::spawn(PtyConfig::default()).unwrap();

        // Resize terminal
        session.resize(120, 40).unwrap();

        let info = session.info();
        assert_eq!(info.cols, 120);
        assert_eq!(info.rows, 40);

        session.kill().unwrap();
    }
}
