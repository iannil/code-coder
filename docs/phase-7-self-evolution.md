# Phase 7 — 自我进化型 Agent

> 设计文档 · 源自 `/grill-me` 深度访谈（14 轮决策树完结合）

## 概述

Phase 7 的目标是让 CodeCoder 具备 **自我进化能力**：当系统无法完成用户请求时，能主动分析能力缺口，自行生成新的 markdown skill 来弥补，并渐进式验证有效性。

这是从"强大的工具执行器"到"真正的自主 agent"的关键一跃。

---

## 1. 核心设计决策（14 项）

| # | 决策 | 结论 |
|---|------|------|
| 1 | **目标层次** | C. 自我进化型自主 |
| 2 | **自省触发方式** | A. 失败驱动的自省 (Failure-Driven Introspection) |
| 3 | **失败判定信号** | D. 分层综合检测（LLM 自我声明 → 工具错误 → 用户反馈） |
| 4 | **修复优先级** | B. Skill 优先，层次升级（markdown → Rust/WASM） |
| 5 | **自我验证方式** | D. 渐进式激活（Draft → Active） |
| 6 | **激活/升级条件** | 多路径并行：首次成功即激活 / 连续 3 次成功 / N 次失败升级 / 显式确认 |
| 7 | **Gap Analysis 方法** | C. 理想步骤 vs 现有能力 Diff |
| 8 | **循环防护** | C. 冷却期（每次自省后至少隔 5 轮对话） |
| 9 | **热加载方式** | D. 分情况处理（TUI 提示 /reload，自主模式自动 reload） |
| 10 | **Skill 元数据格式** | A. 完整 YAML frontmatter |
| 11 | **生成路径** | C. 两条路径平行（generate_skill 工具保持旧格式，self_evolve 内部函数用新格式） |
| 12 | **运行时状态存储** | D. 双写同步（文件 frontmatter + memory store，启动时以文件为准） |
| 13 | **模块结构** | A. 独立模块 `src/self_evolve.rs`，被 AgentLoop 内部调用 |
| 14 | **MVP 范围** | 全部 7 项：设计文档 + 核心循环 + frontmatter 解析 + usage 追踪 + AgentLoop 集成 + 冷却期 + 晋升逻辑 |

---

## 2. 架构总览

```
AgentLoop (react_loop)
    │
    ├── 正常流程
    │    user message → LLM → tool calls → LLM → response
    │
    └── 失败流程（新增）
         detect_failure() ─► self_evolve模块
                                  │
                          ┌───────┼──────────────┐
                          ▼       ▼              ▼
                   gap_analysis  ─► skill_generator
                                      │
                                      ▼
                                  write_skill()
                                  (双写同步: file + memory)
                                      │
                                      ▼
                                  start_cooldown()
                                  ─► next turn continues
```

---

## 3. 模块接口设计

### 3.1 `src/self_evolve.rs` — 公共 API

```rust
// ─── 自省引擎 ─────────────────────────────────────────────────────

/// 自省结果
pub enum IntrospectResult {
    /// 不需要自省（继续正常流程）
    None,
    /// 已生成草稿 skill
    SkillGenerated { skill_name: String, skill_path: String },
    /// 已晋升 skill（draft → active）
    SkillPromoted { skill_name: String },
    /// 需要升级到 Rust 工具（但当前 MVP 不实现）
    NeedsRustTool { reason: String },
}

/// 自省配置（从 Context 或 Config 加载）
pub struct IntrospectConfig {
    /// 冷却期：最少间隔的对话轮数
    pub cooldown_rounds: u32,           // 默认 5
    /// 草稿→激活所需的最低使用次数
    pub activation_threshold: u32,       // 默认 3
    /// 草稿失败几次后标记为需升级
    pub escalation_threshold: u32,       // 默认 3
    /// 每轮对话的自省次数上限
    pub max_per_session: u32,            // 默认 3
}

/// 自省引擎
pub struct SelfEvolve {
    config: IntrospectConfig,
    /// 冷却期追踪（对话轮次计数器）
    last_introspect_round: Cell<u32>,
    /// 本对话已自省次数
    session_introspect_count: Cell<u32>,
}

impl SelfEvolve {
    pub fn new(config: IntrospectConfig) -> Self;

    /// 主要入口：在 AgentLoop 每轮对话结束后调用
    /// 返回是否需要采取行动
    pub fn evaluate(
        &self,
        turn_history: &[Message],
        tool_results: &[ToolResult],
        skills: &SkillRegistry,
        memory: &mut MemoryStore,
        round_number: u32,
    ) -> anyhow::Result<IntrospectResult>;
}
```

### 3.2 核心子流程

#### 3.2.1 失败检测 `detect_failure()`

分层信号，从轻到重：

```
Level 1 — LLM 自我声明
  LLM 输出包含:
  - "我无法完成这个任务"
  - "我缺少 X 工具"
  - "我没有访问 Y 的能力"

Level 2 — 工具调用失败累积
  连续 2+ 个 tool call 返回 error
  且 LLM 没有成功 recover

Level 3 — 用户反馈
  用户重复相似的请求
  或 / 回复包含 "不对"、"不是"、"错误"
```

只有当 **超过某一层级阈值** 时才触发自省，默认 Level 2 及以上触发。Level 1 的 LLM 自我声明可以直接跳过检测进入 gap analysis。

#### 3.2.2 缺口分析 `gap_analysis()`

```
输入: 用户原始请求 + 最近 N 条 tool call 的调用链 + 错误信息

步骤:
1. LLM 生成"理想执行计划"——要完成这个请求需要哪些步骤
2. 将每一步映射到当前 ToolRegistry 和 SkillRegistry 中的能力
3. 找出无法覆盖的步骤 → 这就是能力缺口
4. 判断缺口能否用 markdown skill 补足（能 → 生成 skill / 不能 → 标记为 NeedsRustTool）

输出: SkillSpec { name, description, trigger_keywords, content_template }
```

#### 3.2.3 Skill 生成 `skill_generator()`

生成的 skill 使用完整 frontmatter：

```markdown
---
name: git-workflow-helper
description: 帮助用户完成常见的 git 工作流操作（commit, branch, merge）
version: 0.1.0
status: draft
source: self-generated
trigger: "git"
usage_count: 0
---

## 用途

<!-- 触发条件和使用场景 -->

## 步骤

<!-- 具体的操作步骤 -->

## 示例

<!-- 输入输出示例 -->
```

生成后的流程：
1. 写入 `skills/<name>.md`（带 frontmatter，usage_count=0, status=draft）
2. MemoryStore 写入 `skill:<name>:usage_count = "0"` 和 `skill:<name>:status = "draft"`
3. 如果当前是自主模式 → 调用 `skills.scan()` 热加载
4. 如果当前是 TUI 模式 → 返回提示信息 "/reload 以加载新 skill"
5. 启动冷却期

#### 3.2.4 渐进式激活 `activation_manager()`

每次 skill 被成功使用后：

```rust
fn record_skill_use(skill_name: &str, memory: &mut MemoryStore) -> anyhow::Result<Option<ActivationEvent>> {
    // 1. 从 memory 读取当前 usage_count
    // 2. 递增 usage_count（双写：memory + 文件 frontmatter）
    // 3. 判定是否需要晋升
    //    - usage_count >= activation_threshold (3) → status = "active"
    //    - 连续失败次数 >= escalation_threshold (3) → 标记为需升级
    // 4. 写回文件 frontmatter
    // 5. 写回 memory store
    // 6. 返回晋升事件（如果有）
}
```

晋升条件矩阵：

| 条件 | 动作 |
|------|------|
| 首次使用成功 | 激活（active），记录 success_count=1 |
| 累计使用成功 ≥ 3 次 | 激活（如果尚未激活） |
| 连续失败 ≥ 3 次 | 标记为 `status: needs_upgrade`，下次冷却期满后通知用户 |
| 用户 / 系统显式确认 | 立即激活 |

#### 3.2.5 冷却期 `cooldown()`

```rust
fn can_introspect(round_number: u32, last_round: u32, round_gap: u32) -> bool {
    // round_gap = 默认 5
    round_number - last_round >= round_gap
}
```

冷却期满后重置 `session_introspect_count`。达到 `max_per_session`（默认 3）后本对话不再自省。

---

## 4. 现有模块修改点

### 4.1 `src/skill/mod.rs` — SkillMeta + frontmatter 解析

```rust
#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    // 新增字段
    pub status: SkillStatus,           // draft | active | needs_upgrade
    pub source: SkillSource,           // self-generated | user-created | built-in
    pub trigger: String,               // 关键词列表
    pub usage_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillStatus { Draft, Active, NeedsUpgrade }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource { SelfGenerated, UserCreated, BuiltIn }

impl SkillRegistry {
    // 新增
    pub fn update_usage_count(&mut self, name: &str) -> anyhow::Result<()>;
    pub fn promote(&mut self, name: &str) -> anyhow::Result<()>;
}
```

`scan()` 需要从 markdown 文件头部解析 YAML frontmatter。推荐使用 `serde_yaml`，但为了最小依赖可以手动解析（frontmatter 格式很规整：`---\nkey: value\n---`）。

### 4.2 `src/agent.rs` — AgentLoop 集成

```rust
impl AgentLoop {
    // 新增字段
    self_evolve: SelfEvolve,
    round_counter: u32,

    // handle_message 完成后增加自省检查：
    pub async fn handle_message(
        &mut self, text: &str, tools: &ToolRegistry,
        skills: &mut SkillRegistry, permission_check: &impl Fn(&str, &str) -> bool,
    ) -> anyhow::Result<String> {
        // ... 原有 react_loop 逻辑 ...
        // 在 react_loop 结束后：
        let result = self.self_evolve.evaluate(
            &self.history,
            &tool_results,
            skills,
            &mut memory,
            self.round_counter,
        )?;
        match result {
            IntrospectResult::SkillGenerated { skill_name, .. } => {
                if self.is_autonomous_mode {
                    let _ = skills.scan(&self.project_root);
                }
                response += &format!("\n\n[auto] 检测到能力缺口，已生成草稿 skill: {skill_name}");
            }
            IntrospectResult::SkillPromoted { skill_name } => {
                response += &format!("\n\n[auto] Skill '{skill_name}' 已验证有效，已激活");
            }
            _ => {}
        }
        Ok(response)
    }
}
```

### 4.3 `src/main.rs` — 初始化

```rust
let self_evolve = SelfEvolve::new(IntrospectConfig::default());
// 传入 BackgroundAgent / AgentLoop
```

---

## 5. 双写同步协议

| 操作 | 文件 frontmatter | MemoryStore |
|------|-----------------|-------------|
| 生成 skill | 写入 `usage_count=0, status=draft` | 写入 `skill:<name>:usage_count=0`, `skill:<name>:status=draft` |
| 使用 skill | 更新 `usage_count` + 可能更新 `status` | 同步更新两个 key |
| 晋升 skill | 更新 `status=active` | 更新 `skill:<name>:status=active` |
| 启动加载 | 读取 frontmatter → 写入 memory | 写入（同步） |

**启动时以文件为准**：如果文件 frontmatter 和 memory 不一致，以文件为准覆盖 memory。

---

## 6. 错误处理与边界情况

| 场景 | 处理方式 |
|------|---------|
| LLM gap analysis 返回空或无效 | 放弃本次自省，不生成 skill，冷却期照常启动 |
| 生成 skill 时文件已存在 | 追加版本号后缀（`git-helper-2.md`） |
| usage_count 写文件时 IO 错误 | 仅写入 memory，下次启动时以 memory 为准恢复 |
| 自省期间用户发新消息 | 放弃本次自省（不阻塞用户交互） |
| 生成的 skill 内容包含恶意代码 | 当前由 LLM 安全策略保障（不引入额外沙箱 → 后续 Phase 引入） |
| 磁盘满 / 权限错误 | 记录错误日志，降级为告知用户"无法生成" |

---

## 7. Phase 7 MVP 交付标准

- [ ] `docs/phase-7-self-evolution.md`（本文件）评审通过
- [ ] `src/self_evolve.rs` 模块实现（DetectFailure + GapAnalysis + SkillGenerator）
- [ ] `SkillMeta` 扩展 + `SkillRegistry::scan()` frontmatter 解析
- [ ] usage_count 追踪（双写同步：文件 + memory）
- [ ] AgentLoop 集成（handle_message 后调用自省）
- [ ] 冷却期机制
- [ ] 自动晋升逻辑（draft → active）
- [ ] 单元测试覆盖核心路径
- [ ] 集成测试：模拟"失败→生成→热加载→重试成功"端到端流程
- [ ] 110+ 现有测试仍全部通过

---

## 8. 未纳入 MVP 的后续方向

| 功能 | 说明 | 预计 Phase |
|------|------|-----------|
| 自动生成 Rust 工具 | 当 markdown skill 不够用时 | Phase 8 |
| 文件系统 watcher | 自动检测 skills/ 变化并 reload | Phase 8 |
| Skill 版本回滚 | 如果升级后效果下降 | Phase 8+ |
| 自我进化的进化 | 系统能改进 self_evolve 模块本身 | Phase 9 |
| 跨会话学习 | 一次学会的技能在其他项目也有效 | Phase 9+ |

---

## 9. 附录：Grill-Me 完整决策树

```
Phase 7: 自我进化型 Agent
│
├── 🎯 目标 → C. 自我进化型自主
│
├── 🔍 自省触发 → A. 失败驱动的自省
│   └── 🔬 失败判定 → D. 分层综合检测
│        1. LLM 自我声明
│        2. 工具调用错误累积
│        3. 用户反馈信号
│
├── 🛠️ 修复流程
│   ├── 🔎 Gap Analysis → C. 理想步骤 vs 现有能力 diff
│   ├── 📝 生成路径 → C. 两条路径平行
│   ├── ✅ 验证方式 → D. 渐进式激活
│   │   └── 📊 晋升条件 → 多路径（四种全部）
│   └── 🔄 循环防护 → C. 冷却期
│
├── 📂 Skill 格式 → A. 完整 YAML frontmatter
│   └── 💾 状态存储 → D. 双写同步
│
├── 🔥 热加载 → D. 分情况处理
├── 🧱 模块结构 → A. `src/self_evolve.rs`
└── 🚀 MVP 范围 → 设计文档 + 全部 6 个实现项
```
