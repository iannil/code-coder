# 扩展指南

如何添加新的 Provider、Channel、Tool 等。

## 添加新 Provider

### 1. 创建实现文件

`src/providers/my_provider.rs`:

```rust
use crate::providers::traits::{Message, Provider};
use async_trait::async_trait;

pub struct MyProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl MyProvider {
    pub fn new(api_key: Option<&str>) -> Self {
        Self {
            api_key: api_key.map(String::from),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Provider for MyProvider {
    async fn chat_with_system(
        &self,
        system: &str,
        messages: &[Message],
    ) -> anyhow::Result<String> {
        // 实现 API 调用
        todo!()
    }

    fn name(&self) -> &str {
        "my-provider"
    }
}
```

### 2. 注册到工厂

`src/providers/mod.rs`:

```rust
mod my_provider;

pub fn create_provider(name: &str, api_key: Option<&str>) -> Result<Box<dyn Provider>> {
    match name {
        // ... 现有 providers
        "my-provider" => Ok(Box::new(my_provider::MyProvider::new(api_key))),
        // ...
    }
}
```

### 3. 添加测试

```rust
#[test]
fn test_create_my_provider() {
    assert!(create_provider("my-provider", Some("key")).is_ok());
}
```

### 4. 更新文档

更新 `docs/reference/PROVIDERS.md`。

---

## 添加新 Channel

### 1. 创建实现文件

`src/channels/my_channel.rs`:

```rust
use crate::channels::traits::{Channel, HealthStatus, IncomingMessage};
use async_trait::async_trait;

pub struct MyChannel {
    // 配置字段
}

impl MyChannel {
    pub fn new(config: &ChannelConfig) -> anyhow::Result<Self> {
        // 初始化
        todo!()
    }
}

#[async_trait]
impl Channel for MyChannel {
    async fn send(&self, message: &str) -> anyhow::Result<()> {
        // 发送消息
        todo!()
    }

    async fn listen(&self) -> anyhow::Result<IncomingMessage> {
        // 监听消息
        todo!()
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        // 健康检查
        Ok(HealthStatus::Healthy)
    }

    fn name(&self) -> &str {
        "my-channel"
    }
}
```

### 2. 注册到模块

`src/channels/mod.rs`:

```rust
mod my_channel;
pub use my_channel::MyChannel;
```

### 3. 添加配置支持

`src/config/schema.rs`:

```rust
pub struct ChannelConfig {
    pub kind: String,
    // 添加 my_channel 配置字段
}
```

---

## 添加新 Tool

### 1. 创建实现文件

`src/tools/my_tool.rs`:

```rust
use crate::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct MyTool;

impl MyTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for MyTool {
    async fn execute(&self, params: Value) -> anyhow::Result<ToolResult> {
        // 解析参数
        let input = params.get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing input"))?;

        // 执行操作
        let output = format!("Processed: {}", input);

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input to process"
                }
            },
            "required": ["input"]
        })
    }

    fn name(&self) -> &str {
        "my_tool"
    }

    fn description(&self) -> &str {
        "A custom tool that does something"
    }
}
```

### 2. 注册到模块

`src/tools/mod.rs`:

```rust
mod my_tool;
pub use my_tool::MyTool;

pub fn create_tools() -> Vec<Box<dyn Tool>> {
    vec![
        // ... 现有 tools
        Box::new(MyTool::new()),
    ]
}
```

### 3. 安全考虑

如果工具执行敏感操作:

```rust
impl Tool for MyTool {
    // 添加安全检查
    async fn execute(&self, params: Value) -> anyhow::Result<ToolResult> {
        // 验证输入
        validate_input(&params)?;

        // 检查权限
        check_permissions()?;

        // 执行
        // ...
    }
}
```

---

## 添加新 Memory 后端

### 1. 创建实现文件

`src/memory/my_backend.rs`:

```rust
use crate::memory::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;

pub struct MyBackend {
    // 存储
}

impl MyBackend {
    pub fn new() -> anyhow::Result<Self> {
        todo!()
    }
}

#[async_trait]
impl Memory for MyBackend {
    async fn store(&self, entry: MemoryEntry) -> anyhow::Result<String> {
        todo!()
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        todo!()
    }

    async fn forget(&self, id: &str) -> anyhow::Result<()> {
        todo!()
    }

    async fn list(&self, category: Option<MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        todo!()
    }
}
```

---

## 添加新 Tunnel

### 1. 创建实现文件

`src/tunnel/my_tunnel.rs`:

```rust
use crate::tunnel::Tunnel;
use async_trait::async_trait;

pub struct MyTunnel {
    url: Option<String>,
}

impl MyTunnel {
    pub fn new() -> Self {
        Self { url: None }
    }
}

#[async_trait]
impl Tunnel for MyTunnel {
    async fn start(&mut self, port: u16) -> anyhow::Result<()> {
        // 启动隧道
        // self.url = Some(...)
        todo!()
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        // 停止隧道
        todo!()
    }

    fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}
```

---

## 测试新组件

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_component_creation() {
        let component = MyComponent::new();
        assert_eq!(component.name(), "my-component");
    }

    #[tokio::test]
    async fn test_my_component_functionality() {
        let component = MyComponent::new();
        let result = component.do_something().await;
        assert!(result.is_ok());
    }
}
```

### 集成测试

在 `tests/` 目录创建测试文件。

---

## 提交检查清单

- [ ] 实现对应 trait
- [ ] 添加到工厂/模块注册
- [ ] 单元测试覆盖
- [ ] 通过 clippy 检查
- [ ] 更新文档
- [ ] 更新 CHANGELOG
