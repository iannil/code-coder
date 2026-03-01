# 交互式 2FA 设计文档

## 概述

**文件位置**: `zero-cli/src/tools/auto_login.rs:86-96`

**当前状态**: `request_2fa_code()` 函数直接返回错误，仅支持 TOTP 自动生成。

**目标**: 通过 Agent 确认系统实现交互式 2FA 代码收集。

## 背景

自动登录工具 (`AutoLoginTool`) 用于自动化网站登录流程。当网站需要 2FA 验证时：

- **TOTP 模式**: 如果凭证库中存储了 TOTP 密钥，自动生成验证码 ✅
- **交互式模式**: 需要用户手动输入短信/邮件验证码 ❌ (未实现)

## 设计方案

### 架构图

```text
┌─────────────────────────────────────────────────────────────────┐
│                      AutoLoginTool                               │
│                                                                  │
│  login() ──► 检测 2FA ──► need_interactive_2fa?                 │
│                                 │                                │
│                    ┌────────────┴────────────┐                  │
│                    ▼                         ▼                  │
│              有 TOTP 密钥              无 TOTP 密钥              │
│                    │                         │                  │
│                    ▼                         ▼                  │
│           generate_totp()          request_2fa_code()           │
│                                              │                  │
└──────────────────────────────────────────────│──────────────────┘
                                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Zero Agent Confirmation                       │
│                                                                  │
│  1. 发送确认请求到 CodeCoder API                                 │
│  2. API 转发到用户的 IM 渠道 (Telegram/Feishu/Slack)            │
│  3. 用户在 IM 中输入验证码                                       │
│  4. 验证码通过回调返回                                           │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    IM Channel                                    │
│                                                                  │
│  "🔐 Login to GitHub requires 2FA"                              │
│  "Please enter the verification code sent to your email:"       │
│                                                                  │
│  [User types: 123456]                                           │
│                                                                  │
│  ✅ "Code received, continuing login..."                        │
└─────────────────────────────────────────────────────────────────┘
```

### 数据结构

```rust
/// 2FA 请求类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TwoFactorMethod {
    /// TOTP (自动生成)
    Totp { secret: String },
    /// 短信验证码
    Sms { phone_hint: String },
    /// 邮件验证码
    Email { email_hint: String },
    /// 推送通知确认
    Push { app_name: String },
    /// 硬件密钥
    WebAuthn,
    /// 备用码
    BackupCode,
}

/// 2FA 交互请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorRequest {
    /// 请求 ID
    pub request_id: String,
    /// 服务名称 (如 "GitHub", "AWS")
    pub service: String,
    /// 2FA 方法
    pub method: TwoFactorMethod,
    /// 超时秒数 (默认 300)
    pub timeout_secs: u64,
    /// 提示信息
    pub prompt: String,
}

/// 2FA 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorResponse {
    /// 请求 ID
    pub request_id: String,
    /// 验证码 (6-8 位)
    pub code: String,
    /// 响应时间戳
    pub timestamp: i64,
}
```

### 实现步骤

#### 阶段 1: Agent 确认系统集成 (2 天)

1. **定义 Agent 确认客户端**

```rust
// zero-cli/src/tools/confirmation_client.rs

use reqwest::Client;

pub struct ConfirmationClient {
    api_endpoint: String,
    http: Client,
}

impl ConfirmationClient {
    /// 请求用户输入 2FA 验证码
    pub async fn request_2fa_code(
        &self,
        request: TwoFactorRequest,
    ) -> Result<TwoFactorResponse> {
        // 1. 发送确认请求到 CodeCoder API
        let response = self.http
            .post(&format!("{}/api/v1/confirmation/2fa", self.api_endpoint))
            .json(&request)
            .send()
            .await?;

        // 2. 等待响应 (长轮询或 WebSocket)
        let result: TwoFactorResponse = response.json().await?;

        Ok(result)
    }
}
```

2. **修改 AutoLoginTool**

```rust
impl AutoLoginTool {
    async fn request_2fa_code(
        &self,
        service: &str,
        method: TwoFactorMethod,
        timeout_secs: u64,
    ) -> anyhow::Result<String> {
        let request = TwoFactorRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            service: service.to_string(),
            method,
            timeout_secs,
            prompt: format!("请输入 {} 的验证码", service),
        };

        // 使用确认客户端
        let client = ConfirmationClient::new(&self.api_endpoint);
        let response = client.request_2fa_code(request).await?;

        // 验证码格式检查
        if !response.code.chars().all(|c| c.is_ascii_digit()) {
            anyhow::bail!("Invalid 2FA code format");
        }

        Ok(response.code)
    }
}
```

#### 阶段 2: CodeCoder API 端点 (1 天)

3. **添加确认 API 路由**

```typescript
// packages/ccode/src/api/server/handlers/confirmation.ts

export async function request2FA(req: HttpRequest): Promise<HttpResponse> {
  const body = await req.json<TwoFactorRequest>();

  // 1. 查找用户的活跃 IM 渠道
  const channel = await findUserChannel(body.user_id);

  // 2. 发送 2FA 请求消息
  await channel.sendMessage({
    text: `🔐 ${body.service} 需要验证码\n${body.prompt}`,
    expectReply: true,
    timeout: body.timeout_secs * 1000,
  });

  // 3. 等待用户回复
  const reply = await channel.waitForReply(body.timeout_secs * 1000);

  // 4. 提取验证码
  const code = extractCode(reply.text);

  return jsonResponse({
    request_id: body.request_id,
    code,
    timestamp: Date.now(),
  });
}

function extractCode(text: string): string {
  // 提取 6-8 位数字
  const match = text.match(/\b(\d{6,8})\b/);
  if (!match) {
    throw new Error("No valid code found in response");
  }
  return match[1];
}
```

#### 阶段 3: IM 渠道支持 (1 天)

4. **消息等待回复机制**

```rust
// zero-channels/src/reply_waiter.rs

pub struct ReplyWaiter {
    /// 待回复的消息 (message_id -> oneshot::Sender)
    pending: Arc<RwLock<HashMap<String, oneshot::Sender<ChannelMessage>>>>,
}

impl ReplyWaiter {
    /// 等待指定消息的回复
    pub async fn wait_for_reply(
        &self,
        message_id: &str,
        timeout: Duration,
    ) -> Result<ChannelMessage> {
        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(message_id.to_string(), tx);

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(_)) => Err(anyhow!("Reply channel closed")),
            Err(_) => {
                self.pending.write().await.remove(message_id);
                Err(anyhow!("Timeout waiting for 2FA code"))
            }
        }
    }

    /// 处理收到的消息，检查是否为等待中的回复
    pub async fn handle_message(&self, msg: &ChannelMessage) {
        if let Some(reply_to) = &msg.reply_to_message_id {
            if let Some(tx) = self.pending.write().await.remove(reply_to) {
                let _ = tx.send(msg.clone());
            }
        }
    }
}
```

### 用户体验流程

```
┌──────────────────────────────────────────────────────────────┐
│  Agent 执行自动登录                                           │
│  > auto_login("github.com", "myuser")                        │
└──────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  浏览器自动化                                                 │
│  1. 打开 github.com/login                                    │
│  2. 填入用户名密码                                            │
│  3. 检测到需要 2FA                                            │
└──────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  Telegram 消息                                                │
│                                                               │
│  🔐 GitHub 登录需要验证码                                     │
│                                                               │
│  请输入发送到 ****@email.com 的 6 位验证码                    │
│  (5 分钟内有效)                                               │
│                                                               │
│  User: 847293                                                 │
│                                                               │
│  ✅ 验证码已收到，正在继续登录...                             │
└──────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  Agent 继续执行                                               │
│  > 登录成功！Session 已保存。                                 │
└──────────────────────────────────────────────────────────────┘
```

### 安全考虑

1. **验证码有效期**: 验证码仅在请求后 5 分钟内有效
2. **单次使用**: 每个请求 ID 只能使用一次
3. **用户绑定**: 仅接受绑定用户的回复
4. **传输加密**: 所有通信使用 HTTPS/TLS
5. **日志脱敏**: 不记录验证码原文，仅记录请求 ID

### 降级策略

如果 IM 渠道不可用：

1. **CLI 提示**: 在终端提示用户输入
2. **桌面通知**: 发送系统通知
3. **超时失败**: 明确告知用户配置 TOTP

### 测试计划

| 测试用例 | 描述 |
|----------|------|
| 单元测试 | TwoFactorRequest 序列化/反序列化 |
| 单元测试 | extractCode() 各种输入格式 |
| 集成测试 | ReplyWaiter 超时处理 |
| E2E 测试 | Telegram 完整 2FA 流程 |
| E2E 测试 | 超时后的错误处理 |

### 里程碑

| 阶段 | 交付物 | 预计时间 |
|------|--------|----------|
| M1 | 确认客户端 + API | 2 天 |
| M2 | IM 回复等待机制 | 1 天 |
| M3 | 集成到 AutoLoginTool | 1 天 |
| M4 | 测试 + 文档 | 1 天 |

## 与流式确认按钮的关系

此功能可以复用 **流式确认按钮** 的基础设施：

- `CallbackRouter` → 复用
- `ConfirmationHandler` trait → 扩展支持文本输入
- Webhook 处理 → 复用

建议先完成流式确认按钮，再实现此功能。

## 参考资料

- [TOTP RFC 6238](https://tools.ietf.org/html/rfc6238)
- [WebAuthn](https://webauthn.guide/)
- [Telegram Bot Reply Markup](https://core.telegram.org/bots/api#forcereply)
