/// ─── Sandbox ───────────────────────────────────────────────────────────────
///
/// Graded sandbox for running untrusted code:
///
/// | Level | Method | Use case |
/// |-------|--------|----------|
/// | L0    | None   | Reading docs, API extraction (already done) |
/// | L1    | WASM   | Compiled languages that target wasm (Rust, Go, C) |
/// | L2    | Docker | Full application runtime (any language) |
/// | L3    | VM     | Untrusted binaries (future) |

mod docker;
mod wasm;

use std::path::PathBuf;

pub use docker::DockerSandbox;
pub use wasm::WasmSandbox;

/// A sandbox that can execute untrusted code.
pub trait Sandbox: Send + 'static {
    fn name(&self) -> &str;
    fn run(&self, code: &str, language: &str) -> anyhow::Result<String>;
}

/// Automatically pick the best sandbox level for a given language.
pub fn select_sandbox(language: &str, docker_available: bool) -> Box<dyn Sandbox> {
    let lang = language.to_lowercase();

    // Languages that can be compiled to WASM and run natively -> L1
    let wasm_languages = ["rust", "go", "c", "c++", "wat", "wasm"];
    if wasm_languages.contains(&lang.as_str()) {
        // WASM is preferred when available (zero deps, fast)
        return Box::new(WasmSandbox::new());
    }

    // Everything else -> L2 (Docker fallback)
    if docker_available {
        return Box::new(DockerSandbox::new());
    }

    // No sandbox available — return a no-op that explains
    Box::new(NoSandbox)
}

/// ─── NoSandbox (fallback when no sandbox is configured) ────────────────────

pub struct NoSandbox;

impl Sandbox for NoSandbox {
    fn name(&self) -> &str {
        "none"
    }

    fn run(&self, _code: &str, language: &str) -> anyhow::Result<String> {
        anyhow::bail!(
            "No sandbox available for '{language}'. \
             Install Docker or a WASM runtime to enable code execution."
        )
    }
}

/// Helper: create a temp directory with the code file.
fn prepare_temp_dir(code: &str, extension: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
    #[allow(deprecated)]
    let dir = tempfile::tempdir()
        .map_err(|e| anyhow::anyhow!("cannot create temp dir: {e}"))?
        .into_path();
    let file_path = dir.join(format!("main{extension}"));
    std::fs::write(&file_path, code)
        .map_err(|e| anyhow::anyhow!("cannot write code file: {e}"))?;
    Ok((dir, file_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── NoSandbox ────────────────────────────────────────────────────────

    #[test]
    fn test_no_sandbox_name() {
        let sandbox = NoSandbox;
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    fn test_no_sandbox_run_returns_error() {
        let sandbox = NoSandbox;
        let result = sandbox.run("print('hi')", "python");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No sandbox") || err.contains("Docker") || err.contains("WASM"));
    }

    // ─── select_sandbox ───────────────────────────────────────────────────

    #[test]
    fn test_select_sandbox_rust_uses_wasm() {
        let sandbox = select_sandbox("rust", false);
        // When docker is unavailable, rust should use WASM (since it's in the wasm list)
        assert_eq!(sandbox.name(), "wasm");
    }

    #[test]
    fn test_select_sandbox_python_uses_docker() {
        let sandbox = select_sandbox("python", true);
        assert_eq!(sandbox.name(), "docker");
    }

    #[test]
    fn test_select_sandbox_python_no_docker_uses_none() {
        let sandbox = select_sandbox("python", false);
        assert_eq!(sandbox.name(), "none");
    }

    #[test]
    fn test_select_sandbox_unknown_language_with_docker() {
        let sandbox = select_sandbox("unknown_lang_xyz", true);
        assert_eq!(sandbox.name(), "docker");
    }

    #[test]
    fn test_select_sandbox_case_insensitive() {
        let sandbox = select_sandbox("Rust", false);
        assert_eq!(sandbox.name(), "wasm");
    }

    #[test]
    fn test_select_sandbox_go() {
        let sandbox = select_sandbox("go", false);
        assert_eq!(sandbox.name(), "wasm");
    }

    // ─── prepare_temp_dir ─────────────────────────────────────────────────

    #[test]
    fn test_prepare_temp_dir_creates_files() {
        let (dir, file_path) = prepare_temp_dir("fn main() {}", ".rs").unwrap();
        assert!(file_path.exists());
        assert!(file_path.ends_with("main.rs"));
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "fn main() {}");
        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_prepare_temp_dir_py_extension() {
        let (dir, file_path) = prepare_temp_dir("print('hello')", ".py").unwrap();
        assert!(file_path.ends_with("main.py"));
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "print('hello')");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_prepare_temp_dir_empty_code() {
        let (dir, file_path) = prepare_temp_dir("", ".txt").unwrap();
        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ─── Trait bounds ─────────────────────────────────────────────────────

    #[test]
    fn test_sandbox_trait_is_send() {
        fn check_send<T: Send + 'static>() {}
        check_send::<NoSandbox>();
        check_send::<DockerSandbox>();
        check_send::<WasmSandbox>();
    }
}
