//! NAPI bindings for shell parser module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - parseShellCommand: Parse bash commands using native tree-sitter
//! - assessCommandsRisk: Assess risk level of parsed commands
//! - extractDirectories: Extract directories accessed by commands
//! - extractPermissionPatterns: Extract permission patterns for bash tool

use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::tools::shell_parser::{
    assess_commands_risk as rust_assess_risk, parse_shell_command as rust_parse_command,
    CommandRiskLevel as RustRiskLevel, ParseResult as RustParseResult,
    ParsedCommand as RustParsedCommand, RiskAssessment as RustRiskAssessment,
    ThreadSafeShellParser as RustThreadSafeParser,
};

// ============================================================================
// Shell Parser Types for NAPI
// ============================================================================

/// A parsed shell command with its arguments
#[napi(object)]
pub struct NapiParsedCommand {
    /// The command name (e.g., "cd", "rm", "git")
    pub name: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Raw text of the entire command
    pub raw: String,

    /// Start byte position in source
    pub start_byte: u32,

    /// End byte position in source
    pub end_byte: u32,
}

impl From<RustParsedCommand> for NapiParsedCommand {
    fn from(cmd: RustParsedCommand) -> Self {
        Self {
            name: cmd.name,
            args: cmd.args,
            raw: cmd.raw,
            start_byte: cmd.start_byte as u32,
            end_byte: cmd.end_byte as u32,
        }
    }
}

impl From<NapiParsedCommand> for RustParsedCommand {
    fn from(cmd: NapiParsedCommand) -> Self {
        Self {
            name: cmd.name,
            args: cmd.args,
            raw: cmd.raw,
            start_byte: cmd.start_byte as usize,
            end_byte: cmd.end_byte as usize,
        }
    }
}

/// Result of parsing a shell command string
#[napi(object)]
pub struct NapiShellParseResult {
    /// Successfully parsed commands
    pub commands: Vec<NapiParsedCommand>,

    /// Whether parsing was successful
    pub success: bool,

    /// Error message if parsing failed
    pub error: Option<String>,

    /// Parse duration in milliseconds
    pub duration_ms: u32,
}

impl From<RustParseResult> for NapiShellParseResult {
    fn from(result: RustParseResult) -> Self {
        Self {
            commands: result.commands.into_iter().map(|c| c.into()).collect(),
            success: result.success,
            error: result.error,
            duration_ms: result.duration_ms as u32,
        }
    }
}

/// Risk level for a command
#[napi(string_enum)]
pub enum NapiCommandRiskLevel {
    /// Safe to execute
    Safe,
    /// Low risk (read-only operations)
    Low,
    /// Medium risk (file modifications)
    Medium,
    /// High risk (system modifications, deletions)
    High,
    /// Critical risk (destructive operations)
    Critical,
}

impl From<RustRiskLevel> for NapiCommandRiskLevel {
    fn from(level: RustRiskLevel) -> Self {
        match level {
            RustRiskLevel::Safe => NapiCommandRiskLevel::Safe,
            RustRiskLevel::Low => NapiCommandRiskLevel::Low,
            RustRiskLevel::Medium => NapiCommandRiskLevel::Medium,
            RustRiskLevel::High => NapiCommandRiskLevel::High,
            RustRiskLevel::Critical => NapiCommandRiskLevel::Critical,
        }
    }
}

/// Risk assessment result
#[napi(object)]
pub struct NapiShellRiskAssessment {
    /// Overall risk level as string
    pub level: String,

    /// Reason for the risk level
    pub reason: String,

    /// Whether auto-approval is possible
    pub auto_approvable: bool,

    /// Commands that contributed to the risk level
    pub risky_commands: Vec<String>,
}

impl From<RustRiskAssessment> for NapiShellRiskAssessment {
    fn from(assessment: RustRiskAssessment) -> Self {
        Self {
            level: assessment.level.to_string(),
            reason: assessment.reason,
            auto_approvable: assessment.auto_approvable,
            risky_commands: assessment.risky_commands,
        }
    }
}

/// Permission patterns result
#[napi(object)]
pub struct NapiPermissionPatterns {
    /// Exact command patterns
    pub patterns: Vec<String>,

    /// Wildcard patterns for "always" permissions
    pub always_patterns: Vec<String>,
}

// ============================================================================
// Shell Parser Handle
// ============================================================================

/// Thread-safe shell parser handle for NAPI
#[napi]
pub struct ShellParserHandle {
    inner: Arc<Mutex<RustThreadSafeParser>>,
}

#[napi]
impl ShellParserHandle {
    /// Create a new ShellParser
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustThreadSafeParser::new())),
        }
    }

    /// Parse a shell command string
    #[napi]
    pub fn parse(&self, command: String) -> Result<NapiShellParseResult> {
        let parser = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let result = parser.parse(&command);
        Ok(result.into())
    }

    /// Assess risk of parsed commands
    #[napi]
    pub fn assess_risk(&self, commands: Vec<NapiParsedCommand>) -> Result<NapiShellRiskAssessment> {
        let parser = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_commands: Vec<RustParsedCommand> = commands.into_iter().map(|c| c.into()).collect();
        let assessment = parser.assess_risk(&rust_commands);
        Ok(assessment.into())
    }

    /// Extract directories that will be accessed by commands
    #[napi]
    pub fn extract_directories(&self, commands: Vec<NapiParsedCommand>) -> Result<Vec<String>> {
        let parser = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_commands: Vec<RustParsedCommand> = commands.into_iter().map(|c| c.into()).collect();
        let dirs = parser.extract_directories(&rust_commands);
        Ok(dirs.into_iter().collect())
    }

    /// Extract permission patterns for bash tool
    #[napi]
    pub fn extract_permission_patterns(&self, commands: Vec<NapiParsedCommand>) -> Result<NapiPermissionPatterns> {
        let parser = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_commands: Vec<RustParsedCommand> = commands.into_iter().map(|c| c.into()).collect();
        let (patterns, always) = parser.extract_permission_patterns(&rust_commands);
        Ok(NapiPermissionPatterns {
            patterns: patterns.into_iter().collect(),
            always_patterns: always.into_iter().collect(),
        })
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Parse a shell command using the global parser
#[napi]
pub fn parse_shell_command(command: String) -> NapiShellParseResult {
    rust_parse_command(&command).into()
}

/// Assess risk of commands using the global parser
#[napi]
pub fn assess_shell_commands_risk(commands: Vec<NapiParsedCommand>) -> NapiShellRiskAssessment {
    let rust_commands: Vec<RustParsedCommand> = commands.into_iter().map(|c| c.into()).collect();
    rust_assess_risk(&rust_commands).into()
}

/// Extract directories from commands
#[napi]
pub fn extract_shell_directories(commands: Vec<NapiParsedCommand>) -> Vec<String> {
    let rust_commands: Vec<RustParsedCommand> = commands.into_iter().map(|c| c.into()).collect();
    let parser = crate::tools::shell_parser::global_parser();
    let dirs = parser.extract_directories(&rust_commands);
    dirs.into_iter().collect()
}

/// Extract permission patterns from commands
#[napi]
pub fn extract_shell_permission_patterns(commands: Vec<NapiParsedCommand>) -> NapiPermissionPatterns {
    let rust_commands: Vec<RustParsedCommand> = commands.into_iter().map(|c| c.into()).collect();
    let parser = crate::tools::shell_parser::global_parser();
    let (patterns, always) = parser.extract_permission_patterns(&rust_commands);
    NapiPermissionPatterns {
        patterns: patterns.into_iter().collect(),
        always_patterns: always.into_iter().collect(),
    }
}

/// Check if a command is file-manipulating
#[napi]
pub fn is_file_command(command_name: String) -> bool {
    matches!(
        command_name.as_str(),
        "cd" | "rm" | "cp" | "mv" | "mkdir" | "touch" | "chmod" | "chown" | "cat"
            | "rmdir" | "ln" | "tar" | "unzip" | "zip"
    )
}

/// Check if a command is potentially dangerous
#[napi]
pub fn is_dangerous_command(command_name: String) -> bool {
    matches!(
        command_name.as_str(),
        "rm" | "rmdir" | "dd" | "mkfs" | "fdisk" | "parted"
            | "shutdown" | "reboot" | "poweroff" | "halt"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shell_command() {
        let result = parse_shell_command("ls -la".to_string());
        assert!(result.success);
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0].name, "ls");
    }

    #[test]
    fn test_assess_risk() {
        let result = parse_shell_command("rm -rf /".to_string());
        let assessment = assess_shell_commands_risk(result.commands);
        assert_eq!(assessment.level, "critical");
        assert!(!assessment.auto_approvable);
    }

    #[test]
    fn test_shell_parser_handle() {
        let parser = ShellParserHandle::new();
        let result = parser.parse("echo hello".to_string()).unwrap();
        assert!(result.success);
        assert_eq!(result.commands[0].name, "echo");
    }

    #[test]
    fn test_is_file_command() {
        assert!(is_file_command("rm".to_string()));
        assert!(is_file_command("cp".to_string()));
        assert!(!is_file_command("echo".to_string()));
    }

    #[test]
    fn test_is_dangerous_command() {
        assert!(is_dangerous_command("rm".to_string()));
        assert!(is_dangerous_command("dd".to_string()));
        assert!(!is_dangerous_command("ls".to_string()));
    }
}
