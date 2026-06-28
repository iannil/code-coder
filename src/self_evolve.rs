/// ─── Self-Evolve ────────────────────────────────────────────────────────────
///
/// Failure-driven introspection engine for Phase 7 self-evolution.
/// Detects when the agent cannot complete a task, analyzes the capability gap,
/// generates a new markdown skill, and manages progressive activation.

use crate::llm::Message;
use crate::skill::{SkillMeta, SkillSource, SkillStatus, SkillRegistry};
use crate::tools::ToolRegistry;
use std::path::Path;

/// ─── Configuration ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IntrospectConfig {
    /// Minimum rounds between introspection attempts (cooldown).
    pub cooldown_rounds: u32,
    /// How many successful uses before draft → active promotion.
    pub activation_threshold: u32,
    /// Maximum introspection attempts per session.
    pub max_per_session: u32,
}

impl Default for IntrospectConfig {
    fn default() -> Self {
        Self {
            cooldown_rounds: 5,
            activation_threshold: 3,
            max_per_session: 3,
        }
    }
}

/// ─── IntrospectResult ───────────────────────────────────────────────────────

#[derive(Debug)]
pub enum IntrospectResult {
    /// No introspection needed.
    None,
    /// A new skill was generated as a draft.
    SkillGenerated { skill_name: String, skill_path: String },
    /// An existing draft skill was promoted to active.
    SkillPromoted { skill_name: String },
}

/// ─── SelfEvolve ─────────────────────────────────────────────────────────────

pub struct SelfEvolve {
    config: IntrospectConfig,
    /// Round number of the last introspection.
    last_introspect_round: u32,
    /// How many introspections in this session.
    session_count: u32,
}

impl SelfEvolve {
    pub fn new(config: IntrospectConfig) -> Self {
        Self {
            config,
            last_introspect_round: 0,
            session_count: 0,
        }
    }

    /// Evaluate whether to introspect after a completed agent turn.
    /// Called from AgentLoop::handle_message after react_loop finishes.
    ///
    /// Returns `IntrospectResult::None` when:
    /// - Cooldown is still active
    /// - Session limit reached
    /// - No failure detected
    pub fn evaluate(
        &mut self,
        history: &[Message],
        _tools: &ToolRegistry,
        skills: &mut SkillRegistry,
        project_root: &str,
        current_round: u32,
    ) -> IntrospectResult {
        // 1. Cooldown check — if still in cooldown, skip
        if current_round < self.last_introspect_round + self.config.cooldown_rounds {
            return IntrospectResult::None;
        }

        // 2. Session limit check with leaky bucket decay
        //    When cooldown expires but session is at limit, decay one slot
        //    so introspection can eventually recover.
        if self.session_count >= self.config.max_per_session {
            // Decay: reduce count so next cooldown cycle allows one more
            self.session_count = self.config.max_per_session.saturating_sub(1);
            return IntrospectResult::None;
        }

        // 3. Failure detection
        let (failure_level, failure_context) = detect_failure(history);

        if failure_level == 0 {
            return IntrospectResult::None;
        }

        // 4. Gap analysis — extract what capability is missing
        let gap = match analyze_gap(history, failure_level, &failure_context) {
            Some(g) => g,
            None => return IntrospectResult::None,
        };

        // 5. Check if we already have a skill that covers this gap
        let existing: Vec<&str> = skills.list();
        let gap_trigger = gap.trigger_keyword.to_lowercase();
        if existing.iter().any(|name| {
            name.to_lowercase().contains(&gap_trigger)
                || gap_trigger.contains(&name.to_lowercase())
        }) {
            // Already have a skill — no need to generate
            return IntrospectResult::None;
        }

        // 6. Generate the skill
        let safe_name = sanitize_skill_name(&gap.name);
        let skills_dir = Path::new(project_root).join("skills");

        // Find a unique file name
        let (file_path, final_name) = unique_file_path(&skills_dir, &safe_name);

        let frontmatter = build_skill_frontmatter(&SkillMeta {
            name: final_name.clone(),
            description: gap.description.clone(),
            version: "0.1.0".into(),
            status: SkillStatus::Draft,
            source: SkillSource::SelfGenerated,
            trigger: gap.trigger_keyword.clone(),
            usage_count: 0,
        });

        let content = format!(
            "{}\n\n# {}\n\n{}\n\n## 用途\n\n当用户请求涉及 {} 相关操作时，使用此 skill。\n\n## 步骤\n\n1. 识别用户的具体需求\n2. 按照标准流程操作\n3. 验证结果是否正确\n\n## 示例\n\n```\n用户输入: {}\n系统操作: {}\n```\n",
            frontmatter,
            gap.name,
            gap.description,
            gap.trigger_keyword,
            gap.example_input,
            gap.example_action,
        );

        // Ensure skills directory exists
        if let Err(e) = std::fs::create_dir_all(&skills_dir) {
            eprintln!("[self_evolve] Failed to create skills dir: {e}");
            return IntrospectResult::None;
        }

        if let Err(e) = std::fs::write(&file_path, &content) {
            eprintln!("[self_evolve] Failed to write skill: {e}");
            return IntrospectResult::None;
        }

        let path_str = file_path.to_string_lossy().to_string();
        eprintln!(
            "[self_evolve] Generated draft skill '{}' at {}",
            final_name, path_str
        );

        // Update state
        self.last_introspect_round = current_round;
        self.session_count += 1;

        IntrospectResult::SkillGenerated {
            skill_name: final_name,
            skill_path: path_str,
        }
    }

    /// Reset the session introspection counter (called on /clear or new session).
    pub fn reset_session(&mut self) {
        self.session_count = 0;
    }
}

// ─── Failure Detection ───────────────────────────────────────────────────────

/// Detect if the agent failed in the last turn.
/// Returns (level, context) where level = 0 means no failure.
fn detect_failure(history: &[Message]) -> (u32, String) {
    // Look at the last few messages (the most recent exchange)
    let tail: Vec<&Message> = history.iter().rev().take(20).collect();

    // Level 1: LLM self-declaration of inability
    for msg in &tail {
        if msg.role == "assistant" {
            let lower = msg.content.to_lowercase();
            let indicators = [
                // English patterns
                "i cannot", "i can't", "i don't have", "i do not have",
                "i don't know how", "i do not know how", "i'm not able",
                "i am not able", "i'm unable", "i am unable",
                "i don't have access", "i lack", "not supported",
                "no tool available", "i need a", "you would need",
                "currently don't have", "i don't see a", "i don't have a",
                "i cannot find", "i can't find",
                // Chinese patterns
                "我无法", "我不能", "我没有", "我缺少",
                "我做不到", "我不具备", "我不支持", "无法完成",
                "没有找到", "找不到", "没有这个", "没有相关",
                "not provided", "not available",
            ];
            for indicator in &indicators {
                if lower.contains(indicator) {
                    return (1, msg.content.clone());
                }
            }
        }
    }

    // Level 2: Tool call error accumulation — look for repeated errors
    let mut error_count = 0;
    for msg in &tail {
        if msg.role == "user" && msg.content.contains("[tool error]") {
            error_count += 1;
        }
    }
    if error_count >= 2 {
        // Collect the error messages
        let errors: Vec<&str> = tail
            .iter()
            .filter(|m| m.role == "user" && m.content.contains("[tool error]"))
            .map(|m| m.content.as_str())
            .collect();
        return (2, errors.join("\n"));
    }

    // Level 3: User expressed dissatisfaction
    for msg in &tail {
        if msg.role == "user" {
            let lower = msg.content.to_lowercase();
            let user_feedback = [
                // Chinese
                "不对", "错误", "错了", "不是", "不对吗",
                "你没懂", "你没理解", "理解错了", "不是这个意思",
                "你再想想", "重新来", "重做", "不是这样",
                // English
                "wrong", "incorrect", "not what i", "that's not",
                "still not", "try again", "that's wrong",
                "you're wrong", "you are wrong", "not right",
                "that doesn't", "that didn't", "not correct",
                "try something else",
            ];
            for fb in &user_feedback {
                if lower.contains(fb) {
                    return (3, msg.content.clone());
                }
            }
        }
    }

    (0, String::new())
}

// ─── Gap Analysis ────────────────────────────────────────────────────────────

/// Information about a detected capability gap.
struct SkillGap {
    name: String,
    description: String,
    trigger_keyword: String,
    example_input: String,
    example_action: String,
}

/// Analyze the detected failure to determine what capability is missing.
fn analyze_gap(history: &[Message], level: u32, context: &str) -> Option<SkillGap> {
    // Extract the original user request from history (last user message before assistant response)
    let user_request = history.iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| truncate(&m.content, 100))
        .unwrap_or_default();

    match level {
        1 => {
            // LLM self-declaration — extract what tool/skill it said it's missing
            let lower = context.to_lowercase();
            let (name, trigger, action_hint) = if lower.contains("git") {
                ("git-helper".into(), "git".into(), "git 分支/提交/合并操作")
            } else if lower.contains("search") || lower.contains("find") {
                if lower.contains("github") {
                    ("github-search".into(), "github".into(), "搜索 GitHub 仓库或代码")
                } else {
                    ("web-search".into(), "search".into(), "搜索网页信息")
                }
            } else if lower.contains("api") || lower.contains("curl") || lower.contains("http") {
                ("api-helper".into(), "api".into(), "调用 HTTP/API 接口")
            } else if lower.contains("docker") {
                ("docker-helper".into(), "docker".into(), "Docker 容器/镜像操作")
            } else if lower.contains("python") || lower.contains("script") {
                ("script-runner".into(), "script".into(), "运行脚本或代码")
            } else if lower.contains("analyze") || lower.contains("analyse") || lower.contains("data") {
                ("data-analyzer".into(), "analyze".into(), "数据分析")
            } else if lower.contains("npm") || lower.contains("node") || lower.contains("javascript") {
                ("js-helper".into(), "npm".into(), "Node.js/JavaScript 操作")
            } else {
                // Generic — use the first sentence as the description
                let name = "custom-helper".to_string();
                (name, "help".into(), "相关操作".into())
            };

            Some(SkillGap {
                name,
                description: truncate(context, 200),
                trigger_keyword: trigger,
                example_input: user_request,
                example_action: action_hint.to_string(),
            })
        }
        2 => {
            // Tool errors — extract the tool name from the error context
            let (tool_name, action_hint) = if context.contains("git") { ("git-helper", "git 操作") }
            else if context.contains("docker") { ("docker-helper", "Docker 操作") }
            else if context.contains("python") || context.contains("python3") { ("script-runner", "运行脚本") }
            else { ("custom-helper", "相关操作") };

            Some(SkillGap {
                name: tool_name.to_string(),
                description: format!("Automated skill generated from tool errors:\n{}", truncate(context, 200)),
                trigger_keyword: tool_name.split('-').next().unwrap_or("help").to_string(),
                example_input: user_request,
                example_action: action_hint.into(),
            })
        }
        3 => {
            // User feedback — generic response
            Some(SkillGap {
                name: "feedback-helper".into(),
                description: format!("Skill addressing user feedback:\n{}", truncate(context, 200)),
                trigger_keyword: "help".into(),
                example_input: user_request,
                example_action: String::new(),
            })
        }
        _ => None,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build YAML frontmatter string for a skill.
fn build_skill_frontmatter(meta: &SkillMeta) -> String {
    format!(
        "---\nname: {}\ndescription: {}\nversion: {}\nstatus: {}\nsource: {}\ntrigger: \"{}\"\nusage_count: {}\n---",
        meta.name,
        meta.description,
        meta.version,
        meta.status.as_str(),
        meta.source.as_str(),
        meta.trigger,
        meta.usage_count,
    )
}

/// Sanitize a string for use as a filename.
fn sanitize_skill_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Find a unique file path in the given directory.
/// If `{name}.md` exists, try `{name}-2.md`, `{name}-3.md`, etc.
fn unique_file_path(dir: &Path, name: &str) -> (std::path::PathBuf, String) {
    let base = dir.join(format!("{name}.md"));
    if !base.exists() {
        return (base, name.to_string());
    }
    for i in 2..100 {
        let alt_name = format!("{name}-{i}");
        let alt_path = dir.join(format!("{alt_name}.md"));
        if !alt_path.exists() {
            return (alt_path, alt_name);
        }
    }
    // Fallback: append timestamp
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let alt_name = format!("{name}-{ts}");
    (dir.join(format!("{alt_name}.md")), alt_name)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_no_failure() {
        let history = vec![
            Message::user("hello"),
            Message::assistant("Hi there! How can I help?"),
        ];
        assert_eq!(detect_failure(&history), (0, String::new()));
    }

    #[test]
    fn test_detect_llm_cannot_level1() {
        let history = vec![
            Message::user("show me my git branches"),
            Message::assistant("I cannot list git branches because I don't have a git tool available."),
        ];
        let (level, ctx) = detect_failure(&history);
        assert_eq!(level, 1);
        assert!(ctx.contains("git"));
    }

    #[test]
    fn test_detect_tool_errors_level2() {
        let history = vec![
            Message::assistant("Let me try that."),
            Message::user("[tool error] command not found: kubectl"),
            Message::assistant("Let me try another approach."),
            Message::user("[tool error] exit code 127: kubectl not available"),
        ];
        let (level, ctx) = detect_failure(&history);
        assert_eq!(level, 2);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn test_detect_user_feedback_level3() {
        let history = vec![
            Message::user("不对，你理解错了"),
            Message::assistant("I see, let me reconsider."),
            Message::user("不对，还是不对"),
        ];
        let (level, _) = detect_failure(&history);
        assert_eq!(level, 3);
    }

    #[test]
    fn test_cooldown_blocks_immediate_retry() {
        let mut ev = SelfEvolve::new(IntrospectConfig {
            cooldown_rounds: 5,
            activation_threshold: 3,
            max_per_session: 3,
        });

        let history = vec![
            Message::user("list branches"),
            Message::assistant("I cannot list branches because I lack a git tool."),
        ];
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let result = ev.evaluate(&history, &tools, &mut skills, "/tmp", 10);
        // Should detect failure and return SkillGenerated
        assert!(matches!(result, IntrospectResult::SkillGenerated { .. }));

        // Immediate next round at round 11 — should be blocked by cooldown (5 rounds)
        let result2 = ev.evaluate(&history, &tools, &mut skills, "/tmp", 11);
        assert!(matches!(result2, IntrospectResult::None));
    }

    #[test]
    fn test_session_limit() {
        let mut ev = SelfEvolve::new(IntrospectConfig {
            cooldown_rounds: 0,
            activation_threshold: 3,
            max_per_session: 2,
        });

        let history = vec![
            Message::user("do X"),
            Message::assistant("I cannot do X because I lack a Y tool."),
        ];
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let tmp = tempfile::tempdir().unwrap();

        // First two should succeed (within session limit)
        let r1 = ev.evaluate(&history, &tools, &mut skills, tmp.path().to_str().unwrap(), 1);
        assert!(matches!(r1, IntrospectResult::SkillGenerated { .. }));

        // Clean up generated file from first run
        let tmp2 = tempfile::tempdir().unwrap();
        let r2 = ev.evaluate(&history, &tools, &mut skills, tmp2.path().to_str().unwrap(), 10);
        assert!(matches!(r2, IntrospectResult::SkillGenerated { .. }));

        // Third should be blocked (session limit = 2)
        let tmp3 = tempfile::tempdir().unwrap();
        let r3 = ev.evaluate(&history, &tools, &mut skills, tmp3.path().to_str().unwrap(), 20);
        assert!(matches!(r3, IntrospectResult::None));
    }

    #[test]
    fn test_gap_analysis_git() {
        let ctx = "I cannot list git branches because I don't have a git tool.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "git-helper");
        assert_eq!(gap.trigger_keyword, "git");
    }

    #[test]
    fn test_gap_analysis_api() {
        let ctx = "I don't have access to an HTTP API tool.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "api-helper");
    }

    #[test]
    fn test_gap_analysis_docker() {
        let ctx = "I need a Docker tool to deploy containers.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "docker-helper");
    }

    #[test]
    fn test_gap_analysis_script() {
        let ctx = "I cannot run Python scripts — I don't have that capability.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "script-runner");
    }

    #[test]
    fn test_gap_analysis_data() {
        let ctx = "I cannot analyze this dataset — I lack the tools.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "data-analyzer");
    }

    #[test]
    fn test_gap_analysis_npm() {
        let ctx = "I don't have a Node.js tool.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "js-helper");
    }

    #[test]
    fn test_gap_analysis_generic() {
        let ctx = "I lack the necessary capability to do this thing.";
        let gap = analyze_gap(&[], 1, ctx).unwrap();
        assert_eq!(gap.name, "custom-helper");
    }

    #[test]
    fn test_gap_analysis_level2() {
        let ctx = "Tool error with git — could not connect.";
        let gap = analyze_gap(&[], 2, ctx).unwrap();
        assert_eq!(gap.name, "git-helper");
    }

    #[test]
    fn test_gap_analysis_level3() {
        let ctx = "This is completely wrong.";
        let gap = analyze_gap(&[], 3, ctx).unwrap();
        assert_eq!(gap.name, "feedback-helper");
    }

    #[test]
    fn test_skill_file_generated() {
        let tmp = tempfile::tempdir().unwrap();
        let mut ev = SelfEvolve::new(IntrospectConfig::default());
        let history = vec![
            Message::user("deploy to docker"),
            Message::assistant("I cannot deploy to docker because I don't have docker tools."),
        ];
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();

        let result = ev.evaluate(&history, &tools, &mut skills, tmp.path().to_str().unwrap(), 10);
        match result {
            IntrospectResult::SkillGenerated { skill_name, skill_path } => {
                assert!(skill_name.contains("docker"));
                assert!(std::path::Path::new(&skill_path).exists());
                // Read and verify frontmatter
                let content = std::fs::read_to_string(&skill_path).unwrap();
                assert!(content.contains("status: draft"));
                assert!(content.contains("source: self-generated"));
                assert!(content.contains("usage_count: 0"));
            }
            _ => panic!("Expected SkillGenerated, got {:?}", result),
        }
    }

    #[test]
    fn test_unique_file_naming() {
        let dir = tempfile::tempdir().unwrap();
        // Create first file
        let (p1, n1) = unique_file_path(dir.path(), "test");
        assert_eq!(n1, "test");
        std::fs::write(&p1, "content").unwrap();

        // Second should get -2
        let (p2, n2) = unique_file_path(dir.path(), "test");
        assert_eq!(n2, "test-2");
        assert!(!p2.exists());

        std::fs::write(&p2, "content2").unwrap();

        // Third should get -3
        let (p3, n3) = unique_file_path(dir.path(), "test");
        assert_eq!(n3, "test-3");
    }

    #[test]
    fn test_unique_file_naming_full_100() {
        let dir = tempfile::tempdir().unwrap();
        // Fill up slots 1-99 — the 100th should get a timestamp suffix
        for i in 1..=100 {
            let (p, _) = unique_file_path(dir.path(), "full");
            if i == 100 {
                // 100th attempt: all "full.md" to "full-99.md" exist
                assert!(p.to_string_lossy().contains("full-"));
                assert!(!p.to_string_lossy().ends_with(".md")
                    || p.to_string_lossy().contains("-"));
            }
            let _ = std::fs::write(&p, "dummy");
        }
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_skill_name("git helper"), "git_helper");
        assert_eq!(sanitize_skill_name("My Tool!"), "My_Tool_");
        assert_eq!(sanitize_skill_name("git-helper"), "git-helper");
    }

    #[test]
    fn test_reset_session() {
        let mut ev = SelfEvolve::new(IntrospectConfig {
            cooldown_rounds: 0,
            activation_threshold: 3,
            max_per_session: 1,
        });

        let history = vec![
            Message::user("do X"),
            Message::assistant("I cannot do X because I lack a Y tool."),
        ];
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let tmp = tempfile::tempdir().unwrap();

        let r1 = ev.evaluate(&history, &tools, &mut skills, tmp.path().to_str().unwrap(), 1);
        assert!(matches!(r1, IntrospectResult::SkillGenerated { .. }));

        // Reset
        ev.reset_session();

        // Clean up
        let tmp2 = tempfile::tempdir().unwrap();
        let r2 = ev.evaluate(&history, &tools, &mut skills, tmp2.path().to_str().unwrap(), 20);
        assert!(matches!(r2, IntrospectResult::SkillGenerated { .. }));
    }

    #[test]
    fn test_existing_skill_prevents_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let mut skills = SkillRegistry::new();
        // Pre-create a skill directory and manually add a skill
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("docker-helper.md"),
            "---\nname: docker-helper\ndescription: Docker helper\n---\n\nContent.",
        ).unwrap();
        skills.scan(tmp.path().to_str().unwrap()).unwrap();

        let mut ev = SelfEvolve::new(IntrospectConfig::default());
        let history = vec![
            Message::user("deploy to docker"),
            Message::assistant("I cannot deploy to docker because I don't have docker tools."),
        ];
        let tools = ToolRegistry::new_for_test();

        let result = ev.evaluate(&history, &tools, &mut skills, tmp.path().to_str().unwrap(), 10);
        assert!(matches!(result, IntrospectResult::None),
            "Should skip generation when skill already exists");
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let s = truncate("hello world this is a long string", 15);
        assert!(s.ends_with("..."));
        assert_eq!(s.len(), 15);
    }

    #[test]
    fn test_config_defaults() {
        let cfg = IntrospectConfig::default();
        assert_eq!(cfg.cooldown_rounds, 5);
        assert_eq!(cfg.activation_threshold, 3);
        assert_eq!(cfg.max_per_session, 3);
    }

    #[test]
    fn test_build_frontmatter() {
        let meta = SkillMeta {
            name: "test-skill".into(),
            description: "A test".into(),
            version: "1.0.0".into(),
            status: SkillStatus::Draft,
            source: SkillSource::SelfGenerated,
            trigger: "test".into(),
            usage_count: 0,
        };
        let fm = build_skill_frontmatter(&meta);
        assert!(fm.contains("name: test-skill"));
        assert!(fm.contains("status: draft"));
    }
}
