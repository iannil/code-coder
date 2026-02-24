//! Integration tests for Docker sandbox.
//!
//! Requires Docker to be running.
//! Run with: cargo test --test sandbox_integration -- --ignored

use std::time::Duration;
use zero_cli::sandbox::{DockerSandbox, Language, SandboxConfig};

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_executes_python_successfully() {
    let sandbox = DockerSandbox::new().await.expect("Failed to create sandbox");

    let result = sandbox
        .execute(
            r#"
import sys
print("Hello from Python!")
print(f"Version: {sys.version}")
"#,
            Language::Python,
        )
        .await
        .expect("Execution failed");

    assert!(
        result.success(),
        "Expected success, got exit code {}",
        result.exit_code
    );
    assert!(result.stdout.contains("Hello from Python!"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_captures_stderr() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute(
            "import sys; sys.stderr.write('error output')",
            Language::Python,
        )
        .await
        .unwrap();

    assert!(result.stderr.contains("error output"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_enforces_timeout() {
    let config = SandboxConfig {
        timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let sandbox = DockerSandbox::with_config(config).await.unwrap();

    let result = sandbox
        .execute("import time; time.sleep(10)", Language::Python)
        .await
        .unwrap();

    assert!(result.timed_out);
    assert!(!result.success());
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_shell_execution() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute("echo 'hello' && whoami", Language::Shell)
        .await
        .unwrap();

    assert!(result.success());
    assert!(result.stdout.contains("hello"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_javascript_execution() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute("console.log('Hello from Node.js!');", Language::JavaScript)
        .await
        .unwrap();

    assert!(result.success());
    assert!(result.stdout.contains("Hello from Node.js!"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_handles_syntax_error() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute("this is not valid python syntax ???", Language::Python)
        .await
        .unwrap();

    assert!(!result.success());
    assert!(result.stderr.contains("SyntaxError"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_respects_memory_limit() {
    let config = SandboxConfig {
        memory_limit: 64 * 1024 * 1024, // 64MB
        timeout: Duration::from_secs(30),
        ..Default::default()
    };
    let sandbox = DockerSandbox::with_config(config).await.unwrap();

    // Try to allocate more memory than allowed
    let result = sandbox
        .execute(
            r#"
# Attempt to allocate ~100MB
data = bytearray(100 * 1024 * 1024)
print("Should not reach here")
"#,
            Language::Python,
        )
        .await
        .unwrap();

    // Container should be killed or error
    assert!(!result.success() || result.timed_out);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_network_disabled_by_default() {
    let sandbox = DockerSandbox::new().await.unwrap();

    // Try to make a network request
    let result = sandbox
        .execute(
            r#"
import urllib.request
try:
    urllib.request.urlopen('https://example.com', timeout=5)
    print('NETWORK_AVAILABLE')
except Exception as e:
    print(f'NETWORK_BLOCKED: {e}')
"#,
            Language::Python,
        )
        .await
        .unwrap();

    // Network should be blocked
    assert!(
        result.stdout.contains("NETWORK_BLOCKED"),
        "Network should be disabled: {}",
        result.stdout
    );
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_health_check() {
    let sandbox = DockerSandbox::new().await.unwrap();
    assert!(sandbox.health_check().await);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_multiline_output() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute(
            r#"
for i in range(5):
    print(f"Line {i}")
"#,
            Language::Python,
        )
        .await
        .unwrap();

    assert!(result.success());
    assert!(result.stdout.contains("Line 0"));
    assert!(result.stdout.contains("Line 4"));
}
