//! JAR file reverse engineering command
//!
//! This module provides comprehensive JAR file analysis including:
//! - Class file parsing and decompilation info
//! - Technology stack detection
//! - Dependency analysis
//! - Configuration file extraction

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use zero_core::java::{JarAnalysis, JarAnalyzer};

/// JAR reverse command options
#[derive(Debug, Clone)]
pub struct JarReverseOptions {
    /// Path to the JAR file
    pub jar_path: PathBuf,
    /// Output directory for extracted files
    pub output_dir: Option<PathBuf>,
    /// Maximum number of classes to analyze
    pub max_classes: Option<usize>,
    /// Output format (text, json, markdown)
    pub format: OutputFormat,
    /// Show class details
    pub show_classes: bool,
    /// Show configuration file contents
    pub show_configs: bool,
    /// Extract config files to output directory
    pub extract_configs: bool,
}

impl Default for JarReverseOptions {
    fn default() -> Self {
        Self {
            jar_path: PathBuf::new(),
            output_dir: None,
            max_classes: None,
            format: OutputFormat::Text,
            show_classes: false,
            show_configs: false,
            extract_configs: false,
        }
    }
}

/// Output format for analysis results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Analysis summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub jar_name: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub entry_count: usize,
    pub class_count: usize,
    pub package_count: usize,
    pub config_count: usize,
    pub technology_count: usize,
    pub main_class: Option<String>,
    pub java_version: Option<String>,
    pub build_tool: Option<String>,
}

impl From<&JarAnalysis> for AnalysisSummary {
    fn from(a: &JarAnalysis) -> Self {
        Self {
            jar_name: a.jar_name.clone(),
            size_bytes: a.size_bytes,
            size_human: format_bytes(a.size_bytes),
            entry_count: a.entry_count,
            class_count: a.classes.len(),
            package_count: a.package_names.len(),
            config_count: a.config_files.len(),
            technology_count: a.detections.len(),
            main_class: a.metadata.main_class.clone(),
            java_version: a.metadata.jdk_version.clone(),
            build_tool: a.metadata.build_tool.clone(),
        }
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Print analysis in text format
fn print_text_analysis(analysis: &JarAnalysis, options: &JarReverseOptions) {
    println!("🔍 JAR Analysis: {}", analysis.jar_name);
    println!("═══════════════════════════════════════════════════════\n");

    // Basic info
    println!("📦 Basic Information:");
    println!("   Path:        {}", analysis.jar_path);
    println!("   Size:        {}", format_bytes(analysis.size_bytes));
    println!("   Entries:     {}", analysis.entry_count);
    println!("   Classes:     {}", analysis.classes.len());
    println!("   Packages:    {}", analysis.package_names.len());
    println!();

    // Metadata
    println!("📋 Metadata:");
    if let Some(main) = &analysis.metadata.main_class {
        println!("   Main-Class:  {}", main);
    }
    if let Some(title) = &analysis.metadata.implementation_title {
        println!("   Title:       {}", title);
    }
    if let Some(version) = &analysis.metadata.implementation_version {
        println!("   Version:     {}", version);
    }
    if let Some(vendor) = &analysis.metadata.implementation_vendor {
        println!("   Vendor:      {}", vendor);
    }
    if let Some(jdk) = &analysis.metadata.jdk_version {
        println!("   JDK:         {}", jdk);
    }
    if let Some(build) = &analysis.metadata.build_tool {
        println!("   Build Tool:  {}", build);
    }
    println!();

    // Technologies detected
    if !analysis.detections.is_empty() {
        println!("🔬 Detected Technologies ({}):", analysis.detections.len());
        for detection in &analysis.detections {
            println!(
                "   {} {} [{}] - {}",
                category_emoji_str(&detection.category),
                detection.name,
                detection.confidence,
                detection.category
            );
        }
        println!();
    }

    // Top packages
    if !analysis.packages.is_empty() {
        println!("📁 Top Packages:");
        let mut sorted_packages = analysis.packages.clone();
        sorted_packages.sort_by(|a, b| b.class_count.cmp(&a.class_count));
        for pkg in sorted_packages.iter().take(10) {
            println!("   {} ({} classes)", pkg.name, pkg.class_count);
        }
        if analysis.packages.len() > 10 {
            println!("   ... and {} more packages", analysis.packages.len() - 10);
        }
        println!();
    }

    // Config files
    if !analysis.config_files.is_empty() {
        println!("⚙️  Configuration Files ({}):", analysis.config_files.len());
        for config in &analysis.config_files {
            println!("   [{}] {}", config.file_type, config.path);
            if options.show_configs {
                if let Some(content) = &config.content {
                    // Show first few lines
                    let lines: Vec<_> = content.lines().take(5).collect();
                    for line in &lines {
                        println!("      {}", line);
                    }
                    let total_lines = content.lines().count();
                    if total_lines > 5 {
                        println!("      ... ({} more lines)", total_lines - 5);
                    }
                }
            }
        }
        println!();
    }

    // Class details
    if options.show_classes && !analysis.classes.is_empty() {
        println!("📚 Classes ({}):", analysis.classes.len());
        for class in analysis.classes.iter().take(50) {
            let modifiers = class.modifiers.join(" ");
            println!(
                "   {} {} (Java {}, {})",
                class.class_type, class.name, class.java_version, modifiers
            );
        }
        if analysis.classes.len() > 50 {
            println!("   ... and {} more classes", analysis.classes.len() - 50);
        }
        println!();
    }
}

/// Print analysis in markdown format
fn print_markdown_analysis(analysis: &JarAnalysis, options: &JarReverseOptions) {
    println!("# JAR Analysis: {}\n", analysis.jar_name);

    println!("## Overview\n");
    println!("| Property | Value |");
    println!("|----------|-------|");
    println!("| Path | `{}` |", analysis.jar_path);
    println!("| Size | {} |", format_bytes(analysis.size_bytes));
    println!("| Entries | {} |", analysis.entry_count);
    println!("| Classes | {} |", analysis.classes.len());
    println!("| Packages | {} |", analysis.package_names.len());
    println!();

    if analysis.metadata.main_class.is_some()
        || analysis.metadata.implementation_version.is_some()
    {
        println!("## Metadata\n");
        println!("| Key | Value |");
        println!("|-----|-------|");
        if let Some(v) = &analysis.metadata.main_class {
            println!("| Main-Class | `{}` |", v);
        }
        if let Some(v) = &analysis.metadata.implementation_title {
            println!("| Title | {} |", v);
        }
        if let Some(v) = &analysis.metadata.implementation_version {
            println!("| Version | {} |", v);
        }
        if let Some(v) = &analysis.metadata.jdk_version {
            println!("| JDK | {} |", v);
        }
        if let Some(v) = &analysis.metadata.build_tool {
            println!("| Build Tool | {} |", v);
        }
        println!();
    }

    if !analysis.detections.is_empty() {
        println!("## Detected Technologies\n");
        println!("| Technology | Category | Confidence |");
        println!("|------------|----------|------------|");
        for d in &analysis.detections {
            println!(
                "| {} | {} | {} |",
                d.name,
                d.category,
                d.confidence
            );
        }
        println!();
    }

    if !analysis.config_files.is_empty() {
        println!("## Configuration Files\n");
        for config in &analysis.config_files {
            println!("### `{}`\n", config.path);
            println!("Type: {}\n", config.file_type);
            if options.show_configs {
                if let Some(content) = &config.content {
                    println!("```");
                    println!("{}", content);
                    println!("```\n");
                }
            }
        }
    }

    if options.show_classes && !analysis.classes.is_empty() {
        println!("## Classes\n");
        println!("| Class | Type | Java Version |");
        println!("|-------|------|--------------|");
        for class in analysis.classes.iter().take(100) {
            println!("| `{}` | {} | {} |", class.name, class.class_type, class.java_version);
        }
        if analysis.classes.len() > 100 {
            println!("\n*... and {} more classes*\n", analysis.classes.len() - 100);
        }
    }
}

/// Get emoji for technology category (string-based)
fn category_emoji_str(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "framework" => "🏗️",
        "orm" => "🗄️",
        "web" => "🌐",
        "serialization" => "📦",
        "utility" => "🔧",
        "logging" => "📝",
        "testing" => "🧪",
        "messaging" => "📨",
        "caching" => "⚡",
        "validation" => "✅",
        "security" => "🔐",
        "scheduling" => "⏰",
        "http" => "🌐",
        _ => "📎",
    }
}

/// Extract config files to output directory
fn extract_configs(analysis: &JarAnalysis, output_dir: &PathBuf) -> Result<usize> {
    std::fs::create_dir_all(output_dir)?;

    let mut extracted = 0;
    for config in &analysis.config_files {
        if let Some(content) = &config.content {
            // Create subdirectories if needed
            let file_path = output_dir.join(&config.path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, content)?;
            extracted += 1;
        }
    }

    Ok(extracted)
}

/// Execute the jar-reverse command
pub fn run(options: JarReverseOptions) -> Result<()> {
    // Validate JAR path
    if !options.jar_path.exists() {
        bail!("JAR file not found: {}", options.jar_path.display());
    }

    if !options.jar_path.is_file() {
        bail!("Not a file: {}", options.jar_path.display());
    }

    let extension = options
        .jar_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if !["jar", "war", "ear", "zip"].contains(&extension.to_lowercase().as_str()) {
        println!("⚠️  Warning: File does not have a JAR extension, proceeding anyway...");
    }

    println!("📂 Analyzing: {}\n", options.jar_path.display());

    // Run analysis
    let analysis = JarAnalyzer::analyze(&options.jar_path, options.max_classes)
        .context("Failed to analyze JAR file")?;

    // Output results
    match options.format {
        OutputFormat::Text => print_text_analysis(&analysis, &options),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&analysis)?;
            println!("{}", json);
        }
        OutputFormat::Markdown => print_markdown_analysis(&analysis, &options),
    }

    // Extract configs if requested
    if options.extract_configs {
        if let Some(output_dir) = &options.output_dir {
            let extracted = extract_configs(&analysis, output_dir)?;
            println!(
                "📤 Extracted {} config files to: {}",
                extracted,
                output_dir.display()
            );
        } else {
            println!("⚠️  Cannot extract configs: no output directory specified");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_output_format_parse() {
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("md".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
    }
}
