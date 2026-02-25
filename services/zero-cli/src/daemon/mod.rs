mod api;

use crate::config::Config;
use crate::process::{HealthChecker, ServiceConfig, ServiceManager};
use crate::tools::ToolRegistry;
use anyhow::Result;
use chrono::Utc;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::Duration;

const STATUS_FLUSH_SECONDS: u64 = 5;
const HEALTH_CHECK_INTERVAL_SECS: u64 = 5;
const MANAGEMENT_API_PORT: u16 = 4402;

/// Shared tool registry for the daemon, accessible to all components
static TOOL_REGISTRY: std::sync::OnceLock<Arc<RwLock<ToolRegistry>>> = std::sync::OnceLock::new();

/// Get the global tool registry (initialized during daemon startup)
pub fn get_tool_registry() -> Option<&'static Arc<RwLock<ToolRegistry>>> {
    TOOL_REGISTRY.get()
}

/// Run the daemon in process orchestrator mode.
///
/// This mode spawns zero-gateway, zero-channels, and zero-workflow as separate
/// child processes and monitors them via health checks. It also provides:
/// - MCP tool registry integration
/// - Heartbeat worker for autonomous tasks
/// - State file persistence
/// - Management HTTP API on port 4402
pub async fn run_orchestrator(
    config: Config,
    host: String,
    gateway_port: u16,
    channels_port: u16,
    workflow_port: u16,
    log_dir: Option<PathBuf>,
) -> Result<()> {
    let initial_backoff = config.reliability.channel_initial_backoff_secs.max(1);
    let max_backoff = config
        .reliability
        .channel_max_backoff_secs
        .max(initial_backoff);

    crate::health::mark_component_ok("daemon");

    // ── Initialize tool registry with MCP support ──────────────
    let security = Arc::new(crate::security::SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));
    let mem: Arc<dyn crate::memory::Memory> = Arc::from(crate::memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    let vault_path = config
        .config_path
        .parent()
        .map_or_else(|| config.workspace_dir.clone(), std::path::Path::to_path_buf);

    let registry = ToolRegistry::with_native_tools(
        &security,
        mem,
        &config.browser,
        &config.codecoder,
        &config.vault,
        &vault_path,
    );

    // Connect to MCP servers if configured
    if !config.mcp.servers.is_empty() {
        match registry.connect_mcp_servers(&config.mcp).await {
            Ok(()) => {
                let mcp_count = registry.mcp_tool_count().await;
                if mcp_count > 0 {
                    println!("  🔌 MCP: {mcp_count} tools loaded from external servers");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to connect to some MCP servers: {e}");
            }
        }
    }

    // Store registry globally for other components
    let _ = TOOL_REGISTRY.set(Arc::new(RwLock::new(registry)));
    crate::health::mark_component_ok("mcp");

    // Ensure heartbeat file exists
    if config.heartbeat.enabled {
        let _ =
            crate::heartbeat::engine::HeartbeatEngine::ensure_heartbeat_file(&config.workspace_dir)
                .await;
    }

    // ── Start background workers ──────────────────────────────
    let mut handles: Vec<JoinHandle<()>> = vec![spawn_state_writer(config.clone())];

    // Start MCP supervisor if there are configured servers
    if !config.mcp.servers.is_empty() {
        let mcp_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "mcp",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = mcp_cfg.clone();
                async move { run_mcp_refresh_worker(cfg).await }
            },
        ));
    }

    // Start heartbeat worker if enabled
    if config.heartbeat.enabled {
        let heartbeat_cfg = config.clone();
        handles.push(spawn_component_supervisor(
            "heartbeat",
            initial_backoff,
            max_backoff,
            move || {
                let cfg = heartbeat_cfg.clone();
                async move { run_heartbeat_worker(cfg).await }
            },
        ));
    }

    // ── Find Rust binaries and create service manager ──────────
    let bin_dir = find_rust_bin_dir()?;
    tracing::info!("Using Rust binaries from: {}", bin_dir.display());

    // Compute log directory (default: ../.logs from working dir, i.e., project_root/.logs)
    let resolved_log_dir = log_dir.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|cwd| cwd.join("../.logs"))
            .unwrap_or_else(|_| PathBuf::from(".logs"))
    });
    tracing::info!("Service logs directory: {}", resolved_log_dir.display());

    // Create service manager
    let mut manager = ServiceManager::new(bin_dir);

    // Add zero-gateway service
    manager.add_service(ServiceConfig {
        name: "zero-gateway".into(),
        binary: "zero-gateway".into(),
        port: gateway_port,
        host: host.clone(),
        args: vec![],
        log_file: Some(resolved_log_dir.join("zero-gateway.log")),
    });

    // Add zero-channels service
    manager.add_service(ServiceConfig {
        name: "zero-channels".into(),
        binary: "zero-channels".into(),
        port: channels_port,
        host: host.clone(),
        args: vec![],
        log_file: Some(resolved_log_dir.join("zero-channels.log")),
    });

    // Add zero-workflow service
    manager.add_service(ServiceConfig {
        name: "zero-workflow".into(),
        binary: "zero-workflow".into(),
        port: workflow_port,
        host: host.clone(),
        args: vec![],
        log_file: Some(resolved_log_dir.join("zero-workflow.log")),
    });

    // Start all services
    manager.start_all()?;

    // Wrap manager for sharing between tasks
    let manager_shared = Arc::new(Mutex::new(manager));

    // ── Start management API server ─────────────────────────────
    let api_state = api::ApiState {
        manager: manager_shared.clone(),
        started_at: Utc::now(),
    };
    let api_host = host.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::serve(api_state, &api_host, MANAGEMENT_API_PORT).await {
            tracing::error!("Management API server error: {e}");
        }
    });

    // ── Run health check loop ───────────────────────────────────
    let health_checker = HealthChecker::new();
    let manager_clone = manager_shared.clone();

    let health_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS));
        loop {
            interval.tick().await;

            let mut manager = manager_clone.lock().await;
            manager.health_check_and_restart();

            // Also verify via HTTP health endpoints
            for status in manager.status() {
                if status.running {
                    let healthy = health_checker.check("127.0.0.1", status.port).await;
                    if !healthy {
                        tracing::warn!(
                            "{} process running but HTTP health check failed (port {})",
                            status.name,
                            status.port
                        );
                    }
                }
            }
        }
    });

    // ── Print startup banner ────────────────────────────────────
    let mcp_status = if config.mcp.servers.is_empty() {
        ""
    } else {
        ", mcp"
    };
    let heartbeat_status = if config.heartbeat.enabled {
        ", heartbeat"
    } else {
        ""
    };

    println!("🧠 ZeroBot daemon started (process orchestrator)");
    println!("   Management API: http://{}:{}", host, MANAGEMENT_API_PORT);
    println!("   Managed services:");
    println!("     • zero-gateway:  http://{}:{}", host, gateway_port);
    println!("     • zero-channels: http://{}:{}", host, channels_port);
    println!("     • zero-workflow: http://{}:{}", host, workflow_port);
    println!("   Components: state-writer{mcp_status}{heartbeat_status}");
    println!("   Press Ctrl+C to stop");

    // Wait for shutdown signal (Ctrl+C or SIGTERM)
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        tokio::select! {
            _ = sigterm.recv() => {
                println!("\n🛑 Received SIGTERM, shutting down...");
            }
            _ = sigint.recv() => {
                println!("\n🛑 Received SIGINT, shutting down...");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        println!("\n🛑 Shutting down...");
    }

    crate::health::mark_component_error("daemon", "shutdown requested");

    // Close MCP connections gracefully
    if let Some(registry) = get_tool_registry() {
        if let Ok(reg) = registry.try_read() {
            let _ = reg.close().await;
        }
    }

    // Stop all background workers
    for handle in &handles {
        handle.abort();
    }
    for handle in handles {
        let _ = handle.await;
    }

    // Stop health check and API server
    health_handle.abort();
    let _ = health_handle.await;
    api_handle.abort();
    let _ = api_handle.await;

    // Stop all managed services
    let mut manager = manager_shared.lock().await;
    manager.stop_all();

    println!("✅ All services stopped");

    Ok(())
}

/// Find the Rust binary directory.
fn find_rust_bin_dir() -> Result<PathBuf> {
    // Try common locations
    let candidates = [
        PathBuf::from("services/target/release"),
        PathBuf::from("services/target/debug"),
        PathBuf::from("target/release"),
        PathBuf::from("target/debug"),
    ];

    for path in &candidates {
        if path.exists() {
            // Verify at least one expected binary exists
            if path.join("zero-gateway").exists() || path.join("zero-gateway.exe").exists() {
                return Ok(path.clone());
            }
        }
    }

    // Try from CARGO_MANIFEST_DIR
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let workspace_release = PathBuf::from(manifest_dir)
            .parent()
            .map(|p| p.join("target/release"));
        if let Some(p) = workspace_release {
            if p.exists() {
                return Ok(p);
            }
        }
    }

    anyhow::bail!(
        "Could not find Rust binary directory with zero-gateway. \
         Run 'cargo build --release' in services/ first."
    )
}

pub fn state_file_path(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("daemon_state.json")
}

fn spawn_state_writer(config: Config) -> JoinHandle<()> {
    tokio::spawn(async move {
        let path = state_file_path(&config);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut interval = tokio::time::interval(Duration::from_secs(STATUS_FLUSH_SECONDS));
        loop {
            interval.tick().await;
            let mut json = crate::health::snapshot_json();
            if let Some(obj) = json.as_object_mut() {
                obj.insert(
                    "written_at".into(),
                    serde_json::json!(Utc::now().to_rfc3339()),
                );
            }
            let data = serde_json::to_vec_pretty(&json).unwrap_or_else(|_| b"{}".to_vec());
            let _ = tokio::fs::write(&path, data).await;
        }
    })
}

fn spawn_component_supervisor<F, Fut>(
    name: &'static str,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
    mut run_component: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);

        loop {
            crate::health::mark_component_ok(name);
            match run_component().await {
                Ok(()) => {
                    crate::health::mark_component_error(name, "component exited unexpectedly");
                    tracing::warn!("Daemon component '{name}' exited unexpectedly");
                }
                Err(e) => {
                    crate::health::mark_component_error(name, e.to_string());
                    tracing::error!("Daemon component '{name}' failed: {e}");
                }
            }

            crate::health::bump_component_restart(name);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

async fn run_heartbeat_worker(config: Config) -> Result<()> {
    let observer: std::sync::Arc<dyn crate::observability::Observer> =
        std::sync::Arc::from(crate::observability::create_observer(&config.observability));
    let engine = crate::heartbeat::engine::HeartbeatEngine::new(
        config.heartbeat.clone(),
        config.workspace_dir.clone(),
        observer,
    );

    let interval_mins = config.heartbeat.interval_minutes.max(5);
    let mut interval = tokio::time::interval(Duration::from_secs(u64::from(interval_mins) * 60));

    loop {
        interval.tick().await;

        let tasks = engine.collect_tasks().await?;
        if tasks.is_empty() {
            continue;
        }

        for task in tasks {
            let prompt = format!("[Heartbeat Task] {task}");
            let temp = config.default_temperature;
            if let Err(e) = crate::agent::run(config.clone(), Some(prompt), None, None, temp).await
            {
                crate::health::mark_component_error("heartbeat", e.to_string());
                tracing::warn!("Heartbeat task failed: {e}");
            } else {
                crate::health::mark_component_ok("heartbeat");
            }
        }
    }
}

/// MCP refresh worker - periodically refreshes tools from connected MCP servers
async fn run_mcp_refresh_worker(_config: Config) -> Result<()> {
    // Refresh MCP tools every 5 minutes
    let refresh_interval_secs = 300_u64;
    let mut interval = tokio::time::interval(Duration::from_secs(refresh_interval_secs));

    loop {
        interval.tick().await;

        if let Some(registry) = get_tool_registry() {
            match registry.try_write() {
                Ok(reg) => {
                    if let Err(e) = reg.refresh_mcp_tools().await {
                        tracing::warn!("Failed to refresh MCP tools: {e}");
                        crate::health::mark_component_error("mcp", e.to_string());
                    } else {
                        let count = reg.mcp_tool_count().await;
                        tracing::debug!("MCP tools refreshed: {count} tools available");
                        crate::health::mark_component_ok("mcp");
                    }
                }
                Err(_) => {
                    tracing::debug!("MCP registry busy, skipping refresh");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Config {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        config
    }

    #[test]
    fn state_file_path_uses_config_directory() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(&tmp);

        let path = state_file_path(&config);
        assert_eq!(path, tmp.path().join("daemon_state.json"));
    }

    #[tokio::test]
    async fn supervisor_marks_error_and_restart_on_failure() {
        let handle = spawn_component_supervisor("daemon-test-fail", 1, 1, || async {
            anyhow::bail!("boom")
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-fail"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("boom"));
    }

    #[tokio::test]
    async fn supervisor_marks_unexpected_exit_as_error() {
        let handle = spawn_component_supervisor("daemon-test-exit", 1, 1, || async { Ok(()) });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;

        let snapshot = crate::health::snapshot_json();
        let component = &snapshot["components"]["daemon-test-exit"];
        assert_eq!(component["status"], "error");
        assert!(component["restart_count"].as_u64().unwrap_or(0) >= 1);
        assert!(component["last_error"]
            .as_str()
            .unwrap_or("")
            .contains("component exited unexpectedly"));
    }
}
