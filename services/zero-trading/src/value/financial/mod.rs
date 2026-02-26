//! Financial verification module.
//!
//! This module provides tools for analyzing the financial health of companies
//! using the "Printing Machine" (印钞机) methodology.
//!
//! # Components
//!
//! - **Cash Flow DNA Analyzer**: Classifies companies by their cash flow patterns
//! - **Financial Verifier**: Comprehensive checklist for quality verification

pub mod cash_flow_dna;
pub mod printing_machine_checklist;

pub use cash_flow_dna::CashFlowAnalyzer;
pub use printing_machine_checklist::{FinancialVerifier, VerifierConfig};
