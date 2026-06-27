use super::Tool;

/// Execute a shell command.
pub struct RunCommand;

impl Tool for RunCommand {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its stdout + stderr. Input: shell command."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let cmd = input.trim();
        if cmd.is_empty() {
            anyhow::bail!("run_command requires a command");
        }

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| anyhow::anyhow!("cannot execute command: {e}"))?;

        let mut result = String::new();

        if !output.stdout.is_empty() {
            result.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("stderr: ");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if !output.status.success() && result.is_empty() {
            result.push_str(&format!("exit code: {}", output.status.code().unwrap_or(-1)));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_echo() {
        let result = RunCommand.execute("echo hello").unwrap();
        assert_eq!(result.trim(), "hello");
    }

    #[test]
    fn test_run_empty() {
        assert!(RunCommand.execute("").is_err());
    }

    #[test]
    fn test_run_failing() {
        let result = RunCommand.execute("false").unwrap();
        assert!(!result.is_empty());
    }
}
