//! Web technology fingerprint detection engine
//!
//! Uses aho-corasick for O(n) multi-pattern matching, regardless of pattern count.
//! This is ~5-10x faster than sequential String.includes() calls.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Web technology category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebCategory {
    Frontend,
    Ui,
    State,
    Build,
    Styling,
    Backend,
    Hosting,
    Analytics,
    Monitoring,
    Auth,
    Payment,
}

impl WebCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
            Self::Ui => "ui",
            Self::State => "state",
            Self::Build => "build",
            Self::Styling => "styling",
            Self::Backend => "backend",
            Self::Hosting => "hosting",
            Self::Analytics => "analytics",
            Self::Monitoring => "monitoring",
            Self::Auth => "auth",
            Self::Payment => "payment",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "frontend" => Some(Self::Frontend),
            "ui" => Some(Self::Ui),
            "state" => Some(Self::State),
            "build" => Some(Self::Build),
            "styling" => Some(Self::Styling),
            "backend" => Some(Self::Backend),
            "hosting" => Some(Self::Hosting),
            "analytics" => Some(Self::Analytics),
            "monitoring" => Some(Self::Monitoring),
            "auth" => Some(Self::Auth),
            "payment" => Some(Self::Payment),
            _ => None,
        }
    }
}

/// Pattern type for web fingerprints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebPatternType {
    /// Match in HTML/JS content
    Content,
    /// Match in HTTP headers
    Header,
    /// Match in URL/domain
    Url,
    /// Match in cookie names
    Cookie,
}

/// Confidence level for a pattern match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebConfidence {
    High,
    Medium,
    Low,
}

impl WebConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// A single fingerprint pattern
#[derive(Debug, Clone)]
pub struct WebFingerprintPattern {
    pub pattern: String,
    pub pattern_type: WebPatternType,
    pub confidence: WebConfidence,
    pub notes: Option<String>,
}

/// A web technology fingerprint definition
#[derive(Debug, Clone)]
pub struct WebFingerprint {
    pub name: String,
    pub category: WebCategory,
    pub website: Option<String>,
    pub patterns: Vec<WebFingerprintPattern>,
}

/// Detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDetection {
    pub name: String,
    pub category: String,
    pub website: Option<String>,
    pub matches: Vec<String>,
    pub confidence: String,
}

/// Input for fingerprint detection
#[derive(Debug, Clone, Default)]
pub struct WebFingerprintInput {
    /// HTML/JS content to analyze
    pub content: String,
    /// HTTP headers (header_name -> value)
    pub headers: HashMap<String, String>,
    /// URL being analyzed
    pub url: Option<String>,
    /// Cookie names
    pub cookies: Vec<String>,
}

/// Pre-compiled fingerprint engine for efficient detection
pub struct WebFingerprintEngine {
    /// Content pattern matcher
    content_matcher: AhoCorasick,
    content_patterns: Vec<(usize, usize)>, // (fingerprint_index, pattern_index)

    /// Header pattern matcher
    header_matcher: AhoCorasick,
    header_patterns: Vec<(usize, usize)>,

    /// URL pattern matcher
    url_matcher: AhoCorasick,
    url_patterns: Vec<(usize, usize)>,

    /// Cookie pattern matcher
    cookie_matcher: AhoCorasick,
    cookie_patterns: Vec<(usize, usize)>,

    /// All fingerprint definitions
    fingerprints: Vec<WebFingerprint>,
}

impl WebFingerprintEngine {
    /// Create a new fingerprint engine with all patterns pre-compiled
    pub fn new() -> Self {
        let fingerprints = build_fingerprints();

        let (content_matcher, content_patterns) =
            build_matcher(&fingerprints, WebPatternType::Content);
        let (header_matcher, header_patterns) =
            build_matcher(&fingerprints, WebPatternType::Header);
        let (url_matcher, url_patterns) = build_matcher(&fingerprints, WebPatternType::Url);
        let (cookie_matcher, cookie_patterns) =
            build_matcher(&fingerprints, WebPatternType::Cookie);

        Self {
            content_matcher,
            content_patterns,
            header_matcher,
            header_patterns,
            url_matcher,
            url_patterns,
            cookie_matcher,
            cookie_patterns,
            fingerprints,
        }
    }

    /// Detect technologies from input
    pub fn detect(&self, input: &WebFingerprintInput) -> Vec<WebDetection> {
        let mut detections: HashMap<usize, (HashSet<String>, WebConfidence)> = HashMap::new();

        // Match content
        let content_lower = input.content.to_lowercase();
        for mat in self.content_matcher.find_iter(&content_lower) {
            let (fp_idx, pat_idx) = self.content_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections
                .entry(fp_idx)
                .or_insert_with(|| (HashSet::new(), WebConfidence::Low));
            entry.0.insert(format!(
                "{} ({})",
                pattern.pattern,
                pattern.confidence.as_str()
            ));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Match headers
        let headers_text = input
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        for mat in self.header_matcher.find_iter(&headers_text) {
            let (fp_idx, pat_idx) = self.header_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections
                .entry(fp_idx)
                .or_insert_with(|| (HashSet::new(), WebConfidence::Low));
            entry.0.insert(format!(
                "{} ({})",
                pattern.pattern,
                pattern.confidence.as_str()
            ));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Match URL
        if let Some(url) = &input.url {
            let url_lower = url.to_lowercase();
            for mat in self.url_matcher.find_iter(&url_lower) {
                let (fp_idx, pat_idx) = self.url_patterns[mat.pattern().as_usize()];
                let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
                let entry = detections
                    .entry(fp_idx)
                    .or_insert_with(|| (HashSet::new(), WebConfidence::Low));
                entry.0.insert(format!(
                    "{} ({})",
                    pattern.pattern,
                    pattern.confidence.as_str()
                ));
                entry.1 = max_confidence(entry.1, pattern.confidence);
            }
        }

        // Match cookies
        let cookies_text = input.cookies.join("\n").to_lowercase();
        for mat in self.cookie_matcher.find_iter(&cookies_text) {
            let (fp_idx, pat_idx) = self.cookie_patterns[mat.pattern().as_usize()];
            let pattern = &self.fingerprints[fp_idx].patterns[pat_idx];
            let entry = detections
                .entry(fp_idx)
                .or_insert_with(|| (HashSet::new(), WebConfidence::Low));
            entry.0.insert(format!(
                "{} ({})",
                pattern.pattern,
                pattern.confidence.as_str()
            ));
            entry.1 = max_confidence(entry.1, pattern.confidence);
        }

        // Convert to Detection structs
        detections
            .into_iter()
            .map(|(fp_idx, (matches, confidence))| {
                let fp = &self.fingerprints[fp_idx];
                WebDetection {
                    name: fp.name.clone(),
                    category: fp.category.as_str().to_string(),
                    website: fp.website.clone(),
                    matches: matches.into_iter().collect(),
                    confidence: confidence.as_str().to_string(),
                }
            })
            .collect()
    }

    /// Get all fingerprints
    pub fn fingerprints(&self) -> &[WebFingerprint] {
        &self.fingerprints
    }

    /// Get fingerprints by category
    pub fn fingerprints_by_category(&self, category: WebCategory) -> Vec<&WebFingerprint> {
        self.fingerprints
            .iter()
            .filter(|fp| fp.category == category)
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<WebCategory> {
        vec![
            WebCategory::Frontend,
            WebCategory::Ui,
            WebCategory::State,
            WebCategory::Build,
            WebCategory::Styling,
            WebCategory::Backend,
            WebCategory::Hosting,
            WebCategory::Analytics,
            WebCategory::Monitoring,
            WebCategory::Auth,
            WebCategory::Payment,
        ]
    }
}

impl Default for WebFingerprintEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn build_matcher(
    fingerprints: &[WebFingerprint],
    pattern_type: WebPatternType,
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

    // Handle empty patterns (aho-corasick panics on empty)
    if patterns.is_empty() {
        patterns.push("__IMPOSSIBLE_PATTERN_THAT_WILL_NEVER_MATCH__".to_string());
    }

    let matcher = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostFirst)
        .build(&patterns)
        .expect("Failed to build aho-corasick matcher");

    (matcher, indices)
}

fn max_confidence(a: WebConfidence, b: WebConfidence) -> WebConfidence {
    match (a, b) {
        (WebConfidence::High, _) | (_, WebConfidence::High) => WebConfidence::High,
        (WebConfidence::Medium, _) | (_, WebConfidence::Medium) => WebConfidence::Medium,
        _ => WebConfidence::Low,
    }
}

/// Helper macro for creating patterns
macro_rules! pat {
    ($pattern:expr, $ptype:expr, $conf:expr) => {
        WebFingerprintPattern {
            pattern: $pattern.to_string(),
            pattern_type: $ptype,
            confidence: $conf,
            notes: None,
        }
    };
    ($pattern:expr, $ptype:expr, $conf:expr, $notes:expr) => {
        WebFingerprintPattern {
            pattern: $pattern.to_string(),
            pattern_type: $ptype,
            confidence: $conf,
            notes: Some($notes.to_string()),
        }
    };
}

/// Build all web fingerprint definitions
fn build_fingerprints() -> Vec<WebFingerprint> {
    let mut fps = Vec::new();

    // ============================================
    // FRONTEND FRAMEWORKS
    // ============================================
    fps.push(WebFingerprint {
        name: "React".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://react.dev".to_string()),
        patterns: vec![
            pat!("__react__", WebPatternType::Content, WebConfidence::High),
            pat!("react-root", WebPatternType::Content, WebConfidence::High),
            pat!("data-reactroot", WebPatternType::Content, WebConfidence::High),
            pat!("data-reactid", WebPatternType::Content, WebConfidence::High),
            pat!("reactdom", WebPatternType::Content, WebConfidence::High),
            pat!("react.createelement", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Vue".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://vuejs.org".to_string()),
        patterns: vec![
            pat!("__vue__", WebPatternType::Content, WebConfidence::High),
            pat!("vue.createapp", WebPatternType::Content, WebConfidence::High),
            pat!("new vue(", WebPatternType::Content, WebConfidence::High),
            pat!("v-cloak", WebPatternType::Content, WebConfidence::Medium),
            pat!("data-v-", WebPatternType::Content, WebConfidence::Medium),
            pat!("vue-router", WebPatternType::Content, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Svelte".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://svelte.dev".to_string()),
        patterns: vec![
            pat!("data-svelte-h", WebPatternType::Content, WebConfidence::High),
            pat!("__svelte_component", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Angular".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://angular.io".to_string()),
        patterns: vec![
            pat!("ng-version", WebPatternType::Content, WebConfidence::High),
            pat!("ng-app", WebPatternType::Content, WebConfidence::High),
            pat!("*ngif", WebPatternType::Content, WebConfidence::High),
            pat!("*ngfor", WebPatternType::Content, WebConfidence::High),
            pat!("[(ngmodel)]", WebPatternType::Content, WebConfidence::High),
            pat!("zone.js", WebPatternType::Content, WebConfidence::Medium),
            pat!("@angular/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Solid".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://solidjs.com".to_string()),
        patterns: vec![
            pat!("data-hk", WebPatternType::Content, WebConfidence::High),
            pat!("createsignal", WebPatternType::Content, WebConfidence::High),
            pat!("createeffect", WebPatternType::Content, WebConfidence::High),
            pat!("@solidjs/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Next.js".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://nextjs.org".to_string()),
        patterns: vec![
            pat!("__next_data__", WebPatternType::Content, WebConfidence::High),
            pat!("__next_router__", WebPatternType::Content, WebConfidence::High),
            pat!("/_next/static/", WebPatternType::Content, WebConfidence::High),
            pat!("/_next/image", WebPatternType::Content, WebConfidence::High),
            pat!("next/router", WebPatternType::Content, WebConfidence::Medium),
            pat!("next/link", WebPatternType::Content, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Nuxt".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://nuxt.com".to_string()),
        patterns: vec![
            pat!("__nuxt__", WebPatternType::Content, WebConfidence::High),
            pat!("/_nuxt/", WebPatternType::Content, WebConfidence::High),
            pat!("nuxt-link", WebPatternType::Content, WebConfidence::Medium),
            pat!("@nuxt/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Remix".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://remix.run".to_string()),
        patterns: vec![
            pat!("__remixcontext", WebPatternType::Content, WebConfidence::High),
            pat!("__remixroutemodules", WebPatternType::Content, WebConfidence::High),
            pat!("@remix-run/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Astro".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://astro.build".to_string()),
        patterns: vec![
            pat!("astro-head", WebPatternType::Content, WebConfidence::Medium),
            pat!("@astrojs/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "SvelteKit".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://kit.svelte.dev".to_string()),
        patterns: vec![
            pat!("__sveltekit", WebPatternType::Content, WebConfidence::High),
            pat!("/_app/immutable/", WebPatternType::Content, WebConfidence::High),
            pat!("@sveltejs/kit", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Gatsby".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://www.gatsbyjs.com".to_string()),
        patterns: vec![
            pat!("/page-data/", WebPatternType::Content, WebConfidence::High),
            pat!("@gatsbyjs/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Qwik".to_string(),
        category: WebCategory::Frontend,
        website: Some("https://qwik.builder.io".to_string()),
        patterns: vec![
            pat!("qwik-", WebPatternType::Content, WebConfidence::High),
            pat!("@builder.io/qwik", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // UI LIBRARIES
    // ============================================
    fps.push(WebFingerprint {
        name: "Tailwind CSS".to_string(),
        category: WebCategory::Ui,
        website: Some("https://tailwindcss.com".to_string()),
        patterns: vec![
            pat!("tailwindcss", WebPatternType::Content, WebConfidence::High),
            pat!("@tailwind", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Material-UI (MUI)".to_string(),
        category: WebCategory::Ui,
        website: Some("https://mui.com".to_string()),
        patterns: vec![
            pat!("makestyles", WebPatternType::Content, WebConfidence::High),
            pat!("muibox-", WebPatternType::Content, WebConfidence::High),
            pat!("muibutton-", WebPatternType::Content, WebConfidence::High),
            pat!("muigrid-", WebPatternType::Content, WebConfidence::High),
            pat!("@mui/material", WebPatternType::Content, WebConfidence::High),
            pat!("@mui/icons-material", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Ant Design".to_string(),
        category: WebCategory::Ui,
        website: Some("https://ant.design".to_string()),
        patterns: vec![
            pat!("ant-btn", WebPatternType::Content, WebConfidence::High),
            pat!("ant-input", WebPatternType::Content, WebConfidence::High),
            pat!("ant-modal", WebPatternType::Content, WebConfidence::High),
            pat!("antd", WebPatternType::Content, WebConfidence::High),
            pat!("@ant-design/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Chakra UI".to_string(),
        category: WebCategory::Ui,
        website: Some("https://chakra-ui.com".to_string()),
        patterns: vec![
            pat!("chakra-", WebPatternType::Content, WebConfidence::High),
            pat!("@chakra-ui/react", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "shadcn/ui".to_string(),
        category: WebCategory::Ui,
        website: Some("https://ui.shadcn.com".to_string()),
        patterns: vec![
            pat!("class-variance-authority", WebPatternType::Content, WebConfidence::Medium),
            pat!("@/components/ui/", WebPatternType::Content, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Radix UI".to_string(),
        category: WebCategory::Ui,
        website: Some("https://www.radix-ui.com".to_string()),
        patterns: vec![
            pat!("data-radix-", WebPatternType::Content, WebConfidence::High),
            pat!("@radix-ui/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Element Plus".to_string(),
        category: WebCategory::Ui,
        website: Some("https://element-plus.org".to_string()),
        patterns: vec![
            pat!("el-button", WebPatternType::Content, WebConfidence::High),
            pat!("el-input", WebPatternType::Content, WebConfidence::High),
            pat!("element-plus", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Bootstrap".to_string(),
        category: WebCategory::Ui,
        website: Some("https://getbootstrap.com".to_string()),
        patterns: vec![
            pat!("bootstrap", WebPatternType::Content, WebConfidence::High),
            pat!("/bootstrap.", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Bulma".to_string(),
        category: WebCategory::Ui,
        website: Some("https://bulma.io".to_string()),
        patterns: vec![
            pat!("bulma", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Vuetify".to_string(),
        category: WebCategory::Ui,
        website: Some("https://vuetifyjs.com".to_string()),
        patterns: vec![
            pat!("v-app", WebPatternType::Content, WebConfidence::High),
            pat!("v-btn", WebPatternType::Content, WebConfidence::High),
            pat!("vuetify", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Quasar".to_string(),
        category: WebCategory::Ui,
        website: Some("https://quasar.dev".to_string()),
        patterns: vec![
            pat!("quasar", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // STATE MANAGEMENT
    // ============================================
    fps.push(WebFingerprint {
        name: "Redux".to_string(),
        category: WebCategory::State,
        website: Some("https://redux.js.org".to_string()),
        patterns: vec![
            pat!("__redux_devtools_extension__", WebPatternType::Content, WebConfidence::High),
            pat!("createstore", WebPatternType::Content, WebConfidence::High),
            pat!("configurestore", WebPatternType::Content, WebConfidence::High),
            pat!("@reduxjs/", WebPatternType::Content, WebConfidence::High),
            pat!("usedispatch", WebPatternType::Content, WebConfidence::High),
            pat!("useselector", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Zustand".to_string(),
        category: WebCategory::State,
        website: Some("https://zustand-demo.pmnd.rs".to_string()),
        patterns: vec![
            pat!("zustand", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Pinia".to_string(),
        category: WebCategory::State,
        website: Some("https://pinia.vuejs.org".to_string()),
        patterns: vec![
            pat!("pinia", WebPatternType::Content, WebConfidence::High),
            pat!("definestore", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "MobX".to_string(),
        category: WebCategory::State,
        website: Some("https://mobx.js.org".to_string()),
        patterns: vec![
            pat!("mobx", WebPatternType::Content, WebConfidence::High),
            pat!("makeobservable", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Recoil".to_string(),
        category: WebCategory::State,
        website: Some("https://recoiljs.org".to_string()),
        patterns: vec![
            pat!("data-recoil-", WebPatternType::Content, WebConfidence::High),
            pat!("recoil", WebPatternType::Content, WebConfidence::High),
            pat!("userecoilstate", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Jotai".to_string(),
        category: WebCategory::State,
        website: Some("https://jotai.org".to_string()),
        patterns: vec![
            pat!("jotai", WebPatternType::Content, WebConfidence::High),
            pat!("useatom", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "XState".to_string(),
        category: WebCategory::State,
        website: Some("https://xstate.js.org".to_string()),
        patterns: vec![
            pat!("xstate", WebPatternType::Content, WebConfidence::High),
            pat!("createmachine", WebPatternType::Content, WebConfidence::High),
            pat!("usemachine", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Apollo Client".to_string(),
        category: WebCategory::State,
        website: Some("https://www.apollographql.com".to_string()),
        patterns: vec![
            pat!("@apollo/client", WebPatternType::Content, WebConfidence::High),
            pat!("apolloclient", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "TanStack Query".to_string(),
        category: WebCategory::State,
        website: Some("https://tanstack.com/query".to_string()),
        patterns: vec![
            pat!("@tanstack/react-query", WebPatternType::Content, WebConfidence::High),
            pat!("@tanstack/vue-query", WebPatternType::Content, WebConfidence::High),
            pat!("queryclient", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "SWR".to_string(),
        category: WebCategory::State,
        website: Some("https://swr.vercel.app".to_string()),
        patterns: vec![
            pat!("useswr", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // BUILD TOOLS
    // ============================================
    fps.push(WebFingerprint {
        name: "Vite".to_string(),
        category: WebCategory::Build,
        website: Some("https://vitejs.dev".to_string()),
        patterns: vec![
            pat!("/@vite/", WebPatternType::Content, WebConfidence::High),
            pat!("import.meta.hot", WebPatternType::Content, WebConfidence::High),
            pat!("@vitejs/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Webpack".to_string(),
        category: WebCategory::Build,
        website: Some("https://webpack.js.org".to_string()),
        patterns: vec![
            pat!("__webpack_require__", WebPatternType::Content, WebConfidence::High),
            pat!("webpackchunkname", WebPatternType::Content, WebConfidence::High),
            pat!("__webpack_public_path__", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Rollup".to_string(),
        category: WebCategory::Build,
        website: Some("https://rollupjs.org".to_string()),
        patterns: vec![
            pat!("__rollup__", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "esbuild".to_string(),
        category: WebCategory::Build,
        website: Some("https://esbuild.github.io".to_string()),
        patterns: vec![
            pat!("/* esbuild */", WebPatternType::Content, WebConfidence::High),
            pat!("__esbuild", WebPatternType::Content, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Parcel".to_string(),
        category: WebCategory::Build,
        website: Some("https://parceljs.org".to_string()),
        patterns: vec![
            pat!("parcel-bundler", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Turbopack".to_string(),
        category: WebCategory::Build,
        website: Some("https://turbo.build/pack".to_string()),
        patterns: vec![
            pat!("__turbopack_", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Rspack".to_string(),
        category: WebCategory::Build,
        website: Some("https://www.rspack.dev".to_string()),
        patterns: vec![
            pat!("__rspack__", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "SWC".to_string(),
        category: WebCategory::Build,
        website: Some("https://swc.rs".to_string()),
        patterns: vec![
            pat!("/* @swc */", WebPatternType::Content, WebConfidence::High),
            pat!("@swc/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // STYLING SOLUTIONS
    // ============================================
    fps.push(WebFingerprint {
        name: "CSS Modules".to_string(),
        category: WebCategory::Styling,
        website: Some("https://github.com/css-modules/css-modules".to_string()),
        patterns: vec![
            pat!(".module.", WebPatternType::Content, WebConfidence::High),
            pat!("css-loader", WebPatternType::Content, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Emotion".to_string(),
        category: WebCategory::Styling,
        website: Some("https://emotion.sh".to_string()),
        patterns: vec![
            pat!("@emotion/react", WebPatternType::Content, WebConfidence::High),
            pat!("@emotion/styled", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Styled Components".to_string(),
        category: WebCategory::Styling,
        website: Some("https://styled-components.com".to_string()),
        patterns: vec![
            pat!("styled-components", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "SCSS/Sass".to_string(),
        category: WebCategory::Styling,
        website: Some("https://sass-lang.com".to_string()),
        patterns: vec![
            pat!(".scss", WebPatternType::Content, WebConfidence::High),
            pat!("sass-loader", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Less".to_string(),
        category: WebCategory::Styling,
        website: Some("https://lesscss.org".to_string()),
        patterns: vec![
            pat!(".less", WebPatternType::Content, WebConfidence::High),
            pat!("less-loader", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Panda CSS".to_string(),
        category: WebCategory::Styling,
        website: Some("https://panda-css.com".to_string()),
        patterns: vec![
            pat!("@pandacss/", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // HOSTING / INFRASTRUCTURE
    // ============================================
    fps.push(WebFingerprint {
        name: "Vercel".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://vercel.com".to_string()),
        patterns: vec![
            pat!("x-vercel-", WebPatternType::Header, WebConfidence::High),
            pat!(".vercel.app", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Netlify".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://www.netlify.com".to_string()),
        patterns: vec![
            pat!("x-nf-", WebPatternType::Header, WebConfidence::High),
            pat!(".netlify.app", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Cloudflare".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://www.cloudflare.com".to_string()),
        patterns: vec![
            pat!("cf-ray", WebPatternType::Header, WebConfidence::High),
            pat!(".pages.dev", WebPatternType::Url, WebConfidence::High),
            pat!(".workers.dev", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "AWS".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://aws.amazon.com".to_string()),
        patterns: vec![
            pat!("x-amz-", WebPatternType::Header, WebConfidence::High),
            pat!("x-amzn-", WebPatternType::Header, WebConfidence::High),
            pat!(".amazonaws.com", WebPatternType::Url, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Azure".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://azure.microsoft.com".to_string()),
        patterns: vec![
            pat!("x-azure-", WebPatternType::Header, WebConfidence::High),
            pat!(".azurewebsites.net", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Google Cloud".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://cloud.google.com".to_string()),
        patterns: vec![
            pat!("x-google-", WebPatternType::Header, WebConfidence::High),
            pat!(".appspot.com", WebPatternType::Url, WebConfidence::High),
            pat!(".cloudfunctions.net", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Railway".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://railway.app".to_string()),
        patterns: vec![
            pat!("x-railway-", WebPatternType::Header, WebConfidence::High),
            pat!(".railway.app", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Fly.io".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://fly.io".to_string()),
        patterns: vec![
            pat!("x-fly-", WebPatternType::Header, WebConfidence::High),
            pat!(".fly.dev", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Render".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://render.com".to_string()),
        patterns: vec![
            pat!("x-render-", WebPatternType::Header, WebConfidence::High),
            pat!(".onrender.com", WebPatternType::Url, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Heroku".to_string(),
        category: WebCategory::Hosting,
        website: Some("https://www.heroku.com".to_string()),
        patterns: vec![
            pat!("x-heroku-", WebPatternType::Header, WebConfidence::High),
            pat!(".herokuapp.com", WebPatternType::Url, WebConfidence::High),
        ],
    });

    // ============================================
    // ANALYTICS
    // ============================================
    fps.push(WebFingerprint {
        name: "Google Analytics".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://analytics.google.com".to_string()),
        patterns: vec![
            pat!("ga_measurement_id", WebPatternType::Content, WebConfidence::High),
            pat!("gtag(", WebPatternType::Content, WebConfidence::High),
            pat!("datalayer", WebPatternType::Content, WebConfidence::High),
            pat!("googletagmanager.com", WebPatternType::Content, WebConfidence::High),
            pat!("google-analytics.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Plausible".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://plausible.io".to_string()),
        patterns: vec![
            pat!("plausible.io/js/script.js", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "PostHog".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://posthog.com".to_string()),
        patterns: vec![
            pat!("posthog", WebPatternType::Content, WebConfidence::High),
            pat!("posthog-js", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Segment".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://segment.com".to_string()),
        patterns: vec![
            pat!("segment.com", WebPatternType::Content, WebConfidence::High),
            pat!("analytics-js", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Hotjar".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://www.hotjar.com".to_string()),
        patterns: vec![
            pat!("static.hotjar.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Mixpanel".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://mixpanel.com".to_string()),
        patterns: vec![
            pat!("mixpanel", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Amplitude".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://amplitude.com".to_string()),
        patterns: vec![
            pat!("analytics.amplitude.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Umami".to_string(),
        category: WebCategory::Analytics,
        website: Some("https://umami.is".to_string()),
        patterns: vec![
            pat!("umami.is", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // MONITORING / ERROR TRACKING
    // ============================================
    fps.push(WebFingerprint {
        name: "Sentry".to_string(),
        category: WebCategory::Monitoring,
        website: Some("https://sentry.io".to_string()),
        patterns: vec![
            pat!("__sentry__", WebPatternType::Content, WebConfidence::High),
            pat!("browser.sentry-cdn.com", WebPatternType::Content, WebConfidence::High),
            pat!("sentry.io", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "LogRocket".to_string(),
        category: WebCategory::Monitoring,
        website: Some("https://logrocket.com".to_string()),
        patterns: vec![
            pat!("logrocket", WebPatternType::Content, WebConfidence::High),
            pat!("logrocket.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Bugsnag".to_string(),
        category: WebCategory::Monitoring,
        website: Some("https://www.bugsnag.com".to_string()),
        patterns: vec![
            pat!("bugsnag", WebPatternType::Content, WebConfidence::High),
            pat!("bugsnag.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Datadog".to_string(),
        category: WebCategory::Monitoring,
        website: Some("https://www.datadoghq.com".to_string()),
        patterns: vec![
            pat!("dd_logs", WebPatternType::Content, WebConfidence::High),
            pat!("dd_rum", WebPatternType::Content, WebConfidence::High),
            pat!("datadoghq.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // AUTHENTICATION
    // ============================================
    fps.push(WebFingerprint {
        name: "Auth0".to_string(),
        category: WebCategory::Auth,
        website: Some("https://auth0.com".to_string()),
        patterns: vec![
            pat!("auth0", WebPatternType::Content, WebConfidence::High),
            pat!("cdn.auth0.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Firebase Auth".to_string(),
        category: WebCategory::Auth,
        website: Some("https://firebase.google.com/products/auth".to_string()),
        patterns: vec![
            pat!("firebase.auth()", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Clerk".to_string(),
        category: WebCategory::Auth,
        website: Some("https://clerk.com".to_string()),
        patterns: vec![
            pat!("clerk.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "NextAuth".to_string(),
        category: WebCategory::Auth,
        website: Some("https://next-auth.js.org".to_string()),
        patterns: vec![
            pat!("next-auth", WebPatternType::Content, WebConfidence::High),
            pat!("getserversession", WebPatternType::Content, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Supabase Auth".to_string(),
        category: WebCategory::Auth,
        website: Some("https://supabase.com/auth".to_string()),
        patterns: vec![
            pat!("supabase.auth", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // PAYMENT
    // ============================================
    fps.push(WebFingerprint {
        name: "Stripe".to_string(),
        category: WebCategory::Payment,
        website: Some("https://stripe.com".to_string()),
        patterns: vec![
            pat!("stripe(", WebPatternType::Content, WebConfidence::High),
            pat!("stripe.com", WebPatternType::Content, WebConfidence::High),
            pat!("js.stripe.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "PayPal".to_string(),
        category: WebCategory::Payment,
        website: Some("https://www.paypal.com".to_string()),
        patterns: vec![
            pat!("paypal", WebPatternType::Content, WebConfidence::High),
            pat!("paypalobjects.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Shopify".to_string(),
        category: WebCategory::Payment,
        website: Some("https://www.shopify.com".to_string()),
        patterns: vec![
            pat!("shopify", WebPatternType::Content, WebConfidence::High),
            pat!("cdn.shopify.com", WebPatternType::Content, WebConfidence::High),
        ],
    });

    // ============================================
    // BACKEND (inferred from headers/patterns)
    // ============================================
    fps.push(WebFingerprint {
        name: "Express".to_string(),
        category: WebCategory::Backend,
        website: Some("https://expressjs.com".to_string()),
        patterns: vec![
            pat!("x-powered-by: express", WebPatternType::Header, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Django".to_string(),
        category: WebCategory::Backend,
        website: Some("https://www.djangoproject.com".to_string()),
        patterns: vec![
            pat!("csrftoken", WebPatternType::Cookie, WebConfidence::Medium),
        ],
    });

    fps.push(WebFingerprint {
        name: "Rails".to_string(),
        category: WebCategory::Backend,
        website: Some("https://rubyonrails.org".to_string()),
        patterns: vec![
            pat!("x-runtime:", WebPatternType::Header, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Laravel".to_string(),
        category: WebCategory::Backend,
        website: Some("https://laravel.com".to_string()),
        patterns: vec![
            pat!("xsrf-token", WebPatternType::Cookie, WebConfidence::Medium),
            pat!("laravel_session", WebPatternType::Cookie, WebConfidence::High),
        ],
    });

    fps.push(WebFingerprint {
        name: "Spring Boot".to_string(),
        category: WebCategory::Backend,
        website: Some("https://spring.io/projects/spring-boot".to_string()),
        patterns: vec![
            pat!("x-application-context:", WebPatternType::Header, WebConfidence::High),
        ],
    });

    fps
}

/// Global singleton for web fingerprint engine
pub static WEB_FINGERPRINT_ENGINE: Lazy<WebFingerprintEngine> = Lazy::new(WebFingerprintEngine::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = WebFingerprintEngine::new();
        assert!(!engine.fingerprints().is_empty());
    }

    #[test]
    fn test_detect_react() {
        let engine = WebFingerprintEngine::new();
        let detections = engine.detect(&WebFingerprintInput {
            content: "data-reactroot class=\"App\" __REACT__".to_string(),
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "React"));
    }

    #[test]
    fn test_detect_nextjs() {
        let engine = WebFingerprintEngine::new();
        let detections = engine.detect(&WebFingerprintInput {
            content: r#"<script id="__NEXT_DATA__" type="application/json">"#.to_string(),
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "Next.js"));
    }

    #[test]
    fn test_detect_vercel_hosting() {
        let engine = WebFingerprintEngine::new();
        let mut headers = HashMap::new();
        headers.insert("x-vercel-id".to_string(), "iad1::12345".to_string());

        let detections = engine.detect(&WebFingerprintInput {
            headers,
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "Vercel"));
    }

    #[test]
    fn test_detect_multiple() {
        let engine = WebFingerprintEngine::new();
        let detections = engine.detect(&WebFingerprintInput {
            content: r#"
                data-reactroot __NEXT_DATA__
                tailwindcss @tanstack/react-query
                gtag( googletagmanager.com
            "#.to_string(),
            ..Default::default()
        });

        let names: Vec<_> = detections.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"React"));
        assert!(names.contains(&"Next.js"));
        assert!(names.contains(&"Tailwind CSS"));
        assert!(names.contains(&"TanStack Query"));
        assert!(names.contains(&"Google Analytics"));
    }

    #[test]
    fn test_categories() {
        let engine = WebFingerprintEngine::new();
        let categories = engine.categories();
        assert!(categories.contains(&WebCategory::Frontend));
        assert!(categories.contains(&WebCategory::Ui));
        assert!(categories.contains(&WebCategory::Analytics));
    }

    #[test]
    fn test_fingerprints_by_category() {
        let engine = WebFingerprintEngine::new();
        let frameworks = engine.fingerprints_by_category(WebCategory::Frontend);
        assert!(frameworks.iter().any(|fp| fp.name == "React"));
        assert!(frameworks.iter().any(|fp| fp.name == "Vue"));
        assert!(frameworks.iter().any(|fp| fp.name == "Next.js"));
    }

    #[test]
    fn test_url_detection() {
        let engine = WebFingerprintEngine::new();
        let detections = engine.detect(&WebFingerprintInput {
            url: Some("https://myapp.vercel.app/api".to_string()),
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "Vercel"));
    }

    #[test]
    fn test_global_engine() {
        let detections = WEB_FINGERPRINT_ENGINE.detect(&WebFingerprintInput {
            content: "react-root".to_string(),
            ..Default::default()
        });

        assert!(detections.iter().any(|d| d.name == "React"));
    }
}
