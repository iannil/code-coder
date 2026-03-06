//! Java technology fingerprint detection engine
//!
//! Uses aho-corasick for O(n) multi-pattern matching, regardless of pattern count.
//! This is ~5-10x faster than sequential String.includes() calls.
//!
//! # Example
//!
//! ```ignore
//! let engine = FingerprintEngine::new();
//! let detections = engine.detect(&FingerprintInput {
//!     class_names: vec!["org.springframework.boot.SpringApplication".to_string()],
//!     package_names: vec!["org.springframework.boot".to_string()],
//!     ..Default::default()
//! });
//! ```

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Fingerprint pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    Package,
    Class,
    Config,
    Annotation,
    Manifest,
    Dependency,
}

/// Confidence level for a pattern match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Technology category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FingerprintCategory {
    Framework,
    Orm,
    Web,
    Serialization,
    Utility,
    Logging,
    Testing,
    Messaging,
    Caching,
    Validation,
    Security,
    Scheduling,
    Http,
}

impl FingerprintCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Framework => "framework",
            Self::Orm => "orm",
            Self::Web => "web",
            Self::Serialization => "serialization",
            Self::Utility => "utility",
            Self::Logging => "logging",
            Self::Testing => "testing",
            Self::Messaging => "messaging",
            Self::Caching => "caching",
            Self::Validation => "validation",
            Self::Security => "security",
            Self::Scheduling => "scheduling",
            Self::Http => "http",
        }
    }
}

/// A single fingerprint pattern
#[derive(Debug, Clone)]
pub struct FingerprintPattern {
    pub pattern: String,
    pub pattern_type: PatternType,
    pub confidence: Confidence,
    pub notes: Option<String>,
}

/// A Java technology fingerprint definition
#[derive(Debug, Clone)]
pub struct JavaFingerprint {
    pub name: String,
    pub category: FingerprintCategory,
    pub website: Option<String>,
    pub patterns: Vec<FingerprintPattern>,
}

/// Detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub name: String,
    pub category: String,
    pub website: Option<String>,
    pub matches: Vec<String>,
    pub confidence: String,
}

/// Input for fingerprint detection
#[derive(Debug, Clone, Default)]
pub struct FingerprintInput {
    pub class_names: Vec<String>,
    pub package_names: Vec<String>,
    pub config_files: Vec<String>,
    pub annotations: Vec<String>,
    pub manifest: HashMap<String, String>,
}

/// Pre-compiled fingerprint engine for efficient detection
pub struct FingerprintEngine {
    /// Package pattern matcher
    package_matcher: AhoCorasick,
    package_patterns: Vec<(usize, usize)>, // (fingerprint_index, pattern_index)

    /// Class pattern matcher
    class_matcher: AhoCorasick,
    class_patterns: Vec<(usize, usize)>,

    /// Config pattern matcher
    config_matcher: AhoCorasick,
    config_patterns: Vec<(usize, usize)>,

    /// Manifest pattern matcher
    manifest_matcher: AhoCorasick,
    manifest_patterns: Vec<(usize, usize)>,

    /// All fingerprint definitions
    fingerprints: Vec<JavaFingerprint>,
}

impl FingerprintEngine {
    /// Create a new fingerprint engine with all patterns pre-compiled
    pub fn new() -> Self {
        let fingerprints = build_fingerprints();

        // Build separate matchers for each pattern type
        let (package_matcher, package_patterns) =
            build_matcher(&fingerprints, PatternType::Package);
        let (class_matcher, class_patterns) = build_matcher(&fingerprints, PatternType::Class);
        let (config_matcher, config_patterns) = build_matcher(&fingerprints, PatternType::Config);
        let (manifest_matcher, manifest_patterns) =
            build_matcher(&fingerprints, PatternType::Manifest);

        Self {
            package_matcher,
            package_patterns,
            class_matcher,
            class_patterns,
            config_matcher,
            config_patterns,
            manifest_matcher,
            manifest_patterns,
            fingerprints,
        }
    }

    /// Detect technologies from input
    pub fn detect(&self, input: &FingerprintInput) -> Vec<Detection> {
        let mut detections: HashMap<usize, (HashSet<String>, Confidence)> = HashMap::new();

        // Match packages
        let packages_text = input.package_names.join("\n").to_lowercase();
        for mat in self.package_matcher.find_iter(&packages_text) {
            let (fp_idx, pat_idx) = self.package_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections.entry(fp_idx).or_insert_with(|| (HashSet::new(), Confidence::Low));
            entry.0.insert(format!("{} ({})", pattern.pattern, confidence_str(pattern.confidence)));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Match classes
        let classes_text = input.class_names.join("\n").to_lowercase();
        for mat in self.class_matcher.find_iter(&classes_text) {
            let (fp_idx, pat_idx) = self.class_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections.entry(fp_idx).or_insert_with(|| (HashSet::new(), Confidence::Low));
            entry.0.insert(format!("{} ({})", pattern.pattern, confidence_str(pattern.confidence)));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Match config files
        let configs_text = input.config_files.join("\n").to_lowercase();
        for mat in self.config_matcher.find_iter(&configs_text) {
            let (fp_idx, pat_idx) = self.config_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections.entry(fp_idx).or_insert_with(|| (HashSet::new(), Confidence::Low));
            entry.0.insert(format!("{} ({})", pattern.pattern, confidence_str(pattern.confidence)));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Match manifest
        let manifest_text = input
            .manifest
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        for mat in self.manifest_matcher.find_iter(&manifest_text) {
            let (fp_idx, pat_idx) = self.manifest_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections.entry(fp_idx).or_insert_with(|| (HashSet::new(), Confidence::Low));
            entry.0.insert(format!("{} ({})", pattern.pattern, confidence_str(pattern.confidence)));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Convert to Detection structs
        detections
            .into_iter()
            .map(|(fp_idx, (matches, confidence))| {
                let fp = &self.fingerprints[fp_idx];
                Detection {
                    name: fp.name.clone(),
                    category: fp.category.as_str().to_string(),
                    website: fp.website.clone(),
                    matches: matches.into_iter().collect(),
                    confidence: confidence_str(confidence).to_string(),
                }
            })
            .collect()
    }

    /// Get all fingerprints
    pub fn fingerprints(&self) -> &[JavaFingerprint] {
        &self.fingerprints
    }

    /// Get fingerprints by category
    pub fn fingerprints_by_category(&self, category: FingerprintCategory) -> Vec<&JavaFingerprint> {
        self.fingerprints
            .iter()
            .filter(|fp| fp.category == category)
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<FingerprintCategory> {
        vec![
            FingerprintCategory::Framework,
            FingerprintCategory::Orm,
            FingerprintCategory::Web,
            FingerprintCategory::Serialization,
            FingerprintCategory::Utility,
            FingerprintCategory::Logging,
            FingerprintCategory::Testing,
            FingerprintCategory::Messaging,
            FingerprintCategory::Caching,
            FingerprintCategory::Validation,
            FingerprintCategory::Security,
            FingerprintCategory::Scheduling,
            FingerprintCategory::Http,
        ]
    }
}

impl Default for FingerprintEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn build_matcher(
    fingerprints: &[JavaFingerprint],
    pattern_type: PatternType,
) -> (AhoCorasick, Vec<(usize, usize)>) {
    let mut patterns: Vec<String> = Vec::new();
    let mut indices: Vec<(usize, usize)> = Vec::new();

    for (fp_idx, fp) in fingerprints.iter().enumerate() {
        for (pat_idx, pat) in fp.patterns.iter().enumerate() {
            if pat.pattern_type == pattern_type {
                patterns.push(pat.pattern.to_lowercase());
                indices.push((fp_idx, pat_idx));
            }
        }
    }

    let matcher = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostFirst)
        .build(&patterns)
        .expect("Failed to build aho-corasick matcher");

    (matcher, indices)
}

fn confidence_str(c: Confidence) -> &'static str {
    match c {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    }
}

fn max_confidence(a: Confidence, b: Confidence) -> Confidence {
    match (a, b) {
        (Confidence::High, _) | (_, Confidence::High) => Confidence::High,
        (Confidence::Medium, _) | (_, Confidence::Medium) => Confidence::Medium,
        _ => Confidence::Low,
    }
}

/// Build all fingerprint definitions
fn build_fingerprints() -> Vec<JavaFingerprint> {
    let mut fps = Vec::new();

    // === FRAMEWORKS ===
    fps.push(JavaFingerprint {
        name: "Spring Boot".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://spring.io/projects/spring-boot".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.springframework.boot".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "spring-boot".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "springapplication".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "application.properties".to_string(), pattern_type: PatternType::Config, confidence: Confidence::Medium, notes: None },
            FingerprintPattern { pattern: "application.yml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::Medium, notes: None },
            FingerprintPattern { pattern: "spring.factories".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Spring Framework".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://spring.io/projects/spring-framework".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.springframework".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "applicationcontext".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "spring-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::Medium, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Jakarta EE".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://jakarta.ee".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "jakarta.".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "javax.".to_string(), pattern_type: PatternType::Package, confidence: Confidence::Medium, notes: Some("Legacy Java EE".to_string()) },
            FingerprintPattern { pattern: "web.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::Medium, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Micronaut".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://micronaut.io".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "io.micronaut".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "micronaut-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Quarkus".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://quarkus.io".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "io.quarkus".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "quarkus-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Google Guice".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://github.com/google/guice".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.google.inject".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "injector".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Vaadin".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://vaadin.com".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.vaadin".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "vaadin-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Apache Wicket".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://wicket.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.wicket".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "wicket-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Struts".to_string(),
        category: FingerprintCategory::Framework,
        website: Some("https://struts.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.struts".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "struts-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "struts.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    // === ORM / DATABASE ===
    fps.push(JavaFingerprint {
        name: "Hibernate".to_string(),
        category: FingerprintCategory::Orm,
        website: Some("https://hibernate.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.hibernate".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "hibernate.cfg.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "hibernate-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "EclipseLink".to_string(),
        category: FingerprintCategory::Orm,
        website: Some("https://www.eclipse.org/eclipselink".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.eclipse.persistence".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "eclipselink-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "persistence.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::Medium, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "MyBatis".to_string(),
        category: FingerprintCategory::Orm,
        website: Some("https://mybatis.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.ibatis".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "mybatis-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "mybatis-config.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "JOOQ".to_string(),
        category: FingerprintCategory::Orm,
        website: Some("https://www.jooq.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.jooq".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "jooq-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Spring Data".to_string(),
        category: FingerprintCategory::Orm,
        website: Some("https://spring.io/projects/spring-data".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.springframework.data".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "crudrepository".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "jparepository".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
        ],
    });

    // === WEB SERVERS ===
    fps.push(JavaFingerprint {
        name: "Apache Tomcat".to_string(),
        category: FingerprintCategory::Web,
        website: Some("https://tomcat.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.catalina".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "org.apache.tomcat".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "tomcat-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Jetty".to_string(),
        category: FingerprintCategory::Web,
        website: Some("https://www.eclipse.org/jetty".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.eclipse.jetty".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "jetty-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Netty".to_string(),
        category: FingerprintCategory::Web,
        website: Some("https://netty.io".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "io.netty".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "netty-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "bytebuf".to_string(), pattern_type: PatternType::Class, confidence: Confidence::Medium, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Undertow".to_string(),
        category: FingerprintCategory::Web,
        website: Some("https://undertow.io".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "io.undertow".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "undertow-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === SERIALIZATION ===
    fps.push(JavaFingerprint {
        name: "Jackson".to_string(),
        category: FingerprintCategory::Serialization,
        website: Some("https://github.com/FasterXML/jackson".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.fasterxml.jackson".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "jackson-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "objectmapper".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Gson".to_string(),
        category: FingerprintCategory::Serialization,
        website: Some("https://github.com/google/gson".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.google.gson".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "gson-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "FastJSON".to_string(),
        category: FingerprintCategory::Serialization,
        website: Some("https://github.com/alibaba/fastjson".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.alibaba.fastjson".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "fastjson-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === UTILITIES ===
    fps.push(JavaFingerprint {
        name: "Apache Commons".to_string(),
        category: FingerprintCategory::Utility,
        website: Some("https://commons.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.commons".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "commons-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Google Guava".to_string(),
        category: FingerprintCategory::Utility,
        website: Some("https://github.com/google/guava".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.google.common".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "guava-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "immutablelist".to_string(), pattern_type: PatternType::Class, confidence: Confidence::Medium, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Lombok".to_string(),
        category: FingerprintCategory::Utility,
        website: Some("https://projectlombok.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "lombok".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Apache POI".to_string(),
        category: FingerprintCategory::Utility,
        website: Some("https://poi.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.poi".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "poi-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === LOGGING ===
    fps.push(JavaFingerprint {
        name: "SLF4J".to_string(),
        category: FingerprintCategory::Logging,
        website: Some("http://www.slf4j.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.slf4j".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "slf4j-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "loggerfactory".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Logback".to_string(),
        category: FingerprintCategory::Logging,
        website: Some("https://logback.qos.ch".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "ch.qos.logback".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "logback-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "logback.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Log4j".to_string(),
        category: FingerprintCategory::Logging,
        website: Some("https://logging.apache.org/log4j".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.logging.log4j".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "log4j-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "log4j2.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "log4j.properties".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    // === TESTING ===
    fps.push(JavaFingerprint {
        name: "JUnit".to_string(),
        category: FingerprintCategory::Testing,
        website: Some("https://junit.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.junit".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "junit-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "TestNG".to_string(),
        category: FingerprintCategory::Testing,
        website: Some("https://testng.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.testng".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "testng-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "testng.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Mockito".to_string(),
        category: FingerprintCategory::Testing,
        website: Some("https://site.mockito.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.mockito".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "mockito-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "AssertJ".to_string(),
        category: FingerprintCategory::Testing,
        website: Some("https://assertj.github.io/doc".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.assertj".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "assertj-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Selenium".to_string(),
        category: FingerprintCategory::Testing,
        website: Some("https://www.selenium.dev".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.openqa.selenium".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "selenium-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "webdriver".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Cucumber".to_string(),
        category: FingerprintCategory::Testing,
        website: Some("https://cucumber.io".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "io.cucumber".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "cucumber-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === MESSAGING ===
    fps.push(JavaFingerprint {
        name: "Apache Kafka".to_string(),
        category: FingerprintCategory::Messaging,
        website: Some("https://kafka.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.kafka".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "kafka-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "RabbitMQ".to_string(),
        category: FingerprintCategory::Messaging,
        website: Some("https://www.rabbitmq.com".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.rabbitmq".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "amqp-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "ActiveMQ".to_string(),
        category: FingerprintCategory::Messaging,
        website: Some("https://activemq.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.activemq".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "activemq-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === CACHING ===
    fps.push(JavaFingerprint {
        name: "Caffeine".to_string(),
        category: FingerprintCategory::Caching,
        website: Some("https://github.com/ben-manes/caffeine".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.github.benmanes.caffeine".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "caffeine-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Ehcache".to_string(),
        category: FingerprintCategory::Caching,
        website: Some("https://www.ehcache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.ehcache".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "net.sf.ehcache".to_string(), pattern_type: PatternType::Package, confidence: Confidence::Medium, notes: None },
            FingerprintPattern { pattern: "ehcache-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "ehcache.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Redis (Lettuce/Jedis)".to_string(),
        category: FingerprintCategory::Caching,
        website: Some("https://redis.io".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "io.lettuce".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "redis.clients.jedis".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "lettuce-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "jedis-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Hazelcast".to_string(),
        category: FingerprintCategory::Caching,
        website: Some("https://hazelcast.com".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.hazelcast".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "hazelcast-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "hazelcast.xml".to_string(), pattern_type: PatternType::Config, confidence: Confidence::High, notes: None },
        ],
    });

    // === SECURITY ===
    fps.push(JavaFingerprint {
        name: "Spring Security".to_string(),
        category: FingerprintCategory::Security,
        website: Some("https://spring.io/projects/spring-security".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.springframework.security".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "spring-security-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Apache Shiro".to_string(),
        category: FingerprintCategory::Security,
        website: Some("https://shiro.apache.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.shiro".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "shiro-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "JWT (java-jwt/jjwt)".to_string(),
        category: FingerprintCategory::Security,
        website: Some("https://github.com/auth0/java-jwt".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "com.auth0.jwt".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "io.jsonwebtoken".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "jwt-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === SCHEDULING ===
    fps.push(JavaFingerprint {
        name: "Quartz".to_string(),
        category: FingerprintCategory::Scheduling,
        website: Some("http://www.quartz-scheduler.org".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.quartz".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "quartz-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    // === HTTP CLIENTS ===
    fps.push(JavaFingerprint {
        name: "OkHttp".to_string(),
        category: FingerprintCategory::Http,
        website: Some("https://square.github.io/okhttp".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "okhttp3".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "okhttp-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "okhttpclient".to_string(), pattern_type: PatternType::Class, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Apache HttpClient".to_string(),
        category: FingerprintCategory::Http,
        website: Some("https://hc.apache.org/httpcomponents-client".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.apache.http".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "httpclient-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Retrofit".to_string(),
        category: FingerprintCategory::Http,
        website: Some("https://square.github.io/retrofit".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "retrofit2".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "retrofit-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps.push(JavaFingerprint {
        name: "Feign".to_string(),
        category: FingerprintCategory::Http,
        website: Some("https://github.com/OpenFeign/feign".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "feign".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
        ],
    });

    // === VALIDATION ===
    fps.push(JavaFingerprint {
        name: "Hibernate Validator".to_string(),
        category: FingerprintCategory::Validation,
        website: Some("https://hibernate.org/validator".to_string()),
        patterns: vec![
            FingerprintPattern { pattern: "org.hibernate.validator".to_string(), pattern_type: PatternType::Package, confidence: Confidence::High, notes: None },
            FingerprintPattern { pattern: "hibernate-validator-".to_string(), pattern_type: PatternType::Manifest, confidence: Confidence::High, notes: None },
        ],
    });

    fps
}

/// Global singleton for fingerprint engine
pub static FINGERPRINT_ENGINE: Lazy<FingerprintEngine> = Lazy::new(FingerprintEngine::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = FingerprintEngine::new();
        assert!(!engine.fingerprints().is_empty());
    }

    #[test]
    fn test_detect_spring_boot() {
        let engine = FingerprintEngine::new();
        let detections = engine.detect(&FingerprintInput {
            package_names: vec!["org.springframework.boot.autoconfigure".to_string()],
            config_files: vec!["application.properties".to_string()],
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "Spring Boot"));
    }

    #[test]
    fn test_detect_hibernate() {
        let engine = FingerprintEngine::new();
        let detections = engine.detect(&FingerprintInput {
            package_names: vec!["org.hibernate.Session".to_string()],
            config_files: vec!["hibernate.cfg.xml".to_string()],
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "Hibernate"));
    }

    #[test]
    fn test_detect_multiple() {
        let engine = FingerprintEngine::new();
        let detections = engine.detect(&FingerprintInput {
            package_names: vec![
                "org.springframework.boot.SpringApplication".to_string(),
                "org.hibernate.Session".to_string(),
                "com.fasterxml.jackson.databind.ObjectMapper".to_string(),
            ],
            ..Default::default()
        });

        let names: Vec<_> = detections.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Spring Boot"));
        assert!(names.contains(&"Hibernate"));
        assert!(names.contains(&"Jackson"));
    }

    #[test]
    fn test_categories() {
        let engine = FingerprintEngine::new();
        let categories = engine.categories();
        assert!(categories.contains(&FingerprintCategory::Framework));
        assert!(categories.contains(&FingerprintCategory::Orm));
        assert!(categories.contains(&FingerprintCategory::Testing));
    }

    #[test]
    fn test_fingerprints_by_category() {
        let engine = FingerprintEngine::new();
        let frameworks = engine.fingerprints_by_category(FingerprintCategory::Framework);
        assert!(frameworks.iter().any(|fp| fp.name == "Spring Boot"));
        assert!(frameworks.iter().any(|fp| fp.name == "Quarkus"));
    }
}
