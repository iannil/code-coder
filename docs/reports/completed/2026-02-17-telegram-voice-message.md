# Telegram 语音消息处理实现报告

**日期**: 2026-02-17
**状态**: 已完成

## 概述

为 ZeroBot 的 Telegram channel 添加语音消息处理功能。语音消息通过 OpenAI Whisper API 进行转录，转录后的文本作为普通消息处理。

## 实现内容

### 1. STT 模块 (`src/stt/`)

创建了独立的 Speech-to-Text 模块，遵循项目的 trait 模式：

- `src/stt/traits.rs` - 定义 `SpeechToText` trait
- `src/stt/openai.rs` - OpenAI Whisper API 实现
- `src/stt/mod.rs` - 模块导出和工厂函数 `create_stt()`

### 2. 配置扩展 (`src/config/schema.rs`)

添加了 `TelegramVoiceConfig` 配置：

```rust
pub struct TelegramVoiceConfig {
    pub enabled: bool,           // 是否启用语音转录
    pub stt_provider: String,    // "openai"
    pub stt_api_key: Option<String>,  // 可选，默认使用主 api_key
    pub stt_model: Option<String>,    // 可选，默认 "whisper-1"
}
```

### 3. TelegramChannel 修改 (`src/channels/telegram.rs`)

- 添加 `stt: Option<Arc<dyn SpeechToText>>` 字段
- 新增 `with_stt()` 构造函数
- 新增 `download_file()` 方法用于下载 Telegram 文件
- 修改 `listen()` 方法以处理语音消息：
  - 授权检查在下载前执行（安全考虑）
  - 下载失败或转录失败时向用户发送错误提示

### 4. 渠道集成 (`src/channels/mod.rs`)

在 `start_channels()` 函数中根据配置创建带 STT 的 TelegramChannel。

### 5. 配置示例更新

`examples/config.toml.example` 添加了语音配置示例：

```toml
[channels_config.telegram.voice]
enabled = true
stt_provider = "openai"
# stt_api_key = "sk-..."  # 可选
# stt_model = "whisper-1"  # 可选
```

## 测试

- 所有 944 个单元测试通过
- 7 个集成测试通过
- Clippy 检查通过（零警告）

## 文件变更清单

| 文件 | 操作 |
|------|------|
| `src/stt/mod.rs` | 新建 |
| `src/stt/traits.rs` | 新建 |
| `src/stt/openai.rs` | 新建 |
| `src/lib.rs` | 修改 - 导出 stt 模块 |
| `src/main.rs` | 修改 - 添加 stt 模块 |
| `src/config/schema.rs` | 修改 - 添加 TelegramVoiceConfig |
| `src/channels/telegram.rs` | 修改 - 添加语音处理逻辑 |
| `src/channels/mod.rs` | 修改 - 构造时传入 STT |
| `src/daemon/mod.rs` | 修改 - 添加 voice 字段 |
| `src/onboard/wizard.rs` | 修改 - 添加 voice 字段 |
| `src/integrations/registry.rs` | 修改 - 添加 voice 字段 |
| `examples/config.toml.example` | 修改 - 添加配置示例 |

## 使用方法

1. 在配置文件中添加语音配置：

```toml
[channels_config.telegram]
bot_token = "YOUR_BOT_TOKEN"
allowed_users = ["YOUR_USER_ID"]

[channels_config.telegram.voice]
enabled = true
stt_provider = "openai"
```

2. 确保已设置 OpenAI API key（通过 `api_key` 配置或 `ZERO_BOT_API_KEY` 环境变量）

3. 启动 ZeroBot，发送语音消息到 Telegram bot，消息将被自动转录并处理

## 错误处理

- 语音下载失败：向用户发送 "Unable to download voice file, please try again"
- 语音文件为空：向用户发送 "Voice file appears to be empty, please try again"
- 语音文件过小（<1KB）：返回错误 "Audio file too small"
- 转录失败：向用户发送 "Voice transcription failed, please try again or use text"
- STT 未配置：静默跳过语音消息（记录 debug 日志）

## 2026-02-17 13:10 更新 - Multipart 解析错误修复

### 问题描述

使用 UniAPI 作为 STT 提供商时，出现以下错误：

```
Voice transcription failed: STT API error (500 Internal Server Error):
{"error":{"code":"internal_error","message":"Provider API error: error parsing multipart form: multipart: NextPart: EOF"}}
```

### 根本原因分析

"multipart: NextPart: EOF" 错误通常发生在：
1. **音频字节为空或过小** - 下载成功但返回空/不完整数据
2. **文件格式问题** - Telegram 语音消息使用 OGG/Opus 格式
3. **服务器端 multipart 解析问题** - 某些服务器对 multipart 请求有严格要求

### 修复内容

1. **telegram.rs** - 添加音频字节验证：
   - 下载后检查是否为空
   - 添加详细的调试日志（文件大小、file_id）
   - 空文件时向用户发送明确错误消息

2. **compatible.rs** - 增强 STT 请求处理：
   - 添加最小文件大小检查（1KB）
   - 添加详细的请求/响应日志
   - 改进错误消息，包含音频大小信息

### 调试建议

如果问题持续存在，检查以下内容：

1. **启用调试日志**：设置 `RUST_LOG=debug` 查看详细日志
2. **检查音频大小**：日志应显示 "Voice file downloaded: XXX bytes"
3. **验证 API key**：确保 STT API key 正确配置
4. **尝试其他提供商**：切换到 `openai` 直接调用 Whisper API

### 配置示例

```toml
[channels_config.telegram.voice]
enabled = true
stt_provider = "uniapi"  # 或 "openai", "groq", "deepinfra"
stt_api_key = "your-stt-api-key"
stt_model = "whisper-1"  # 可选
stt_base_url = "https://hk.uniapi.io"  # 可选，uniapi 的默认值
```

