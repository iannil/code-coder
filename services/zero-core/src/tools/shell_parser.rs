//! Shell command parser using native tree-sitter
//!
//! This module provides high-performance bash command parsing without WASM overhead.
//! It extracts commands, arguments, directories, and permission patterns from shell commands.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tree_sitter::{Language, Node, Parser, Tree};

// Tree-sitter language (lazily initialized)
static BASH_LANGUAGE: OnceLock<Language> = OnceLock::new();

fn get_bash_language() -> &'static Language {
    BASH_LANGUAGE.get_or_init(|| tree_sitter_bash::LANGUAGE.into())
}

/// A parsed shell command with its arguments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedCommand {
    /// The command name (e.g., "cd", "rm", "git")
    pub name: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Raw text of the entire command
    pub raw: String,

    /// Start byte position in source
    pub start_byte: usize,

    /// End byte position in source
    pub end_byte: usize,
}

impl ParsedCommand {
    /// Get the full command string (name + args joined)
    pub fn full_command(&self) -> String {
        if self.args.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.args.join(" "))
        }
    }

    /// Check if this is a file-manipulating command
    pub fn is_file_command(&self) -> bool {
        matches!(
            self.name.as_str(),
            "cd" | "rm" | "cp" | "mv" | "mkdir" | "touch" | "chmod" | "chown" | "cat"
                | "rmdir" | "ln" | "tar" | "unzip" | "zip"
        )
    }

    /// Check if this is a potentially dangerous command
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self.name.as_str(),
            "rm" | "rmdir" | "dd" | "mkfs" | "fdisk" | "parted"
                | "shutdown" | "reboot" | "poweroff" | "halt"
        )
    }

    /// Get path arguments (non-flag arguments that could be paths)
    pub fn path_args(&self) -> Vec<&str> {
        self.args
            .iter()
            .filter(|arg| !arg.starts_with('-') && !arg.starts_with('+'))
            .map(|s| s.as_str())
            .collect()
    }
}

/// Result of parsing a shell command string
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// Successfully parsed commands
    pub commands: Vec<ParsedCommand>,

    /// Whether parsing was successful
    pub success: bool,

    /// Error message if parsing failed
    pub error: Option<String>,

    /// Parse duration in milliseconds
    pub duration_ms: u64,
}

/// Risk level for a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandRiskLevel {
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

impl std::fmt::Display for CommandRiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandRiskLevel::Safe => write!(f, "safe"),
            CommandRiskLevel::Low => write!(f, "low"),
            CommandRiskLevel::Medium => write!(f, "medium"),
            CommandRiskLevel::High => write!(f, "high"),
            CommandRiskLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk level
    pub level: CommandRiskLevel,

    /// Reason for the risk level
    pub reason: String,

    /// Whether auto-approval is possible
    pub auto_approvable: bool,

    /// Commands that contributed to the risk level
    pub risky_commands: Vec<String>,
}

/// Shell command parser using native tree-sitter
pub struct ShellParser {
    parser: Parser,
}

impl Default for ShellParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellParser {
    /// Create a new shell parser
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&get_bash_language())
            .expect("Error loading Bash grammar");

        Self { parser }
    }

    /// Parse a shell command string
    pub fn parse(&mut self, command: &str) -> ParseResult {
        let start = std::time::Instant::now();

        match self.parser.parse(command, None) {
            Some(tree) => {
                let commands = self.extract_commands(&tree, command);
                ParseResult {
                    commands,
                    success: true,
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                }
            }
            None => ParseResult {
                commands: vec![],
                success: false,
                error: Some("Failed to parse command".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Extract all commands from a parsed tree
    fn extract_commands(&self, tree: &Tree, source: &str) -> Vec<ParsedCommand> {
        let mut commands = Vec::new();
        let root = tree.root_node();

        self.visit_node(root, source, &mut commands);

        commands
    }

    /// Visit a node and extract commands recursively
    fn visit_node(&self, node: Node, source: &str, commands: &mut Vec<ParsedCommand>) {
        if node.kind() == "command" {
            if let Some(cmd) = self.parse_command_node(node, source) {
                commands.push(cmd);
            }
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child, source, commands);
        }
    }

    /// Parse a single command node
    fn parse_command_node(&self, node: Node, source: &str) -> Option<ParsedCommand> {
        let mut name = String::new();
        let mut args = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let text = &source[child.start_byte()..child.end_byte()];

            match child.kind() {
                "command_name" => {
                    name = text.to_string();
                }
                "word" | "string" | "raw_string" | "concatenation" => {
                    if name.is_empty() {
                        name = text.to_string();
                    } else {
                        args.push(text.to_string());
                    }
                }
                _ => {
                    // Other node types (redirections, etc.) are skipped
                }
            }
        }

        if name.is_empty() {
            return None;
        }

        Some(ParsedCommand {
            name,
            args,
            raw: source[node.start_byte()..node.end_byte()].to_string(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        })
    }

    /// Assess the risk level of a command
    pub fn assess_risk(&self, commands: &[ParsedCommand]) -> RiskAssessment {
        let mut max_level = CommandRiskLevel::Safe;
        let mut risky_commands = Vec::new();
        let mut reasons = Vec::new();

        for cmd in commands {
            let (level, reason) = self.assess_single_command(cmd);

            if level as u8 > max_level as u8 {
                max_level = level;
            }

            if level as u8 >= CommandRiskLevel::Medium as u8 {
                risky_commands.push(cmd.full_command());
                reasons.push(reason);
            }
        }

        let reason = if reasons.is_empty() {
            "No risky operations detected".to_string()
        } else {
            reasons.join("; ")
        };

        let auto_approvable = max_level as u8 <= CommandRiskLevel::Medium as u8;

        RiskAssessment {
            level: max_level,
            reason,
            auto_approvable,
            risky_commands,
        }
    }

    /// Assess risk for a single command
    fn assess_single_command(&self, cmd: &ParsedCommand) -> (CommandRiskLevel, String) {
        match cmd.name.as_str() {
            // Critical: Destructive system operations
            "rm" => {
                if cmd.args.iter().any(|a| a.contains("-rf") || a.contains("-r")) {
                    (
                        CommandRiskLevel::Critical,
                        format!("Recursive removal: {}", cmd.full_command()),
                    )
                } else {
                    (
                        CommandRiskLevel::High,
                        format!("File deletion: {}", cmd.full_command()),
                    )
                }
            }
            "dd" | "mkfs" | "fdisk" | "parted" => (
                CommandRiskLevel::Critical,
                format!("Disk operation: {}", cmd.full_command()),
            ),
            "shutdown" | "reboot" | "poweroff" | "halt" => (
                CommandRiskLevel::Critical,
                format!("System power control: {}", cmd.full_command()),
            ),

            // High: System modifications
            "chmod" | "chown" => (
                CommandRiskLevel::High,
                format!("Permission change: {}", cmd.full_command()),
            ),
            "sudo" | "su" => (
                CommandRiskLevel::High,
                format!("Privilege escalation: {}", cmd.full_command()),
            ),
            "kill" | "pkill" | "killall" => (
                CommandRiskLevel::High,
                format!("Process termination: {}", cmd.full_command()),
            ),

            // Medium: File modifications
            "mv" | "cp" => (
                CommandRiskLevel::Medium,
                format!("File operation: {}", cmd.full_command()),
            ),
            "mkdir" | "touch" | "rmdir" => (
                CommandRiskLevel::Medium,
                format!("Directory/file creation: {}", cmd.full_command()),
            ),
            "tar" | "unzip" | "zip" | "gzip" | "gunzip" => (
                CommandRiskLevel::Medium,
                format!("Archive operation: {}", cmd.full_command()),
            ),
            "wget" | "curl" => {
                // Check if writing to file
                if cmd.args.iter().any(|a| a.starts_with("-o") || a == "-O") {
                    (
                        CommandRiskLevel::Medium,
                        format!("Download with write: {}", cmd.full_command()),
                    )
                } else {
                    (
                        CommandRiskLevel::Low,
                        format!("HTTP request: {}", cmd.full_command()),
                    )
                }
            }
            "npm" | "pnpm" | "yarn" | "bun" => {
                if cmd.args.first().map(|s| s.as_str()) == Some("install")
                    || cmd.args.first().map(|s| s.as_str()) == Some("i")
                    || cmd.args.first().map(|s| s.as_str()) == Some("add")
                {
                    (
                        CommandRiskLevel::Medium,
                        format!("Package installation: {}", cmd.full_command()),
                    )
                } else {
                    (CommandRiskLevel::Low, "Package manager".to_string())
                }
            }

            // Low: Read-only or safe operations
            "ls" | "pwd" | "cd" | "echo" | "cat" | "head" | "tail" | "less" | "more" | "grep"
            | "find" | "which" | "whereis" | "type" | "file" | "stat" | "wc" | "sort" | "uniq"
            | "diff" | "date" | "cal" | "whoami" | "hostname" | "uname" => {
                (CommandRiskLevel::Low, "Read-only operation".to_string())
            }

            // Safe: Version/help queries
            _ if cmd.args.iter().any(|a| a == "--version" || a == "-v" || a == "--help" || a == "-h") => {
                (CommandRiskLevel::Safe, "Version/help query".to_string())
            }

            // Default: Low risk for unknown commands
            _ => (
                CommandRiskLevel::Low,
                format!("Unknown command: {}", cmd.name),
            ),
        }
    }

    /// Extract directories that will be accessed by commands
    pub fn extract_directories(&self, commands: &[ParsedCommand]) -> HashSet<String> {
        let mut dirs = HashSet::new();

        for cmd in commands {
            if cmd.is_file_command() {
                for arg in cmd.path_args() {
                    // Skip flags
                    if !arg.starts_with('-') {
                        dirs.insert(arg.to_string());
                    }
                }
            }
        }

        dirs
    }

    /// Extract permission patterns for bash tool
    pub fn extract_permission_patterns(&self, commands: &[ParsedCommand]) -> (HashSet<String>, HashSet<String>) {
        let mut patterns = HashSet::new();
        let mut always_patterns = HashSet::new();

        for cmd in commands {
            if cmd.name != "cd" {
                let full = cmd.full_command();
                patterns.insert(full.clone());

                // Create prefix pattern for "always" permissions
                let prefix = if cmd.args.is_empty() {
                    format!("{}*", cmd.name)
                } else {
                    format!("{} {}*", cmd.name, cmd.args.first().unwrap_or(&String::new()))
                };
                always_patterns.insert(prefix);
            }
        }

        (patterns, always_patterns)
    }
}

// Thread-safe parser wrapper for NAPI
pub struct ThreadSafeShellParser {
    inner: std::sync::Mutex<ShellParser>,
}

impl Default for ThreadSafeShellParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadSafeShellParser {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(ShellParser::new()),
        }
    }

    pub fn parse(&self, command: &str) -> ParseResult {
        let mut parser = self.inner.lock().expect("Parser lock poisoned");
        parser.parse(command)
    }

    pub fn assess_risk(&self, commands: &[ParsedCommand]) -> RiskAssessment {
        let parser = self.inner.lock().expect("Parser lock poisoned");
        parser.assess_risk(commands)
    }

    pub fn extract_directories(&self, commands: &[ParsedCommand]) -> HashSet<String> {
        let parser = self.inner.lock().expect("Parser lock poisoned");
        parser.extract_directories(commands)
    }

    pub fn extract_permission_patterns(&self, commands: &[ParsedCommand]) -> (HashSet<String>, HashSet<String>) {
        let parser = self.inner.lock().expect("Parser lock poisoned");
        parser.extract_permission_patterns(commands)
    }
}

// Global parser instance for convenience
static GLOBAL_PARSER: OnceLock<ThreadSafeShellParser> = OnceLock::new();

/// Get or create the global shell parser
pub fn global_parser() -> &'static ThreadSafeShellParser {
    GLOBAL_PARSER.get_or_init(ThreadSafeShellParser::new)
}

/// Parse a shell command using the global parser
pub fn parse_shell_command(command: &str) -> ParseResult {
    global_parser().parse(command)
}

/// Assess risk of commands using the global parser
pub fn assess_commands_risk(commands: &[ParsedCommand]) -> RiskAssessment {
    global_parser().assess_risk(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut parser = ShellParser::new();
        let result = parser.parse("ls -la");

        assert!(result.success);
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0].name, "ls");
        assert_eq!(result.commands[0].args, vec!["-la"]);
    }

    #[test]
    fn test_piped_commands() {
        let mut parser = ShellParser::new();
        let result = parser.parse("cat file.txt | grep pattern | wc -l");

        assert!(result.success);
        assert_eq!(result.commands.len(), 3);
        assert_eq!(result.commands[0].name, "cat");
        assert_eq!(result.commands[1].name, "grep");
        assert_eq!(result.commands[2].name, "wc");
    }

    #[test]
    fn test_chained_commands() {
        let mut parser = ShellParser::new();
        let result = parser.parse("mkdir foo && cd foo && touch bar.txt");

        assert!(result.success);
        assert_eq!(result.commands.len(), 3);
        assert_eq!(result.commands[0].name, "mkdir");
        assert_eq!(result.commands[1].name, "cd");
        assert_eq!(result.commands[2].name, "touch");
    }

    #[test]
    fn test_rm_risk_assessment() {
        let mut parser = ShellParser::new();
        let result = parser.parse("rm -rf /tmp/test");

        assert!(result.success);
        let risk = parser.assess_risk(&result.commands);
        assert_eq!(risk.level, CommandRiskLevel::Critical);
        assert!(!risk.auto_approvable);
    }

    #[test]
    fn test_safe_command_risk() {
        let mut parser = ShellParser::new();
        let result = parser.parse("ls -la");

        assert!(result.success);
        let risk = parser.assess_risk(&result.commands);
        assert_eq!(risk.level, CommandRiskLevel::Low);
        assert!(risk.auto_approvable);
    }

    #[test]
    fn test_extract_directories() {
        let mut parser = ShellParser::new();
        let result = parser.parse("cd /tmp && rm file.txt && cp src dst");

        let dirs = parser.extract_directories(&result.commands);
        assert!(dirs.contains("/tmp"));
        assert!(dirs.contains("file.txt"));
        assert!(dirs.contains("src"));
        assert!(dirs.contains("dst"));
    }

    #[test]
    fn test_extract_permission_patterns() {
        let mut parser = ShellParser::new();
        let result = parser.parse("git status && npm install express");

        let (patterns, always) = parser.extract_permission_patterns(&result.commands);
        assert!(patterns.contains("git status"));
        assert!(patterns.contains("npm install express"));
        assert!(always.contains("git status*"));
        assert!(always.contains("npm install*"));
    }

    #[test]
    fn test_global_parser() {
        let result = parse_shell_command("echo hello");
        assert!(result.success);
        assert_eq!(result.commands[0].name, "echo");
    }

    #[test]
    fn test_thread_safe_parser() {
        use std::thread;

        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let cmd = format!("echo test_{}", i);
                    let result = parse_shell_command(&cmd);
                    assert!(result.success);
                    result.commands[0].name.clone()
                })
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.join().unwrap(), "echo");
        }
    }
}
