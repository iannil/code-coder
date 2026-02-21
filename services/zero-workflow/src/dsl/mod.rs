//! Workflow DSL - Expression evaluation and control flow.
//!
//! Provides a simple expression language for workflow conditions and variable interpolation.
//!
//! # Expression Syntax
//!
//! ## Variable Access
//! - `$event.type` - Access event data
//! - `$steps.step_name.output` - Access step output
//! - `$vars.my_var` - Access workflow variables
//!
//! ## Operators
//! - Comparison: `==`, `!=`, `>`, `<`, `>=`, `<=`
//! - Logical: `&&`, `||`, `!`
//! - String: `contains`, `startsWith`, `endsWith`
//! - Arithmetic: `+`, `-`, `*`, `/`
//!
//! ## Examples
//! - `$event.action == "opened"`
//! - `$steps.review.status == "success" && $steps.review.issues == 0`
//! - `$event.branch contains "feature/"`

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

mod expression;
mod parser;

pub use expression::{evaluate_expression, EvalContext, EvalError, EvalResult};
pub use parser::{parse_expression, Expr, ParseError};

/// Control flow types for workflow steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "control", rename_all = "lowercase")]
pub enum ControlFlow {
    /// Sequential execution (default)
    Sequence {
        steps: Vec<crate::Step>,
    },
    /// Parallel execution of steps
    Parallel {
        steps: Vec<crate::Step>,
        /// Maximum concurrent executions
        #[serde(default = "default_max_concurrent")]
        max_concurrent: usize,
    },
    /// Conditional branch
    If {
        /// Condition expression
        condition: String,
        /// Steps to execute if condition is true
        then_steps: Vec<crate::Step>,
        /// Steps to execute if condition is false
        #[serde(default)]
        else_steps: Vec<crate::Step>,
    },
    /// Switch/case control flow
    Switch {
        /// Expression to evaluate
        expression: String,
        /// Cases to match
        cases: Vec<SwitchCase>,
        /// Default case steps
        #[serde(default)]
        default: Vec<crate::Step>,
    },
    /// Loop control flow
    ForEach {
        /// Variable name for each item
        item_var: String,
        /// Expression returning an array to iterate over
        items: String,
        /// Steps to execute for each item
        steps: Vec<crate::Step>,
    },
    /// Repeat until condition is met
    RepeatUntil {
        /// Condition to check after each iteration
        condition: String,
        /// Maximum iterations
        #[serde(default = "default_max_iterations")]
        max_iterations: usize,
        /// Steps to execute each iteration
        steps: Vec<crate::Step>,
    },
    /// Retry on failure
    Retry {
        /// Maximum retry attempts
        #[serde(default = "default_max_retries")]
        max_retries: usize,
        /// Delay between retries in seconds
        #[serde(default = "default_retry_delay")]
        retry_delay_secs: u64,
        /// Exponential backoff multiplier
        #[serde(default = "default_backoff_multiplier")]
        backoff_multiplier: f64,
        /// Steps to execute with retry
        steps: Vec<crate::Step>,
    },
    /// Try-catch error handling
    TryCatch {
        /// Steps to try
        try_steps: Vec<crate::Step>,
        /// Steps to execute on error
        catch_steps: Vec<crate::Step>,
        /// Steps to always execute
        #[serde(default)]
        finally_steps: Vec<crate::Step>,
    },
}

/// Switch case definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCase {
    /// Value to match
    pub value: Value,
    /// Steps to execute if matched
    pub steps: Vec<crate::Step>,
}

fn default_max_concurrent() -> usize {
    4
}

fn default_max_iterations() -> usize {
    100
}

fn default_max_retries() -> usize {
    3
}

fn default_retry_delay() -> u64 {
    5
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

/// Template string interpolation.
///
/// Replaces `{{ expression }}` patterns with evaluated values.
pub fn interpolate(template: &str, context: &EvalContext) -> Result<String, EvalError> {
    let mut result = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for opening {{
        if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
            // Find closing }}
            let expr_start = i + 2;
            let mut expr_end = None;
            let mut j = expr_start;
            while j + 1 < chars.len() {
                if chars[j] == '}' && chars[j + 1] == '}' {
                    expr_end = Some(j);
                    break;
                }
                j += 1;
            }

            if let Some(end) = expr_end {
                let expr: String = chars[expr_start..end].iter().collect();
                let expr = expr.trim();

                // Evaluate expression
                let value = evaluate_expression(expr, context)?;
                match value {
                    Value::String(s) => result.push_str(&s),
                    Value::Null => result.push_str("null"),
                    v => result.push_str(&v.to_string()),
                }

                i = end + 2; // Skip past }}
            } else {
                return Err(EvalError::Parse("Unclosed template expression".into()));
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Ok(result)
}

/// Workflow execution state.
#[derive(Debug, Clone, Default)]
pub struct ExecutionState {
    /// Step outputs keyed by step name
    pub step_outputs: HashMap<String, Value>,
    /// Current loop variables
    pub loop_vars: HashMap<String, Value>,
    /// Error state
    pub error: Option<String>,
}

impl ExecutionState {
    /// Create a new execution state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a step output.
    pub fn set_step_output(&mut self, name: &str, output: Value) {
        self.step_outputs.insert(name.to_string(), output);
    }

    /// Get a step output.
    pub fn get_step_output(&self, name: &str) -> Option<&Value> {
        self.step_outputs.get(name)
    }

    /// Set a loop variable.
    pub fn set_loop_var(&mut self, name: &str, value: Value) {
        self.loop_vars.insert(name.to_string(), value);
    }

    /// Clear loop variables.
    pub fn clear_loop_vars(&mut self) {
        self.loop_vars.clear();
    }

    /// Build evaluation context.
    pub fn to_eval_context(&self, event: &Value, vars: &Value) -> EvalContext {
        let mut steps = serde_json::Map::new();
        for (k, v) in &self.step_outputs {
            steps.insert(k.clone(), v.clone());
        }

        let mut loop_vars = serde_json::Map::new();
        for (k, v) in &self.loop_vars {
            loop_vars.insert(k.clone(), v.clone());
        }

        EvalContext {
            event: event.clone(),
            steps: Value::Object(steps),
            vars: vars.clone(),
            loop_vars: Value::Object(loop_vars),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_simple() {
        let context = EvalContext {
            event: serde_json::json!({"name": "test", "count": 42}),
            steps: Value::Null,
            vars: Value::Null,
            loop_vars: Value::Null,
        };

        let result = interpolate("Hello {{ $event.name }}, count is {{ $event.count }}", &context);
        assert_eq!(result.unwrap(), "Hello test, count is 42");
    }

    #[test]
    fn test_interpolate_no_expressions() {
        let context = EvalContext::default();
        let result = interpolate("Plain text without expressions", &context);
        assert_eq!(result.unwrap(), "Plain text without expressions");
    }

    #[test]
    fn test_execution_state() {
        let mut state = ExecutionState::new();
        state.set_step_output("step1", serde_json::json!({"status": "success"}));

        let event = serde_json::json!({"type": "push"});
        let vars = serde_json::json!({"version": "1.0"});

        let ctx = state.to_eval_context(&event, &vars);

        assert_eq!(ctx.event["type"], "push");
        assert_eq!(ctx.steps["step1"]["status"], "success");
    }
}
