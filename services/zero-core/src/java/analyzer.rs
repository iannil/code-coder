//! High-level JAR analyzer
//!
//! Combines class file parsing, JAR extraction, and fingerprint detection
//! into a unified API matching the TypeScript JarAnalyzer interface.
//!
//! # Example
//!
//! ```ignore
//! let result = JarAnalyzer::analyze("myapp.jar", None)?;
//! println!("Classes: {}, Technologies: {}", result.classes.len(), result.detections.len());
//! ```

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::classfile::ClassInfo;
use super::fingerprint::{Detection, FingerprintInput, FINGERPRINT_ENGINE};
use super::jar::{parse_manifest, JarReader};

/// Errors during JAR analysis
#[derive(Error, Debug)]
pub enum AnalyzerError {
    #[error("JAR error: {0}")]
    Jar(#[from] super::jar::JarError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// JAR metadata extracted from MANIFEST.MF
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JarMetadata {
    pub main_class: Option<String>,
    pub implementation_title: Option<String>,
    pub implementation_version: Option<String>,
    pub implementation_vendor: Option<String>,
    pub specification_title: Option<String>,
    pub specification_version: Option<String>,
    pub build_tool: Option<String>,
    pub jdk_version: Option<String>,
    pub created_by: Option<String>,
    pub bundle_name: Option<String>,
    pub bundle_version: Option<String>,
    pub bundle_symbolic_name: Option<String>,
}

impl JarMetadata {
    /// Parse metadata from manifest key-value pairs
    pub fn from_manifest(manifest: &HashMap<String, String>) -> Self {
        let mut metadata = Self::default();

        for (key, value) in manifest {
            match key.as_str() {
                "Main-Class" => metadata.main_class = Some(value.clone()),
                "Implementation-Title" => metadata.implementation_title = Some(value.clone()),
                "Implementation-Version" => metadata.implementation_version = Some(value.clone()),
                "Implementation-Vendor" => metadata.implementation_vendor = Some(value.clone()),
                "Specification-Title" => metadata.specification_title = Some(value.clone()),
                "Specification-Version" => metadata.specification_version = Some(value.clone()),
                "Created-By" => metadata.created_by = Some(value.clone()),
                "Build-Jdk" | "Build-Jdk-Spec" => metadata.jdk_version = Some(value.clone()),
                "Bundle-Name" => metadata.bundle_name = Some(value.clone()),
                "Bundle-Version" => metadata.bundle_version = Some(value.clone()),
                "Bundle-SymbolicName" => metadata.bundle_symbolic_name = Some(value.clone()),
                "Bundle-ManifestVersion" => metadata.build_tool = Some("OSGi".to_string()),
                "Archiver-Version" => {
                    if value.contains("Maven") {
                        metadata.build_tool = Some("Maven".to_string());
                    }
                }
                "Gradle-Version" => metadata.build_tool = Some("Gradle".to_string()),
                _ => {}
            }
        }

        metadata
    }
}

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAnalysis {
    pub name: String,
    pub class_count: usize,
}

/// Class analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassAnalysis {
    pub name: String,
    pub package_name: String,
    pub simple_name: String,
    pub class_type: String,
    pub modifiers: Vec<String>,
    pub bytecode_version: u16,
    pub java_version: String,
}

impl From<ClassInfo> for ClassAnalysis {
    fn from(ci: ClassInfo) -> Self {
        Self {
            name: ci.name,
            package_name: ci.package_name,
            simple_name: ci.simple_name,
            class_type: ci.class_type.as_str().to_string(),
            modifiers: ci.modifiers,
            bytecode_version: ci.bytecode_version,
            java_version: ci.java_version,
        }
    }
}

/// Configuration file info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileInfo {
    pub path: String,
    pub file_type: String,
    pub content: Option<String>,
}

impl ConfigFileInfo {
    pub fn detect_type(path: &str) -> String {
        let path_lower = path.to_lowercase();
        let file_name = path.rsplit('/').next().unwrap_or(path);

        if path_lower.ends_with(".properties") {
            "properties".to_string()
        } else if path_lower.ends_with(".yml") || path_lower.ends_with(".yaml") {
            "yaml".to_string()
        } else if path_lower.ends_with(".xml") {
            if file_name.starts_with("spring") || file_name == "applicationContext.xml" {
                "spring-config".to_string()
            } else if file_name == "web.xml" {
                "web-config".to_string()
            } else if file_name == "persistence.xml" {
                "jpa-config".to_string()
            } else if file_name.starts_with("hibernate") {
                "hibernate-config".to_string()
            } else {
                "xml".to_string()
            }
        } else {
            "unknown".to_string()
        }
    }
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub version: Option<String>,
    pub scope: Option<String>,
}

/// Complete JAR analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JarAnalysis {
    /// Path to the JAR file
    pub jar_path: String,
    /// JAR file name
    pub jar_name: String,
    /// Extracted metadata
    pub metadata: JarMetadata,
    /// All class names (fully qualified)
    pub class_names: Vec<String>,
    /// All package names (unique)
    pub package_names: Vec<String>,
    /// Package analysis with class counts
    pub packages: Vec<PackageAnalysis>,
    /// Parsed class information
    pub classes: Vec<ClassAnalysis>,
    /// Configuration files found
    pub config_files: Vec<ConfigFileInfo>,
    /// Maven dependencies (from pom.properties)
    pub dependencies: Vec<DependencyInfo>,
    /// Detected technologies
    pub detections: Vec<Detection>,
    /// Total entry count
    pub entry_count: usize,
    /// File size in bytes
    pub size_bytes: u64,
}

/// JAR analyzer with configurable options
pub struct JarAnalyzer;

impl JarAnalyzer {
    /// Analyze a JAR file
    ///
    /// # Arguments
    /// * `jar_path` - Path to the JAR file
    /// * `max_classes` - Maximum number of classes to parse (None = unlimited)
    ///
    /// # Returns
    /// Complete analysis result including classes, technologies, and metadata
    pub fn analyze<P: AsRef<Path>>(
        jar_path: P,
        max_classes: Option<usize>,
    ) -> Result<JarAnalysis, AnalyzerError> {
        let path = jar_path.as_ref();
        let jar_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown.jar".to_string());
        let jar_path_str = path.to_string_lossy().to_string();

        // Get file size
        let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        // Open JAR
        let mut reader = JarReader::open(path)?;
        let entry_count = reader.len();

        // Parse manifest
        let manifest_content = reader.read_manifest();
        let manifest_map = manifest_content
            .as_ref()
            .map(|c| parse_manifest(c))
            .unwrap_or_default();
        let metadata = JarMetadata::from_manifest(&manifest_map);

        // Parse classes (parallel)
        let classes: Vec<ClassAnalysis> = reader
            .parse_classes_parallel(max_classes)?
            .into_iter()
            .map(|ci| ci.into())
            .collect();

        // Extract class and package names
        let class_names: Vec<String> = classes.iter().map(|c| c.name.clone()).collect();
        let package_names: Vec<String> = classes
            .iter()
            .map(|c| c.package_name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Build package analysis
        let mut package_counts: HashMap<String, usize> = HashMap::new();
        for class in &classes {
            *package_counts.entry(class.package_name.clone()).or_insert(0) += 1;
        }
        let mut packages: Vec<PackageAnalysis> = package_counts
            .into_iter()
            .map(|(name, class_count)| PackageAnalysis { name, class_count })
            .collect();
        packages.sort_by(|a, b| a.name.cmp(&b.name));

        // Read config files
        let config_paths = reader.find_config_files();
        let config_contents = reader.read_config_files();
        let config_files: Vec<ConfigFileInfo> = config_paths
            .into_iter()
            .map(|path| ConfigFileInfo {
                file_type: ConfigFileInfo::detect_type(&path),
                content: config_contents.get(&path).and_then(|c| c.clone()),
                path,
            })
            .collect();

        // Detect technologies using the global fingerprint engine
        let detections = FINGERPRINT_ENGINE.detect(&FingerprintInput {
            class_names: class_names.clone(),
            package_names: package_names.clone(),
            config_files: config_files.iter().map(|c| c.path.clone()).collect(),
            manifest: manifest_map,
            ..Default::default()
        });

        // TODO: Extract Maven dependencies from META-INF/maven/**/pom.properties
        let dependencies = Vec::new();

        Ok(JarAnalysis {
            jar_path: jar_path_str,
            jar_name,
            metadata,
            class_names,
            package_names,
            packages,
            classes,
            config_files,
            dependencies,
            detections,
            entry_count,
            size_bytes,
        })
    }

    /// Get a summary string of the analysis
    pub fn summary(analysis: &JarAnalysis) -> String {
        let mut lines = Vec::new();

        lines.push(format!("JAR: {}", analysis.jar_name));
        lines.push(format!("Size: {}", format_bytes(analysis.size_bytes)));
        lines.push(format!("Entries: {}", analysis.entry_count));
        lines.push(format!("Classes: {}", analysis.classes.len()));
        lines.push(format!("Packages: {}", analysis.packages.len()));

        if let Some(ref main_class) = analysis.metadata.main_class {
            lines.push(format!("Main Class: {}", main_class));
        }

        if let Some(ref build_tool) = analysis.metadata.build_tool {
            lines.push(format!("Build Tool: {}", build_tool));
        }

        if let Some(ref version) = analysis.metadata.implementation_version {
            lines.push(format!("Version: {}", version));
        }

        let tech_count = analysis.detections.len();
        if tech_count > 0 {
            lines.push(format!("Detected Technologies: {}", tech_count));
        }

        lines.join("\n")
    }
}

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
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_jar() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());

        // Add a Spring Boot class
        let class_bytes = create_spring_class();
        let options = SimpleFileOptions::default();
        zip.start_file("org/springframework/boot/SpringApplication.class", options)
            .unwrap();
        zip.write_all(&class_bytes).unwrap();

        // Add manifest
        zip.start_file("META-INF/MANIFEST.MF", options).unwrap();
        zip.write_all(
            b"Manifest-Version: 1.0\nMain-Class: com.example.Main\nImplementation-Version: 1.0.0\n",
        )
        .unwrap();

        // Add application.properties
        zip.start_file("application.properties", options).unwrap();
        zip.write_all(b"spring.application.name=test\n").unwrap();

        zip.finish().unwrap();
        file
    }

    fn create_spring_class() -> Vec<u8> {
        let name = "org/springframework/boot/SpringApplication";
        let mut bytes = Vec::new();

        // Magic
        bytes.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        // Version (Java 17)
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&61u16.to_be_bytes());
        // Constant pool count (3)
        bytes.extend_from_slice(&3u16.to_be_bytes());

        // UTF8 entry with class name
        bytes.push(1);
        bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());

        // Class entry pointing to UTF8
        bytes.push(7);
        bytes.extend_from_slice(&1u16.to_be_bytes());

        // Access flags, this_class, super_class, interfaces_count
        bytes.extend_from_slice(&0x0001u16.to_be_bytes()); // public
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());

        bytes
    }

    #[test]
    fn test_analyze_jar() {
        let jar = create_test_jar();
        let result = JarAnalyzer::analyze(jar.path(), None).unwrap();

        assert_eq!(result.jar_name, jar.path().file_name().unwrap().to_string_lossy());
        assert_eq!(result.classes.len(), 1);
        assert!(result
            .classes
            .iter()
            .any(|c| c.name == "org.springframework.boot.SpringApplication"));
    }

    #[test]
    fn test_detect_spring_boot() {
        let jar = create_test_jar();
        let result = JarAnalyzer::analyze(jar.path(), None).unwrap();

        assert!(result.detections.iter().any(|d| d.name == "Spring Boot"));
    }

    #[test]
    fn test_metadata_extraction() {
        let jar = create_test_jar();
        let result = JarAnalyzer::analyze(jar.path(), None).unwrap();

        assert_eq!(
            result.metadata.main_class,
            Some("com.example.Main".to_string())
        );
        assert_eq!(
            result.metadata.implementation_version,
            Some("1.0.0".to_string())
        );
    }

    #[test]
    fn test_config_files() {
        let jar = create_test_jar();
        let result = JarAnalyzer::analyze(jar.path(), None).unwrap();

        assert!(result
            .config_files
            .iter()
            .any(|c| c.path == "application.properties"));
    }

    #[test]
    fn test_summary() {
        let jar = create_test_jar();
        let result = JarAnalyzer::analyze(jar.path(), None).unwrap();
        let summary = JarAnalyzer::summary(&result);

        assert!(summary.contains("Classes: 1"));
        assert!(summary.contains("Main Class: com.example.Main"));
    }

    #[test]
    fn test_max_classes() {
        let jar = create_test_jar();
        let result = JarAnalyzer::analyze(jar.path(), Some(0)).unwrap();

        assert_eq!(result.classes.len(), 0);
    }
}
