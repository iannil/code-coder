use super::Tool;
use std::time::Duration;

/// Default timeout for command execution (30 seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Execute a shell command with a timeout.
pub struct RunCommand;

impl Tool for RunCommand {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its stdout + stderr. Input: shell command. Prefix with 'timeout:<N>' to override the 30s default timeout."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let input = input.trim();
        if input.is_empty() {
            anyhow::bail!("run_command requires a command");
        }

        // Parse optional timeout prefix
        let (cmd, timeout) = if let Some(rest) = input.strip_prefix("timeout:") {
            let colon = rest.find(' ').unwrap_or(rest.len());
            let secs: u64 = rest[..colon].parse()
                .unwrap_or(DEFAULT_TIMEOUT_SECS);
            let cmd = rest[colon..].trim();
            (cmd, Duration::from_secs(secs))
        } else {
            (input, Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        };

        if cmd.is_empty() {
            anyhow::bail!("run_command requires a command");
        }

        run_with_timeout(cmd, timeout)
    }
}

/// Run a shell command with a specified timeout.
fn run_with_timeout(cmd: &str, timeout: Duration) -> anyhow::Result<String> {
    use std::sync::mpsc;

    let cmd_owned = cmd.to_string();
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("run_command".into())
        .spawn(move || {
            let result = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd_owned)
                .output()
                .map_err(|e| anyhow::anyhow!("cannot execute command: {e}"));

            let _ = tx.send(result);
        })
        .map_err(|e| anyhow::anyhow!("failed to spawn command thread: {e}"))?;

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
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
        Ok(Err(e)) => Err(e),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            anyhow::bail!("Command timed out after {} seconds: '{}'", timeout.as_secs(), cmd)
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            anyhow::bail!("Command thread terminated unexpectedly: '{}'", cmd)
        }
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

    #[test]
    fn test_run_with_custom_timeout() {
        let result = RunCommand.execute("timeout:5 echo hi").unwrap();
        assert_eq!(result.trim(), "hi");
    }

    #[test]
    fn test_run_timeout_triggers() {
        let result = RunCommand.execute("timeout:1 sleep 10");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timed out"), "error should mention timeout: {err}");
    }
}
