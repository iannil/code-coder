//! Evaluation Power Analysis Module.
//!
//! Analyzes a company's position in the supply chain to determine its "evaluation power"
//! (评估权). The core question: "Who is asking whom?" (谁在求谁)

mod analyzer;

pub use analyzer::{
    BargainingPower, EvaluationPowerAnalyzer, EvaluationPowerConfig, EvaluationPowerInput,
    SupplyChainPosition,
};
