# Phase 4: 工单自动分发 (GitHub Issues Integration)

**完成时间**: 2026-02-22
**状态**: ✅ 完成

## 概述

实现了自动化工单系统，让用户反馈通过 ZeroBot 自动分流：
- 常见问题 → LLM 直接回复（知识库搜索）
- 技术 Bug → 创建 GitHub Issue → 推送到开发群
- 功能请求 → 创建 GitHub Issue（较低优先级）

## 实现内容

### 1. 扩展 GitHub 客户端 (Issue 操作)

**文件**: `services/zero-workflow/src/github/client.rs`

新增方法:
- `create_issue()` - 创建 Issue
- `get_issue()` - 获取 Issue
- `add_labels()` - 添加标签

新增类型:
- `CreateIssueRequest` - 创建 Issue 请求
- `IssueResponse` - Issue 响应
- `Label` - 标签信息
- `IssueUser` - Issue 用户信息

### 2. 创建工单桥接器

**文件**: `services/zero-workflow/src/ticket_bridge.rs` (新建)

核心结构:
- `TicketBridge` - 工单桥接核心
- `Feedback` - 用户反馈结构
- `FeedbackCategory` - 反馈分类枚举 (Faq, Bug, TechnicalIssue, FeatureRequest, General)
- `FeedbackClassification` - LLM 分类结果
- `TicketResult` - 处理结果

功能:
- `process_feedback()` - 处理用户反馈的主入口
- `classify_feedback()` - 使用 LLM 分类反馈
- `search_knowledge()` - 搜索知识库回答 FAQ
- `create_github_issue()` - 创建 GitHub Issue
- `notify_team()` - 发送 IM 通知

### 3. 添加配置结构

**文件**: `services/zero-common/src/config.rs`

新增配置:
- `TicketConfig` - 工单配置
- `GitHubTicketConfig` - GitHub 工单配置
- `TicketNotificationConfig` - IM 通知配置

配置层级: `workflow.ticket.github` / `workflow.ticket.notification`

### 4. 添加反馈检测

**文件**: `services/zero-channels/src/bridge.rs`

新增检测函数:
- `is_bug_report()` - 检测 Bug 报告
- `is_feature_request()` - 检测功能请求

新增类型:
- `BugReportInfo` - Bug 报告信息
- `FeatureRequestInfo` - 功能请求信息

支持中英文模式检测：
- 崩溃/crash/白屏/闪退
- 报错/error/异常
- 无法登录/can't login
- 希望添加/feature request

### 5. 更新模块导出

**文件**: `services/zero-workflow/src/lib.rs`

- 添加 `ticket_bridge` 模块
- 导出新类型: `Feedback`, `FeedbackCategory`, `TicketBridge`, `TicketIMConfig`, `TicketResult`
- 在 `WorkflowService.build_router()` 中初始化 TicketBridge

**文件**: `services/zero-workflow/src/github/mod.rs`

- 导出新类型: `CreateIssueRequest`, `IssueResponse`, `IssueUser`, `Label`

**文件**: `services/zero-workflow/src/webhook.rs`

- 添加 `ticket_bridge` 字段到 `WebhookState`
- 添加 `with_ticket_bridge()` 方法

## 配置示例

```json
{
  "workflow": {
    "ticket": {
      "enabled": true,
      "github": {
        "default_repo": "company/product",
        "bug_labels": ["bug", "triage"],
        "feature_labels": ["enhancement"]
      },
      "notification": {
        "enabled": true,
        "channel_type": "feishu",
        "channel_id": "dev-group-id"
      }
    },
    "git": {
      "enabled": true,
      "github_token": "ghp_xxx"
    }
  }
}
```

## 工作流程

```
用户反馈 (IM)
    │
    ▼
ZeroBot bridge.rs
    │ is_bug_report() / is_feature_request()
    ▼
TicketBridge.process_feedback()
    │
    ├── classify_feedback() ──────────┐
    │                                 │ LLM 分类
    ▼                                 ▼
┌─────────────┐              ┌─────────────────┐
│ FAQ/简单问题 │              │ Bug/技术问题    │
└──────┬──────┘              └────────┬────────┘
       │                              │
       ▼                              ▼
知识库搜索                    create_github_issue()
       │                              │
       ▼                              ▼
直接回复用户                  发送 IM 通知到开发群
```

## 测试结果

- ✅ zero-workflow: 55 passed
- ✅ zero-common: 89 passed
- ✅ zero-channels: 23 passed

## 关键文件变更

| 文件 | 操作 | 描述 |
|------|------|------|
| `services/zero-workflow/src/github/client.rs` | 修改 | 添加 Issue API |
| `services/zero-workflow/src/github/mod.rs` | 修改 | 导出新类型 |
| `services/zero-workflow/src/ticket_bridge.rs` | 新建 | 工单桥接核心 |
| `services/zero-workflow/src/lib.rs` | 修改 | 添加模块导出和初始化 |
| `services/zero-workflow/src/webhook.rs` | 修改 | 添加 ticket_bridge 支持 |
| `services/zero-common/src/config.rs` | 修改 | 添加 TicketConfig |
| `services/zero-channels/src/bridge.rs` | 修改 | 添加反馈检测 |

## 后续扩展建议

1. **Webhook 回调**: Issue 状态变更时通知原反馈用户
2. **用户关联**: 自动关联用户 IM 账号到 Issue
3. **优先级自动判定**: 基于关键词和用户 VIP 等级判定 P0/P1/P2
4. **重复检测**: 检测相似 Issue 避免重复创建
5. **工单追踪**: 允许用户查询自己提交的工单状态
