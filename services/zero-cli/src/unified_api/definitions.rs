//! Agent Definitions Module
//!
//! This module provides native Rust definitions for all 29 agents.
//! Agent definitions include metadata, permissions, observer capabilities,
//! and auto-approve configuration.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        Agent Definition Storage                              │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │   Static Definitions:    BUILTIN_AGENTS (29 agents)                         │
//! │   Runtime Customization: Custom agents via API                              │
//! │   Prompt Files:          packages/ccode/src/agent/prompt/*.txt              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # API Endpoints
//!
//! - GET  /api/v1/definitions/agents          - List all agents
//! - GET  /api/v1/definitions/agents/:name    - Get agent details + prompt
//! - PUT  /api/v1/definitions/agents/:name    - Update agent (hot-reload)
//! - POST /api/v1/definitions/agents          - Create custom agent

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Core Types
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher types in the Observer Network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatcherType {
    /// CodeWatch - code repository scanning, git changes, build status
    Code,
    /// WorldWatch - market data, news, API changes
    World,
    /// SelfWatch - agent behavior, decision logs, error patterns
    #[serde(rename = "self")]
    Self_,
    /// MetaWatch - observation quality, system health, blind spots
    Meta,
}

/// Observer capability for agents to participate in the Observer Network
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserverCapability {
    /// Which watcher types this agent can contribute to
    #[serde(default)]
    pub can_watch: Vec<WatcherType>,
    /// Whether this agent's observations contribute to consensus formation
    #[serde(default = "default_true")]
    pub contribute_to_consensus: bool,
    /// Whether this agent reports to MetaWatch for quality monitoring
    #[serde(default = "default_true")]
    pub report_to_meta: bool,
}

/// Auto-approve configuration for safe tool execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoApproveConfig {
    /// Whether auto-approve is enabled
    #[serde(default)]
    pub enabled: bool,
    /// List of tools that can be auto-approved
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Risk threshold: "safe", "low", "medium", "high"
    #[serde(default = "default_risk_threshold")]
    pub risk_threshold: String,
}

/// Agent mode defining how the agent can be used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Can only be invoked as a subagent by other agents
    Subagent,
    /// Can be used as the primary agent (user-facing)
    Primary,
    /// Can be used in any context
    All,
}

impl Default for AgentMode {
    fn default() -> Self {
        Self::Subagent
    }
}

/// Model configuration override
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    pub provider_id: String,
    pub model_id: String,
}

/// Options for agent execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentOptions {
    /// Maximum output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
    /// Thinking mode configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

/// Thinking mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThinkingConfig {
    Enabled { budget_tokens: usize },
    Disabled,
}

/// Complete agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    /// Agent identifier (unique)
    pub name: String,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent mode (subagent, primary, all)
    pub mode: AgentMode,
    /// Whether this is a built-in native agent
    #[serde(default)]
    pub native: bool,
    /// Whether this agent is hidden from listings
    #[serde(default)]
    pub hidden: bool,
    /// Temperature for generation (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p for generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Display color in TUI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Model configuration override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelConfig>,
    /// Maximum execution steps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<usize>,
    /// Agent options (output tokens, thinking, etc.)
    #[serde(default)]
    pub options: AgentOptions,
    /// Auto-approve configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_approve: Option<AutoApproveConfig>,
    /// Observer capability for Observer Network participation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_capability: Option<ObserverCapability>,
}

fn default_true() -> bool {
    true
}

fn default_risk_threshold() -> String {
    "safe".to_string()
}

// ══════════════════════════════════════════════════════════════════════════════
// Built-in Agent Definitions
// ══════════════════════════════════════════════════════════════════════════════

/// Get all built-in agent definitions
pub fn builtin_agents() -> HashMap<String, AgentDefinition> {
    let mut agents = HashMap::new();

    // ─────────────────────────────────────────────────────────────────────────
    // Primary Mode Agents (4)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "build".to_string(),
        AgentDefinition {
            name: "build".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "plan".to_string(),
        AgentDefinition {
            name: "plan".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions {
                max_output_tokens: Some(128_000),
                thinking: None,
            },
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "writer".to_string(),
        AgentDefinition {
            name: "writer".to_string(),
            description: Some(
                "Specialized agent for writing long-form content (20k+ words). \
                 Handles outline generation, chapter-by-chapter writing, and style consistency."
                    .to_string(),
            ),
            mode: AgentMode::Primary,
            native: true,
            hidden: false,
            temperature: Some(0.7),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions {
                max_output_tokens: Some(128_000),
                thinking: Some(ThinkingConfig::Disabled),
            },
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "autonomous".to_string(),
        AgentDefinition {
            name: "autonomous".to_string(),
            description: Some(
                "自主模式 - 完全自主的执行代理，使用CLOSE决策框架，遵循祝融说哲学，\
                 可自主规划、决策、执行TDD循环、自我纠错"
                    .to_string(),
            ),
            mode: AgentMode::Primary,
            native: true,
            hidden: false,
            temperature: Some(0.6),
            top_p: None,
            color: Some("magenta".to_string()),
            model: None,
            steps: None,
            options: AgentOptions {
                max_output_tokens: Some(128_000),
                thinking: Some(ThinkingConfig::Disabled),
            },
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Hidden System Agents (3)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "compaction".to_string(),
        AgentDefinition {
            name: "compaction".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: true,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "title".to_string(),
        AgentDefinition {
            name: "title".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: true,
            temperature: Some(0.5),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "summary".to_string(),
        AgentDefinition {
            name: "summary".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: true,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Subagent Mode - Engineering Quality (6)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "general".to_string(),
        AgentDefinition {
            name: "general".to_string(),
            description: Some(
                "General-purpose agent for researching complex questions and executing \
                 multi-step tasks. Use this agent to execute multiple units of work in parallel."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: Some(AutoApproveConfig {
                enabled: true,
                allowed_tools: vec![
                    "Read".to_string(),
                    "Glob".to_string(),
                    "Grep".to_string(),
                    "LS".to_string(),
                ],
                risk_threshold: "safe".to_string(),
            }),
            observer_capability: None,
        },
    );

    agents.insert(
        "explore".to_string(),
        AgentDefinition {
            name: "explore".to_string(),
            description: Some(
                "Fast agent specialized for exploring codebases. Use this when you need to \
                 quickly find files by patterns (eg. \"src/components/**/*.tsx\"), search \
                 code for keywords (eg. \"API endpoints\"), or answer questions about the \
                 codebase (eg. \"how do API endpoints work?\")."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: Some(AutoApproveConfig {
                enabled: true,
                allowed_tools: vec![
                    "Read".to_string(),
                    "Glob".to_string(),
                    "Grep".to_string(),
                    "LS".to_string(),
                    "WebFetch".to_string(),
                    "WebSearch".to_string(),
                ],
                risk_threshold: "low".to_string(),
            }),
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Code],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "code-reviewer".to_string(),
        AgentDefinition {
            name: "code-reviewer".to_string(),
            description: Some(
                "Performs comprehensive code quality reviews with specific, actionable feedback"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "security-reviewer".to_string(),
        AgentDefinition {
            name: "security-reviewer".to_string(),
            description: Some(
                "Analyzes code for security vulnerabilities and best practices".to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "tdd-guide".to_string(),
        AgentDefinition {
            name: "tdd-guide".to_string(),
            description: Some(
                "Enforces test-driven development methodology throughout the implementation"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "architect".to_string(),
        AgentDefinition {
            name: "architect".to_string(),
            description: Some(
                "Designs system architecture, defines interfaces, and establishes patterns"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: None,
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Code],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Subagent Mode - Content Creation (3)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "expander".to_string(),
        AgentDefinition {
            name: "expander".to_string(),
            description: Some(
                "Unified content expansion specialist supporting fiction, non-fiction, \
                 and general content. Auto-detects domain or accepts explicit \
                 [DOMAIN:fiction|nonfiction] tag."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.7),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions {
                max_output_tokens: Some(128_000),
                thinking: Some(ThinkingConfig::Disabled),
            },
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "proofreader".to_string(),
        AgentDefinition {
            name: "proofreader".to_string(),
            description: Some(
                "Specialized agent for proofreading long-form text content. Checks grammar, \
                 spelling, punctuation, style, terminology, flow, readability, and structure."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.3),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions {
                max_output_tokens: Some(128_000),
                thinking: Some(ThinkingConfig::Disabled),
            },
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "verifier".to_string(),
        AgentDefinition {
            name: "verifier".to_string(),
            description: Some(
                "Verification agent for comprehensive validation. Performs build check, \
                 type check, lint check, test suite execution, console.log audit, \
                 git status analysis, and coverage analysis."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.1),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Subagent Mode - Reverse Engineering (2)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "code-reverse".to_string(),
        AgentDefinition {
            name: "code-reverse".to_string(),
            description: Some(
                "Website reverse engineering agent for pixel-perfect recreation planning. \
                 Analyzes websites, identifies technology stacks, extracts design systems, \
                 and generates comprehensive development plans."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.3),
            top_p: None,
            color: Some("cyan".to_string()),
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "jar-code-reverse".to_string(),
        AgentDefinition {
            name: "jar-code-reverse".to_string(),
            description: Some(
                "JAR reverse engineering agent for Java source code reconstruction. \
                 Analyzes JAR files, identifies Java frameworks and libraries, \
                 extracts class structure, and generates comprehensive development plans."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.3),
            top_p: None,
            color: Some("magenta".to_string()),
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Subagent Mode - ZhuRong (祝融说) Series (8)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "observer".to_string(),
        AgentDefinition {
            name: "observer".to_string(),
            description: Some(
                "基于'祝融说'观察者理论分析问题，用可能性基底、观察收敛、观察共识等\
                 核心概念重新诠释现象，揭示隐藏的可能性空间"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.7),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Meta],
                contribute_to_consensus: true,
                report_to_meta: false, // MetaWatch doesn't report to itself
            }),
        },
    );

    agents.insert(
        "decision".to_string(),
        AgentDefinition {
            name: "decision".to_string(),
            description: Some(
                "基于可持续决策理论的决策智慧师，使用CLOSE五维评估框架分析选择，\
                 帮助保持选择权和可用余量"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.6),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "macro".to_string(),
        AgentDefinition {
            name: "macro".to_string(),
            description: Some(
                "宏观经济分析师，基于18章课程体系解读GDP、工业、投资、消费、贸易、\
                 货币政策等数据，构建分析框架"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.5),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "trader".to_string(),
        AgentDefinition {
            name: "trader".to_string(),
            description: Some(
                "超短线交易指南，提供情绪周期、模式识别、仓位管理等技术分析框架\
                 （仅供教育参考，不构成投资建议）"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.5),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "picker".to_string(),
        AgentDefinition {
            name: "picker".to_string(),
            description: Some(
                "选品专家，基于'爆品之眼'方法论，使用七宗罪选品法和数据验证框架\
                 识别市场机会和爆款潜力"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.6),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    agents.insert(
        "miniproduct".to_string(),
        AgentDefinition {
            name: "miniproduct".to_string(),
            description: Some(
                "极小产品教练，指导独立开发者从0到1构建可盈利软件产品，涵盖需求验证、\
                 AI辅助开发、变现和退出策略"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.6),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "ai-engineer".to_string(),
        AgentDefinition {
            name: "ai-engineer".to_string(),
            description: Some(
                "AI工程师导师，基于实战课程体系，从Python基础到LLM应用开发、\
                 RAG系统构建、微调和性能优化"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.5),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "value-analyst".to_string(),
        AgentDefinition {
            name: "value-analyst".to_string(),
            description: Some(
                "价值分析师，基于《价值逻辑》的'观察者建构论'框架，分析国家共识、\
                 商业评估权和财务硬实在，识别核心资产"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.5),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Subagent Mode - Product & Feasibility (2)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "prd-generator".to_string(),
        AgentDefinition {
            name: "prd-generator".to_string(),
            description: Some(
                "产品需求文档(PRD)生成专家，将会议纪要或需求讨论转化为结构化PRD，\
                 包含用户分析、功能需求、交互设计、技术方案和开发计划"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.5),
            top_p: None,
            color: Some("blue".to_string()),
            model: None,
            steps: None,
            options: AgentOptions {
                max_output_tokens: Some(64_000),
                thinking: None,
            },
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents.insert(
        "feasibility-assess".to_string(),
        AgentDefinition {
            name: "feasibility-assess".to_string(),
            description: Some(
                "技术可行性评估专家，基于代码库语义图分析需求复杂度、现有能力、\
                 变更清单、依赖关系和风险，输出结构化JSON评估报告"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.3),
            top_p: None,
            color: Some("yellow".to_string()),
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: Some(ObserverCapability {
                can_watch: vec![WatcherType::Code],
                contribute_to_consensus: true,
                report_to_meta: true,
            }),
        },
    );

    // ─────────────────────────────────────────────────────────────────────────
    // Subagent Mode - Other (1)
    // ─────────────────────────────────────────────────────────────────────────

    agents.insert(
        "synton-assistant".to_string(),
        AgentDefinition {
            name: "synton-assistant".to_string(),
            description: Some(
                "SYNTON-DB助手，帮助理解和使用专为LLM设计的记忆数据库，\
                 包括张量图存储、PaQL查询、Graph-RAG检索"
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            hidden: false,
            temperature: Some(0.5),
            top_p: None,
            color: None,
            model: None,
            steps: None,
            options: AgentOptions::default(),
            auto_approve: None,
            observer_capability: None,
        },
    );

    agents
}

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct AgentDefinitionListResponse {
    pub success: bool,
    pub agents: Vec<AgentDefinitionInfo>,
    pub total: usize,
    pub builtin_count: usize,
    pub custom_count: usize,
}

#[derive(Debug, Serialize)]
pub struct AgentDefinitionInfo {
    pub name: String,
    pub description: Option<String>,
    pub mode: AgentMode,
    pub native: bool,
    pub hidden: bool,
    pub temperature: Option<f64>,
    pub color: Option<String>,
    pub has_observer_capability: bool,
    pub has_auto_approve: bool,
}

impl From<&AgentDefinition> for AgentDefinitionInfo {
    fn from(def: &AgentDefinition) -> Self {
        Self {
            name: def.name.clone(),
            description: def.description.clone(),
            mode: def.mode,
            native: def.native,
            hidden: def.hidden,
            temperature: def.temperature,
            color: def.color.clone(),
            has_observer_capability: def.observer_capability.is_some(),
            has_auto_approve: def.auto_approve.is_some(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinitionDetailResponse {
    pub success: bool,
    pub agent: AgentDefinition,
    /// Prompt content (loaded from file)
    pub prompt: Option<String>,
    /// Prompt file modification time
    pub prompt_modified_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentDefinitionRequest {
    pub description: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub color: Option<String>,
    pub options: Option<AgentOptions>,
    pub auto_approve: Option<AutoApproveConfig>,
    pub observer_capability: Option<ObserverCapability>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentDefinitionRequest {
    pub name: String,
    pub description: Option<String>,
    pub mode: Option<AgentMode>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub color: Option<String>,
    pub options: Option<AgentOptions>,
    pub auto_approve: Option<AutoApproveConfig>,
    pub observer_capability: Option<ObserverCapability>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/definitions/agents - List all agent definitions
pub async fn list_agent_definitions(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    // Load builtin agents
    let builtins = builtin_agents();
    let builtin_count = builtins.len();

    // Get any custom agents from state
    let custom_agents = state.custom_agents.read().await;
    let custom_count = custom_agents.len();

    // Merge and convert to response
    let mut agents: Vec<AgentDefinitionInfo> = builtins
        .values()
        .filter(|a| !a.hidden)
        .map(AgentDefinitionInfo::from)
        .collect();

    agents.extend(
        custom_agents
            .values()
            .filter(|a| !a.hidden)
            .map(AgentDefinitionInfo::from),
    );

    // Sort by name
    agents.sort_by(|a, b| a.name.cmp(&b.name));

    let total = agents.len();

    Json(AgentDefinitionListResponse {
        success: true,
        agents,
        total,
        builtin_count,
        custom_count,
    })
}

/// GET /api/v1/definitions/agents/:name - Get agent definition details
pub async fn get_agent_definition(
    State(state): State<Arc<UnifiedApiState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Try builtin first
    let builtins = builtin_agents();
    let mut agent = builtins.get(&name).cloned();

    // Then try custom agents
    if agent.is_none() {
        let custom_agents = state.custom_agents.read().await;
        agent = custom_agents.get(&name).cloned();
    }

    match agent {
        Some(def) => {
            // Load prompt from file
            let prompt_path = state.prompts_dir.join(format!("{}.txt", name));
            let (prompt, prompt_modified_at) = if prompt_path.exists() {
                let content = std::fs::read_to_string(&prompt_path).ok();
                let modified_at = std::fs::metadata(&prompt_path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
                (content, modified_at)
            } else {
                (None, None)
            };

            Json(AgentDefinitionDetailResponse {
                success: true,
                agent: def,
                prompt,
                prompt_modified_at,
            })
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Agent definition not found: {}", name),
            }),
        )
            .into_response(),
    }
}

/// PUT /api/v1/definitions/agents/:name - Update agent definition (hot-reload)
pub async fn update_agent_definition(
    State(state): State<Arc<UnifiedApiState>>,
    Path(name): Path<String>,
    Json(request): Json<UpdateAgentDefinitionRequest>,
) -> impl IntoResponse {
    let builtins = builtin_agents();

    // Check if agent exists (builtin or custom)
    let mut agent = builtins.get(&name).cloned();

    {
        let custom_agents = state.custom_agents.read().await;
        if let Some(custom) = custom_agents.get(&name) {
            agent = Some(custom.clone());
        }
    }

    match agent {
        Some(mut def) => {
            // Apply updates
            if let Some(desc) = request.description {
                def.description = Some(desc);
            }
            if let Some(temp) = request.temperature {
                def.temperature = Some(temp);
            }
            if let Some(top_p) = request.top_p {
                def.top_p = Some(top_p);
            }
            if let Some(color) = request.color {
                def.color = Some(color);
            }
            if let Some(options) = request.options {
                def.options = options;
            }
            if let Some(auto_approve) = request.auto_approve {
                def.auto_approve = Some(auto_approve);
            }
            if let Some(observer_cap) = request.observer_capability {
                def.observer_capability = Some(observer_cap);
            }

            // Store as custom (overrides builtin)
            {
                let mut custom_agents = state.custom_agents.write().await;
                custom_agents.insert(name.clone(), def.clone());
            }

            // Reload agent metadata in the main agents cache
            if let Err(e) = state.reload_agent(&name).await {
                tracing::warn!("Failed to reload agent after update: {}", e);
            }

            Json(AgentDefinitionDetailResponse {
                success: true,
                agent: def,
                prompt: None,
                prompt_modified_at: None,
            })
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Agent definition not found: {}", name),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/definitions/agents - Create custom agent definition
pub async fn create_agent_definition(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<CreateAgentDefinitionRequest>,
) -> impl IntoResponse {
    let name = request.name.clone();

    // Check if name is already taken
    let builtins = builtin_agents();
    if builtins.contains_key(&name) {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                success: false,
                error: format!("Agent '{}' already exists as a builtin agent", name),
            }),
        )
            .into_response();
    }

    {
        let custom_agents = state.custom_agents.read().await;
        if custom_agents.contains_key(&name) {
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Agent '{}' already exists", name),
                }),
            )
                .into_response();
        }
    }

    // Create new agent definition
    let def = AgentDefinition {
        name: name.clone(),
        description: request.description,
        mode: request.mode.unwrap_or(AgentMode::Subagent),
        native: false,
        hidden: false,
        temperature: request.temperature,
        top_p: request.top_p,
        color: request.color,
        model: None,
        steps: None,
        options: request.options.unwrap_or_default(),
        auto_approve: request.auto_approve,
        observer_capability: request.observer_capability,
    };

    // Store in custom agents
    {
        let mut custom_agents = state.custom_agents.write().await;
        custom_agents.insert(name.clone(), def.clone());
    }

    (
        StatusCode::CREATED,
        Json(AgentDefinitionDetailResponse {
            success: true,
            agent: def,
            prompt: None,
            prompt_modified_at: None,
        }),
    )
        .into_response()
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_agents_count() {
        let agents = builtin_agents();
        // 4 primary + 3 hidden + 6 engineering + 3 content + 2 reverse + 8 zhurong + 2 product + 1 other = 29
        assert_eq!(agents.len(), 29);
    }

    #[test]
    fn test_builtin_agents_unique_names() {
        let agents = builtin_agents();
        let names: std::collections::HashSet<_> = agents.keys().collect();
        assert_eq!(names.len(), agents.len());
    }

    #[test]
    fn test_primary_agents() {
        let agents = builtin_agents();
        let primary_count = agents
            .values()
            .filter(|a| matches!(a.mode, AgentMode::Primary) && !a.hidden)
            .count();
        assert_eq!(primary_count, 4); // build, plan, writer, autonomous
    }

    #[test]
    fn test_hidden_agents() {
        let agents = builtin_agents();
        let hidden_count = agents.values().filter(|a| a.hidden).count();
        assert_eq!(hidden_count, 3); // compaction, title, summary
    }

    #[test]
    fn test_observer_capable_agents() {
        let agents = builtin_agents();
        let observer_count = agents
            .values()
            .filter(|a| a.observer_capability.is_some())
            .count();
        // explore, code-reviewer, security-reviewer, tdd-guide, architect, verifier,
        // observer, decision, macro, trader, picker, value-analyst, autonomous, feasibility-assess
        assert!(observer_count >= 14);
    }

    #[test]
    fn test_auto_approve_agents() {
        let agents = builtin_agents();
        let auto_approve_count = agents
            .values()
            .filter(|a| a.auto_approve.is_some())
            .count();
        assert_eq!(auto_approve_count, 2); // general, explore
    }

    #[test]
    fn test_watcher_type_serialization() {
        assert_eq!(
            serde_json::to_string(&WatcherType::Code).unwrap(),
            "\"code\""
        );
        assert_eq!(
            serde_json::to_string(&WatcherType::World).unwrap(),
            "\"world\""
        );
        assert_eq!(
            serde_json::to_string(&WatcherType::Self_).unwrap(),
            "\"self\""
        );
        assert_eq!(
            serde_json::to_string(&WatcherType::Meta).unwrap(),
            "\"meta\""
        );
    }

    #[test]
    fn test_agent_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&AgentMode::Primary).unwrap(),
            "\"primary\""
        );
        assert_eq!(
            serde_json::to_string(&AgentMode::Subagent).unwrap(),
            "\"subagent\""
        );
        assert_eq!(
            serde_json::to_string(&AgentMode::All).unwrap(),
            "\"all\""
        );
    }
}
