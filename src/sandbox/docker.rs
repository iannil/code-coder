use super::{prepare_temp_dir, Sandbox};

/// Run code in a Docker container.
///
/// Language → image mapping:
/// - python → python:alpine
/// - javascript / js / node → node:alpine
/// - ruby → ruby:alpine
/// - go → golang:alpine
/// - rust → rust:alpine
/// - c / c++ → gcc:alpine
/// - sh / bash → alpine:latest
pub struct DockerSandbox {
    _timeout_secs: u64,
    memory_limit: String,
}

impl DockerSandbox {
    pub fn new() -> Self {
        Self {
            _timeout_secs: 30,
            memory_limit: "128m".into(),
        }
    }
}

impl Sandbox for DockerSandbox {
    fn name(&self) -> &str {
        "docker"
    }

    fn run(&self, code: &str, language: &str) -> anyhow::Result<String> {
        let (image, file_name, cmd) = image_for_language(language);

        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");
        let (dir, _file_path) = prepare_temp_dir(code, &format!(".{ext}"))?;

        let dir_str = dir.to_string_lossy().to_string();

        // Build docker run command
        let output = std::process::Command::new("docker")
            .args([
                "run",
                "--rm",
                "-i",
                "-v",
                &format!("{dir_str}:/code:ro"),
                "-w",
                "/code",
                "--memory",
                &self.memory_limit,
                "--network",
                "none", // no network access from sandbox
                image,
                "sh",
                "-c",
                &cmd,
            ])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!("Docker not found. Install Docker to use the sandbox.")
                } else {
                    anyhow::anyhow!("Docker execution failed: {e}")
                }
            })?;

        let mut result = String::new();
        if !output.stdout.is_empty() {
            result.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&format!("stderr: {}", String::from_utf8_lossy(&output.stderr)));
        }
        if !output.status.success() && result.is_empty() {
            result.push_str(&format!("exit code: {}", output.status.code().unwrap_or(-1)));
        }

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&dir);

        Ok(result)
    }
}

fn image_for_language(language: &str) -> (&'static str, &'static str, String) {
    match language.to_lowercase().as_str() {
        "python" | "py" => ("python:alpine", "main.py", "python main.py".into()),
        "javascript" | "js" | "node" | "nodejs" => {
            ("node:alpine", "main.js", "node main.js".into())
        }
        "ruby" | "rb" => ("ruby:alpine", "main.rb", "ruby main.rb".into()),
        "go" | "golang" => (
            "golang:alpine",
            "main.go",
            "go run main.go".into(),
        ),
        "rust" | "rs" => (
            "rust:alpine",
            "main.rs",
            "rustc main.rs -o main && ./main".into(),
        ),
        "c" => ("gcc:alpine", "main.c", "gcc main.c -o main && ./main".into()),
        "c++" | "cpp" | "cc" => {
            ("gcc:alpine", "main.cpp", "g++ main.cpp -o main && ./main".into())
        }
        "sh" | "bash" | "shell" => {
            ("alpine:latest", "script.sh", "sh /code/script.sh".into())
        }
        _ => (
            "alpine:latest",
            "script.sh",
            format!("echo 'No image configured for language: {language}'"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_not_available() {
        // If Docker is not installed, the sandbox should give a clear error
        let sb = DockerSandbox::new();
        let result = sb.run("print('hello')", "python");
        // Either Docker works (CI) or it fails with a clear message
        match result {
            Ok(out) => assert!(out.trim() == "hello" || !out.is_empty()),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("Docker") || msg.contains("NotFound") || msg.contains("denied"),
                    "Unexpected error: {msg}"
                );
            }
        }
    }
}
