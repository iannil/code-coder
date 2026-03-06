//! Prompt template engine

use std::collections::HashMap;
use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context for prompt rendering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptContext {
    /// Variables to substitute
    pub variables: HashMap<String, Value>,
}

impl PromptContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a variable
    pub fn set(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Add a string variable
    pub fn set_str(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.set(key, Value::String(value.into()))
    }
}

/// A prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template name
    pub name: String,

    /// Template content (Handlebars syntax)
    pub content: String,

    /// Default variables
    #[serde(default)]
    pub defaults: HashMap<String, Value>,
}

impl PromptTemplate {
    /// Create a new template
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            defaults: HashMap::new(),
        }
    }

    /// Add a default variable
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }
}

/// Template engine using Handlebars
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);
        Self { handlebars }
    }

    /// Register a template
    pub fn register(&mut self, template: &PromptTemplate) -> Result<()> {
        self.handlebars
            .register_template_string(&template.name, &template.content)
            .with_context(|| format!("Failed to register template: {}", template.name))?;
        Ok(())
    }

    /// Render a template with context
    pub fn render(&self, template_name: &str, context: &PromptContext) -> Result<String> {
        self.handlebars
            .render(template_name, &context.variables)
            .with_context(|| format!("Failed to render template: {}", template_name))
    }

    /// Render a template string directly
    pub fn render_string(&self, template: &str, context: &PromptContext) -> Result<String> {
        self.handlebars
            .render_template(template, &context.variables)
            .with_context(|| "Failed to render template string")
    }

    /// Render a PromptTemplate
    pub fn render_template(&self, template: &PromptTemplate, context: &PromptContext) -> Result<String> {
        // Merge defaults with provided context
        let mut vars = template.defaults.clone();
        vars.extend(context.variables.clone());

        self.handlebars
            .render_template(&template.content, &vars)
            .with_context(|| format!("Failed to render template: {}", template.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render() {
        let engine = TemplateEngine::new();
        let context = PromptContext::new()
            .set_str("name", "World");

        let result = engine.render_string("Hello, {{name}}!", &context).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_template_registration() {
        let mut engine = TemplateEngine::new();
        let template = PromptTemplate::new("greeting", "Hello, {{name}}!");

        engine.register(&template).unwrap();

        let context = PromptContext::new().set_str("name", "Rust");
        let result = engine.render("greeting", &context).unwrap();
        assert_eq!(result, "Hello, Rust!");
    }

    #[test]
    fn test_template_with_defaults() {
        let engine = TemplateEngine::new();
        let template = PromptTemplate::new("greeting", "Hello, {{name}}!")
            .with_default("name", Value::String("Default".into()));

        let context = PromptContext::new();
        let result = engine.render_template(&template, &context).unwrap();
        assert_eq!(result, "Hello, Default!");

        // Override default
        let context = PromptContext::new().set_str("name", "Override");
        let result = engine.render_template(&template, &context).unwrap();
        assert_eq!(result, "Hello, Override!");
    }

    #[test]
    fn test_complex_template() {
        let engine = TemplateEngine::new();
        let template = r#"
You are {{role}}.
{{#if instructions}}
Instructions: {{instructions}}
{{/if}}
{{#each tools}}
- {{this}}
{{/each}}
"#;

        let context = PromptContext::new()
            .set_str("role", "an AI assistant")
            .set_str("instructions", "Be helpful")
            .set("tools", serde_json::json!(["search", "read", "write"]));

        let result = engine.render_string(template, &context).unwrap();
        assert!(result.contains("AI assistant"));
        assert!(result.contains("Be helpful"));
        assert!(result.contains("- search"));
    }
}
