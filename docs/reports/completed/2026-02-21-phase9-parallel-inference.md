# Phase 9: 多模型并行推理 (Multi-Model Parallel Inference)

**日期**: 2026-02-21
**状态**: ✅ 已完成

## 实现概要

Phase 9 实现了多模型并行推理功能，允许同一请求发送到多个 LLM 提供商，返回聚合结果供比较。

## 完成的任务

### 1. Provider 抽象层 (services/zero-gateway/src/provider/)

**mod.rs** - 统一 Provider 接口定义：
- ✅ `Provider` trait 定义统一的 LLM 提供商接口
- ✅ `ChatRequest` / `ChatResponse` 统一请求/响应格式
- ✅ `ProviderRegistry` 管理已注册的提供商
- ✅ `create_registry()` 工厂函数按配置创建提供商
- ✅ 支持按模型名称自动路由到正确的提供商

**anthropic.rs** - Anthropic (Claude) 提供商实现：
- ✅ 完整的 Anthropic Messages API 集成
- ✅ 支持所有 Claude 模型 (claude-opus-4, claude-sonnet-4, claude-3-* 系列)
- ✅ 自定义 base_url 支持（代理、自托管）
- ✅ 响应解析和 Token 使用统计

**openai.rs** - OpenAI 提供商实现：
- ✅ 完整的 OpenAI Chat Completions API 集成
- ✅ 支持所有 GPT 和 o1 模型
- ✅ 自定义 base_url 支持（Azure OpenAI、兼容 API）
- ✅ 系统消息自动插入

### 2. 并行推理端点 (services/zero-gateway/src/parallel.rs)

- ✅ `POST /api/v1/parallel` 端点
- ✅ 支持最多 5 个模型并行查询
- ✅ 使用 `JoinSet` 实现真正的并发执行
- ✅ 结果按模型名称排序保证一致性
- ✅ 错误隔离（单个模型失败不影响其他）
- ✅ 聚合统计（总 tokens、最大延迟）

### 3. 配置扩展 (services/zero-common/src/config.rs)

- ✅ 新增 `ApiKeysConfig` 配置结构
- ✅ 支持 Anthropic、OpenAI、Google、DeepSeek API keys
- ✅ 环境变量 fallback 支持

### 4. 路由集成 (services/zero-gateway/src/routes.rs)

- ✅ 并行路由集成到主路由器
- ✅ 认证中间件保护
- ✅ API keys 从配置或环境变量加载

## 数据流

```
POST /api/v1/parallel
{
  "models": ["claude-sonnet-4", "gpt-4o"],
  "messages": [{"role": "user", "content": "Hello"}]
}
    ↓
ParallelState.registry
    ↓
┌──────────────────────────────────────────────┐
│         并行执行 (JoinSet)                    │
├──────────────────┬───────────────────────────┤
│ AnthropicProvider│ OpenAIProvider            │
│ (claude-sonnet-4)│ (gpt-4o)                  │
└──────────────────┴───────────────────────────┘
    ↓
聚合结果
    ↓
{
  "results": [
    {"model": "claude-sonnet-4", "content": "...", "tokens": {...}},
    {"model": "gpt-4o", "content": "...", "tokens": {...}}
  ],
  "total_tokens": 500,
  "total_latency_ms": 1500
}
```

## 新增文件

| 文件 | 描述 |
|------|------|
| `services/zero-gateway/src/provider/mod.rs` | Provider trait 和 Registry |
| `services/zero-gateway/src/provider/anthropic.rs` | Anthropic 提供商实现 |
| `services/zero-gateway/src/provider/openai.rs` | OpenAI 提供商实现 |
| `services/zero-gateway/src/parallel.rs` | 并行推理端点 |

## 修改的文件

| 文件 | 修改 |
|------|------|
| `services/zero-common/src/config.rs` | 添加 ApiKeysConfig |
| `services/zero-gateway/src/lib.rs` | 导出新模块 |
| `services/zero-gateway/src/routes.rs` | 集成并行路由 |

## 测试覆盖

- `test_anthropic_provider_models` - Anthropic 模型支持检测
- `test_anthropic_request_serialization` - Anthropic 请求序列化
- `test_openai_provider_models` - OpenAI 模型支持检测
- `test_openai_request_serialization` - OpenAI 请求序列化
- `test_parallel_request_deserialization` - 并行请求反序列化
- `test_parallel_response_serialization` - 并行响应序列化
- `test_parallel_state_creation` - ParallelState 创建
- `test_provider_registry` - Provider 注册表功能
- `test_chat_request_serialization` - ChatRequest 序列化
- `test_chat_response_serialization` - ChatResponse 序列化

## API 使用示例

### 请求

```bash
curl -X POST http://localhost:4402/api/v1/parallel \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "models": ["claude-sonnet-4", "gpt-4o"],
    "messages": [{"role": "user", "content": "What is 2+2?"}],
    "max_tokens": 100
  }'
```

### 响应

```json
{
  "results": [
    {
      "model": "claude-sonnet-4",
      "provider": "anthropic",
      "content": "2 + 2 = 4",
      "tokens": {"input": 12, "output": 8, "total": 20},
      "latency_ms": 450
    },
    {
      "model": "gpt-4o",
      "provider": "openai",
      "content": "The answer is 4.",
      "tokens": {"input": 10, "output": 6, "total": 16},
      "latency_ms": 380
    }
  ],
  "total_tokens": 36,
  "total_latency_ms": 450
}
```

## 配置示例

```json
{
  "api_keys": {
    "anthropic": "sk-ant-...",
    "openai": "sk-proj-...",
    "google": "AIza...",
    "deepseek": "sk-..."
  }
}
```

或使用环境变量：

```bash
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-proj-...
```

## 设计决策

### 1. JoinSet vs futures::join_all

选择 `tokio::task::JoinSet` 因为：
- 任务在独立线程池执行，不阻塞主线程
- 支持任务取消和超时
- 提供更好的错误隔离

### 2. 错误处理策略

采用"部分成功"模式：
- 单个模型失败返回 `error` 字段
- 其他模型结果正常返回
- 不中断整个请求

### 3. 模型路由

使用前缀匹配 + 精确匹配：
1. 先查 ProviderRegistry 的精确模型映射
2. fallback 到 `supports_model()` 前缀匹配 (claude*, gpt-*, o1*)

## 后续优化 (P2)

1. 支持更多提供商 (Google Gemini, DeepSeek, Ollama)
2. 添加请求超时配置
3. 添加流式响应支持 (SSE)
4. 实现智能重试策略
5. 添加模型评分和 A/B 测试框架

---

*记录时间: 2026-02-21*
*总测试数: Gateway 74 tests (零失败)*
