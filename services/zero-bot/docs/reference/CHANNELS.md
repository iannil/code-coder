# Channel 列表

ZeroBot 支持 8 个消息通道。

## 通道概览

| Channel | 名称 | 说明 |
|---------|------|------|
| **CLI** | `cli` | 终端交互 |
| **Telegram** | `telegram` | Telegram Bot |
| **Discord** | `discord` | Discord Bot |
| **Slack** | `slack` | Slack App |
| **Matrix** | `matrix` | Matrix 协议 |
| **WhatsApp** | `whatsapp` | WhatsApp Business |
| **iMessage** | `imessage` | iMessage (macOS) |
| **Email** | `email` | IMAP/SMTP |

---

## CLI

终端交互通道。

```bash
zero-bot agent
```

**特点**:
- 无需配置
- 即时反馈
- 开发调试用

---

## Telegram

Telegram Bot 通道。

### 配置

```toml
[[channels]]
kind = "telegram"
token = "123456:ABC-..."
```

### 环境变量

```bash
export TELEGRAM_BOT_TOKEN="123456:ABC-..."
```

### 设置步骤

1. 联系 @BotFather 创建 bot
2. 获取 token
3. 配置 webhook 或轮询

---

## Discord

Discord Bot 通道。

### 配置

```toml
[[channels]]
kind = "discord"
token = "..."
application_id = "..."
```

### 环境变量

```bash
export DISCORD_TOKEN="..."
export DISCORD_APPLICATION_ID="..."
```

### 设置步骤

1. 在 Discord Developer Portal 创建应用
2. 创建 Bot 并获取 token
3. 邀请 Bot 到服务器

---

## Slack

Slack App 通道。

### 配置

```toml
[[channels]]
kind = "slack"
token = "xoxb-..."
signing_secret = "..."
```

### 环境变量

```bash
export SLACK_BOT_TOKEN="xoxb-..."
export SLACK_SIGNING_SECRET="..."
```

### 设置步骤

1. 在 Slack API 创建应用
2. 配置 Bot Token Scopes
3. 安装到工作区
4. 配置 Event Subscriptions

---

## Matrix

Matrix 协议通道。

### 配置

```toml
[[channels]]
kind = "matrix"
homeserver = "https://matrix.org"
user_id = "@bot:matrix.org"
access_token = "..."
```

### 设置步骤

1. 在 Matrix 服务器注册账号
2. 获取 access token
3. 配置 homeserver URL

---

## WhatsApp

WhatsApp Business API 通道。

### 配置

```toml
[[channels]]
kind = "whatsapp"
phone_number_id = "..."
access_token = "..."
```

### 设置步骤

1. 注册 WhatsApp Business API
2. 创建应用
3. 获取 Phone Number ID 和 Access Token

---

## iMessage

iMessage 通道 (仅 macOS)。

### 配置

```toml
[[channels]]
kind = "imessage"
```

### 要求

- macOS 系统
- 登录 iCloud 账号
- 允许自动化权限

### 注意

- 仅支持 macOS
- 需要系统权限
- 使用 AppleScript/osascript

---

## Email

Email 通道 (IMAP/SMTP)。

### 配置

```toml
[[channels]]
kind = "email"
imap_host = "imap.gmail.com"
imap_port = 993
smtp_host = "smtp.gmail.com"
smtp_port = 587
username = "bot@example.com"
password = "..."
```

### 设置步骤

1. 配置 IMAP 收取邮件
2. 配置 SMTP 发送邮件
3. 如使用 Gmail，需启用应用密码

---

## 通道管理命令

```bash
# 列出通道
zero-bot channel list

# 添加通道
zero-bot channel add telegram '{"token": "..."}'

# 移除通道
zero-bot channel remove my-channel

# 启动通道
zero-bot channel start

# 健康检查
zero-bot channel doctor
```

---

## Webhook 模式

使用 Gateway + Tunnel 接收 webhook:

```bash
# 启动 gateway
zero-bot gateway

# 或启动完整 daemon
zero-bot daemon
```

Webhook 端点:
- `/webhook/telegram`
- `/webhook/slack`
- `/webhook/discord`
- ...

---

## 健康检查

每个通道实现 `health_check()` 方法:

```bash
zero-bot channel doctor
```

返回状态:
- `Healthy` — 正常
- `Degraded(msg)` — 降级（部分功能可用）
- `Unhealthy(msg)` — 不可用
