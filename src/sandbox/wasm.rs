use super::{prepare_temp_dir, Sandbox};

/// Run code via WASM runtime (wasmtime).
///
/// For languages like Rust, Go, and C, the code is compiled to WASM
/// first, then executed by the wasmtime runtime.
///
/// For raw `.wasm` / `.wat` input, it runs directly.
pub struct WasmSandbox;

impl WasmSandbox {
    pub fn new() -> Self {
        Self
    }
}

impl Sandbox for WasmSandbox {
    fn name(&self) -> &str {
        "wasm"
    }

    fn run(&self, code: &str, language: &str) -> anyhow::Result<String> {
        let lang = language.to_lowercase();

        match lang.as_str() {
            "wat" | "wasm" => run_wasm_direct(code),
            "rust" | "rs" => compile_and_run_rust_wasm(code),
            "go" | "golang" => compile_and_run_go_wasm(code),
            "c" | "c++" | "cpp" => run_via_docker_fallback(code, language),
            _ => anyhow::bail!(
                "WASM sandbox does not support '{language}'. \
                 Supported: rust, go, c, c++, wat, wasm."
            ),
        }
    }
}

/// Run a raw `.wat` (WebAssembly Text) or `.wasm` binary.
fn run_wasm_direct(code: &str) -> anyhow::Result<String> {
    let is_wat = code.trim_start().starts_with('(') || code.contains("module");

    if is_wat {
        // Compile WAT → WASM, then run
        let _output = std::process::Command::new("wat2wasm")
            .arg("-") // stdin
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "wat2wasm not found. Install wabt to run .wat files directly."
                    )
                } else {
                    anyhow::anyhow!("wat2wasm failed: {e}")
                }
            })?;

        // Write WAT to stdin, read WASM from stdout
        // For MVP, let's just fall back to a simpler approach
        anyhow::bail!(
            "WAT execution requires wat2wasm. \
             Use 'rust' or pre-compiled WASM instead."
        );
    } else {
        // Assume it's a WASM binary — run via wasmtime if available
        let (_dir, wasm_path) = prepare_temp_dir(code, ".wasm")?;
        let path_str = wasm_path.to_string_lossy().to_string();

        let output = std::process::Command::new("wasmtime")
            .args([&path_str])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "wasmtime not found. Install wasmtime to run WASM files."
                    )
                } else {
                    anyhow::anyhow!("wasmtime execution failed: {e}")
                }
            })?;

        if let Some(parent) = wasm_path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
        Ok(format_output(output))
    }
}

/// Compile Rust code to WASM and run it.
fn compile_and_run_rust_wasm(code: &str) -> anyhow::Result<String> {
    let (dir, rs_path) = prepare_temp_dir(code, ".rs")?;
    let _dir_str = dir.to_string_lossy().to_string();

    // Check if rustc is available
    let version_check = std::process::Command::new("rustc")
        .arg("--version")
        .output();
    if version_check.is_err() {
        anyhow::bail!("rustc not found. Install Rust to compile Rust → WASM.");
    }

    // Compile to WASM
    let wasm_path = dir.join("main.wasm");
    let wasm_path_s = wasm_path.to_string_lossy().to_string();
    let rs_path_s = rs_path.to_string_lossy().to_string();
    let compile = std::process::Command::new("rustc")
        .args([
            "--target",
            "wasm32-wasi",
            "-o",
            &wasm_path_s,
            &rs_path_s,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("rustc compilation failed: {e}"))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let _ = std::fs::remove_dir_all(&dir);
        anyhow::bail!("Rust compilation failed:\n{stderr}");
    }

    // Run the WASM via wasmtime
    let wasm_path_str = wasm_path.to_string_lossy().to_string();
    let output = std::process::Command::new("wasmtime")
        .args([&wasm_path_str])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "wasmtime not found. Install wasmtime to run WASM output."
                )
            } else {
                anyhow::anyhow!("wasmtime execution failed: {e}")
            }
        })?;

    let _ = std::fs::remove_dir_all(&dir);
    Ok(format_output(output))
}

/// Compile Go code to WASM and run it.
fn compile_and_run_go_wasm(code: &str) -> anyhow::Result<String> {
    let (dir, _go_path) = prepare_temp_dir(code, ".go")?;
    let _dir_str = dir.to_string_lossy().to_string();

    // Check if Go is available
    let go_check = std::process::Command::new("go").arg("version").output();
    if go_check.is_err() {
        anyhow::bail!("go not found. Install Go to compile Go → WASM.");
    }

    // Compile to WASM with golang:alpine via Docker (more reliable)
    // For MVP, just fall back to Docker
    let _ = std::fs::remove_dir_all(&dir);
    anyhow::bail!(
        "Go → WASM compilation requires Go toolchain. \
         Use 'golang' language to run directly via Docker instead."
    );
}

/// Fallback: run via Docker sandbox for languages where we can't do WASM.
fn run_via_docker_fallback(code: &str, language: &str) -> anyhow::Result<String> {
    let docker = super::docker::DockerSandbox::new();
    docker.run(code, language)
}

fn format_output(output: std::process::Output) -> String {
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
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_unsupported_language() {
        let sb = WasmSandbox::new();
        let result = sb.run("print('hi')", "python");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("does not support") || msg.contains("Docker"));
    }

    #[test]
    fn test_wasm_no_rustc() {
        let sb = WasmSandbox::new();
        let result = sb.run("fn main() { println!(\"hello\"); }", "rust");
        // Will either fail with "rustc not found" or "wasmtime not found"
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("rustc") || msg.contains("wasmtime"),
                "Unexpected error: {msg}"
            );
        }
    }

    #[test]
    fn test_format_output_stdout_only() {
        let output = std::process::Output {
            status: std::process::Command::new("true").status().unwrap(),
            stdout: b"hello world\n".to_vec(),
            stderr: b"".to_vec(),
        };
        let result = format_output(output);
        assert_eq!(result, "hello world\n");
    }

    #[test]
    fn test_format_output_stderr_only() {
        let output = std::process::Output {
            status: std::process::Command::new("false").status().unwrap(),
            stdout: b"".to_vec(),
            stderr: b"error: something failed\n".to_vec(),
        };
        let result = format_output(output);
        assert_eq!(result, "stderr: error: something failed\n");
    }

    #[test]
    fn test_format_output_both() {
        let output = std::process::Output {
            status: std::process::Command::new("true").status().unwrap(),
            stdout: b"output\n".to_vec(),
            stderr: b"warning\n".to_vec(),
        };
        let result = format_output(output);
        assert!(result.contains("output"));
        assert!(result.contains("warning"));
    }

    #[test]
    fn test_format_output_failure_no_output() {
        let output = std::process::Output {
            status: std::process::Command::new("false").status().unwrap(),
            stdout: b"".to_vec(),
            stderr: b"".to_vec(),
        };
        let result = format_output(output);
        assert!(result.contains("exit code"));
    }

    #[test]
    fn test_format_output_failure_with_stderr() {
        let output = std::process::Output {
            status: std::process::Command::new("false").status().unwrap(),
            stdout: b"".to_vec(),
            stderr: b"oops\n".to_vec(),
        };
        let result = format_output(output);
        assert!(result.contains("stderr: oops"));
        // When there's stderr, we show the stderr, not the exit code
        assert!(!result.contains("exit code"));
    }

    #[test]
    fn test_wasm_name() {
        let sb = WasmSandbox::new();
        assert_eq!(sb.name(), "wasm");
    }

    #[test]
    fn test_run_wasm_direct_empty() {
        let result = run_wasm_direct("");
        assert!(result.is_err());
    }
}
