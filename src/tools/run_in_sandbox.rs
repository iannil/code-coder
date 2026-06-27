use super::Tool;
use crate::sandbox::{select_sandbox, Sandbox};

/// Run untrusted code in a sandboxed environment.
///
/// Input format:
/// ```
/// <language>
/// ---
/// <code>
/// ```
///
/// Supported languages: python, javascript, go, rust, c, c++, sh, ruby
/// The system automatically selects WASM (Level 1) or Docker (Level 2)
/// based on the language.
pub struct RunInSandbox;

impl Tool for RunInSandbox {
    fn name(&self) -> &str {
        "run_in_sandbox"
    }

    fn description(&self) -> &str {
        "Run code in a sandbox. Input: '<language>\\n---\\n<code>'"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let input = input.trim();

        let sep = input.find("\n---\n");
        let (language, code) = match sep {
            Some(pos) => (input[..pos].trim(), input[pos + 5..].trim()),
            None => anyhow::bail!("input must be '<language>\\n---\\n<code>'"),
        };

        if language.is_empty() {
            anyhow::bail!("language cannot be empty");
        }
        if code.is_empty() {
            anyhow::bail!("code cannot be empty");
        }

        // Check if Docker is available (for Level 2 fallback)
        let docker_available = std::process::Command::new("docker")
            .arg("--version")
            .output()
            .is_ok();

        let sandbox: Box<dyn Sandbox> = select_sandbox(language, docker_available);

        let result = sandbox.run(code, language)?;
        Ok(result.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_separator() {
        let result = RunInSandbox.execute("python print('hi')");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("---"));
    }

    #[test]
    fn test_empty_language() {
        let result = RunInSandbox.execute("\n---\nprint('hi')");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_code() {
        let result = RunInSandbox.execute("python\n---\n");
        assert!(result.is_err());
    }
}
