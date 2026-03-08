//! Expression evaluation for workflow DSL.

use serde_json::Value;
use thiserror::Error;

/// Evaluation error types.
#[derive(Debug, Error)]
pub enum EvalError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),
}

/// Evaluation result type alias.
pub type EvalResult<T> = Result<T, EvalError>;

/// Evaluation context containing all available variables.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// Event data ($event)
    pub event: Value,
    /// Step outputs ($steps)
    pub steps: Value,
    /// Workflow variables ($vars)
    pub vars: Value,
    /// Loop variables
    pub loop_vars: Value,
}

impl EvalContext {
    /// Resolve a variable path like "$event.type" or "$steps.review.status".
    pub fn resolve(&self, path: &str) -> EvalResult<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Err(EvalError::VariableNotFound(path.to_string()));
        }

        let root = match parts[0] {
            "$event" => &self.event,
            "$steps" => &self.steps,
            "$vars" => &self.vars,
            "$item" | "$index" | "$loop" => &self.loop_vars,
            other if other.starts_with('$') => {
                // Check loop vars for custom variable names
                if let Value::Object(map) = &self.loop_vars {
                    if let Some(v) = map.get(&other[1..]) {
                        return Self::traverse_path(v, &parts[1..]);
                    }
                }
                return Err(EvalError::VariableNotFound(other.to_string()));
            }
            _ => return Err(EvalError::VariableNotFound(parts[0].to_string())),
        };

        Self::traverse_path(root, &parts[1..])
    }

    fn traverse_path(value: &Value, parts: &[&str]) -> EvalResult<Value> {
        let mut current = value.clone();

        for part in parts {
            current = match current {
                Value::Object(map) => map
                    .get(*part)
                    .cloned()
                    .unwrap_or(Value::Null),
                Value::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        arr.get(idx).cloned().unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                _ => Value::Null,
            };
        }

        Ok(current)
    }
}

/// Evaluate a simple expression.
///
/// Supports:
/// - Variable access: `$event.type`, `$steps.name.output`
/// - Literals: `"string"`, `123`, `true`, `false`, `null`
/// - Comparisons: `==`, `!=`, `>`, `<`, `>=`, `<=`
/// - Logical: `&&`, `||`, `!`
/// - String operations: `contains`, `startsWith`, `endsWith`
pub fn evaluate_expression(expr: &str, context: &EvalContext) -> EvalResult<Value> {
    let expr = expr.trim();

    // Handle empty expression
    if expr.is_empty() {
        return Ok(Value::Null);
    }

    // Handle parentheses
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len() - 1];
        // Check for balanced parens
        let mut depth = 0;
        let mut balanced = true;
        for c in inner.chars() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        balanced = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if balanced && depth == 0 {
            return evaluate_expression(inner, context);
        }
    }

    // Handle logical operators (lowest precedence)
    if let Some(result) = try_logical_ops(expr, context)? {
        return Ok(result);
    }

    // Handle comparison operators
    if let Some(result) = try_comparison_ops(expr, context)? {
        return Ok(result);
    }

    // Handle string operations
    if let Some(result) = try_string_ops(expr, context)? {
        return Ok(result);
    }

    // Handle arithmetic operators
    if let Some(result) = try_arithmetic_ops(expr, context)? {
        return Ok(result);
    }

    // Handle NOT operator
    if let Some(inner) = expr.strip_prefix('!') {
        let value = evaluate_expression(inner.trim(), context)?;
        return Ok(Value::Bool(!value_to_bool(&value)));
    }

    // Handle variable access
    if expr.starts_with('$') {
        return context.resolve(expr);
    }

    // Handle string literals
    if (expr.starts_with('"') && expr.ends_with('"'))
        || (expr.starts_with('\'') && expr.ends_with('\''))
    {
        return Ok(Value::String(expr[1..expr.len() - 1].to_string()));
    }

    // Handle numeric literals
    if let Ok(n) = expr.parse::<i64>() {
        return Ok(Value::Number(n.into()));
    }
    if let Ok(n) = expr.parse::<f64>() {
        return Ok(serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null));
    }

    // Handle boolean literals
    match expr {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        "null" => return Ok(Value::Null),
        _ => {}
    }

    Err(EvalError::Parse(format!("Cannot parse expression: {}", expr)))
}

fn try_logical_ops(expr: &str, context: &EvalContext) -> EvalResult<Option<Value>> {
    // Find && or || outside of strings and parentheses
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    let chars: Vec<char> = expr.chars().collect();

    for i in 0..chars.len().saturating_sub(1) {
        let c = chars[i];
        let next = chars[i + 1];

        if !in_string {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    string_char = c;
                }
                '(' => depth += 1,
                ')' => depth -= 1,
                '&' if next == '&' && depth == 0 => {
                    let left = evaluate_expression(&expr[..i], context)?;
                    let right = evaluate_expression(&expr[i + 2..], context)?;
                    return Ok(Some(Value::Bool(
                        value_to_bool(&left) && value_to_bool(&right),
                    )));
                }
                '|' if next == '|' && depth == 0 => {
                    let left = evaluate_expression(&expr[..i], context)?;
                    let right = evaluate_expression(&expr[i + 2..], context)?;
                    return Ok(Some(Value::Bool(
                        value_to_bool(&left) || value_to_bool(&right),
                    )));
                }
                _ => {}
            }
        } else if c == string_char && (i == 0 || chars[i - 1] != '\\') {
            in_string = false;
        }
    }

    Ok(None)
}

fn try_comparison_ops(expr: &str, context: &EvalContext) -> EvalResult<Option<Value>> {
    // Find comparison operators outside of strings
    let ops = ["==", "!=", ">=", "<=", ">", "<"];

    for op in &ops {
        if let Some(pos) = find_operator(expr, op) {
            let left = evaluate_expression(&expr[..pos], context)?;
            let right = evaluate_expression(&expr[pos + op.len()..], context)?;

            let result = match *op {
                "==" => values_equal(&left, &right),
                "!=" => !values_equal(&left, &right),
                ">" => compare_values(&left, &right) == Some(std::cmp::Ordering::Greater),
                "<" => compare_values(&left, &right) == Some(std::cmp::Ordering::Less),
                ">=" => matches!(
                    compare_values(&left, &right),
                    Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                ),
                "<=" => matches!(
                    compare_values(&left, &right),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                ),
                _ => false,
            };

            return Ok(Some(Value::Bool(result)));
        }
    }

    Ok(None)
}

fn try_string_ops(expr: &str, context: &EvalContext) -> EvalResult<Option<Value>> {
    let ops = [" contains ", " startsWith ", " endsWith "];

    for op in &ops {
        if let Some(pos) = expr.find(op) {
            let left = evaluate_expression(&expr[..pos], context)?;
            let right = evaluate_expression(&expr[pos + op.len()..], context)?;

            let left_str = match &left {
                Value::String(s) => s.clone(),
                _ => left.to_string(),
            };
            let right_str = match &right {
                Value::String(s) => s.clone(),
                _ => right.to_string(),
            };

            let result = match *op {
                " contains " => left_str.contains(&right_str),
                " startsWith " => left_str.starts_with(&right_str),
                " endsWith " => left_str.ends_with(&right_str),
                _ => false,
            };

            return Ok(Some(Value::Bool(result)));
        }
    }

    Ok(None)
}

fn try_arithmetic_ops(expr: &str, context: &EvalContext) -> EvalResult<Option<Value>> {
    let ops = ["+", "-", "*", "/"];

    for op in &ops {
        if let Some(pos) = find_operator(expr, op) {
            let left = evaluate_expression(&expr[..pos], context)?;
            let right = evaluate_expression(&expr[pos + op.len()..], context)?;

            let left_num = value_to_f64(&left).ok_or_else(|| {
                EvalError::TypeError(format!("Cannot convert to number: {:?}", left))
            })?;
            let right_num = value_to_f64(&right).ok_or_else(|| {
                EvalError::TypeError(format!("Cannot convert to number: {:?}", right))
            })?;

            let result = match *op {
                "+" => left_num + right_num,
                "-" => left_num - right_num,
                "*" => left_num * right_num,
                "/" => {
                    if right_num == 0.0 {
                        return Err(EvalError::DivisionByZero);
                    }
                    left_num / right_num
                }
                _ => return Ok(None),
            };

            return Ok(Some(
                serde_json::Number::from_f64(result)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            ));
        }
    }

    Ok(None)
}

fn find_operator(expr: &str, op: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    let chars: Vec<char> = expr.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];

        if !in_string {
            match c {
                '"' | '\'' => {
                    in_string = true;
                    string_char = c;
                }
                '(' => depth += 1,
                ')' => depth -= 1,
                _ if depth == 0 => {
                    if expr[i..].starts_with(op) {
                        return Some(i);
                    }
                }
                _ => {}
            }
        } else if c == string_char && (i == 0 || chars[i - 1] != '\\') {
            in_string = false;
        }
    }

    None
}

fn value_to_bool(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    a == b
}

fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let a = a.as_f64()?;
            let b = b.as_f64()?;
            a.partial_cmp(&b)
        }
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> EvalContext {
        EvalContext {
            event: serde_json::json!({
                "type": "push",
                "branch": "main",
                "count": 42
            }),
            steps: serde_json::json!({
                "review": {
                    "status": "success",
                    "issues": 0
                }
            }),
            vars: serde_json::json!({
                "version": "1.0.0"
            }),
            loop_vars: Value::Null,
        }
    }

    #[test]
    fn test_variable_access() {
        let ctx = make_context();

        let result = evaluate_expression("$event.type", &ctx).unwrap();
        assert_eq!(result, Value::String("push".to_string()));

        let result = evaluate_expression("$event.count", &ctx).unwrap();
        assert_eq!(result, serde_json::json!(42));

        let result = evaluate_expression("$steps.review.status", &ctx).unwrap();
        assert_eq!(result, Value::String("success".to_string()));
    }

    #[test]
    fn test_literals() {
        let ctx = make_context();

        assert_eq!(evaluate_expression("42", &ctx).unwrap(), serde_json::json!(42));
        assert_eq!(
            evaluate_expression("\"hello\"", &ctx).unwrap(),
            Value::String("hello".to_string())
        );
        assert_eq!(evaluate_expression("true", &ctx).unwrap(), Value::Bool(true));
        assert_eq!(evaluate_expression("false", &ctx).unwrap(), Value::Bool(false));
        assert_eq!(evaluate_expression("null", &ctx).unwrap(), Value::Null);
    }

    #[test]
    fn test_comparison() {
        let ctx = make_context();

        assert_eq!(
            evaluate_expression("$event.type == \"push\"", &ctx).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            evaluate_expression("$event.count > 10", &ctx).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            evaluate_expression("$event.count < 10", &ctx).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            evaluate_expression("$steps.review.issues == 0", &ctx).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_logical() {
        let ctx = make_context();

        assert_eq!(
            evaluate_expression("$event.type == \"push\" && $event.count > 10", &ctx).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            evaluate_expression("$event.type == \"pr\" || $event.count > 10", &ctx).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            evaluate_expression("!false", &ctx).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_string_ops() {
        let ctx = make_context();

        assert_eq!(
            evaluate_expression("$event.branch contains \"main\"", &ctx).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            evaluate_expression("$event.branch startsWith \"ma\"", &ctx).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            evaluate_expression("$event.branch endsWith \"in\"", &ctx).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_arithmetic() {
        let ctx = make_context();

        assert_eq!(
            evaluate_expression("10 + 5", &ctx).unwrap(),
            serde_json::json!(15.0)
        );
        assert_eq!(
            evaluate_expression("$event.count * 2", &ctx).unwrap(),
            serde_json::json!(84.0)
        );
    }
}
