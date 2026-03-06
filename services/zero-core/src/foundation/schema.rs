//! JSON Schema validation for configuration files
//!
//! Provides compile-time and runtime schema validation.

use anyhow::{Context, Result};
use jsonschema::{Draft, Validator};
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Schema validator for configuration files
pub struct SchemaValidator {
    schema: Validator,
}

impl SchemaValidator {
    /// Create a validator from a schema file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read schema: {:?}", path.as_ref()))?;
        let schema_value: Value =
            serde_json::from_str(&content).context("Failed to parse schema JSON")?;
        Self::from_value(schema_value)
    }

    /// Create a validator from a JSON value
    pub fn from_value(schema: Value) -> Result<Self> {
        let compiled = Validator::options()
            .with_draft(Draft::Draft7)
            .build(&schema)
            .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;
        Ok(Self { schema: compiled })
    }

    /// Validate a configuration value
    pub fn validate(&self, config: &Value) -> Result<Vec<ValidationIssue>> {
        let result = self.schema.validate(config);
        match result {
            Ok(_) => Ok(vec![]),
            Err(err) => {
                // validate() returns only first error, use iter_errors for all
                let issues: Vec<_> = self
                    .schema
                    .iter_errors(config)
                    .map(|e| ValidationIssue {
                        path: e.instance_path.to_string(),
                        message: e.to_string(),
                    })
                    .collect();
                if issues.is_empty() {
                    // Shouldn't happen, but just in case - return the first error
                    Ok(vec![ValidationIssue {
                        path: String::new(),
                        message: err.to_string(),
                    }])
                } else {
                    Ok(issues)
                }
            }
        }
    }

    /// Check if config is valid (no issues)
    pub fn is_valid(&self, config: &Value) -> bool {
        self.schema.is_valid(config)
    }
}

/// A validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// JSON path to the issue
    pub path: String,
    /// Human-readable message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "theme": { "type": "string" },
                "model": { "type": "string" }
            },
            "required": ["theme"]
        });

        let validator = SchemaValidator::from_value(schema).unwrap();

        // Valid config
        let valid = json!({ "theme": "dark" });
        assert!(validator.is_valid(&valid));

        // Invalid config (missing required field)
        let invalid = json!({ "model": "opus" });
        assert!(!validator.is_valid(&invalid));
    }

    #[test]
    fn test_schema_validation_with_issues() {
        let schema = json!({
            "type": "object",
            "properties": {
                "port": { "type": "integer", "minimum": 1, "maximum": 65535 }
            },
            "required": ["port"]
        });

        let validator = SchemaValidator::from_value(schema).unwrap();

        // Invalid: port is a string instead of integer
        let invalid = json!({ "port": "not-a-number" });
        let issues = validator.validate(&invalid).unwrap();
        assert!(!issues.is_empty());
        assert!(issues[0].message.contains("type"));
    }

    #[test]
    fn test_schema_validation_nested() {
        let schema = json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "object",
                    "properties": {
                        "port": { "type": "integer" },
                        "host": { "type": "string" }
                    },
                    "required": ["port"]
                }
            }
        });

        let validator = SchemaValidator::from_value(schema).unwrap();

        // Valid nested config
        let valid = json!({
            "server": {
                "port": 8080,
                "host": "localhost"
            }
        });
        assert!(validator.is_valid(&valid));

        // Invalid: missing required nested field
        let invalid = json!({
            "server": {
                "host": "localhost"
            }
        });
        assert!(!validator.is_valid(&invalid));
    }

    #[test]
    fn test_schema_from_invalid_json() {
        let invalid_schema = json!({
            "type": "invalid-type-that-does-not-exist"
        });

        // jsonschema 0.26+ validates schemas strictly and rejects invalid type values
        let validator = SchemaValidator::from_value(invalid_schema);
        assert!(validator.is_err());
    }

    #[test]
    fn test_empty_schema_accepts_anything() {
        let schema = json!({});
        let validator = SchemaValidator::from_value(schema).unwrap();

        assert!(validator.is_valid(&json!({})));
        assert!(validator.is_valid(&json!({ "any": "value" })));
        assert!(validator.is_valid(&json!(123)));
        assert!(validator.is_valid(&json!("string")));
    }
}
