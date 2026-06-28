/// ─── PlanTool ──────────────────────────────────────────────────────────────
///
/// Agent outputs a structured execution plan, pauses for user approval,
/// then proceeds or adjusts.
///
/// Uses the same static response channel pattern as AskUserTool.
///
/// Input: {"title": "Refactor X", "steps": ["Step 1...", "Step 2..."], "context": "optional"}
/// Output: "approved" or "rejected" (user decision)

use super::Tool;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

static PLAN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Registry of pending plan approvals.
static PENDING_PLANS: std::sync::LazyLock<Mutex<HashMap<u64, mpsc::Sender<String>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// The response channel to the TUI (same channel as AskUserTool, shared).
static PLAN_RESPONSE_TX: std::sync::LazyLock<Mutex<Option<mpsc::Sender<crate::agent::AgentResponse>>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

pub struct PlanTool;

impl PlanTool {
    /// Set the response channel used to send plans to the TUI.
    pub fn set_response_tx(tx: mpsc::Sender<crate::agent::AgentResponse>) {
        *PLAN_RESPONSE_TX.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tx);
    }

    /// Deliver a plan approval/rejection.
    pub fn deliver_decision(request_id: u64, decision: String) -> bool {
        let mut pending = PENDING_PLANS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(tx) = pending.remove(&request_id) {
            let _ = tx.send(decision);
            true
        } else {
            false
        }
    }
}

impl Tool for PlanTool {
    fn name(&self) -> &str {
        "plan"
    }

    fn description(&self) -> &str {
        "Present a plan for user approval. Input JSON: {\"title\":\"...\", \"steps\":[\"...\"], \"context\":\"...\"}. Returns 'approved' or 'rejected'."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct PlanInput {
            title: String,
            #[serde(default)]
            steps: Vec<String>,
            #[serde(default)]
            context: String,
        }

        let parsed: PlanInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid plan input: {e}"))?;

        let id = PLAN_COUNTER.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel::<String>();

        // Format the plan
        let mut plan_text = format!("## {}\n\n", parsed.title);
        if !parsed.context.is_empty() {
            plan_text.push_str(&format!("_{}_\n\n", parsed.context));
        }
        for (i, step) in parsed.steps.iter().enumerate() {
            plan_text.push_str(&format!("{}. {}\n", i + 1, step));
        }

        // Register pending plan
        {
            let mut pending = PENDING_PLANS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            pending.insert(id, tx);
        }

        // Send to TUI
        {
            let resp_tx = PLAN_RESPONSE_TX.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(ref tx) = *resp_tx {
                let _ = tx.send(crate::agent::AgentResponse::PlanRequest {
                    title: parsed.title.clone(),
                    plan: plan_text,
                    request_id: id,
                });
            }
        }

        // Wait for approval (timeout: 600s)
        match rx.recv_timeout(std::time::Duration::from_secs(600)) {
            Ok(decision) => {
                let normalized = decision.to_lowercase();
                if normalized == "y" || normalized == "yes" || normalized == "approved" {
                    Ok("approved".into())
                } else {
                    Ok("rejected".into())
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                PENDING_PLANS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id);
                Ok("rejected (timeout)".into())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Ok("rejected (disconnected)".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_tool_name() {
        let tool = PlanTool;
        assert_eq!(tool.name(), "plan");
    }

    #[test]
    fn test_plan_tool_description_not_empty() {
        let tool = PlanTool;
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("plan"));
    }

    #[test]
    fn test_plan_tool_execute_invalid_json() {
        let tool = PlanTool;
        let result = tool.execute("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_tool_execute_missing_title() {
        let tool = PlanTool;
        let result = tool.execute(r#"{"steps":["do X"]}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_tool_deliver_decision_nonexistent() {
        let result = PlanTool::deliver_decision(99999, "yes".into());
        assert!(!result, "delivering to non-existent request should return false");
    }

    #[test]
    fn test_plan_tool_set_response_tx_and_deliver() {
        let (tx, rx) = mpsc::channel();
        PlanTool::set_response_tx(tx);

        // Execute a plan — it will block on rx.recv_timeout, so we need to deliver in parallel
        let tool = PlanTool;
        let input = r#"{"title":"Test Plan","steps":["Step 1","Step 2"]}"#;

        // Spawn a thread to deliver the decision
        let handle = std::thread::spawn(move || {
            // First wait for the plan request to arrive at the TUI
            let response = rx.recv().unwrap();
            match response {
                crate::agent::AgentResponse::PlanRequest { request_id, .. } => {
                    PlanTool::deliver_decision(request_id, "approved".into());
                }
                _ => panic!("Expected PlanRequest"),
            }
        });

        let result = tool.execute(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "approved");

        handle.join().unwrap();
    }
}
