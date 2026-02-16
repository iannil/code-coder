# 贡献指南

感谢你对 ZeroBot 的贡献兴趣。

## 开发环境设置

### 前置要求

- Rust 1.75+ (推荐使用 rustup)
- Git

### 克隆和构建

```bash
git clone <repo-url>
cd zero-bot
cargo build
```

### 启用 Pre-push Hook

```bash
git config core.hooksPath .githooks
```

这会在推送前运行 `cargo fmt`、`cargo clippy`、`cargo test`。

## 代码风格

### Rust 规范

- **禁止 `unwrap()`** — 使用 `?` 或显式错误处理
- **Clippy pedantic** — 所有代码必须通过 `cargo clippy -- -D warnings`
- **格式化** — 使用 `cargo fmt`

### 文件组织

- 每个模块一个文件夹
- `mod.rs` 作为模块入口
- `traits.rs` 定义 trait
- 每个实现一个文件

### 测试

- 每个文件底部使用 `#[cfg(test)] mod tests {}`
- 测试命名: `test_<功能>_<场景>`
- 目标覆盖率: 80%+

## 提交规范

### Commit Message 格式

```
<type>: <description>

<optional body>
```

**类型**:
- `feat`: 新功能
- `fix`: Bug 修复
- `refactor`: 重构
- `docs`: 文档
- `test`: 测试
- `chore`: 杂项
- `perf`: 性能优化

**示例**:
```
feat: 添加 Perplexity provider

- 实现 OpenAI 兼容接口
- 添加单元测试
- 更新文档
```

## Pull Request 流程

1. Fork 仓库
2. 创建功能分支: `git checkout -b feat/my-feature`
3. 提交更改
4. 确保通过所有检查:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```
5. 推送分支
6. 创建 Pull Request

### PR 标题

使用与 commit 相同的格式:
```
feat: 添加 Perplexity provider
```

### PR 描述

```markdown
## 摘要
简述此 PR 的目的

## 变更内容
- 变更 1
- 变更 2

## 测试
- [ ] 单元测试
- [ ] 集成测试
- [ ] 手动测试
```

## 添加新组件

### 添加新 Provider

1. 创建 `src/providers/your_provider.rs`
2. 实现 `Provider` trait
3. 在 `src/providers/mod.rs` 注册
4. 添加测试
5. 更新 `docs/reference/PROVIDERS.md`

### 添加新 Channel

1. 创建 `src/channels/your_channel.rs`
2. 实现 `Channel` trait
3. 在 `src/channels/mod.rs` 注册
4. 添加测试
5. 更新 `docs/reference/CHANNELS.md`

### 添加新 Tool

1. 创建 `src/tools/your_tool.rs`
2. 实现 `Tool` trait
3. 在 `src/tools/mod.rs` 注册
4. 添加测试
5. 更新文档

## 安全考量

### 敏感更改

以下目录的更改需要特别审查:
- `src/security/` — 安全策略
- `src/tools/shell.rs` — 命令执行
- `src/tools/file_*.rs` — 文件操作

### 检查清单

- [ ] 无硬编码密钥
- [ ] 输入验证
- [ ] 错误信息不泄露敏感信息
- [ ] 无 unsafe 代码（除非必要并有充分理由）

## 获取帮助

- 查看现有 Issues
- 阅读 `/docs` 目录下的文档
- 提交 Issue 讨论
