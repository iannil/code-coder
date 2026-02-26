//! Task category and type definitions for the scheduler.
//!
//! Defines the categorization of automated tasks into preparation (24/7)
//! and execution (trading hours only) types.

use serde::{Deserialize, Serialize};

/// Task category determining when a task should run.
///
/// - **Preparation**: Tasks that can run 24/7, primarily data loading and
///   precomputation that prepares the system for trading hours.
/// - **Execution**: Tasks that only run during active trading hours,
///   involving signal detection and order execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    /// Preparation tasks (24/7 operation)
    Preparation,
    /// Execution tasks (trading hours only)
    Execution,
}

/// Specific task types with their scheduling category.
///
/// Each task is categorized as either Preparation or Execution, which
/// determines when it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    // ===== Preparation Tasks (24/7) =====

    /// Preload historical market data into cache
    DataPreload,

    /// Precompute technical indicator parameters
    ParameterPrecompute,

    /// Background macro economic analysis updates
    MacroAnalysis,

    // ===== Execution Tasks (Trading Hours) =====

    /// Generate trading signals based on strategy
    SignalGeneration,

    /// Execute buy/sell orders
    OrderExecution,

    /// Monitor price levels for stop-loss/take-profit
    PriceMonitoring,

    /// Risk management calculations
    RiskManagement,
}

impl TaskType {
    /// Get the category for this task type.
    pub fn category(&self) -> TaskCategory {
        match self {
            TaskType::DataPreload
            | TaskType::ParameterPrecompute
            | TaskType::MacroAnalysis => TaskCategory::Preparation,

            TaskType::SignalGeneration
            | TaskType::OrderExecution
            | TaskType::PriceMonitoring
            | TaskType::RiskManagement => TaskCategory::Execution,
        }
    }

    /// Check if this task runs 24/7 (preparation) or trading hours only (execution).
    pub fn is_24_7(&self) -> bool {
        self.category() == TaskCategory::Preparation
    }

    /// Get a human-readable name for the task.
    pub fn name(&self) -> &'static str {
        match self {
            TaskType::DataPreload => "Data Preload",
            TaskType::ParameterPrecompute => "Parameter Precompute",
            TaskType::MacroAnalysis => "Macro Analysis",
            TaskType::SignalGeneration => "Signal Generation",
            TaskType::OrderExecution => "Order Execution",
            TaskType::PriceMonitoring => "Price Monitoring",
            TaskType::RiskManagement => "Risk Management",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_category() {
        assert_eq!(TaskType::DataPreload.category(), TaskCategory::Preparation);
        assert_eq!(TaskType::SignalGeneration.category(), TaskCategory::Execution);
    }

    #[test]
    fn test_is_24_7() {
        assert!(TaskType::DataPreload.is_24_7());
        assert!(TaskType::ParameterPrecompute.is_24_7());
        assert!(TaskType::MacroAnalysis.is_24_7());

        assert!(!TaskType::SignalGeneration.is_24_7());
        assert!(!TaskType::OrderExecution.is_24_7());
        assert!(!TaskType::PriceMonitoring.is_24_7());
        assert!(!TaskType::RiskManagement.is_24_7());
    }

    #[test]
    fn test_task_names() {
        assert_eq!(TaskType::DataPreload.name(), "Data Preload");
        assert_eq!(TaskType::SignalGeneration.name(), "Signal Generation");
    }

    #[test]
    fn test_serialization() {
        let task = TaskType::DataPreload;
        let json = serde_json::to_string(&task).unwrap();
        let parsed: TaskType = serde_json::from_str(&json).unwrap();
        assert_eq!(task, parsed);
    }
}
