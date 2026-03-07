//! NAPI bindings for JSON Schema validation
//!
//! Provides standalone schema validation with:
//! - Reusable compiled schemas (compile once, validate many)
//! - Schema registry for named schema lookup
//! - Batch validation support
//!
//! This complements ConfigLoaderHandle.validateSchema() for cases where
//! schema validation is needed outside of config loading.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::foundation::schema::{SchemaValidator as RustSchemaValidator, ValidationIssue};

// ============================================================================
// Types
// ============================================================================

/// A validation issue from schema validation
#[napi(object)]
pub struct NapiSchemaIssue {
    /// JSON path to the issue (e.g., "/server/port")
    pub path: String,
    /// Human-readable error message
    pub message: String,
}

impl From<ValidationIssue> for NapiSchemaIssue {
    fn from(i: ValidationIssue) -> Self {
        Self {
            path: i.path,
            message: i.message,
        }
    }
}

/// Result of schema validation
#[napi(object)]
pub struct NapiSchemaValidationResult {
    /// Whether the document is valid
    pub valid: bool,
    /// Number of issues found
    pub issue_count: u32,
    /// All validation issues
    pub issues: Vec<NapiSchemaIssue>,
}

/// Result of batch validation
#[napi(object)]
pub struct NapiBatchValidationResult {
    /// Number of documents validated
    pub total: u32,
    /// Number of valid documents
    pub valid_count: u32,
    /// Number of invalid documents
    pub invalid_count: u32,
    /// Results for each document (indexed by input order)
    pub results: Vec<NapiSchemaValidationResult>,
}

// ============================================================================
// SchemaValidatorHandle
// ============================================================================

/// Handle to a compiled JSON schema for efficient reuse
#[napi]
pub struct SchemaValidatorHandle {
    inner: Arc<RustSchemaValidator>,
    schema_name: Option<String>,
}

#[napi]
impl SchemaValidatorHandle {
    /// Create a new schema validator from a JSON schema string
    #[napi(constructor)]
    pub fn new(schema_json: String) -> Result<Self> {
        let schema: Value = serde_json::from_str(&schema_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON schema: {}", e)))?;

        let validator = RustSchemaValidator::from_value(schema)
            .map_err(|e| Error::from_reason(format!("Failed to compile schema: {}", e)))?;

        Ok(Self {
            inner: Arc::new(validator),
            schema_name: None,
        })
    }

    /// Create a validator from a schema with a name for debugging
    #[napi(factory)]
    pub fn with_name(schema_json: String, name: String) -> Result<Self> {
        let mut handle = Self::new(schema_json)?;
        handle.schema_name = Some(name);
        Ok(handle)
    }

    /// Validate a document against the schema
    #[napi]
    pub fn validate(&self, document_json: String) -> Result<NapiSchemaValidationResult> {
        let document: Value = serde_json::from_str(&document_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON document: {}", e)))?;

        let issues: Vec<NapiSchemaIssue> = self.inner.validate(&document)
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(NapiSchemaValidationResult {
            valid: issues.is_empty(),
            issue_count: issues.len() as u32,
            issues,
        })
    }

    /// Check if a document is valid (quick check, no issue details)
    #[napi]
    pub fn is_valid(&self, document_json: String) -> Result<bool> {
        let document: Value = serde_json::from_str(&document_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON document: {}", e)))?;

        Ok(self.inner.is_valid(&document))
    }

    /// Validate multiple documents in batch
    #[napi]
    pub fn validate_batch(&self, documents_json: Vec<String>) -> Result<NapiBatchValidationResult> {
        let mut results = Vec::with_capacity(documents_json.len());
        let mut valid_count = 0u32;

        for doc_json in &documents_json {
            let result = self.validate(doc_json.clone())?;
            if result.valid {
                valid_count += 1;
            }
            results.push(result);
        }

        let total = documents_json.len() as u32;

        Ok(NapiBatchValidationResult {
            total,
            valid_count,
            invalid_count: total - valid_count,
            results,
        })
    }

    /// Get the schema name (if set)
    #[napi]
    pub fn schema_name(&self) -> Option<String> {
        self.schema_name.clone()
    }
}

// ============================================================================
// SchemaRegistryHandle
// ============================================================================

/// A registry of named schemas for efficient lookup and reuse
#[napi]
pub struct SchemaRegistryHandle {
    schemas: Arc<RwLock<HashMap<String, Arc<RustSchemaValidator>>>>,
}

#[napi]
impl SchemaRegistryHandle {
    /// Create a new empty schema registry
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a schema by name
    #[napi]
    pub fn register(&self, name: String, schema_json: String) -> Result<()> {
        let schema: Value = serde_json::from_str(&schema_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON schema '{}': {}", name, e)))?;

        let validator = RustSchemaValidator::from_value(schema)
            .map_err(|e| Error::from_reason(format!("Failed to compile schema '{}': {}", name, e)))?;

        let mut schemas = self.schemas.write()
            .map_err(|_| Error::from_reason("Failed to acquire schema registry lock"))?;
        schemas.insert(name, Arc::new(validator));

        Ok(())
    }

    /// Unregister a schema by name
    #[napi]
    pub fn unregister(&self, name: String) -> bool {
        match self.schemas.write() {
            Ok(mut schemas) => schemas.remove(&name).is_some(),
            Err(_) => false,
        }
    }

    /// Check if a schema is registered
    #[napi]
    pub fn has(&self, name: String) -> bool {
        match self.schemas.read() {
            Ok(schemas) => schemas.contains_key(&name),
            Err(_) => false,
        }
    }

    /// List all registered schema names
    #[napi]
    pub fn list(&self) -> Vec<String> {
        match self.schemas.read() {
            Ok(schemas) => schemas.keys().cloned().collect(),
            Err(_) => vec![],
        }
    }

    /// Get the number of registered schemas
    #[napi]
    pub fn count(&self) -> u32 {
        match self.schemas.read() {
            Ok(schemas) => schemas.len() as u32,
            Err(_) => 0,
        }
    }

    /// Validate a document against a named schema
    #[napi]
    pub fn validate(&self, schema_name: String, document_json: String) -> Result<NapiSchemaValidationResult> {
        let schemas = self.schemas.read()
            .map_err(|_| Error::from_reason("Failed to acquire schema registry lock"))?;

        let validator = schemas.get(&schema_name)
            .ok_or_else(|| Error::from_reason(format!("Schema '{}' not found", schema_name)))?;

        let document: Value = serde_json::from_str(&document_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON document: {}", e)))?;

        let issues: Vec<NapiSchemaIssue> = validator.validate(&document)
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(NapiSchemaValidationResult {
            valid: issues.is_empty(),
            issue_count: issues.len() as u32,
            issues,
        })
    }

    /// Check if a document is valid against a named schema (quick check)
    #[napi]
    pub fn is_valid(&self, schema_name: String, document_json: String) -> Result<bool> {
        let schemas = self.schemas.read()
            .map_err(|_| Error::from_reason("Failed to acquire schema registry lock"))?;

        let validator = schemas.get(&schema_name)
            .ok_or_else(|| Error::from_reason(format!("Schema '{}' not found", schema_name)))?;

        let document: Value = serde_json::from_str(&document_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON document: {}", e)))?;

        Ok(validator.is_valid(&document))
    }

    /// Clear all registered schemas
    #[napi]
    pub fn clear(&self) {
        if let Ok(mut schemas) = self.schemas.write() {
            schemas.clear();
        }
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Create a new schema validator
#[napi]
pub fn create_schema_validator(schema_json: String) -> Result<SchemaValidatorHandle> {
    SchemaValidatorHandle::new(schema_json)
}

/// Create a new schema registry
#[napi]
pub fn create_schema_registry() -> SchemaRegistryHandle {
    SchemaRegistryHandle::new()
}

/// One-shot schema validation (compiles schema each time)
#[napi]
pub fn validate_json_schema(
    document_json: String,
    schema_json: String,
) -> Result<NapiSchemaValidationResult> {
    let validator = SchemaValidatorHandle::new(schema_json)?;
    validator.validate(document_json)
}

/// Quick one-shot schema validation (returns boolean only)
#[napi]
pub fn is_valid_json_schema(document_json: String, schema_json: String) -> Result<bool> {
    let validator = SchemaValidatorHandle::new(schema_json)?;
    validator.is_valid(document_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validator() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer", "minimum": 0 }
            },
            "required": ["name"]
        }"#;

        let validator = SchemaValidatorHandle::new(schema.to_string()).unwrap();

        // Valid document
        let valid = r#"{"name": "Alice", "age": 30}"#;
        let result = validator.validate(valid.to_string()).unwrap();
        assert!(result.valid);
        assert_eq!(result.issue_count, 0);

        // Invalid document (missing required field)
        let invalid = r#"{"age": 30}"#;
        let result = validator.validate(invalid.to_string()).unwrap();
        assert!(!result.valid);
        assert!(result.issue_count > 0);
    }

    #[test]
    fn test_batch_validation() {
        let schema = r#"{"type": "object", "properties": {"id": {"type": "integer"}}}"#;
        let validator = SchemaValidatorHandle::new(schema.to_string()).unwrap();

        let docs = vec![
            r#"{"id": 1}"#.to_string(),
            r#"{"id": "not-a-number"}"#.to_string(),
            r#"{"id": 3}"#.to_string(),
        ];

        let result = validator.validate_batch(docs).unwrap();
        assert_eq!(result.total, 3);
        assert_eq!(result.valid_count, 2);
        assert_eq!(result.invalid_count, 1);
    }

    #[test]
    fn test_schema_registry() {
        let registry = SchemaRegistryHandle::new();

        let person_schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}"#;
        let number_schema = r#"{"type": "number"}"#;

        registry.register("person".to_string(), person_schema.to_string()).unwrap();
        registry.register("number".to_string(), number_schema.to_string()).unwrap();

        assert!(registry.has("person".to_string()));
        assert!(registry.has("number".to_string()));
        assert!(!registry.has("unknown".to_string()));
        assert_eq!(registry.count(), 2);

        // Validate against person schema
        let valid_person = r#"{"name": "Bob"}"#;
        let result = registry.validate("person".to_string(), valid_person.to_string()).unwrap();
        assert!(result.valid);

        // Validate against number schema
        assert!(registry.is_valid("number".to_string(), "42".to_string()).unwrap());
        assert!(!registry.is_valid("number".to_string(), "\"not a number\"".to_string()).unwrap());

        // List schemas
        let names = registry.list();
        assert!(names.contains(&"person".to_string()));
        assert!(names.contains(&"number".to_string()));
    }
}
