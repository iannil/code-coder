# 本地 Whisper 语音识别集成

**完成时间**: 2026-02-18
**状态**: 已完成

## 概述

为 ZeroBot 添加本地 Whisper 语音识别支持，使用 `faster-whisper-server` Docker 容器作为后端。

## 变更文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `services/zero-bot/src/stt/mod.rs` | 修改 | 添加 "local"/"faster-whisper"/"whisper-local" provider |
| `services/zero-bot/src/stt/compatible.rs` | 修改 | 支持空 api_key（本地服务不需要认证） |
| `ops.sh` | 修改 | 添加 whisper 服务管理（Docker, 端口 4403） |
| `services/zero-bot/examples/config.toml.example` | 修改 | 添加本地 STT 配置示例 |
| `services/zero-bot/docs/guides/CONFIGURATION.md` | 修改 | 添加本地 STT 文档 |
| `services/zero-bot/src/agent/confirmation.rs` | 修改 | 修复测试使用 ConfirmationResponse enum |

## 运维脚本 (ops.sh)

### 前置要求

- Docker Desktop 已安装并运行

### 命令

```bash
# 启动 Whisper 服务（Docker 容器，端口 4403）
./ops.sh start whisper

# 停止服务
./ops.sh stop whisper

# 重启服务
./ops.sh restart whisper

# 查看所有服务状态
./ops.sh status

# 查看日志
./ops.sh logs whisper
./ops.sh tail whisper  # 实时跟踪
```

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `WHISPER_MODEL` | base | 模型大小: tiny, base, small, medium, large |
| `WHISPER_IMAGE` | fedirz/faster-whisper-server:latest-cpu | Docker 镜像 |

### 使用示例

```bash
# 使用 small 模型启动
WHISPER_MODEL=small ./ops.sh start whisper

# 使用 GPU 镜像（需要 NVIDIA GPU + nvidia-docker）
WHISPER_IMAGE=fedirz/faster-whisper-server:latest-cuda ./ops.sh start whisper
```

## 服务端口分配

| 服务 | 端口 | 类型 |
|------|------|------|
| CodeCoder API Server | 4400 | 本地进程 |
| Web Frontend | 4401 | 本地进程 |
| ZeroBot Daemon | 4402 | 本地进程 |
| Whisper STT Server | 4403 | Docker 容器 |

## Docker 配置

- 容器名称: `codecoder-whisper`
- 默认镜像: `fedirz/faster-whisper-server:latest-cpu`
- 端口映射: `4403:8000`
- 模型缓存: `~/.cache/huggingface:/root/.cache/huggingface`
- 启动参数: `--rm` (容器停止时自动删除)

## 核心实现

### 1. CompatibleStt 修改

当 `api_key` 为空时跳过 Authorization header：

```rust
let mut request = self.client.post(&url).multipart(form);
if !self.api_key.is_empty() {
    request = request.header("Authorization", format!("Bearer {}", self.api_key));
}
```

### 2. create_stt() 工厂函数

添加 "local" provider 分支：

```rust
"local" | "faster-whisper" | "whisper-local" => {
    let url = base_url.ok_or_else(|| {
        anyhow::anyhow!(
            "base_url is required for 'local' STT provider (e.g., http://localhost:4403)"
        )
    })?;
    Ok(Arc::new(CompatibleStt::new(
        api_key.to_string(),
        url.to_string(),
        model.map(ToString::to_string),
    )))
}
```

## 配置示例

```toml
[channels_config.telegram.voice]
enabled = true
stt_provider = "local"
stt_base_url = "http://localhost:4403"
stt_model = "base"  # tiny, base, small, medium, large
# stt_api_key 可省略
```

## 测试结果

- 新增 5 个 STT 测试用例
- 修复 5 个 confirmation.rs 测试用例
- 全部 994 测试通过

## 支持的 Provider

| provider | 说明 | 需要 api_key | 需要 base_url |
|----------|------|-------------|---------------|
| `openai` | OpenAI Whisper API | 是 | 否 |
| `uniapi` | UniAPI 兼容服务 | 是 | 否 |
| `groq` | Groq Whisper | 是 | 否 |
| `deepinfra` | DeepInfra | 是 | 否 |
| `local` | 本地 Whisper 服务 | 否 | 是 |
| `compatible` | OpenAI 兼容 API | 是 | 是 |
