# 测试指南

## 测试概览

- **测试数量**: 1,811+
- **框架**: Rust 内置测试框架
- **目标覆盖率**: 80%+

## 运行测试

### 全部测试

```bash
cargo test
```

### 特定测试

```bash
cargo test test_name
cargo test module_name::
```

### 带输出

```bash
cargo test -- --nocapture
```

### 特定文件

```bash
cargo test --test memory_comparison
```

## 测试分类

### 单元测试

位于每个源文件底部:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_works() {
        // 测试代码
    }
}
```

### 集成测试

位于 `tests/` 目录:

```
tests/
├── memory_comparison.rs   # SQLite vs Markdown 基准
└── ...
```

## 测试规范

### 命名

```rust
// 格式: test_<功能>_<场景>
#[test]
fn test_provider_creation_with_api_key() { }

#[test]
fn test_provider_creation_without_api_key() { }

#[test]
fn test_provider_creation_fails_for_unknown() { }
```

### 结构

```rust
#[test]
fn test_feature() {
    // Arrange - 准备
    let input = setup_test_data();

    // Act - 执行
    let result = function_under_test(input);

    // Assert - 断言
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
}
```

### 异步测试

```rust
#[tokio::test]
async fn test_async_feature() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

## 模块测试

### Providers

```rust
// src/providers/mod.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_create_openrouter_provider() {
        assert!(create_provider("openrouter", Some("sk-test")).is_ok());
    }

    #[test]
    fn test_all_providers_have_unique_names() {
        // 确保没有重复的 provider 名称
    }
}
```

### Memory

```rust
// src/memory/sqlite.rs
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_store_and_recall() {
        let mem = SqliteMemory::new(&temp_dir())?;

        // Store
        let id = mem.store(entry).await?;

        // Recall
        let results = mem.recall("query", 5).await?;
        assert!(!results.is_empty());
    }
}
```

### Security

```rust
// src/security/secrets.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let store = SecretStore::new(&temp_dir(), true);
        let secret = "my-api-key";

        let encrypted = store.encrypt(secret)?;
        let decrypted = store.decrypt(&encrypted)?;

        assert_eq!(decrypted, secret);
    }
}
```

## 基准测试

### Memory 对比

```bash
cargo test --test memory_comparison -- --nocapture
```

比较 SQLite 和 Markdown 后端的:
- 写入速度
- 读取速度
- 搜索速度
- 内存占用

## Mocking

### Provider Mock

```rust
struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    async fn chat_with_system(
        &self,
        _system: &str,
        _messages: &[Message],
    ) -> Result<String> {
        Ok("Mock response".to_string())
    }

    fn name(&self) -> &str { "mock" }
}
```

### Channel Mock

```rust
struct MockChannel {
    sent: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Channel for MockChannel {
    async fn send(&self, message: &str) -> Result<()> {
        self.sent.lock().unwrap().push(message.to_string());
        Ok(())
    }
    // ...
}
```

## CI/CD 集成

### Pre-push Hook

`.githooks/pre-push`:

```bash
#!/bin/sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

启用:

```bash
git config core.hooksPath .githooks
```

### GitHub Actions (示例)

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
```

## 测试覆盖率

### 使用 cargo-tarpaulin

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### 目标

- 整体: 80%+
- 核心模块 (providers, memory, security): 90%+
- 工具模块: 85%+

## 常见问题

### 测试失败: 权限问题

```bash
# macOS 上某些测试需要文件系统权限
chmod +x target/debug/deps/zero_bot-*
```

### 测试超时

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_with_timeout() {
    tokio::time::timeout(Duration::from_secs(5), async {
        // 测试代码
    }).await.unwrap();
}
```

### 环境变量

```rust
#[test]
fn test_with_env() {
    std::env::set_var("TEST_VAR", "value");
    // 测试
    std::env::remove_var("TEST_VAR");
}
```
