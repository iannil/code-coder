//! Project Fingerprinting
//!
//! Detects project type, frameworks, build tools, test frameworks,
//! and other characteristics for contextual understanding.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

// ============================================================================
// Types
// ============================================================================

/// Project language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectLanguage {
    TypeScript,
    JavaScript,
    Python,
    Go,
    Rust,
    Java,
    CSharp,
    Other,
}

impl Default for ProjectLanguage {
    fn default() -> Self {
        Self::Other
    }
}

/// Package manager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Bun,
    Yarn,
    Pnpm,
    Pip,
    Poetry,
    Cargo,
    Go,
    Maven,
    Gradle,
    Nuget,
    Unknown,
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Framework type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameworkType {
    Frontend,
    Backend,
    Fullstack,
    Mobile,
    Desktop,
    Cli,
    Library,
}

/// Framework information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkInfo {
    /// Framework name
    pub name: String,
    /// Framework version (if detected)
    pub version: Option<String>,
    /// Framework type
    #[serde(rename = "type")]
    pub framework_type: FrameworkType,
}

/// Build tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildToolInfo {
    /// Tool name
    pub name: String,
    /// Config file path (if found)
    pub config: Option<String>,
    /// Version (if detected)
    pub version: Option<String>,
}

/// Test framework information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFrameworkInfo {
    /// Framework name
    pub name: String,
    /// Config file path (if found)
    pub config: Option<String>,
    /// Test runner command (if detected)
    pub runner: Option<String>,
}

/// Package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: Option<String>,
    /// Package version
    pub version: Option<String>,
    /// Package manager
    pub manager: PackageManager,
}

impl Default for PackageInfo {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            manager: PackageManager::Unknown,
        }
    }
}

/// Configuration file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    /// File path
    pub path: String,
    /// File name
    pub name: String,
}

/// Directory structure information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryInfo {
    /// Source directory
    pub src: Option<String>,
    /// Components directory
    pub components: Option<String>,
    /// Pages directory
    pub pages: Option<String>,
    /// Routes directory
    pub routes: Option<String>,
    /// Test directories
    pub tests: Vec<String>,
    /// Library directory
    pub lib: Option<String>,
    /// Distribution directory
    pub dist: Option<String>,
    /// Build directory
    pub build: Option<String>,
    /// Public assets directory
    pub public: Option<String>,
}

/// Full fingerprint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintInfo {
    /// Project identifier (hash)
    #[serde(rename = "projectId")]
    pub project_id: String,
    /// Detected frameworks
    pub frameworks: Vec<FrameworkInfo>,
    /// Detected build tools
    #[serde(rename = "buildTools")]
    pub build_tools: Vec<BuildToolInfo>,
    /// Detected test frameworks
    #[serde(rename = "testFrameworks")]
    pub test_frameworks: Vec<TestFrameworkInfo>,
    /// Package information
    pub package: PackageInfo,
    /// Configuration files
    pub configs: Vec<ConfigFile>,
    /// Primary language
    pub language: ProjectLanguage,
    /// Whether TypeScript is used
    #[serde(rename = "hasTypeScript")]
    pub has_typescript: bool,
    /// Language version (if detected)
    #[serde(rename = "languageVersion")]
    pub language_version: Option<String>,
    /// Directory structure
    pub directories: DirectoryInfo,
    /// Fingerprint hash
    pub hash: String,
}

// ============================================================================
// Pattern Definitions
// ============================================================================

/// Framework detection pattern
struct FrameworkPattern {
    name: &'static str,
    dependencies: &'static [&'static str],
    framework_type: FrameworkType,
}

const FRAMEWORK_PATTERNS: &[FrameworkPattern] = &[
    FrameworkPattern { name: "React", dependencies: &["react", "react-dom"], framework_type: FrameworkType::Frontend },
    FrameworkPattern { name: "Vue", dependencies: &["vue"], framework_type: FrameworkType::Frontend },
    FrameworkPattern { name: "Svelte", dependencies: &["svelte"], framework_type: FrameworkType::Frontend },
    FrameworkPattern { name: "Angular", dependencies: &["@angular/core"], framework_type: FrameworkType::Frontend },
    FrameworkPattern { name: "Next.js", dependencies: &["next"], framework_type: FrameworkType::Fullstack },
    FrameworkPattern { name: "Nuxt", dependencies: &["nuxt"], framework_type: FrameworkType::Fullstack },
    FrameworkPattern { name: "Remix", dependencies: &["@remix-run/react"], framework_type: FrameworkType::Fullstack },
    FrameworkPattern { name: "SvelteKit", dependencies: &["@sveltejs/kit"], framework_type: FrameworkType::Fullstack },
    FrameworkPattern { name: "Astro", dependencies: &["astro"], framework_type: FrameworkType::Frontend },
    FrameworkPattern { name: "NestJS", dependencies: &["@nestjs/core"], framework_type: FrameworkType::Backend },
    FrameworkPattern { name: "Express", dependencies: &["express"], framework_type: FrameworkType::Backend },
    FrameworkPattern { name: "Fastify", dependencies: &["fastify"], framework_type: FrameworkType::Backend },
    FrameworkPattern { name: "Hono", dependencies: &["hono"], framework_type: FrameworkType::Backend },
    FrameworkPattern { name: "Electron", dependencies: &["electron"], framework_type: FrameworkType::Desktop },
    FrameworkPattern { name: "Tauri", dependencies: &["@tauri-apps/api"], framework_type: FrameworkType::Desktop },
    FrameworkPattern { name: "React Native", dependencies: &["react-native"], framework_type: FrameworkType::Mobile },
    FrameworkPattern { name: "Expo", dependencies: &["expo"], framework_type: FrameworkType::Mobile },
];

/// Build tool detection pattern
struct BuildToolPattern {
    name: &'static str,
    config_files: &'static [&'static str],
    package_key: &'static str,
}

const BUILD_TOOL_PATTERNS: &[BuildToolPattern] = &[
    BuildToolPattern { name: "Vite", config_files: &["vite.config.ts", "vite.config.js", "vite.config.mjs"], package_key: "vite" },
    BuildToolPattern { name: "Webpack", config_files: &["webpack.config.js", "webpack.config.ts"], package_key: "webpack" },
    BuildToolPattern { name: "Rollup", config_files: &["rollup.config.js", "rollup.config.ts"], package_key: "rollup" },
    BuildToolPattern { name: "esbuild", config_files: &["esbuild.config.js", "esbuild.js", "esbuild.ts", "esbuild.mjs"], package_key: "esbuild" },
    BuildToolPattern { name: "Turborepo", config_files: &["turbo.json"], package_key: "turbo" },
    BuildToolPattern { name: "Babel", config_files: &[".babelrc", ".babelrc.js", "babel.config.js", "babel.config.json"], package_key: "@babel/core" },
    BuildToolPattern { name: "SWC", config_files: &[".swcrc"], package_key: "@swc/core" },
    BuildToolPattern { name: "TailwindCSS", config_files: &["tailwind.config.js", "tailwind.config.ts", "tailwind.config.cjs"], package_key: "tailwindcss" },
];

/// Test framework detection pattern
struct TestFrameworkPattern {
    name: &'static str,
    config_files: &'static [&'static str],
    package_key: &'static str,
}

const TEST_FRAMEWORK_PATTERNS: &[TestFrameworkPattern] = &[
    TestFrameworkPattern { name: "Jest", config_files: &["jest.config.js", "jest.config.ts", "jest.config.json", ".jestrc"], package_key: "jest" },
    TestFrameworkPattern { name: "Vitest", config_files: &["vitest.config.ts", "vitest.config.js"], package_key: "vitest" },
    TestFrameworkPattern { name: "Mocha", config_files: &[".mocharc.js", ".mocharc.json", "mocha.opts"], package_key: "mocha" },
    TestFrameworkPattern { name: "Cypress", config_files: &["cypress.config.ts", "cypress.config.js"], package_key: "cypress" },
    TestFrameworkPattern { name: "Playwright", config_files: &["playwright.config.ts", "playwright.config.js"], package_key: "@playwright/test" },
];

const TEST_DIRECTORIES: &[&str] = &["test", "tests", "__tests__", "__test__", "spec", "specs", "e2e", "integration"];

// ============================================================================
// Fingerprint Implementation
// ============================================================================

/// Project fingerprinter
pub struct Fingerprint;

impl Fingerprint {
    /// Generate fingerprint for a project directory
    pub fn generate(root: &Path) -> Result<FingerprintInfo> {
        let project_id = Self::compute_project_id(root);

        // Read package.json if it exists
        let (package_info, dependencies) = Self::read_package_json(root);

        // Detect language
        let (language, has_typescript, language_version) = Self::detect_language(root, &dependencies);

        // Detect frameworks
        let frameworks = Self::detect_frameworks(&dependencies);

        // Detect build tools
        let build_tools = Self::detect_build_tools(root, &dependencies);

        // Detect test frameworks
        let test_frameworks = Self::detect_test_frameworks(root, &dependencies);

        // Detect directories
        let directories = Self::detect_directories(root);

        // Find config files
        let configs = Self::find_config_files(root);

        // Compute content hash for the fingerprint
        let hash = Self::compute_hash(&frameworks, &build_tools, &test_frameworks, &language);

        Ok(FingerprintInfo {
            project_id,
            frameworks,
            build_tools,
            test_frameworks,
            package: package_info,
            configs,
            language,
            has_typescript,
            language_version,
            directories,
            hash,
        })
    }

    /// Compute a project ID from the path
    fn compute_project_id(root: &Path) -> String {
        let path_str = root.to_string_lossy();
        format!("{:016x}", xxh3_64(path_str.as_bytes()))
    }

    /// Read package.json and extract dependencies
    fn read_package_json(root: &Path) -> (PackageInfo, HashMap<String, String>) {
        let package_json_path = root.join("package.json");
        let mut dependencies = HashMap::new();

        if !package_json_path.exists() {
            return (PackageInfo::default(), dependencies);
        }

        let content = match std::fs::read_to_string(&package_json_path) {
            Ok(c) => c,
            Err(_) => return (PackageInfo::default(), dependencies),
        };

        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(j) => j,
            Err(_) => return (PackageInfo::default(), dependencies),
        };

        // Extract package info
        let name = json.get("name").and_then(|v| v.as_str()).map(String::from);
        let version = json.get("version").and_then(|v| v.as_str()).map(String::from);

        // Detect package manager
        let manager = Self::detect_package_manager(root);

        // Collect all dependencies
        for dep_key in &["dependencies", "devDependencies", "peerDependencies"] {
            if let Some(deps) = json.get(*dep_key).and_then(|v| v.as_object()) {
                for (k, v) in deps {
                    if let Some(version) = v.as_str() {
                        dependencies.insert(k.clone(), version.to_string());
                    }
                }
            }
        }

        (
            PackageInfo {
                name,
                version,
                manager,
            },
            dependencies,
        )
    }

    /// Detect package manager from lock files
    fn detect_package_manager(root: &Path) -> PackageManager {
        if root.join("pnpm-lock.yaml").exists() {
            PackageManager::Pnpm
        } else if root.join("yarn.lock").exists() {
            PackageManager::Yarn
        } else if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
            PackageManager::Bun
        } else if root.join("package-lock.json").exists() {
            PackageManager::Npm
        } else if root.join("Cargo.lock").exists() {
            PackageManager::Cargo
        } else if root.join("go.sum").exists() {
            PackageManager::Go
        } else if root.join("poetry.lock").exists() {
            PackageManager::Poetry
        } else if root.join("requirements.txt").exists() || root.join("Pipfile").exists() {
            PackageManager::Pip
        } else {
            PackageManager::Unknown
        }
    }

    /// Detect primary language
    fn detect_language(
        root: &Path,
        dependencies: &HashMap<String, String>,
    ) -> (ProjectLanguage, bool, Option<String>) {
        // Check for TypeScript
        let tsconfig_exists = root.join("tsconfig.json").exists();
        if tsconfig_exists {
            let version = dependencies.get("typescript").cloned();
            return (ProjectLanguage::TypeScript, true, version);
        }

        // Check for other languages by markers
        if root.join("Cargo.toml").exists() {
            return (ProjectLanguage::Rust, false, None);
        }
        if root.join("go.mod").exists() {
            return (ProjectLanguage::Go, false, None);
        }
        if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
            return (ProjectLanguage::Python, false, None);
        }
        if root.join("pom.xml").exists() || root.join("build.gradle").exists() {
            return (ProjectLanguage::Java, false, None);
        }
        if root.join(".csproj").exists() || root.join(".sln").exists() {
            return (ProjectLanguage::CSharp, false, None);
        }
        if root.join("package.json").exists() {
            return (ProjectLanguage::JavaScript, false, None);
        }

        (ProjectLanguage::Other, false, None)
    }

    /// Detect frameworks from dependencies
    fn detect_frameworks(dependencies: &HashMap<String, String>) -> Vec<FrameworkInfo> {
        let mut frameworks = Vec::new();

        for pattern in FRAMEWORK_PATTERNS {
            for dep in pattern.dependencies {
                if let Some(version) = dependencies.get(*dep) {
                    frameworks.push(FrameworkInfo {
                        name: pattern.name.to_string(),
                        version: Some(version.clone()),
                        framework_type: pattern.framework_type,
                    });
                    break;
                }
            }
        }

        frameworks
    }

    /// Detect build tools
    fn detect_build_tools(root: &Path, dependencies: &HashMap<String, String>) -> Vec<BuildToolInfo> {
        let mut tools = Vec::new();

        for pattern in BUILD_TOOL_PATTERNS {
            let mut found = false;
            let mut config_path = None;

            // Check for config files
            for config_file in pattern.config_files {
                let path = root.join(config_file);
                if path.exists() {
                    config_path = Some(config_file.to_string());
                    found = true;
                    break;
                }
            }

            // Check in dependencies
            if !found && dependencies.contains_key(pattern.package_key) {
                found = true;
            }

            if found {
                tools.push(BuildToolInfo {
                    name: pattern.name.to_string(),
                    config: config_path,
                    version: dependencies.get(pattern.package_key).cloned(),
                });
            }
        }

        tools
    }

    /// Detect test frameworks
    fn detect_test_frameworks(root: &Path, dependencies: &HashMap<String, String>) -> Vec<TestFrameworkInfo> {
        let mut frameworks = Vec::new();

        for pattern in TEST_FRAMEWORK_PATTERNS {
            let mut found = dependencies.contains_key(pattern.package_key);
            let mut config_path = None;

            // Check for config files
            for config_file in pattern.config_files {
                let path = root.join(config_file);
                if path.exists() {
                    config_path = Some(config_file.to_string());
                    found = true;
                    break;
                }
            }

            if found {
                frameworks.push(TestFrameworkInfo {
                    name: pattern.name.to_string(),
                    config: config_path,
                    runner: None,
                });
            }
        }

        frameworks
    }

    /// Detect directory structure
    fn detect_directories(root: &Path) -> DirectoryInfo {
        let mut info = DirectoryInfo::default();

        // Check common directories
        if root.join("src").is_dir() {
            info.src = Some("src".to_string());
        }
        if root.join("components").is_dir() {
            info.components = Some("components".to_string());
        }
        if root.join("pages").is_dir() {
            info.pages = Some("pages".to_string());
        }
        if root.join("routes").is_dir() {
            info.routes = Some("routes".to_string());
        }
        if root.join("lib").is_dir() {
            info.lib = Some("lib".to_string());
        }
        if root.join("dist").is_dir() {
            info.dist = Some("dist".to_string());
        }
        if root.join("build").is_dir() {
            info.build = Some("build".to_string());
        }
        if root.join("public").is_dir() {
            info.public = Some("public".to_string());
        }

        // Check for test directories
        for test_dir in TEST_DIRECTORIES {
            let dir_path = root.join(test_dir);
            if dir_path.is_dir() {
                info.tests.push(test_dir.to_string());
            }
        }

        info
    }

    /// Find configuration files
    fn find_config_files(root: &Path) -> Vec<ConfigFile> {
        let mut configs = Vec::new();

        // Check common config files directly
        let common_configs = [
            "package.json",
            "tsconfig.json",
            "vite.config.ts",
            "next.config.js",
            "tailwind.config.js",
            ".eslintrc.json",
            ".prettierrc",
            "jest.config.js",
            "vitest.config.ts",
        ];

        for config_name in &common_configs {
            let path = root.join(config_name);
            if path.exists() {
                configs.push(ConfigFile {
                    path: path.to_string_lossy().to_string(),
                    name: config_name.to_string(),
                });
            }
        }

        configs
    }

    /// Compute hash of fingerprint data
    fn compute_hash(
        frameworks: &[FrameworkInfo],
        build_tools: &[BuildToolInfo],
        test_frameworks: &[TestFrameworkInfo],
        language: &ProjectLanguage,
    ) -> String {
        let mut data = String::new();

        for f in frameworks {
            data.push_str(&f.name);
        }
        for t in build_tools {
            data.push_str(&t.name);
        }
        for t in test_frameworks {
            data.push_str(&t.name);
        }
        data.push_str(&format!("{:?}", language));

        format!("{:016x}", xxh3_64(data.as_bytes()))
    }

    /// Compare two fingerprints for similarity (0.0 - 1.0)
    pub fn similarity(a: &FingerprintInfo, b: &FingerprintInfo) -> f64 {
        let mut score = 0.0;
        let mut weight = 0.0;

        // Language match (40% weight)
        weight += 0.4;
        if a.language == b.language {
            score += 0.4;
        }

        // Framework overlap (30% weight)
        weight += 0.3;
        let a_frameworks: std::collections::HashSet<_> = a.frameworks.iter().map(|f| &f.name).collect();
        let b_frameworks: std::collections::HashSet<_> = b.frameworks.iter().map(|f| &f.name).collect();
        let intersection = a_frameworks.intersection(&b_frameworks).count();
        let union = a_frameworks.union(&b_frameworks).count();
        if union > 0 {
            score += 0.3 * (intersection as f64 / union as f64);
        } else {
            // Both empty = perfect match for this category
            score += 0.3;
        }

        // Build tool overlap (15% weight)
        weight += 0.15;
        let a_tools: std::collections::HashSet<_> = a.build_tools.iter().map(|t| &t.name).collect();
        let b_tools: std::collections::HashSet<_> = b.build_tools.iter().map(|t| &t.name).collect();
        let intersection = a_tools.intersection(&b_tools).count();
        let union = a_tools.union(&b_tools).count();
        if union > 0 {
            score += 0.15 * (intersection as f64 / union as f64);
        } else {
            // Both empty = perfect match for this category
            score += 0.15;
        }

        // Test framework overlap (15% weight)
        weight += 0.15;
        let a_tests: std::collections::HashSet<_> = a.test_frameworks.iter().map(|t| &t.name).collect();
        let b_tests: std::collections::HashSet<_> = b.test_frameworks.iter().map(|t| &t.name).collect();
        let intersection = a_tests.intersection(&b_tests).count();
        let union = a_tests.union(&b_tests).count();
        if union > 0 {
            score += 0.15 * (intersection as f64 / union as f64);
        } else {
            // Both empty = perfect match for this category
            score += 0.15;
        }

        score / weight
    }

    /// Generate a human-readable description
    pub fn describe(fingerprint: &FingerprintInfo) -> String {
        let mut parts = Vec::new();

        // Language
        if fingerprint.language != ProjectLanguage::Other {
            parts.push(format!("{:?}", fingerprint.language));
        }

        // Frameworks
        if !fingerprint.frameworks.is_empty() {
            let names: Vec<_> = fingerprint.frameworks.iter().map(|f| f.name.as_str()).collect();
            parts.push(names.join(", "));
        }

        // Build tools
        if !fingerprint.build_tools.is_empty() {
            let names: Vec<_> = fingerprint.build_tools.iter().map(|t| t.name.as_str()).collect();
            parts.push(names.join(", "));
        }

        if parts.is_empty() {
            "Unknown project type".to_string()
        } else {
            parts.join(" • ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compute_project_id() {
        let path = Path::new("/test/project");
        let id = Fingerprint::compute_project_id(path);
        assert!(!id.is_empty());
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_detect_package_manager() {
        let dir = tempdir().unwrap();

        // No lock file = Unknown
        let manager = Fingerprint::detect_package_manager(dir.path());
        assert_eq!(manager, PackageManager::Unknown);

        // Create bun.lock
        std::fs::write(dir.path().join("bun.lock"), "").unwrap();
        let manager = Fingerprint::detect_package_manager(dir.path());
        assert_eq!(manager, PackageManager::Bun);
    }

    #[test]
    fn test_detect_language() {
        let dir = tempdir().unwrap();
        let deps = HashMap::new();

        // Create tsconfig.json
        std::fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        let (lang, has_ts, _) = Fingerprint::detect_language(dir.path(), &deps);
        assert_eq!(lang, ProjectLanguage::TypeScript);
        assert!(has_ts);
    }

    #[test]
    fn test_detect_frameworks() {
        let mut deps = HashMap::new();
        deps.insert("react".to_string(), "18.0.0".to_string());
        deps.insert("react-dom".to_string(), "18.0.0".to_string());
        deps.insert("next".to_string(), "14.0.0".to_string());

        let frameworks = Fingerprint::detect_frameworks(&deps);
        assert!(!frameworks.is_empty());
        assert!(frameworks.iter().any(|f| f.name == "React"));
        assert!(frameworks.iter().any(|f| f.name == "Next.js"));
    }

    #[test]
    fn test_detect_directories() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::create_dir(dir.path().join("test")).unwrap();
        std::fs::create_dir(dir.path().join("public")).unwrap();

        let dirs = Fingerprint::detect_directories(dir.path());
        assert_eq!(dirs.src, Some("src".to_string()));
        assert_eq!(dirs.public, Some("public".to_string()));
        assert!(dirs.tests.contains(&"test".to_string()));
    }

    #[test]
    fn test_fingerprint_similarity() {
        let a = FingerprintInfo {
            project_id: "a".to_string(),
            frameworks: vec![FrameworkInfo {
                name: "React".to_string(),
                version: None,
                framework_type: FrameworkType::Frontend,
            }],
            build_tools: vec![BuildToolInfo {
                name: "Vite".to_string(),
                config: None,
                version: None,
            }],
            test_frameworks: vec![],
            package: PackageInfo::default(),
            configs: vec![],
            language: ProjectLanguage::TypeScript,
            has_typescript: true,
            language_version: None,
            directories: DirectoryInfo::default(),
            hash: "".to_string(),
        };

        // Same fingerprint should have similarity 1.0
        let sim = Fingerprint::similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.01);

        // Different fingerprint
        let b = FingerprintInfo {
            project_id: "b".to_string(),
            frameworks: vec![],
            build_tools: vec![],
            test_frameworks: vec![],
            package: PackageInfo::default(),
            configs: vec![],
            language: ProjectLanguage::Python,
            has_typescript: false,
            language_version: None,
            directories: DirectoryInfo::default(),
            hash: "".to_string(),
        };

        let sim = Fingerprint::similarity(&a, &b);
        assert!(sim < 0.5);
    }

    #[test]
    fn test_describe() {
        let fingerprint = FingerprintInfo {
            project_id: "test".to_string(),
            frameworks: vec![FrameworkInfo {
                name: "React".to_string(),
                version: Some("18.0.0".to_string()),
                framework_type: FrameworkType::Frontend,
            }],
            build_tools: vec![BuildToolInfo {
                name: "Vite".to_string(),
                config: None,
                version: None,
            }],
            test_frameworks: vec![],
            package: PackageInfo::default(),
            configs: vec![],
            language: ProjectLanguage::TypeScript,
            has_typescript: true,
            language_version: None,
            directories: DirectoryInfo::default(),
            hash: "".to_string(),
        };

        let desc = Fingerprint::describe(&fingerprint);
        assert!(desc.contains("TypeScript"));
        assert!(desc.contains("React"));
        assert!(desc.contains("Vite"));
    }

    #[test]
    fn test_project_language_serialization() {
        let json = serde_json::to_string(&ProjectLanguage::TypeScript).unwrap();
        assert_eq!(json, "\"typescript\"");
    }

    #[test]
    fn test_package_manager_serialization() {
        let json = serde_json::to_string(&PackageManager::Bun).unwrap();
        assert_eq!(json, "\"bun\"");
    }
}
