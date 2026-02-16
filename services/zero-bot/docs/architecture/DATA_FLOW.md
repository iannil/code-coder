# 数据流

## 请求处理流程

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            消息处理流程                                       │
└─────────────────────────────────────────────────────────────────────────────┘

    ┌─────────┐
    │  用户   │
    └────┬────┘
         │ 消息
         ▼
┌─────────────────┐
│    Channel      │  CLI / Telegram / Discord / ...
│    (入口)       │
└────────┬────────┘
         │ IncomingMessage
         ▼
┌─────────────────┐
│    Agent        │
│    (编排)       │◄──────────────────────────────┐
└────────┬────────┘                               │
         │                                        │
         ▼                                        │
┌─────────────────┐     ┌─────────────────┐      │
│    Memory       │◄───►│    Provider     │      │
│   (召回上下文)   │     │   (LLM 调用)    │      │
└─────────────────┘     └────────┬────────┘      │
                                 │               │
                                 ▼               │
                        ┌─────────────────┐      │
                        │   Tool 决策?    │──否──┘
                        └────────┬────────┘
                                 │ 是
                                 ▼
                        ┌─────────────────┐
                        │   Security      │
                        │   (沙箱检查)    │
                        └────────┬────────┘
                                 │
                                 ▼
                        ┌─────────────────┐
                        │     Tool        │
                        │    (执行)       │
                        └────────┬────────┘
                                 │ ToolResult
                                 ▼
                        ┌─────────────────┐
                        │   Observer      │
                        │   (记录指标)    │
                        └────────┬────────┘
                                 │
                                 ▼
                         回到 Agent 继续
```

## 组件交互

### 1. Channel → Agent

```rust
// Channel 接收消息
let msg = channel.listen().await?;

// 转发给 Agent
let response = agent.process(msg.content).await?;

// 通过 Channel 回复
channel.send(&response).await?;
```

### 2. Agent → Provider

```rust
// Agent 构建消息
let messages = vec![
    Message { role: Role::User, content: user_input },
];

// 调用 Provider
let response = provider.chat_with_system(
    &system_prompt,
    &messages,
).await?;
```

### 3. Agent → Memory

```rust
// 召回相关记忆
let memories = memory.recall(&query, 5).await?;

// 将记忆注入上下文
let context = format!("相关记忆:\n{}", memories.join("\n"));

// 存储新记忆
memory.store(MemoryEntry {
    content: important_info,
    category: MemoryCategory::Fact,
    ..
}).await?;
```

### 4. Agent → Tool

```rust
// LLM 决定使用工具
if let Some(tool_call) = parse_tool_call(&response) {
    // 安全检查
    security.validate(&tool_call)?;

    // 执行工具
    let result = tools.get(&tool_call.name)?
        .execute(tool_call.params).await?;

    // 记录
    observer.record_event(&Event {
        name: "tool_executed".into(),
        ..
    });

    // 将结果返回给 LLM
    messages.push(Message {
        role: Role::Tool,
        content: result.output,
    });
}
```

## Gateway 模式

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Gateway 模式                                       │
└─────────────────────────────────────────────────────────────────────────────┘

    外部服务 (Telegram/Slack/...)
         │ Webhook POST
         ▼
┌─────────────────┐
│    Tunnel       │  Cloudflare / ngrok / Tailscale
│   (公网入口)    │
└────────┬────────┘
         │ HTTPS
         ▼
┌─────────────────┐
│    Gateway      │  Axum HTTP 服务器
│   (路由处理)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Agent        │
│    (处理)       │
└────────┬────────┘
         │
         ▼
     返回响应
```

## Daemon 模式

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Daemon 模式                                        │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐
│    Daemon       │
│   (主循环)      │
└────────┬────────┘
         │
    ┌────┴────┬────────────┬────────────┐
    │         │            │            │
    ▼         ▼            ▼            ▼
┌───────┐ ┌───────┐  ┌──────────┐ ┌──────────┐
│Channel│ │Channel│  │ Cron     │ │Heartbeat │
│ Poller│ │ Poller│  │ Scheduler│ │ Engine   │
└───────┘ └───────┘  └──────────┘ └──────────┘
    │         │            │            │
    └────┬────┴────────────┴────────────┘
         │
         ▼
┌─────────────────┐
│    Agent        │
│   (统一处理)    │
└─────────────────┘
```

## 错误处理流程

```
┌─────────────────┐
│   任意操作      │
└────────┬────────┘
         │
         ▼
    ┌────────────┐
    │  Result?   │
    └─────┬──────┘
          │
    ┌─────┴─────┐
    ▼           ▼
   Ok        Err
    │           │
    ▼           ▼
 继续       ┌─────────────────┐
            │   Observer      │
            │  (记录错误)     │
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │   返回友好      │
            │   错误消息      │
            └─────────────────┘
```

## 配置加载流程

```
启动
  │
  ▼
┌─────────────────┐
│  Config::load() │  ~/.codecoder/config.toml
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  SecretStore    │  解密 API keys
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  create_*()     │  创建各组件
│  工厂函数       │
└────────┬────────┘
         │
    ┌────┴────┬────────┬────────┐
    ▼         ▼        ▼        ▼
Provider  Channel   Memory   Tools
```
