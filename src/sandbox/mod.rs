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
