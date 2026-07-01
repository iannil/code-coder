# 项目清理报告

> **清理时间**: 2026-07-01
> **范围**: 文档、配置、日志、过期数据

## ✅ 已修复的内容

### AGENTS.md
- **问题**: 状态写为"设计完成，待编码（尚无 Cargo.toml）"，与实际严重不符
- **修复**: 完全重写为"Phase 1-6 编码完成并测试通过"，添加模块架构表、快速统计、已知问题
- **参考**: `AGENTS.md`

### README.md
- **问题1**: 工具列表写"11 个内置工具"，实际是 20 个（缺少 glob、grep、diff、edit_file、commit、review、plan、ask_user、agent）
- **问题2**: 测试数写"56 个测试"，实际是 696
- **问题3**: 文件系统图示包含不存在的目录（`prompts/`、`tools/`、`knowledge/`）
- **问题4**: 最后 wikilink `[[codecoder-design]]` 指向不存在的文件
- **修复**: 全部修正为实际数据

### docs/ 文件夹重组
- **之前**: 所有文档散落在 docs/ 根目录，审计文件和设计文档平铺
- **之后**:
  - `docs/adr/` — 7 个 ADR（永久记录，不动）
  - `docs/audit/` — 审计报告（已完成）
  - `docs/design/` — 设计规格（Phase 7）
  - `docs/archive/` — 已完成实施计划（归档）
  - `docs/README.md` — 文档索引
- **参考**: `docs/README.md`

### codecoder.log
- **问题**: 1,858 行的运行时日志，不应提交
- **修复**: 已删除（已在 .gitignore 中）

## 📋 已识别但未修改的问题

### P1 - 建议下一轮修复

| 问题 | 位置 | 说明 |
|------|------|------|
| 37 个编译警告 | 源码各处 | 主要是 dead_code / unused field，可通过清理或 `#[allow()]` 解决 |
| `.superpowers/sdd/` 临时文件 | `.superpowers/sdd/` | 61 个文件（548K），28 个 review diff、19 个 task report、11 个 task brief、3 个 progress log。SDD 工作流的副产品 |

### P2 - 建议后续迭代优化

| 问题 | 说明 |
|------|------|
| 238 个 session 文件 | 其中 95 个 < 500 bytes（含不完整的测试对话），占用 1.0 MB |
| 仅 1 个示例 skill | `skills/greeter.md` 是唯一的 skill 文件 |
| Phase 7 未完全集成 | `self_evolve.rs` 代码存在但默认不启用 |
| `memory/` 仅 2 个文件 | 技能状态和计数，无实质性记忆 |

### 死代码清单

| 符号 | 位置 | 说明 |
|------|------|------|
| `EventBus` trait | `src/event.rs:5` | 定义了 trait 但无实现 |
| `ToolCall`/`ToolResult` 事件变体 | `src/event.rs` | 定义了但在事件循环中未构造 |
| `SkillPromoted` 事件变体 | `src/event.rs` | 同上 |
| `DeleteMessage` 事件变体 | `src/event.rs` | 同上 |
| `ListToolsParams` 结构体 | `src/mcp/` | 从未构造 |
| `render_markdown` 函数 | `src/tui/` | 可能被 markdown.rs 替代 |
| `format_context_bar` 函数 | `src/tui/` | status_bar 中有重复实现 |
| `SkillRegistry::get_mut`, `::list_by_status`, `::promote`, `::record_usage` | `src/skill/mod.rs` | 定义但未调用 |
| `build_frontmatter`, `update_frontmatter_in_file` | `src/skill/mod.rs` | 未调用 |

---

**总结**: 清理 4 项过期文档 + 1 个日志文件，识别 10+ 处死代码和 5 项待优化问题。
