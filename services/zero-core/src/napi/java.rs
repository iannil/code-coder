//! NAPI bindings for Java/JAR analysis module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - JarAnalyzerHandle: JAR file analysis
//! - FingerprintEngineHandle: Technology detection
//! - Utility functions: parse_class_file_sync

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::java::{
    analyzer::{
        ClassAnalysis as RustClassAnalysis, ConfigFileInfo as RustConfigFileInfo,
        DependencyInfo as RustDependencyInfo, JarAnalysis as RustJarAnalysis,
        JarAnalyzer as RustJarAnalyzer, JarMetadata as RustJarMetadata,
        PackageAnalysis as RustPackageAnalysis,
    },
    classfile::{parse_class_file, ClassInfo as RustClassInfo},
    fingerprint::{
        Detection as RustDetection, FingerprintCategory, FingerprintEngine as RustFingerprintEngine,
        FingerprintInput as RustFingerprintInput, JavaFingerprint as RustJavaFingerprint,
        FINGERPRINT_ENGINE,
    },
    jar::JarReader as RustJarReader,
};

// ============================================================================
// NAPI Types
// ============================================================================

/// JAR metadata from MANIFEST.MF
#[napi(object)]
pub struct NapiJarMetadata {
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

impl From<RustJarMetadata> for NapiJarMetadata {
    fn from(m: RustJarMetadata) -> Self {
        Self {
            main_class: m.main_class,
            implementation_title: m.implementation_title,
            implementation_version: m.implementation_version,
            implementation_vendor: m.implementation_vendor,
            specification_title: m.specification_title,
            specification_version: m.specification_version,
            build_tool: m.build_tool,
            jdk_version: m.jdk_version,
            created_by: m.created_by,
            bundle_name: m.bundle_name,
            bundle_version: m.bundle_version,
            bundle_symbolic_name: m.bundle_symbolic_name,
        }
    }
}

/// Class information
#[napi(object)]
pub struct NapiClassInfo {
    pub name: String,
    pub package_name: String,
    pub simple_name: String,
    pub class_type: String,
    pub modifiers: Vec<String>,
    pub bytecode_version: u32,
    pub java_version: String,
}

impl From<RustClassInfo> for NapiClassInfo {
    fn from(c: RustClassInfo) -> Self {
        Self {
            name: c.name,
            package_name: c.package_name,
            simple_name: c.simple_name,
            class_type: c.class_type.as_str().to_string(),
            modifiers: c.modifiers,
            bytecode_version: c.bytecode_version as u32,
            java_version: c.java_version,
        }
    }
}

impl From<RustClassAnalysis> for NapiClassInfo {
    fn from(c: RustClassAnalysis) -> Self {
        Self {
            name: c.name,
            package_name: c.package_name,
            simple_name: c.simple_name,
            class_type: c.class_type,
            modifiers: c.modifiers,
            bytecode_version: c.bytecode_version as u32,
            java_version: c.java_version,
        }
    }
}

/// Package analysis
#[napi(object)]
pub struct NapiJavaPackageInfo {
    pub name: String,
    pub class_count: u32,
}

impl From<RustPackageAnalysis> for NapiJavaPackageInfo {
    fn from(p: RustPackageAnalysis) -> Self {
        Self {
            name: p.name,
            class_count: p.class_count as u32,
        }
    }
}

/// Configuration file info
#[napi(object)]
pub struct NapiJavaConfigFile {
    pub path: String,
    pub file_type: String,
    pub content: Option<String>,
}

impl From<RustConfigFileInfo> for NapiJavaConfigFile {
    fn from(c: RustConfigFileInfo) -> Self {
        Self {
            path: c.path,
            file_type: c.file_type,
            content: c.content,
        }
    }
}

/// Dependency info
#[napi(object)]
pub struct NapiDependency {
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub version: Option<String>,
    pub scope: Option<String>,
}

impl From<RustDependencyInfo> for NapiDependency {
    fn from(d: RustDependencyInfo) -> Self {
        Self {
            group_id: d.group_id,
            artifact_id: d.artifact_id,
            version: d.version,
            scope: d.scope,
        }
    }
}

/// Technology detection result
#[napi(object)]
pub struct NapiDetection {
    pub name: String,
    pub category: String,
    pub website: Option<String>,
    pub matches: Vec<String>,
    pub confidence: String,
}

impl From<RustDetection> for NapiDetection {
    fn from(d: RustDetection) -> Self {
        Self {
            name: d.name,
            category: d.category,
            website: d.website,
            matches: d.matches,
            confidence: d.confidence,
        }
    }
}

/// Complete JAR analysis result
#[napi(object)]
pub struct NapiJarAnalysis {
    pub jar_path: String,
    pub jar_name: String,
    pub metadata: NapiJarMetadata,
    pub class_names: Vec<String>,
    pub package_names: Vec<String>,
    pub packages: Vec<NapiJavaPackageInfo>,
    pub classes: Vec<NapiClassInfo>,
    pub config_files: Vec<NapiJavaConfigFile>,
    pub dependencies: Vec<NapiDependency>,
    pub detections: Vec<NapiDetection>,
    pub entry_count: u32,
    pub size_bytes: i64,
}

impl From<RustJarAnalysis> for NapiJarAnalysis {
    fn from(a: RustJarAnalysis) -> Self {
        Self {
            jar_path: a.jar_path,
            jar_name: a.jar_name,
            metadata: a.metadata.into(),
            class_names: a.class_names,
            package_names: a.package_names,
            packages: a.packages.into_iter().map(|p| p.into()).collect(),
            classes: a.classes.into_iter().map(|c| c.into()).collect(),
            config_files: a.config_files.into_iter().map(|c| c.into()).collect(),
            dependencies: a.dependencies.into_iter().map(|d| d.into()).collect(),
            detections: a.detections.into_iter().map(|d| d.into()).collect(),
            entry_count: a.entry_count as u32,
            size_bytes: a.size_bytes as i64,
        }
    }
}

/// Java fingerprint info
#[napi(object)]
pub struct NapiJavaFingerprint {
    pub name: String,
    pub category: String,
    pub website: Option<String>,
    pub pattern_count: u32,
}

impl From<&RustJavaFingerprint> for NapiJavaFingerprint {
    fn from(fp: &RustJavaFingerprint) -> Self {
        Self {
            name: fp.name.clone(),
            category: fp.category.as_str().to_string(),
            website: fp.website.clone(),
            pattern_count: fp.patterns.len() as u32,
        }
    }
}

/// Input for fingerprint detection
#[napi(object)]
pub struct NapiFingerprintInput {
    pub class_names: Option<Vec<String>>,
    pub package_names: Option<Vec<String>>,
    pub config_files: Option<Vec<String>>,
    pub annotations: Option<Vec<String>>,
    pub manifest: Option<HashMap<String, String>>,
}

impl From<NapiFingerprintInput> for RustFingerprintInput {
    fn from(i: NapiFingerprintInput) -> Self {
        Self {
            class_names: i.class_names.unwrap_or_default(),
            package_names: i.package_names.unwrap_or_default(),
            config_files: i.config_files.unwrap_or_default(),
            annotations: i.annotations.unwrap_or_default(),
            manifest: i.manifest.unwrap_or_default(),
        }
    }
}

// ============================================================================
// JarAnalyzerHandle
// ============================================================================

/// Thread-safe JAR analyzer handle
#[napi]
pub struct JarAnalyzerHandle {
    inner: Arc<Mutex<Option<RustJarReader>>>,
    path: String,
}

#[napi]
impl JarAnalyzerHandle {
    /// Open a JAR file for analysis
    #[napi(factory)]
    pub fn open(path: String) -> Result<Self> {
        let reader = RustJarReader::open(&path).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(Some(reader))),
            path,
        })
    }

    /// Get the JAR file path
    #[napi(getter)]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Get the number of entries in the JAR
    #[napi(getter)]
    pub fn entry_count(&self) -> Result<u32> {
        let guard = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let reader = guard.as_ref().ok_or_else(|| Error::from_reason("JAR reader closed"))?;
        Ok(reader.len() as u32)
    }

    /// Analyze the JAR and return complete results
    #[napi]
    pub fn analyze(&self, max_classes: Option<u32>) -> Result<NapiJarAnalysis> {
        let result = RustJarAnalyzer::analyze(&self.path, max_classes.map(|m| m as usize))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(result.into())
    }

    /// Get a summary string
    #[napi]
    pub fn summary(&self) -> Result<String> {
        let result = RustJarAnalyzer::analyze(&self.path, None)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(RustJarAnalyzer::summary(&result))
    }

    /// List all class file paths
    #[napi]
    pub fn class_file_paths(&self) -> Result<Vec<String>> {
        let mut guard = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let reader = guard.as_mut().ok_or_else(|| Error::from_reason("JAR reader closed"))?;
        Ok(reader.class_file_paths())
    }

    /// List all config file paths
    #[napi]
    pub fn config_file_paths(&self) -> Result<Vec<String>> {
        let mut guard = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let reader = guard.as_mut().ok_or_else(|| Error::from_reason("JAR reader closed"))?;
        Ok(reader.find_config_files())
    }

    /// Close the JAR reader
    #[napi]
    pub fn close(&self) -> Result<()> {
        let mut guard = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        *guard = None;
        Ok(())
    }
}

// ============================================================================
// FingerprintEngineHandle
// ============================================================================

/// Thread-safe fingerprint engine handle
#[napi]
pub struct FingerprintEngineHandle {
    inner: Arc<RustFingerprintEngine>,
}

#[napi]
impl FingerprintEngineHandle {
    /// Create a new fingerprint engine
    #[napi(factory)]
    pub fn create() -> Self {
        Self {
            inner: Arc::new(RustFingerprintEngine::new()),
        }
    }

    /// Detect technologies from input
    #[napi]
    pub fn detect(&self, input: NapiFingerprintInput) -> Vec<NapiDetection> {
        let rust_input: RustFingerprintInput = input.into();
        self.inner
            .detect(&rust_input)
            .into_iter()
            .map(|d| d.into())
            .collect()
    }

    /// Get all fingerprint definitions
    #[napi]
    pub fn fingerprints(&self) -> Vec<NapiJavaFingerprint> {
        self.inner
            .fingerprints()
            .iter()
            .map(|fp| fp.into())
            .collect()
    }

    /// Get fingerprints by category
    #[napi]
    pub fn fingerprints_by_category(&self, category: String) -> Vec<NapiJavaFingerprint> {
        let cat = match category.as_str() {
            "framework" => FingerprintCategory::Framework,
            "orm" => FingerprintCategory::Orm,
            "web" => FingerprintCategory::Web,
            "serialization" => FingerprintCategory::Serialization,
            "utility" => FingerprintCategory::Utility,
            "logging" => FingerprintCategory::Logging,
            "testing" => FingerprintCategory::Testing,
            "messaging" => FingerprintCategory::Messaging,
            "caching" => FingerprintCategory::Caching,
            "validation" => FingerprintCategory::Validation,
            "security" => FingerprintCategory::Security,
            "scheduling" => FingerprintCategory::Scheduling,
            "http" => FingerprintCategory::Http,
            _ => return Vec::new(),
        };

        self.inner
            .fingerprints_by_category(cat)
            .into_iter()
            .map(|fp| fp.into())
            .collect()
    }

    /// Get all category names
    #[napi]
    pub fn categories(&self) -> Vec<String> {
        self.inner
            .categories()
            .into_iter()
            .map(|c| c.as_str().to_string())
            .collect()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Analyze a JAR file (standalone function)
#[napi]
pub fn analyze_jar(jar_path: String, max_classes: Option<u32>) -> Result<NapiJarAnalysis> {
    let result = RustJarAnalyzer::analyze(&jar_path, max_classes.map(|m| m as usize))
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(result.into())
}

/// Parse a class file from bytes
#[napi]
pub fn parse_class_file_sync(data: Buffer) -> Result<NapiClassInfo> {
    let bytes: &[u8] = &data;
    let class_file = parse_class_file(bytes).map_err(|e| Error::from_reason(e.to_string()))?;
    let class_info: RustClassInfo = class_file.into();
    Ok(class_info.into())
}

/// Detect technologies using the global fingerprint engine
#[napi]
pub fn detect_java_technologies(input: NapiFingerprintInput) -> Vec<NapiDetection> {
    let rust_input: RustFingerprintInput = input.into();
    FINGERPRINT_ENGINE
        .detect(&rust_input)
        .into_iter()
        .map(|d| d.into())
        .collect()
}

/// Get JAR analysis summary
#[napi]
pub fn jar_analysis_summary(jar_path: String) -> Result<String> {
    let result = RustJarAnalyzer::analyze(&jar_path, None)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(RustJarAnalyzer::summary(&result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_engine_handle() {
        let engine = FingerprintEngineHandle::create();
        let categories = engine.categories();
        assert!(!categories.is_empty());
        assert!(categories.contains(&"framework".to_string()));
    }

    #[test]
    fn test_detect_technologies() {
        let input = NapiFingerprintInput {
            package_names: Some(vec!["org.springframework.boot".to_string()]),
            class_names: None,
            config_files: None,
            annotations: None,
            manifest: None,
        };

        let detections = detect_java_technologies(input);
        assert!(detections.iter().any(|d| d.name == "Spring Boot"));
    }

    #[test]
    fn test_fingerprints_by_category() {
        let engine = FingerprintEngineHandle::create();
        let frameworks = engine.fingerprints_by_category("framework".to_string());
        assert!(!frameworks.is_empty());
        assert!(frameworks.iter().any(|fp| fp.name == "Spring Boot"));
    }
}
