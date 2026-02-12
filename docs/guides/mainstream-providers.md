# 主流 AI 提供商配置指南

本指南详细介绍如何为 Anthropic、OpenAI、Google、Azure、Amazon Bedrock 等主流 AI 提供商配置自定义端点和 API Key。

## 目录

1. [配置概述](#配置概述)
2. [Anthropic (Claude)](#anthropic-claude)
3. [OpenAI (GPT)](#openai-gpt)
4. [Google (Gemini)](#google-gemini)
5. [Azure OpenAI](#azure-openai)
6. [Amazon Bedrock](#amazon-bedrock)
7. [使用代理服务器](#使用代理服务器)
8. [私有部署模型](#私有部署模型)
9. [配置验证](#配置验证)

---

## 配置概述

### 配置文件位置

CodeCoder 按以下优先级查找配置：

1. **项目配置** - 项目根目录的 `codecoder.json`
2. **全局配置** - `~/.codecoder/config.json`
3. **环境变量** - 通过 `CCODE_CONFIG` 指定

### 通用配置模板

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "provider-id/model-id",
  "provider": {
    "provider-id": {
      "options": {
        "baseURL": "https://custom-endpoint.com",
        "apiKey": "{env:API_KEY}"
      }
    }
  }
}
```

### API Key 存储优先级

1. **环境变量** - 最高优先级，推荐方式
2. **认证存储** - `codecoder auth login`
3. **配置文件** - 直接写在 `options.apiKey`

> **安全建议**: 尽量使用环境变量或认证存储，避免将 API Key 硬编码在配置文件中。

---

## Anthropic (Claude)

### 默认配置

```json
{
  "model": "anthropic/claude-sonnet-4-20250514"
}
```

### 自定义端点

使用代理或自定义端点：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "options": {
        "baseURL": "https://custom.anthropic.com",
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

### 使用环境变量

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
```

配置文件：

```json
{
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "env": ["ANTHROPIC_API_KEY"]
    }
  }
}
```

### 通过代理访问

```json
{
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "options": {
        "baseURL": "https://proxy.example.com/anthropic"
      }
    }
  }
}
```

### 支持的 Claude 模型

| 模型 ID | 说明 | 上下文长度 |
|---------|------|-----------|
| `claude-sonnet-4-20250514` | Claude Sonnet 4 | 200K |
| `claude-opus-4-20250514` | Claude Opus 4 | 200K |
| `claude-haiku-4-20250514` | Claude Haiku 4 | 200K |
| `claude-3-5-sonnet-20241022` | Claude 3.5 Sonnet | 200K |

---

## OpenAI (GPT)

### 默认配置

```json
{
  "model": "openai/gpt-4o"
}
```

### 自定义端点

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "openai/gpt-4o",
  "provider": {
    "openai": {
      "options": {
        "baseURL": "https://custom.openai.com/v1",
        "apiKey": "{env:OPENAI_API_KEY}"
      }
    }
  }
}
```

### 使用环境变量

```bash
export OPENAI_API_KEY="sk-your-key-here"
```

### 通过代理访问

```json
{
  "model": "openai/gpt-4o",
  "provider": {
    "openai": {
      "options": {
        "baseURL": "https://proxy.example.com/openai/v1"
      }
    }
  }
}
```

### Azure OpenAI 端点

如果使用 Azure OpenAI 服务提供的 OpenAI 兼容端点：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "openai/gpt-4o",
  "provider": {
    "openai": {
      "options": {
        "baseURL": "https://your-resource.openai.azure.com/openai/deployments/your-deployment",
        "apiKey": "{env:AZURE_OPENAI_API_KEY}"
      }
    }
  }
}
```

### 支持的 GPT 模型

| 模型 ID | 说明 | 上下文长度 |
|---------|------|-----------|
| `gpt-4o` | GPT-4 Omni | 128K |
| `gpt-4o-mini` | GPT-4 Omni Mini | 128K |
| `gpt-4-turbo` | GPT-4 Turbo | 128K |
| `gpt-4` | GPT-4 | 8K/32K |
| `gpt-3.5-turbo` | GPT-3.5 Turbo | 16K |

---

## Google (Gemini)

### 默认配置

```json
{
  "model": "google/gemini-2.0-flash-exp"
}
```

### 自定义配置

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "google/gemini-2.0-flash-exp",
  "provider": {
    "google": {
      "options": {
        "apiKey": "{env:GOOGLE_API_KEY}"
      }
    }
  }
}
```

### 使用环境变量

```bash
export GOOGLE_API_KEY="your-google-api-key-here"
```

### 支持的 Gemini 模型

| 模型 ID | 说明 | 上下文长度 |
|---------|------|-----------|
| `gemini-2.0-flash-exp` | Gemini 2.0 Flash | 1M |
| `gemini-1.5-pro` | Gemini 1.5 Pro | 2M |
| `gemini-1.5-flash` | Gemini 1.5 Flash | 1M |
| `gemini-1.0-pro` | Gemini 1.0 Pro | 32K |

---

## Azure OpenAI

### 基本配置

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "azure/gpt-4o",
  "provider": {
    "azure": {
      "options": {
        "baseURL": "https://your-resource.openai.azure.com/openai/deployments/your-deployment",
        "apiKey": "{env:AZURE_OPENAI_API_KEY}"
      }
    }
  }
}
```

### 完整 Azure 配置

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "azure/gpt-4o",
  "provider": {
    "azure": {
      "resourceName": "your-resource-name",
      "deploymentName": "your-deployment-name",
      "apiVersion": "2024-02-15-preview",
      "options": {
        "apiKey": "{env:AZURE_OPENAI_API_KEY}"
      }
    }
  }
}
```

### 使用环境变量

```bash
export AZURE_OPENAI_API_KEY="your-azure-api-key"
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
```

### 多个 Azure 部署

```json
{
  "provider": {
    "azure-prod": {
      "api": "https://prod-resource.openai.azure.com",
      "npm": "@ai-sdk/azure",
      "options": {
        "apiKey": "{env:AZURE_PROD_KEY}"
      },
      "models": {
        "gpt4": {
          "id": "gpt-4o-prod-deployment"
        }
      }
    },
    "azure-dev": {
      "api": "https://dev-resource.openai.azure.com",
      "npm": "@ai-sdk/azure",
      "options": {
        "apiKey": "{env:AZURE_DEV_KEY}"
      },
      "models": {
        "gpt4": {
          "id": "gpt-4o-dev-deployment"
        }
      }
    }
  }
}
```

---

## Amazon Bedrock

### 基本配置

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "amazon-bedrock/anthropic.claude-sonnet-4-5",
  "provider": {
    "amazon-bedrock": {
      "options": {
        "region": "us-east-1"
      }
    }
  }
}
```

### 使用 AWS Profile

```json
{
  "model": "amazon-bedrock/anthropic.claude-sonnet-4-5",
  "provider": {
    "amazon-bedrock": {
      "options": {
        "region": "us-east-1",
        "profile": "default"
      }
    }
  }
}
```

### 使用自定义凭证

```json
{
  "model": "amazon-bedrock/anthropic.claude-sonnet-4-5",
  "provider": {
    "amazon-bedrock": {
      "options": {
        "region": "us-east-1",
        "accessKeyId": "{env:AWS_ACCESS_KEY_ID}",
        "secretAccessKey": "{env:AWS_SECRET_ACCESS_KEY}"
      }
    }
  }
}
```

### 使用环境变量

```bash
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_REGION="us-east-1"
```

### 支持的 Bedrock 模型

| 模型 ID | 说明 |
|---------|------|
| `anthropic.claude-sonnet-4-5` | Claude Sonnet 4.5 |
| `anthropic.claude-opus-4-5` | Claude Opus 4.5 |
| `anthropic.claude-haiku-4-5` | Claude Haiku 4.5 |
| `us.anthropic.claude-3-5-sonnet-20241022-v2:0` | Claude 3.5 Sonnet (US) |
| `meta.llama3-70b-instruct-v1:0` | Llama 3 70B |

---

## 使用代理服务器

### 统一代理配置

如果你有一个代理服务器处理所有 AI 请求：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "options": {
        "baseURL": "https://proxy.example.com/anthropic",
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    },
    "openai": {
      "options": {
        "baseURL": "https://proxy.example.com/openai/v1",
        "apiKey": "{env:OPENAI_API_KEY}"
      }
    },
    "google": {
      "options": {
        "baseURL": "https://proxy.example.com/google"
      }
    }
  }
}
```

### Nginx 代理配置示例

```nginx
server {
    listen 443 ssl;
    server_name proxy.example.com;

    location /anthropic/ {
        proxy_pass https://api.anthropic.com/;
        proxy_set_header Host api.anthropic.com;
        proxy_set_header x-api-key $http_x_api_key;
    }

    location /openai/ {
        proxy_pass https://api.openai.com/;
        proxy_set_header Host api.openai.com;
        proxy_set_header Authorization $http_authorization;
    }
}
```

### Cloudflare Workers 代理示例

```javascript
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const targetUrl = url.pathname.startsWith('/anthropic')
      ? 'https://api.anthropic.com' + url.pathname.replace('/anthropic', '')
      : 'https://api.openai.com' + url.pathname.replace('/openai', '');

    return fetch(targetUrl, {
      method: request.method,
      headers: request.headers,
      body: request.body
    });
  }
};
```

---

## 私有部署模型

### Ollama 本地模型

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "ollama/llama3",
  "provider": {
    "ollama": {
      "name": "Ollama",
      "api": "http://localhost:11434",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "baseURL": "http://localhost:11434/v1"
      }
    }
  }
}
```

### vLLM 部署

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "vllm/mixtral",
  "provider": {
    "vllm": {
      "name": "vLLM",
      "api": "http://localhost:8000",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "baseURL": "http://localhost:8000/v1"
      }
    }
  }
}
```

### LM Studio 本地服务器

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "lmstudio/local",
  "provider": {
    "lmstudio": {
      "name": "LM Studio",
      "api": "http://localhost:1234",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "baseURL": "http://localhost:1234/v1"
      }
    }
  }
}
```

### 自定义 OpenAI 兼容端点

任何兼容 OpenAI API 格式的服务都可以配置：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "custom/gpt-model",
  "provider": {
    "custom": {
      "name": "自定义端点",
      "api": "https://your-custom-endpoint.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "baseURL": "https://your-custom-endpoint.com/v1",
        "apiKey": "your-api-key"
      }
    }
  }
}
```

---

## 配置验证

### 列出可用模型

```bash
# 列出所有模型
codecoder models

# 列出特定提供商的模型
codecoder models anthropic
codecoder models openai
```

### 测试连接

```bash
# 使用特定模型发送测试消息
bun dev -m anthropic/claude-sonnet-4-20250514 "你好，请介绍一下你自己"

# 使用 OpenAI 模型测试
bun dev -m openai/gpt-4o "你好，请介绍一下你自己"
```

### 查看详细日志

```bash
# 启用调试日志
export CCODE_LOG_LEVEL=debug

# 运行测试
bun dev "test"
```

### 验证 API Key

```bash
# 使用认证存储验证
codecoder auth login

# 检查当前认证
codecoder auth status
```

---

## 完整配置示例

### 多提供商配置

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "small_model": "google/gemini-2.0-flash-exp",
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    },
    "openai": {
      "options": {
        "apiKey": "{env:OPENAI_API_KEY}"
      }
    },
    "google": {
      "options": {
        "apiKey": "{env:GOOGLE_API_KEY}"
      }
    },
    "azure": {
      "options": {
        "baseURL": "https://your-resource.openai.azure.com/openai/deployments/your-deployment",
        "apiKey": "{env:AZURE_OPENAI_API_KEY}"
      }
    },
    "amazon-bedrock": {
      "options": {
        "region": "us-east-1",
        "profile": "default"
      }
    }
  }
}
```

### 使用代理的多提供商配置

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "anthropic": {
      "options": {
        "baseURL": "https://proxy.company.com/anthropic",
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    },
    "openai": {
      "options": {
        "baseURL": "https://proxy.company.com/openai/v1",
        "apiKey": "{env:OPENAI_API_KEY}"
      }
    },
    "google": {
      "options": {
        "baseURL": "https://proxy.company.com/google",
        "apiKey": "{env:GOOGLE_API_KEY}"
      }
    }
  }
}
```

---

## 常见问题

### Q: 如何切换不同的提供商？

**A**: 使用 `-m` 参数临时切换：

```bash
bun dev -m openai/gpt-4o "your message"
bun dev -m google/gemini-2.0-flash-exp "your message"
```

### Q: API Key 不工作怎么办？

**A**: 检查以下几点：

1. API Key 是否正确
2. 环境变量是否正确设置
3. 配置文件格式是否正确
4. 网络连接是否正常

```bash
# 检查环境变量
echo $ANTHROPIC_API_KEY

# 测试网络连接
curl https://api.anthropic.com/v1/messages
```

### Q: 如何使用国内 AI 服务？

**A**: 参考 [自定义提供商配置指南](./custom-provider.md)，了解如何配置 DeepSeek、月之暗面、智谱 AI 等国内服务。

### Q: 代理服务器如何配置？

**A**: 在 `provider.<id>.options.baseURL` 中设置代理地址，参考 [使用代理服务器](#使用代理服务器) 章节。

### Q: 配置文件应该放在哪里？

**A**:
- 项目级配置: `项目根目录/codecoder.json`
- 全局配置: `~/.codecoder/config.json`

---

## 参考资源

- [API Key 管理指南](./api-key-management.md)
- [自定义提供商配置](./custom-provider.md)
- [新手入门指南](./beginners-guide.md)
- [Anthropic API 文档](https://docs.anthropic.com)
- [OpenAI API 文档](https://platform.openai.com/docs)
- [Google AI 文档](https://ai.google.dev)
- [Azure OpenAI 文档](https://learn.microsoft.com/azure/ai-services/openai)
- [AWS Bedrock 文档](https://docs.aws.amazon.com/bedrock)
