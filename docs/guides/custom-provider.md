# 设置自定义 AI 提供商指南

CodeCoder 支持通过配置文件设置自定义 AI 提供商。本指南将详细介绍如何配置各种提供商。

## 目录

1. [配置文件位置](#配置文件位置)
2. [配置文件结构](#配置文件结构)
3. [主流提供商配置](#主流提供商配置)
4. [使用 OpenAI-Compatible API](#使用-openai-compatible-api)
5. [配置示例](#配置示例)
6. [高级选项](#高级选项)

---

## 配置文件位置

CodeCoder 按以下优先级查找配置文件：

1. **项目配置** - 项目根目录下的 `codecoder.json` 或 `codecoder.jsonc`
2. **全局配置** - `~/.codecoder/config.json`
3. **环境变量** - 通过 `CCODE_CONFIG` 指定

---

## 配置文件结构

### 基本模板

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "provider-id/model-id",
  "provider": {
    "provider-id": {
      "name": "显示名称",
      "api": "https://api.example.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "apiKey": "your-api-key"
      }
    }
  }
}
```

---

## 主流提供商配置

对于 Anthropic、OpenAI、Google、Azure、Amazon Bedrock 等主流提供商，CodeCoder 提供了完整的自定义配置支持。

### 快速链接

| 提供商 | 配置说明 |
|--------|----------|
| **Anthropic (Claude)** | 自定义端点、API Key 配置 |
| **OpenAI (GPT)** | 自定义端点、API Key 配置 |
| **Google (Gemini)** | API Key 配置 |
| **Azure OpenAI** | 自定义资源配置 |
| **Amazon Bedrock** | 自定义区域和端点配置 |

详细配置方法请参阅 **[主流提供商配置指南](./mainstream-providers.md)**。

### API Key 管理

关于如何安全地存储和管理 API Key，请参阅 **[API Key 管理指南](./api-key-management.md)**。

---

## 使用 OpenAI-Compatible API

对于任何兼容 OpenAI API 格式的提供商（如国内的各种 AI 服务），使用 `@ai-sdk/openai-compatible`。

### 示例：配置国内 AI 服务

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "deepseek/chat",
  "provider": {
    "deepseek": {
      "name": "DeepSeek",
      "api": "https://api.deepseek.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "apiKey": "sk-your-deepseek-api-key"
      },
      "models": {
        "chat": {
          "id": "deepseek-chat",
          "name": "DeepSeek Chat",
          "capabilities": {
            "temperature": true,
            "toolcall": true,
            "attachment": false
          },
          "cost": {
            "input": 0.001,
            "output": 0.002
          },
          "limit": {
            "context": 128000,
            "output": 8192
          }
        }
      }
    }
  }
}
```

### 示例：配置月之暗面 (Moonshot)

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "moonshot/chat",
  "provider": {
    "moonshot": {
      "name": "Moonshot AI",
      "api": "https://api.moonshot.cn/v1",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["MOONSHOT_API_KEY"],
      "options": {
        "baseURL": "https://api.moonshot.cn/v1"
      },
      "models": {
        "chat": {
          "id": "moonshot-v1-8k",
          "name": "Moonshot V1 8K",
          "capabilities": {
            "temperature": true,
            "toolcall": true
          },
          "cost": {
            "input": 0.012,
            "output": 0.012
          },
          "limit": {
            "context": 8192,
            "output": 4096
          }
        }
      }
    }
  }
}
```

### 示例：配置智谱 AI (GLM)

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "zhipu/chat",
  "provider": {
    "zhipu": {
      "name": "智谱 AI",
      "api": "https://open.bigmodel.cn/api/paas/v4",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["ZHIPU_API_KEY"],
      "options": {
        "baseURL": "https://open.bigmodel.cn/api/paas/v4"
      },
      "models": {
        "chat": {
          "id": "glm-4-flash",
          "name": "GLM-4 Flash",
          "capabilities": {
            "temperature": true,
            "toolcall": true
          },
          "cost": {
            "input": 0.0001,
            "output": 0.0001
          },
          "limit": {
            "context": 128000,
            "output": 4096
          }
        }
      }
    }
  }
}
```

### 示例：配置阿里云通义千问

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "qwen/chat",
  "provider": {
    "qwen": {
      "name": "阿里云通义千问",
      "api": "https://dashscope.aliyuncs.com/compatible-mode/v1",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["DASHSCOPE_API_KEY"],
      "options": {
        "baseURL": "https://dashscope.aliyuncs.com/compatible-mode/v1"
      },
      "models": {
        "chat": {
          "id": "qwen-turbo",
          "name": "qwen-turbo",
          "capabilities": {
            "temperature": true,
            "toolcall": true
          },
          "cost": {
            "input": 0.0003,
            "output": 0.0006
          },
          "limit": {
            "context": 8192,
            "output": 2048
          }
        }
      }
    }
  }
}
```

### 示例：配置百度文心一言

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "ernie/chat",
  "provider": {
    "ernie": {
      "name": "百度文心一言",
      "api": "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["BAIDU_API_KEY", "BAIDU_SECRET_KEY"],
      "options": {
        "baseURL": "https://aip.baidubce.com/rpc/2.0/ai_custom/v1/wenxinworkshop"
      },
      "models": {
        "chat": {
          "id": "ernie-speed-128k",
          "name": "ERNIE Speed 128K",
          "capabilities": {
            "temperature": true,
            "toolcall": false
          },
          "limit": {
            "context": 128000,
            "output": 4096
          }
        }
      }
    }
  }
}
```

### 示例：配置零一万物 (01.AI)

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "yi/chat",
  "provider": {
    "yi": {
      "name": "零一万物",
      "api": "https://api.lingyiwanwu.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["YI_API_KEY"],
      "options": {
        "baseURL": "https://api.lingyiwanwu.com/v1"
      },
      "models": {
        "chat": {
          "id": "yi-lightning",
          "name": "Yi Lightning",
          "capabilities": {
            "temperature": true,
            "toolcall": true
          },
          "cost": {
            "input": 0.0001,
            "output": 0.0001
          },
          "limit": {
            "context": 16000,
            "output": 4096
          }
        }
      }
    }
  }
}
```

### 示例：配置 MiniMax

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "minimax/chat",
  "provider": {
    "minimax": {
      "name": "MiniMax",
      "api": "https://api.minimax.chat/v1",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["MINIMAX_API_KEY"],
      "options": {
        "baseURL": "https://api.minimax.chat/v1"
      },
      "models": {
        "chat": {
          "id": "abab6.5s-chat",
          "name": "ABAB6.5s-chat",
          "capabilities": {
            "temperature": true,
            "toolcall": true
          },
          "limit": {
            "context": 245760,
            "output": 4096
          }
        }
      }
    }
  }
}
```

---

## 配置示例

### 使用环境变量存储 API Key

为了避免在配置文件中硬编码 API 密钥，可以使用环境变量：

```json
{
  "$schema": "https://code-coder.com/config.json",
  "provider": {
    "my-provider": {
      "name": "我的自定义提供商",
      "api": "https://api.example.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["MY_API_KEY"],
      "options": {
        "baseURL": "https://api.example.com/v1"
      }
    }
  }
}
```

然后设置环境变量：
```bash
export MY_API_KEY="your-api-key-here"
```

### 配置多个模型

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "my-provider/gpt-4",
  "provider": {
    "my-provider": {
      "name": "我的提供商",
      "api": "https://api.example.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "apiKey": "sk-xxx"
      },
      "models": {
        "gpt-3.5": {
          "id": "gpt-3.5-turbo",
          "name": "GPT-3.5 Turbo",
          "cost": { "input": 0.0005, "output": 0.0015 }
        },
        "gpt-4": {
          "id": "gpt-4",
          "name": "GPT-4",
          "cost": { "input": 0.003, "output": 0.006 }
        },
        "gpt-4-turbo": {
          "id": "gpt-4-turbo",
          "name": "GPT-4 Turbo",
          "cost": { "input": 0.001, "output": 0.003 }
        }
      }
    }
  }
}
```

---

## 高级选项

### 模型白名单/黑名单

```json
{
  "provider": {
    "my-provider": {
      "whitelist": ["gpt-4", "gpt-4-turbo"],
      "blacklist": ["gpt-3.5"]
    }
  }
}
```

### 自定义请求头

```json
{
  "provider": {
    "my-provider": {
      "options": {
        "headers": {
          "X-Custom-Header": "value",
          "Authorization": "Bearer token"
        }
      }
    }
  }
}
```

### 配置代理

```json
{
  "provider": {
    "my-provider": {
      "options": {
        "baseURL": "http://localhost:8080/v1"
      }
    }
  }
}
```

### 禁用特定提供商

```json
{
  "disabled_providers": ["openai", "anthropic"]
}
```

### 只启用特定提供商

```json
{
  "enabled_providers": ["my-provider", "another-provider"]
}
```

---

## 完整配置示例

### 项目级配置 (`codecoder.json`)

```json
{
  "$schema": "https://code-coder.com/config.json",
  "model": "deepseek/chat",
  "small_model": "deepseek/chat",
  "disabled_providers": ["openai"],
  "provider": {
    "deepseek": {
      "name": "DeepSeek",
      "api": "https://api.deepseek.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "env": ["DEEPSEEK_API_KEY"],
      "options": {
        "baseURL": "https://api.deepseek.com/v1"
      },
      "models": {
        "chat": {
          "id": "deepseek-chat",
          "name": "DeepSeek Chat",
          "capabilities": {
            "temperature": true,
            "toolcall": true,
            "attachment": false,
            "input": { "text": true },
            "output": { "text": true }
          },
          "cost": {
            "input": 0.001,
            "output": 0.002,
            "cache": { "read": 0, "write": 0 }
          },
          "limit": {
            "context": 128000,
            "output": 8192
          }
        },
        "coder": {
          "id": "deepseek-coder",
          "name": "DeepSeek Coder",
          "capabilities": {
            "temperature": true,
            "toolcall": true,
            "attachment": false,
            "input": { "text": true },
            "output": { "text": true }
          },
          "cost": {
            "input": 0.001,
            "output": 0.002,
            "cache": { "read": 0, "write": 0 }
          },
          "limit": {
            "context": 128000,
            "output": 8192
          }
        }
      }
    }
  }
}
```

---

## 验证配置

配置完成后，可以验证提供商是否正确设置：

```bash
# 列出所有可用模型
codecoder models

# 列出特定提供商的模型
codecoder models deepseek

# 测试对话
bun dev "你好，请介绍一下你自己"
```

---

## 常见问题

### Q: API Key 应该放在哪里？

**A**: 推荐使用环境变量，避免将密钥硬编码在配置文件中：

```json
{
  "provider": {
    "my-provider": {
      "env": ["MY_API_KEY"]
    }
  }
}
```

### Q: 如何使用多个提供商？

**A**: 在 `provider` 字段中配置多个提供商：

```json
{
  "provider": {
    "provider-1": { ... },
    "provider-2": { ... },
    "provider-3": { ... }
  }
}
```

### Q: 模型 ID 不匹配怎么办？

**A**: 确保配置中的 `id` 与 API 返回的模型 ID 完全一致。可以使用 `models` 字段映射 ID：

```json
{
  "models": {
    "my-model": {
      "id": "actual-api-model-id"
    }
  }
}
```

### Q: 如何调试配置问题？

**A**: 查看日志：

```bash
# 设置详细日志
export CCODE_LOG_LEVEL=debug
bun dev "test"
```

---

## 参考资源

- [主流提供商配置指南](./mainstream-providers.md) - Anthropic、OpenAI、Google、Azure、Bedrock 配置
- [API Key 管理指南](./api-key-management.md) - 安全存储和管理 API Key
- [新手入门指南](./beginners-guide.md)
- [AI SDK 包列表](https://www.npmjs.com/org/ai-sdk)
- [OpenAI API 文档](https://platform.openai.com/docs/api-reference)
- [配置架构文档](../Architecture-Guide.md)
