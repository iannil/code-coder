//! Trading command handlers for Zero CLI.
//!
//! This module provides CLI commands that interact with the zero-trading
//! HTTP API for managing automated trading (PO3 + SMT divergence strategy).

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Base URL for the trading service.
const TRADING_BASE_URL: &str = "http://127.0.0.1:4434";

/// Trading CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum TradingCommands {
    /// Start the trading service
    Start {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "4434")]
        port: u16,
    },

    /// Stop the trading service
    Stop,

    /// Show trading service status
    Status,

    /// Paper trading subcommands
    Paper {
        #[command(subcommand)]
        paper_command: PaperCommands,
    },

    /// Run a backtest
    Backtest {
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,

        /// Initial capital
        #[arg(long, default_value = "10000.0")]
        capital: f64,
    },
}

/// Paper trading subcommands.
#[derive(Subcommand, Debug)]
pub enum PaperCommands {
    /// Start paper trading
    Start {
        /// Initial capital
        #[arg(long, default_value = "10000.0")]
        capital: f64,

        /// Duration (e.g., "4h", "1d", "30m")
        #[arg(long)]
        duration: Option<String>,

        /// Maximum number of positions
        #[arg(long, default_value = "5")]
        max_positions: usize,

        /// Disable notifications
        #[arg(long)]
        no_notify: bool,
    },

    /// Stop paper trading
    Stop,

    /// Show paper trading status
    Status,

    /// List recent trades
    Trades,

    /// Generate trading report
    Report,
}

/// HTTP client for zero-trading API.
pub struct TradingClient {
    base_url: String,
    client: reqwest::Client,
}

/// API response wrapper.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// Paper trading start request.
#[derive(Debug, Serialize)]
struct PaperStartRequest {
    capital: f64,
    duration_secs: Option<u64>,
    max_positions: usize,
    notify: bool,
}

/// Paper trading status response.
#[derive(Debug, Deserialize)]
pub struct PaperStatus {
    pub running: bool,
    pub capital: f64,
    pub pnl: f64,
    pub pnl_percent: f64,
    pub open_positions: usize,
    pub total_trades: usize,
    pub started_at: Option<String>,
    pub duration_remaining: Option<u64>,
}

/// Trade record.
#[derive(Debug, Deserialize)]
pub struct Trade {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub quantity: f64,
    pub pnl: Option<f64>,
    pub opened_at: String,
    pub closed_at: Option<String>,
}

/// Trading report.
#[derive(Debug, Deserialize)]
pub struct TradingReport {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
    pub start_capital: f64,
    pub end_capital: f64,
}

/// Service status response.
#[derive(Debug, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub version: String,
    pub uptime_secs: u64,
    pub paper_trading_active: bool,
}

impl TradingClient {
    /// Create a new trading client with the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Start paper trading.
    pub async fn paper_start(
        &self,
        capital: f64,
        duration_secs: Option<u64>,
        max_positions: usize,
        notify: bool,
    ) -> Result<String> {
        let url = format!("{}/api/v1/paper/start", self.base_url);

        let req = PaperStartRequest {
            capital,
            duration_secs,
            max_positions,
            notify,
        };

        let resp: ApiResponse<String> = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to connect to trading service")?
            .json()
            .await
            .context("Failed to parse response")?;

        if resp.success {
            Ok(resp.data.unwrap_or_else(|| "Paper trading started".into()))
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to start paper trading".into()))
        }
    }

    /// Stop paper trading.
    pub async fn paper_stop(&self) -> Result<String> {
        let url = format!("{}/api/v1/paper/stop", self.base_url);

        let resp: ApiResponse<String> = self
            .client
            .post(&url)
            .send()
            .await
            .context("Failed to connect to trading service")?
            .json()
            .await
            .context("Failed to parse response")?;

        if resp.success {
            Ok(resp.data.unwrap_or_else(|| "Paper trading stopped".into()))
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to stop paper trading".into()))
        }
    }

    /// Get paper trading status.
    pub async fn paper_status(&self) -> Result<PaperStatus> {
        let url = format!("{}/api/v1/paper/status", self.base_url);

        let resp: ApiResponse<PaperStatus> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to trading service")?
            .json()
            .await
            .context("Failed to parse response")?;

        if resp.success {
            resp.data.context("Missing status data in response")
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to get paper trading status".into()))
        }
    }

    /// Get recent trades.
    pub async fn paper_trades(&self) -> Result<Vec<Trade>> {
        let url = format!("{}/api/v1/paper/trades", self.base_url);

        let resp: ApiResponse<Vec<Trade>> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to trading service")?
            .json()
            .await
            .context("Failed to parse response")?;

        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to get trades".into()))
        }
    }

    /// Get trading report.
    pub async fn paper_report(&self) -> Result<TradingReport> {
        let url = format!("{}/api/v1/paper/report", self.base_url);

        let resp: ApiResponse<TradingReport> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to trading service")?
            .json()
            .await
            .context("Failed to parse response")?;

        if resp.success {
            resp.data.context("Missing report data in response")
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to get trading report".into()))
        }
    }

    /// Get service status.
    pub async fn service_status(&self) -> Result<ServiceStatus> {
        let url = format!("{}/api/v1/status", self.base_url);

        let resp: ApiResponse<ServiceStatus> = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to trading service")?
            .json()
            .await
            .context("Failed to parse response")?;

        if resp.success {
            resp.data.context("Missing status data in response")
        } else {
            anyhow::bail!(resp.error.unwrap_or_else(|| "Failed to get service status".into()))
        }
    }
}

/// Parse a duration string (e.g., "4h", "1d", "30m") to seconds.
///
/// Supported units:
/// - `s` or `sec` or `secs` - seconds
/// - `m` or `min` or `mins` - minutes
/// - `h` or `hr` or `hrs` or `hour` or `hours` - hours
/// - `d` or `day` or `days` - days
/// - `w` or `week` or `weeks` - weeks
pub fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim().to_lowercase();

    if s.is_empty() {
        anyhow::bail!("Duration string cannot be empty");
    }

    // Find the split point between digits and unit
    let digit_end = s
        .chars()
        .position(|c| !c.is_ascii_digit())
        .unwrap_or(s.len());

    if digit_end == 0 {
        anyhow::bail!("Duration must start with a number: {s}");
    }

    let (num_str, unit) = s.split_at(digit_end);
    let num: u64 = num_str
        .parse()
        .with_context(|| format!("Invalid number in duration: {num_str}"))?;

    let unit = unit.trim();
    let multiplier: u64 = match unit {
        "" | "s" | "sec" | "secs" => 1,
        "m" | "min" | "mins" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
        "d" | "day" | "days" => 86400,
        "w" | "week" | "weeks" => 604_800,
        other => anyhow::bail!("Unknown duration unit: {other}"),
    };

    num.checked_mul(multiplier)
        .context("Duration overflow")
}

/// Handle trading CLI commands.
#[allow(clippy::too_many_lines)]
pub async fn handle_command(command: TradingCommands, config: &Config) -> Result<()> {
    // Get trading service endpoint from config or use default
    let trading_endpoint = config
        .trading_host
        .as_ref()
        .map(|host| {
            format!(
                "http://{}:{}",
                host,
                config.trading_port.unwrap_or(4434)
            )
        })
        .unwrap_or_else(|| TRADING_BASE_URL.to_string());

    let client = TradingClient::new(&trading_endpoint);

    match command {
        TradingCommands::Start { host, port } => {
            println!("Starting trading service on {}:{}", host, port);
            println!();
            println!("Note: The trading service should be started via the daemon:");
            println!("  zero-cli daemon --trading-port {port}");
            println!();
            println!("Or run the standalone trading service:");
            println!("  zero-trading --host {host} --port {port}");
            Ok(())
        }

        TradingCommands::Stop => {
            println!("Stopping trading service...");
            println!();
            println!("Note: Stop the trading service via the daemon or process manager.");
            Ok(())
        }

        TradingCommands::Status => {
            let status = client.service_status().await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get service status: {}. Is the trading service running at {}?",
                    e,
                    trading_endpoint
                )
            })?;

            println!("Trading Service Status");
            println!("======================");
            println!("Running:         {}", if status.running { "Yes" } else { "No" });
            println!("Version:         {}", status.version);
            println!("Uptime:          {} seconds", status.uptime_secs);
            println!(
                "Paper Trading:   {}",
                if status.paper_trading_active { "Active" } else { "Inactive" }
            );
            Ok(())
        }

        TradingCommands::Paper { paper_command } => {
            handle_paper_command(paper_command, &client, &trading_endpoint).await
        }

        TradingCommands::Backtest { start, end, capital } => {
            println!("Running backtest...");
            println!();
            println!("Period:   {} to {}", start, end);
            println!("Capital:  ${:.2}", capital);
            println!();
            println!("Note: Backtest functionality is not yet implemented.");
            println!("This will run the PO3+SMT strategy against historical data.");
            Ok(())
        }
    }
}

/// Handle paper trading subcommands.
async fn handle_paper_command(
    command: PaperCommands,
    client: &TradingClient,
    endpoint: &str,
) -> Result<()> {
    match command {
        PaperCommands::Start {
            capital,
            duration,
            max_positions,
            no_notify,
        } => {
            let duration_secs = duration
                .as_ref()
                .map(|d| parse_duration(d))
                .transpose()?;

            let result = client
                .paper_start(capital, duration_secs, max_positions, !no_notify)
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to start paper trading: {}. Is the trading service running at {}?",
                        e,
                        endpoint
                    )
                })?;

            println!("Paper Trading Started");
            println!("=====================");
            println!("Capital:        ${:.2}", capital);
            println!("Max Positions:  {}", max_positions);
            println!(
                "Notifications:  {}",
                if no_notify { "Disabled" } else { "Enabled" }
            );
            if let Some(secs) = duration_secs {
                println!("Duration:       {} seconds", secs);
            } else {
                println!("Duration:       Unlimited");
            }
            println!();
            println!("{result}");
            Ok(())
        }

        PaperCommands::Stop => {
            let result = client.paper_stop().await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to stop paper trading: {}. Is the trading service running at {}?",
                    e,
                    endpoint
                )
            })?;

            println!("Paper trading stopped.");
            println!("{result}");
            Ok(())
        }

        PaperCommands::Status => {
            let status = client.paper_status().await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get paper trading status: {}. Is the trading service running at {}?",
                    e,
                    endpoint
                )
            })?;

            println!("Paper Trading Status");
            println!("====================");
            println!("Running:         {}", if status.running { "Yes" } else { "No" });
            println!("Capital:         ${:.2}", status.capital);
            println!(
                "P&L:             ${:.2} ({:+.2}%)",
                status.pnl, status.pnl_percent
            );
            println!("Open Positions:  {}", status.open_positions);
            println!("Total Trades:    {}", status.total_trades);
            if let Some(ref started) = status.started_at {
                println!("Started At:      {started}");
            }
            if let Some(remaining) = status.duration_remaining {
                println!("Time Remaining:  {} seconds", remaining);
            }
            Ok(())
        }

        PaperCommands::Trades => {
            let trades = client.paper_trades().await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get trades: {}. Is the trading service running at {}?",
                    e,
                    endpoint
                )
            })?;

            if trades.is_empty() {
                println!("No trades yet.");
                return Ok(());
            }

            println!("Recent Trades ({}):", trades.len());
            println!("==================");
            for trade in &trades {
                let status = if trade.exit_price.is_some() {
                    "CLOSED"
                } else {
                    "OPEN"
                };
                let pnl_str = trade
                    .pnl
                    .map(|p| format!("${:.2}", p))
                    .unwrap_or_else(|| "-".into());

                println!(
                    "  {} {} {} @ {:.4} | {} | P&L: {}",
                    trade.id, trade.side, trade.symbol, trade.entry_price, status, pnl_str
                );
            }
            Ok(())
        }

        PaperCommands::Report => {
            let report = client.paper_report().await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get report: {}. Is the trading service running at {}?",
                    e,
                    endpoint
                )
            })?;

            println!("Trading Report");
            println!("==============");
            println!("Total Trades:    {}", report.total_trades);
            println!(
                "Win/Loss:        {} / {}",
                report.winning_trades, report.losing_trades
            );
            println!("Win Rate:        {:.1}%", report.win_rate * 100.0);
            println!("Total P&L:       ${:.2}", report.total_pnl);
            println!("Max Drawdown:    {:.1}%", report.max_drawdown * 100.0);
            println!("Sharpe Ratio:    {:.2}", report.sharpe_ratio);
            println!();
            println!("Start Capital:   ${:.2}", report.start_capital);
            println!("End Capital:     ${:.2}", report.end_capital);
            println!(
                "Return:          {:+.1}%",
                ((report.end_capital - report.start_capital) / report.start_capital) * 100.0
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), 30);
        assert_eq!(parse_duration("60sec").unwrap(), 60);
        assert_eq!(parse_duration("120secs").unwrap(), 120);
        assert_eq!(parse_duration("45").unwrap(), 45); // default to seconds
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("1m").unwrap(), 60);
        assert_eq!(parse_duration("30min").unwrap(), 1800);
        assert_eq!(parse_duration("5mins").unwrap(), 300);
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("1h").unwrap(), 3600);
        assert_eq!(parse_duration("4hr").unwrap(), 14400);
        assert_eq!(parse_duration("2hrs").unwrap(), 7200);
        assert_eq!(parse_duration("1hour").unwrap(), 3600);
        assert_eq!(parse_duration("24hours").unwrap(), 86400);
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("1d").unwrap(), 86400);
        assert_eq!(parse_duration("7day").unwrap(), 604800);
        assert_eq!(parse_duration("30days").unwrap(), 2592000);
    }

    #[test]
    fn test_parse_duration_weeks() {
        assert_eq!(parse_duration("1w").unwrap(), 604800);
        assert_eq!(parse_duration("2week").unwrap(), 1209600);
        assert_eq!(parse_duration("4weeks").unwrap(), 2419200);
    }

    #[test]
    fn test_parse_duration_whitespace() {
        assert_eq!(parse_duration("  30m  ").unwrap(), 1800);
        assert_eq!(parse_duration("4 h").unwrap(), 14400);
    }

    #[test]
    fn test_parse_duration_case_insensitive() {
        assert_eq!(parse_duration("1H").unwrap(), 3600);
        assert_eq!(parse_duration("30M").unwrap(), 1800);
        assert_eq!(parse_duration("1D").unwrap(), 86400);
    }

    #[test]
    fn test_parse_duration_empty() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("   ").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        assert!(parse_duration("30x").is_err());
        assert!(parse_duration("1year").is_err());
    }

    #[test]
    fn test_parse_duration_no_number() {
        assert!(parse_duration("hours").is_err());
        assert!(parse_duration("d").is_err());
    }

    #[test]
    fn test_trading_client_creation() {
        let client = TradingClient::new("http://localhost:4434");
        assert_eq!(client.base_url, "http://localhost:4434");
    }

    #[test]
    fn test_trading_client_trailing_slash() {
        let client = TradingClient::new("http://localhost:4434/");
        assert_eq!(client.base_url, "http://localhost:4434");
    }
}
