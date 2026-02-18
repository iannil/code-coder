# ZeroBot 飞书渠道 + TTS 功能实现报告

## 完成时间
2026-02-18

## 实现概述

本次实现为 ZeroBot 添加了两个主要功能：
1. **飞书/Lark 渠道** - 支持国内企业用户通过飞书机器人与 ZeroBot 交互
2. **TTS (Text-to-Speech) 模块** - 支持语音合成响应功能

---

## 第一部分：飞书渠道实现

### 文件变更

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/channels/feishu.rs` | 新增 | 完整的飞书渠道实现 (~450 行) |
| `src/channels/mod.rs` | 修改 | 注册 FeishuChannel |
| `src/config/schema.rs` | 修改 | 添加 FeishuConfig 配置结构 |
| `src/config/mod.rs` | 修改 | 导出 FeishuConfig |
| `src/onboard/wizard.rs` | 修改 | 支持 Feishu 渠道配置 |

### 配置结构

```rust
pub struct FeishuConfig {
    pub app_id: String,           // 飞书 App ID
    pub app_secret: String,       // 飞书 App Secret
    pub encrypt_key: Option<String>,      // 事件加密密钥
    pub verification_token: Option<String>, // 验证 Token
    pub allowed_users: Vec<String>,        // 允许的用户 open_id
}
```

### 功能特性

- **双端支持**: 同时支持飞书 (feishu.cn) 和 Lark (larksuite.com) API
- **Token 缓存**: 自动缓存和刷新 tenant_access_token
- **消息发送**: 支持发送文本消息到用户或群聊
- **用户授权**: 基于 open_id 的用户白名单
- **事件回调处理**: 支持 Webhook 事件接收和 URL 验证

### 使用示例

配置 `~/.codecoder/config.json`:
```json
{
  "zerobot": {
    "channels": {
      "feishu": {
        "app_id": "cli_xxx",
        "app_secret": "secret",
        "allowed_users": ["ou_xxx", "*"]
      }
    }
  }
}
```

---

## 第二部分：TTS 模块实现

### 文件变更

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/tts/mod.rs` | 新增 | TTS 工厂函数和模块导出 |
| `src/tts/traits.rs` | 新增 | TextToSpeech trait 定义 |
| `src/tts/openai.rs` | 新增 | OpenAI TTS 实现 |
| `src/tts/elevenlabs.rs` | 新增 | ElevenLabs TTS 实现 |
| `src/config/schema.rs` | 修改 | 添加 TtsConfig 和 VoiceWakeConfig |
| `src/config/mod.rs` | 修改 | 导出 TtsConfig, VoiceWakeConfig |
| `src/main.rs` | 修改 | 注册 tts 模块 |
| `src/channels/telegram.rs` | 修改 | 添加 send_voice_bytes 方法 |

### TTS Trait

```rust
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> anyhow::Result<Vec<u8>>;

    async fn list_voices(&self) -> anyhow::Result<Vec<VoiceInfo>>;
    fn default_voice(&self) -> &str;
    fn provider_name(&self) -> &str;
}
```

### 支持的提供商

| 提供商 | 模型 | 语音 |
|--------|------|------|
| OpenAI | tts-1, tts-1-hd | alloy, echo, fable, onyx, nova, shimmer |
| ElevenLabs | eleven_multilingual_v2, eleven_turbo_v2 | 自定义语音库 |

### 配置结构

```rust
pub struct TtsConfig {
    pub enabled: bool,
    pub provider: String,      // "openai", "elevenlabs"
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub base_url: Option<String>,
}
```

### 使用示例

```json
{
  "zerobot": {
    "tts": {
      "enabled": true,
      "provider": "openai",
      "voice": "nova"
    }
  }
}
```

### Telegram 集成

新增 `send_voice_bytes` 方法，支持直接发送 TTS 合成的音频字节：

```rust
pub async fn send_voice_bytes(
    &self,
    chat_id: &str,
    file_bytes: Vec<u8>,
    file_name: &str,
    caption: Option<&str>,
) -> anyhow::Result<()>
```

---

## 第三部分：语音唤醒配置

添加了 VoiceWakeConfig 配置结构，为未来语音唤醒功能预留接口：

```rust
pub struct VoiceWakeConfig {
    pub enabled: bool,
    pub wake_phrases: Vec<String>,  // ["hey zero", "小零"]
    pub sensitivity: f32,           // 0.0 - 1.0
    pub audio_device: Option<String>,
}
```

---

## 测试结果

- 全部 **1016** 个单元测试通过
- 新增测试覆盖：
  - Feishu Channel: 7 个测试
  - TTS traits: 4 个测试
  - OpenAI TTS: 6 个测试
  - ElevenLabs TTS: 5 个测试
  - TTS 工厂函数: 7 个测试

---

## 后续工作

1. **飞书 Webhook 服务器**: 集成到 gateway 模块，实现实时消息推送
2. **语音唤醒实现**: 基于 VAD + STT 的唤醒词检测
3. **TTS 流式合成**: 支持分段合成和流式播放
4. **飞书配置向导**: 在 onboard wizard 中添加飞书配置交互

---

## 相关文档

- [飞书开放平台文档](https://open.feishu.cn/document/)
- [OpenAI TTS API](https://platform.openai.com/docs/guides/text-to-speech)
- [ElevenLabs API](https://elevenlabs.io/docs/api-reference)
