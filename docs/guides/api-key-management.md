# API Key 管理指南

本指南详细介绍如何在 CodeCoder 中安全地管理和存储 AI 提供商的 API Key。

## 目录

1. [存储方式优先级](#存储方式优先级)
2. [使用环境变量](#使用环境变量)
3. [使用认证存储](#使用认证存储)
4. [配置文件存储](#配置文件存储)
5. [安全最佳实践](#安全最佳实践)
6. [常见使用场景](#常见使用场景)
7. [故障排除](#故障排除)

---

## 存储方式优先级

CodeCoder 按以下优先级查找 API Key：

```
1. 环境变量 (最高优先级，推荐)
   ↓
2. 认证存储 (codecoder auth login)
   ↓
3. 配置文件 (provider.<id>.options.apiKey)
```

### 优先级说明

| 方式 | 优先级 | 安全性 | 推荐场景 |
|------|--------|--------|----------|
| 环境变量 | 1 | ⭐⭐⭐⭐⭐ | 生产环境、CI/CD |
| 认证存储 | 2 | ⭐⭐⭐⭐ | 个人开发 |
| 配置文件 | 3 | ⭐⭐ | 快速测试 |

> **注意**: 如果环境变量中存在 API Key，它将覆盖认证存储和配置文件中的设置。

---

## 使用环境变量

### 为什么推荐环境变量？

- **安全性**: 密钥不会被写入配置文件或版本控制
- **灵活性**: 不同环境可以使用不同的密钥
- **标准**: 符合 12-Factor App 最佳实践

### 环境变量命名

CodeCoder 支持主流提供商的标准环境变量名：

| 提供商 | 环境变量名 |
|--------|-----------|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| Google | `GOOGLE_API_KEY` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` |
| AWS Bedrock | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` |
| Groq | `GROQ_API_KEY` |
| Hugging Face | `HUGGINGFACE_API_KEY` |
| Cohere | `COHERE_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| Perplexity | `PERPLEXITY_API_KEY` |

### 设置环境变量

#### macOS / Linux (Bash/Zsh)

**临时设置（当前会话）**:

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
export OPENAI_API_KEY="sk-your-key-here"
export GOOGLE_API_KEY="your-google-key-here"
```

**永久设置**:

```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
echo 'export ANTHROPIC_API_KEY="sk-ant-your-key-here"' >> ~/.zshrc
echo 'export OPENAI_API_KEY="sk-your-key-here"' >> ~/.zshrc
source ~/.zshrc
```

#### Windows (PowerShell)

```powershell
# 临时设置
$env:ANTHROPIC_API_KEY="sk-ant-your-key-here"
$env:OPENAI_API_KEY="sk-your-key-here"

# 永久设置
[System.Environment]::SetEnvironmentVariable('ANTHROPIC_API_KEY', 'sk-ant-your-key-here', 'User')
```

#### Windows (CMD)

```cmd
REM 临时设置
set ANTHROPIC_API_KEY=sk-ant-your-key-here
set OPENAI_API_KEY=sk-your-key-here
```

### 使用 .env 文件

在项目根目录创建 `.env` 文件（确保添加到 `.gitignore`）：

```bash
# .env
ANTHROPIC_API_KEY=sk-ant-your-key-here
OPENAI_API_KEY=sk-your-key-here
GOOGLE_API_KEY=your-google-key-here
```

然后在运行前加载：

```bash
# 使用 bun
bun run --env-file=.env dev

# 或使用 dotenv
bun run -r dotenv/config dev
```

### .gitignore 配置

确保 `.env` 文件不被提交到版本控制：

```gitignore
# .gitignore
.env
.env.local
.env.*.local
```

---

## 使用认证存储

CodeCoder 提供内置的认证管理命令，可以安全地存储 API Key。

### 登录命令

```bash
# 交互式登录（推荐）
codecoder auth login

# 指定提供商
codecoder auth login --provider anthropic

# 指定 API Key
codecoder auth login --provider anthropic --key sk-ant-your-key-here
```

### 查看认证状态

```bash
# 查看所有认证
codecoder auth status

# 查看特定提供商
codecoder auth status --provider anthropic
```

### 登出命令

```bash
# 登出所有提供商
codecoder auth logout

# 登出特定提供商
codecoder auth logout --provider anthropic
```

### 认证存储位置

认证信息存储在：

| 平台 | 存储位置 |
|------|----------|
| macOS | `~/Library/Application Support/codecoder/auth.json` |
| Linux | `~/.config/codecoder/auth.json` |
| Windows | `%APPDATA%/codecoder/auth.json` |

### 认证存储安全

认证存储使用以下安全措施：

- **文件权限**: 自动设置为仅用户可读写（`600`）
- **加密**: 支持操作系统密钥链加密（可选）

---

## 配置文件存储

### 直接在配置文件中存储

> **警告**: 不推荐在生产环境中使用此方式。

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "sk-ant-your-key-here"
      }
    }
  }
}
```

### 使用环境变量引用

更安全的方式是在配置文件中引用环境变量：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

### 使用认证存储引用

引用认证存储中的密钥：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "env": ["ANTHROPIC_API_KEY"]
    }
  }
}
```

---

## 安全最佳实践

### 1. 永远不要提交 API Key

**错误示例**:

```json
{
  "apiKey": "sk-ant-1234567890abcdef"  // 危险！
}
```

**正确示例**:

```json
{
  "provider": {
    "anthropic": {
      "env": ["ANTHROPIC_API_KEY"]  // 安全
    }
  }
}
```

### 2. 使用 .gitignore 保护敏感文件

```gitignore
# .gitignore
.env
.env.local
codecoder.json.local
~/.codecoder/config.json.local
```

### 3. 设置正确的文件权限

```bash
# 配置文件仅用户可读写
chmod 600 ~/.codecoder/config.json

# .env 文件仅用户可读写
chmod 600 .env

# 认证存储仅用户可读写
chmod 600 ~/.config/codecoder/auth.json
```

### 4. 定期轮换 API Key

建议定期（每 90 天）更换 API Key：

1. 在提供商控制台生成新密钥
2. 更新环境变量或认证存储
3. 测试新密钥工作正常
4. 撤销旧密钥

### 5. 使用项目专用密钥

为不同项目创建不同的 API Key，以便：

- 追踪使用情况
- 限制权限范围
- 单独撤销密钥

### 6. 限制 API Key 权限

在提供商控制台中：

- 设置使用限额
- 限制可访问的模型
- 设置 IP 白名单（如支持）
- 启用审计日志

### 7. 使用密钥管理服务

对于企业部署，考虑使用：

- **AWS Secrets Manager**
- **Azure Key Vault**
- **Google Secret Manager**
- **HashiCorp Vault**

### 8. 审计和监控

定期检查：

```bash
# 查看认证状态
codecoder auth status

# 列出所有模型（验证连接）
codecoder models

# 检查配置文件
cat ~/.codecoder/config.json
```

---

## 常见使用场景

### 场景 1: 个人开发

**推荐**: 使用认证存储

```bash
# 登录一次
codecoder auth login

# 之后无需重复设置
bun dev "your message"
```

### 场景 2: 团队协作

**推荐**: 使用环境变量 + .env.example

创建 `.env.example`:

```bash
# .env.example
ANTHROPIC_API_KEY=your-anthropic-api-key-here
OPENAI_API_KEY=your-openai-api-key-here
```

团队成员创建自己的 `.env` 文件。

### 场景 3: CI/CD 部署

**推荐**: 使用 CI/CD 平台的秘密管理

**GitHub Actions**:

```yaml
# .github/workflows/test.yml
- name: Run tests
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
  run: bun test
```

**GitLab CI**:

```yaml
# .gitlab-ci.yml
test:
  script:
    - export ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY
    - bun test
```

### 场景 4: Docker 部署

**推荐**: 使用 Docker secrets 或环境变量

```dockerfile
# Dockerfile
ENV ANTHROPIC_API_KEY=""
```

```yaml
# docker-compose.yml
services:
  codecoder:
    image: codecoder:latest
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
    env_file:
      - .env
```

### 场景 5: 多提供商配置

**推荐**: 为每个提供商设置独立的环境变量

```bash
export ANTHROPIC_API_KEY="sk-ant-xxx"
export OPENAI_API_KEY="sk-yyy"
export GOOGLE_API_KEY="zzz"
```

配置文件：

```json
{
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "env": ["ANTHROPIC_API_KEY"]
    },
    "openai": {
      "env": ["OPENAI_API_KEY"]
    },
    "google": {
      "env": ["GOOGLE_API_KEY"]
    }
  }
}
```

### 场景 6: 代理场景

当使用代理服务器时，可能需要单独配置代理认证：

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "baseURL": "https://proxy.company.com/anthropic",
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

---

## 故障排除

### 问题 1: API Key 无效

**症状**:

```
Error: Invalid API key
```

**解决方案**:

1. 检查 API Key 是否正确复制
2. 确认 API Key 未被撤销
3. 检查账户余额和配额

```bash
# 验证环境变量
echo $ANTHROPIC_API_KEY

# 重新登录
codecoder auth logout
codecoder auth login
```

### 问题 2: 环境变量未生效

**症状**: 配置文件中的 API Key 被使用，而非环境变量

**解决方案**:

```bash
# 确认环境变量已设置
echo $ANTHROPIC_API_KEY

# 检查是否在当前 shell 中
export ANTHROPIC_API_KEY="sk-ant-xxx"

# 重启终端或重新加载配置
source ~/.zshrc
```

### 问题 3: 多个配置冲突

**症状**: 不知道哪个 API Key 被使用

**解决方案**: 查看详细日志

```bash
export CCODE_LOG_LEVEL=debug
bun dev "test"
```

日志会显示：

```
[debug] Loading API Key from environment variable: ANTHROPIC_API_KEY
[debug] Using provider: anthropic
[debug] Using model: claude-sonnet-4-20250514
```

### 问题 4: 权限错误

**症状**: 无法读取配置文件

```
Error: Permission denied: ~/.codecoder/config.json
```

**解决方案**:

```bash
# 修复文件权限
chmod 600 ~/.codecoder/config.json
chmod 600 ~/.config/codecoder/auth.json
```

### 问题 5: Docker 中环境变量未传递

**症状**: 容器内无法访问环境变量

**解决方案**:

```bash
# 确保使用 -e 或 --env-file
docker run -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" codecoder

# 或使用 env_file
docker run --env-file .env codecoder
```

---

## 配置检查清单

使用此清单验证 API Key 配置：

- [ ] API Key 未提交到版本控制
- [ ] `.env` 文件已添加到 `.gitignore`
- [ ] 配置文件权限设置为 `600`
- [ ] 环境变量正确设置
- [ ] 认证状态正常（`codecoder auth status`）
- [ ] 模型列表可访问（`codecoder models`）
- [ ] 测试消息可以正常发送
- [ ] CI/CD 环境变量已配置
- [ ] Docker 环境变量已传递
- [ ] API Key 有使用限额和监控

---

## 参考资源

- [主流提供商配置指南](./mainstream-providers.md)
- [自定义提供商配置](./custom-provider.md)
- [新手入门指南](./beginners-guide.md)
- [Anthropic 控制台](https://console.anthropic.com)
- [OpenAI 控制台](https://platform.openai.com/api-keys)
- [Google AI 控制台](https://aistudio.google.com/app/apikey)
