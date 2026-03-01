# 流式确认按钮设计文档

## 概述

**文件位置**: `zero-channels/src/progress.rs:1235-1242`

**当前状态**: `TaskEvent::Confirmation` 事件仅记录警告日志，不做实际处理。

**目标**: 在流式消息模式下，通过 IM 平台的内联按钮实现确认交互。

## 背景

当 Agent 执行需要用户确认的操作时（如文件删除、API 调用、付款），会发送 `TaskEvent::Confirmation` 事件。目前：

- **非流式模式**: 通过 HITL (Human-In-The-Loop) 系统处理
- **流式模式**: 无法处理，仅记录警告

## 设计方案

### 架构图

```text
┌─────────────────────────────────────────────────────────────────┐
│                        TaskEvent::Confirmation                   │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                     StreamingProgressHandler                     │
│                                                                  │
│  1. 解析确认请求内容                                              │
│  2. 生成内联按钮配置                                              │
│  3. 调用渠道适配器发送按钮消息                                     │
│  4. 等待回调或超时                                                │
│  5. 返回确认结果给 Agent                                         │
└─────────────────────────────────────────────────────────────────┘
                               │
           ┌───────────────────┼───────────────────┐
           ▼                   ▼                   ▼
    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
    │  Telegram   │     │   Feishu    │     │   Slack     │
    │  Adapter    │     │   Adapter   │     │   Adapter   │
    │             │     │             │     │             │
    │ InlineKeyb  │     │ CardAction  │     │ Block Kit   │
    └─────────────┘     └─────────────┘     └─────────────┘
           │                   │                   │
           └───────────────────┼───────────────────┘
                               ▼
                    ┌─────────────────────┐
                    │   Callback Router   │
                    │  (Webhook Handler)  │
                    └─────────────────────┘
```

### 数据结构

```rust
/// 确认请求数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationRequest {
    /// 唯一确认 ID
    pub confirmation_id: String,
    /// 操作描述
    pub action: String,
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 详细说明
    pub details: Option<String>,
    /// 超时秒数
    pub timeout_secs: u64,
    /// 可选按钮配置
    pub buttons: Vec<ConfirmationButton>,
}

/// 确认按钮
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationButton {
    /// 按钮 ID
    pub id: String,
    /// 显示文本
    pub label: String,
    /// 按钮样式 (primary/danger/default)
    pub style: ButtonStyle,
    /// 确认结果值
    pub value: ConfirmationResult,
}

/// 确认结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfirmationResult {
    Approve,
    Deny,
    Skip,
    Custom(String),
}
```

### 实现步骤

#### 阶段 1: 核心框架 (1-2 天)

1. **定义 Trait**

```rust
// zero-channels/src/confirmation.rs
#[async_trait]
pub trait ConfirmationHandler: Send + Sync {
    /// 发送确认请求到渠道
    async fn send_confirmation(
        &self,
        channel_id: &str,
        request: &ConfirmationRequest,
    ) -> Result<()>;

    /// 处理确认回调
    async fn handle_callback(
        &self,
        callback_id: &str,
        data: &str,
    ) -> Result<ConfirmationResult>;
}
```

2. **回调路由器**

```rust
// zero-channels/src/callback_router.rs
pub struct CallbackRouter {
    /// 待处理的确认请求 (confirmation_id -> oneshot::Sender)
    pending: Arc<RwLock<HashMap<String, oneshot::Sender<ConfirmationResult>>>>,
    /// 超时配置
    default_timeout: Duration,
}

impl CallbackRouter {
    /// 注册确认请求，返回结果接收器
    pub fn register(&self, confirmation_id: String, timeout: Duration)
        -> oneshot::Receiver<ConfirmationResult>;

    /// 处理回调，解析并发送结果
    pub async fn handle(&self, callback_data: &str) -> Result<()>;
}
```

#### 阶段 2: 渠道适配器 (2-3 天)

3. **Telegram 适配器**

```rust
impl ConfirmationHandler for TelegramAdapter {
    async fn send_confirmation(&self, channel_id: &str, request: &ConfirmationRequest) -> Result<()> {
        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: vec![
                request.buttons.iter().map(|b| InlineKeyboardButton {
                    text: b.label.clone(),
                    callback_data: Some(format!("confirm:{}:{}", request.confirmation_id, b.id)),
                    ..Default::default()
                }).collect()
            ],
        };

        self.bot.send_message(channel_id, &request.action)
            .reply_markup(keyboard)
            .await?;

        Ok(())
    }
}
```

4. **飞书/Slack 适配器**

- 飞书: 使用卡片消息 + 交互式组件
- Slack: 使用 Block Kit + 交互式按钮

#### 阶段 3: 集成到流式处理 (1 天)

5. **修改 progress.rs**

```rust
TaskEvent::Confirmation(data) => {
    // 解析确认请求
    let request = ConfirmationRequest::from_event_data(&data)?;

    // 生成内联按钮
    let buttons = self.generate_buttons(&request);

    // 发送到渠道
    self.channel_adapter.send_confirmation(&msg.channel_id, &request).await?;

    // 注册回调等待
    let result = self.callback_router
        .register(request.confirmation_id.clone(), Duration::from_secs(request.timeout_secs))
        .await?;

    // 发送结果回 Agent
    self.send_confirmation_result(&request.confirmation_id, result).await?;

    Ok(false)
}
```

### 安全考虑

1. **回调验证**: 验证 callback_data 签名防止伪造
2. **超时处理**: 默认 5 分钟超时，自动拒绝
3. **权限检查**: 仅原始请求用户可确认
4. **审计日志**: 记录所有确认操作

### 测试计划

| 测试用例 | 描述 |
|----------|------|
| 单元测试 | ConfirmationRequest 序列化 |
| 集成测试 | 回调路由器超时处理 |
| E2E 测试 | Telegram 内联按钮完整流程 |

### 里程碑

| 阶段 | 交付物 | 预计时间 |
|------|--------|----------|
| M1 | 核心框架 + Telegram | 3 天 |
| M2 | 飞书 + Slack | 2 天 |
| M3 | 测试 + 文档 | 1 天 |

## 参考资料

- [Telegram Inline Keyboards](https://core.telegram.org/bots/api#inlinekeyboardmarkup)
- [飞书消息卡片](https://open.feishu.cn/document/ukTMukTMukTM/uEjNwUjLxYDM14SM2ATN)
- [Slack Block Kit](https://api.slack.com/block-kit)
