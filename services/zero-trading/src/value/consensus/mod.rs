//! National Consensus Analyzer (国家共识分析器).
//!
//! This module implements the first layer of the "Observer Constructionism" framework:
//! identifying and tracking national policy directions to determine "hard consensus" themes.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    国家共识分析流程                               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  输入：政策文本（政府工作报告、中央经济工作会议公报等）            │
//! │    ↓                                                            │
//! │  LLM 分析：提取安全主题、追踪政策变化                            │
//! │    ↓                                                            │
//! │  输出：ConsensusSignal（主题、强度、关键词、政策来源）           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Features
//!
//! - **Safety Theme Detection**: Identifies national security priorities
//!   (energy, food, financial, industrial, technology, military)
//! - **Policy Change Tracking**: Detects new formulations, ordering changes,
//!   and adjective changes in policy language
//! - **LLM Integration**: Uses CodeCoder API for deep policy text analysis

mod analyzer;

pub use analyzer::{
    ConsensusAnalyzer, ConsensusAnalyzerConfig, ConsensusAnalysis, PolicyChangeType,
    PolicyDocument, ThemeStrength,
};
