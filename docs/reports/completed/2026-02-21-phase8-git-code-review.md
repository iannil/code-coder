# Phase 8: Git Code Review 自动化

**日期**: 2026-02-21
**状态**: ✅ 已完成

## 实现概要

Phase 8 实现了 Git Code Review 自动化功能，使 zero-workflow 能够在收到 GitHub PR 或 GitLab MR webhook 时自动调用 CodeCoder 的 code-reviewer Agent 进行代码审查，并将结果评论回 Git 平台。

## 完成的任务

### 1. GitHub 集成 (services/zero-workflow/src/github/)

**mod.rs** - GitHub webhook 事件类型定义：
- ✅ `PullRequestEvent` 完整的 PR webhook 事件解析
- ✅ `should_review()` 智能判断是否需要审查（排除 draft/closed）
- ✅ 支持 `opened`, `synchronize`, `reopened` 动作

**client.rs** - GitHub API 客户端：
- ✅ JWT Bearer 认证
- ✅ 获取 PR 详情和文件列表
- ✅ 获取 PR diff
- ✅ 创建 PR Review
- ✅ 创建 Issue Comment
- ✅ 支持 GitHub Enterprise（自定义 base_url）

### 2. GitLab 集成 (services/zero-workflow/src/gitlab/)

**mod.rs** - GitLab webhook 事件类型定义：
- ✅ `MergeRequestEvent` 完整的 MR webhook 事件解析
- ✅ `should_review()` 智能判断（排除 draft/WIP）
- ✅ 支持 `open`, `reopen`, `update` 动作

**client.rs** - GitLab API 客户端：
- ✅ PRIVATE-TOKEN 认证
- ✅ 获取 MR 详情和 changes
- ✅ 获取 MR diffs
- ✅ 创建 Note（评论）
- ✅ 创建 Discussion（线程评论）
- ✅ 支持自托管 GitLab（自定义 base_url）

### 3. Review Bridge (services/zero-workflow/src/review_bridge.rs)

- ✅ 连接 Git 平台 webhook 到 CodeCoder code-reviewer Agent
- ✅ 构建审查提示（diff + 文件列表 + 描述）
- ✅ 解析 CodeCoder 响应，提取审查结论
- ✅ 格式化 Markdown 审查报告
- ✅ 支持 APPROVE / REQUEST_CHANGES / COMMENT 三种结论
- ✅ diff 截断处理（防止超长 prompt）

### 4. Webhook 集成 (services/zero-workflow/src/webhook.rs)

- ✅ 更新 `WebhookState` 支持 `review_bridge`
- ✅ GitHub `pull_request` 事件自动触发审查
- ✅ GitLab `Merge Request Hook` 事件自动触发审查
- ✅ 后台异步处理审查（不阻塞 webhook 响应）

## 数据流

```
GitHub PR 创建
    ↓
Webhook 接收 (POST /webhook/github)
    ↓
解析 X-GitHub-Event: pull_request
    ↓
PullRequestEvent.should_review()? ── No → 返回 OK
    ↓ Yes
获取 PR diff (GitHub API)
    ↓
构建审查 prompt
    ↓
调用 CodeCoder /api/v1/chat (agent: code-reviewer)
    ↓
解析响应，提取结论
    ↓
格式化 Markdown 报告
    ↓
POST 评论到 GitHub PR
```

## 新增文件

| 文件 | 描述 |
|------|------|
| `services/zero-workflow/src/github/mod.rs` | GitHub webhook 事件类型 |
| `services/zero-workflow/src/github/client.rs` | GitHub API 客户端 |
| `services/zero-workflow/src/gitlab/mod.rs` | GitLab webhook 事件类型 |
| `services/zero-workflow/src/gitlab/client.rs` | GitLab API 客户端 |
| `services/zero-workflow/src/review_bridge.rs` | 审查桥接逻辑 |

## 修改的文件

| 文件 | 修改 |
|------|------|
| `services/zero-workflow/src/lib.rs` | 添加新模块导出 |
| `services/zero-workflow/src/webhook.rs` | 集成审查触发 |

## 测试覆盖

- `test_github_pr_event_parsing` - GitHub PR 事件解析
- `test_github_pr_draft_should_not_review` - Draft PR 不触发审查
- `test_github_pr_closed_should_not_review` - 关闭的 PR 不触发审查
- `test_gitlab_mr_event_parsing` - GitLab MR 事件解析
- `test_gitlab_mr_draft_should_not_review` - Draft MR 不触发审查
- `test_review_bridge_creation` - ReviewBridge 创建

## 配置示例

```json
{
  "workflow": {
    "git": {
      "enabled": true,
      "github_secret": "your-github-webhook-secret",
      "gitlab_token": "your-gitlab-webhook-token"
    }
  }
}
```

## 审查报告格式

```markdown
## ✅ Code Review: Approved

This PR looks good overall. The code is clean and follows best practices.

### Findings

- **🟡 MEDIUM** (`src/main.rs:42`): Consider using const here

---
*Automated review by CodeCoder*
```

## 使用方式

1. 配置 GitHub/GitLab webhook 指向 `/webhook/github` 或 `/webhook/gitlab`
2. 设置 webhook secret 在配置文件中
3. 创建 PR/MR 时自动触发代码审查
4. 审查结果自动评论到 PR/MR

## 后续优化 (P2)

1. 支持行级别评论（而非只是 PR 整体评论）
2. 支持配置跳过某些文件类型
3. 支持自定义审查 Agent
4. 支持重新触发审查命令

---

*记录时间: 2026-02-21*
*总测试数: 29 (零失败)*
