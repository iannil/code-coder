# CodeCoder 文档索引

> 本文档是 CodeCoder 项目的文档入口。每类文档按目录组织，新贡献者应从 `docs/project-status.md` 开始。

## 文档目录

| 目录 | 内容 | 状态 |
|------|------|------|
| `docs/adr/` | 架构决策记录（0001-0007） | ✅ 永久记录 |
| `docs/audit/` | TUI 保真度审计报告 | ✅ 已完成 |
| `docs/design/` | 系统设计规格（Phase 7 等） | 🔧 部分实现 |
| `docs/archive/` | 已完成实施计划的归档 | ✅ 归档 |
| `.superpowers/sdd/` | SDD 工作流记录（外部） | 🗄️ 待清理 |

## 文档清单

### ADR（架构决策记录）

| 编号 | 标题 | 文件 |
|------|------|------|
| 0001 | TUI 键位绑定与模式语义 | `adr/0001-tui-keybinding-and-mode-semantics.md` |
| 0002 | 斜杠命令本地分发 | `adr/0002-slash-command-local-dispatch.md` |
| 0003 | 中心化主题结构 | `adr/0003-central-theme-struct.md` |
| 0004 | 会话持久化与迁移 | `adr/0004-session-persistence-and-migration.md` |
| 0005 | 权限作用域与会话允许列表 | `adr/0005-permission-scope-and-session-allowlist.md` |
| 0006 | 确认对话框模式 | `adr/0006-confirm-dialog-pattern.md` |
| 0007 | 提示注入式斜杠命令 | `adr/0007-prompt-injecting-slash-commands.md` |

### 审计报告

| 文件 | 说明 |
|------|------|
| `audit/audit-tui-fidelity.md` | TUI 行为保真度审计（对照 Claude Code） |
| `audit/audit-tui-visual-fidelity.md` | TUI 视觉保真度审计（A/B/C/D/E 五组） |

### 设计文档

| 文件 | 说明 | 实现状态 |
|------|------|---------|
| `design/phase-7-self-evolution.md` | Phase 7 自我进化型 Agent 设计（14 项决策树） | 🔧 `src/self_evolve.rs` 已实现，待完全集成 |

### 归档

| 文件 | 原用途 | 完成时间 |
|------|--------|---------|
| `archive/superpowers/2026-07-01-diff-rendering-pipeline.md` | Diff 渲染管线实现计划 | ✅ 已实现于 `src/tui/diff.rs` |
| `archive/superpowers/2026-07-01-input-multiline.md` | 多行输入实现计划 | ✅ 已实现于 `src/tui/input_area.rs` |
| `archive/superpowers/2026-07-01-tui-visual-fidelity-audit.md` | 视觉审计实施计划 | ✅ 已产出 `audit/audit-tui-visual-fidelity.md` |
| `archive/superpowers/*-design.md` | 对应设计规格 | 📎 参考归档 |

## 相关文档

- `AGENTS.md` — 项目声明、模块架构、Phase Status（顶层）
- `CONTEXT.md` — 项目术语表
- `README.md` — 用户快速入门
- `docs/project-status.md` — 完整项目状态（推荐起始点）

## 维护规范

1. **ADR** — 不可删除，新决策按序号延续（0008、0009...）
2. **审计报告** — 完成后不再修改；新审计另建新文件
3. **设计文档** — 随实现进度更新状态标记，不删除
4. **实现计划** — 完成后移入 `docs/archive/`，保留历史参考价值
5. **新文档** — 遵循最少一个类别归属原则，在本文档索引中登记
