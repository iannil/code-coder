//! Test module for exporting TypeScript bindings.
//!
//! Run with:
//! ```bash
//! cargo test --package zero-common --features ts-bindings,hitl-client export_bindings --release
//! ```
//!
//! Or use the script:
//! ```bash
//! ./script/generate-ts-bindings.sh
//! ```

#[cfg(feature = "ts-bindings")]
mod bindings {
    use ts_rs::{Config, TS};

    // Guardrails types
    use zero_common::guardrails::{
        Action, ActionCategory, ApprovalRequest as GuardrailsApprovalRequest,
        ApprovalStatus as GuardrailsApprovalStatus, Decision,
        RiskLevel as GuardrailsRiskLevel,
    };

    // HitL types (requires hitl-client feature)
    #[cfg(feature = "hitl-client")]
    use zero_common::hitl_client::{
        ApprovalRequest as HitLApprovalRequest, ApprovalResponse,
        ApprovalStatus as HitLApprovalStatus, ApprovalType, CreateApprovalRequest,
        RiskLevel as HitLRiskLevel,
    };

    // Event types
    use zero_common::events::{
        AgentInfoData, AgentSwitchData, ConfirmationData, DebugInfoData, HeartbeatData,
        OutputData, ProgressData, SkillUseData, StreamEvent, TaskCompletedData,
        TaskCreatedData, TaskEvent, TaskFailedData, TaskStartedData, TaskState, TaskStatus,
        TaskUsage, ThoughtData, ToolUseData,
    };

    #[test]
    fn export_all_bindings() {
        let config = Config::default();

        // Export Guardrails types
        GuardrailsRiskLevel::export_all(&config).expect("Failed to export GuardrailsRiskLevel");
        ActionCategory::export_all(&config).expect("Failed to export ActionCategory");
        Action::export_all(&config).expect("Failed to export Action");
        Decision::export_all(&config).expect("Failed to export Decision");
        GuardrailsApprovalRequest::export_all(&config)
            .expect("Failed to export GuardrailsApprovalRequest");
        GuardrailsApprovalStatus::export_all(&config)
            .expect("Failed to export GuardrailsApprovalStatus");

        // Export HitL types (requires hitl-client feature)
        #[cfg(feature = "hitl-client")]
        {
            HitLRiskLevel::export_all(&config).expect("Failed to export HitLRiskLevel");
            ApprovalType::export_all(&config).expect("Failed to export ApprovalType");
            HitLApprovalStatus::export_all(&config).expect("Failed to export HitLApprovalStatus");
            HitLApprovalRequest::export_all(&config).expect("Failed to export HitLApprovalRequest");
            CreateApprovalRequest::export_all(&config)
                .expect("Failed to export CreateApprovalRequest");
            ApprovalResponse::export_all(&config).expect("Failed to export ApprovalResponse");
        }

        // Export Event types
        TaskEvent::export_all(&config).expect("Failed to export TaskEvent");
        TaskCreatedData::export_all(&config).expect("Failed to export TaskCreatedData");
        TaskStartedData::export_all(&config).expect("Failed to export TaskStartedData");
        ThoughtData::export_all(&config).expect("Failed to export ThoughtData");
        ToolUseData::export_all(&config).expect("Failed to export ToolUseData");
        ProgressData::export_all(&config).expect("Failed to export ProgressData");
        OutputData::export_all(&config).expect("Failed to export OutputData");
        ConfirmationData::export_all(&config).expect("Failed to export ConfirmationData");
        AgentSwitchData::export_all(&config).expect("Failed to export AgentSwitchData");
        HeartbeatData::export_all(&config).expect("Failed to export HeartbeatData");
        DebugInfoData::export_all(&config).expect("Failed to export DebugInfoData");
        AgentInfoData::export_all(&config).expect("Failed to export AgentInfoData");
        SkillUseData::export_all(&config).expect("Failed to export SkillUseData");
        TaskCompletedData::export_all(&config).expect("Failed to export TaskCompletedData");
        TaskFailedData::export_all(&config).expect("Failed to export TaskFailedData");
        TaskUsage::export_all(&config).expect("Failed to export TaskUsage");
        StreamEvent::export_all(&config).expect("Failed to export StreamEvent");
        TaskState::export_all(&config).expect("Failed to export TaskState");
        TaskStatus::export_all(&config).expect("Failed to export TaskStatus");

        println!("All TypeScript bindings exported successfully!");
    }
}
