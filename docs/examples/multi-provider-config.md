# CodeCoder 多提供商配置指南

本指南展示如何同时配置自定义 OpenAI 兼容接口和自定义 Claude 兼容接口。

## 配置文件位置

CodeCoder 按以下优先级加载配置（从低到高）：

1. **远程配置** - `https://your-domain/.well-known/codecoder`
2. **全局配置** - `~/.codecoder/config.json` 或 `~/.codecoder/codecoder.jsonc`
3. **项目配置** - `<project>/.codecoder/config.json`
4. **环境变量** - 可通过 `CCODE_CONFIG_CONTENT` 内联配置

## 配置示例

### 完整配置文件 (~/.codecoder/config.json)

```json
{
  "$schema": "https://codecoder.ai/config.json",

  // ========== 提供商配置 ==========
  "provider": {
    // 自定义 OpenAI 兼容接口（如 DeepSeek、Qwen、通义千问等）
    "openai-compatible": {
      "options": {
        "baseURL": "https://api.deepseek.com/v1",
        "apiKey": "sk-your-deepseek-api-key"
      },
      "models": {
        "deepseek-chat": {
          "id": "deepseek-chat",
          "name": "DeepSeek Chat",
          "context": 128000,
          "cost": {
            "input": 0.00014,
            "output": 0.00028
          }
        },
        "deepseek-coder": {
          "id": "deepseek-coder",
          "name": "DeepSeek Coder",
          "context": 128000,
          "cost": {
            "input": 0.00014,
            "output": 0.00028
          }
        }
      }
    },

    // 自定义 Claude 兼容接口（如阿里云百炼、火山引擎等）
    "anthropic": {
      "options": {
        "baseURL": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "apiKey": "sk-your-dashscope-api-key"
      },
      "models": {
        "qwen-max": {
          "id": "qwen-max",
          "name": "Qwen Max",
          "context": 32000,
          "cost": {
            "input": 0.00012,
            "output": 0.00012
          }
        },
        "qwen-coder-plus": {
          "id": "qwen-coder-plus",
          "name": "Qwen Coder Plus",
          "context": 32000,
          "cost": {
            "input": 0.00012,
            "output": 0.00012
          }
        }
      }
    },

    // GitHub Copilot（可选）
    "github-copilot": {
      "options": {
        "enterpriseUrl": "https://github.your-company.com"
      }
    }
  },

  // ========== 默认模型选择 ==========
  "model": "openai-compatible/deepseek-chat",
  "small_model": "anthropic/qwen-coder-plus",

  // ========== Agent 配置 ==========
  "agent": {
    "plan": {
      "model": "anthropic/qwen-max",
      "temperature": 0.7
    },
    "build": {
      "model": "openai-compatible/deepseek-coder",
      "temperature": 0.3
    },
    "general": {
      "model": "openai-compatible/deepseek-chat"
    }
  },

  // ========== Crazy Mode 配置（可选）==========
  "crazyMode": {
    "enabled": true,
    "autonomyLevel": "crazy",
    "unattended": false,
    "resourceLimits": {
      "maxTokens": 1000000,
      "maxCostUSD": 10.0,
      "maxDurationMinutes": 30,
      "maxFilesChanged": 50,
      "maxActions": 100
    }
  },

  // ========== 其他配置 ==========
  "theme": "github-dark",
  "logLevel": "info",
  "username": "Developer"
}
```

### 使用环境变量的配置示例

```bash
# OpenAI 兼容接口（如 DeepSeek）
export OPENAI_COMPATIBLE_API_KEY="sk-your-deepseek-api-key"
export OPENAI_COMPATIBLE_BASE_URL="https://api.deepseek.com/v1"

# Claude 兼容接口（如阿里云百炼）
export ANTHROPIC_API_KEY="sk-your-dashscope-api-key"
export ANTHROPIC_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"

# GitHub Copilot（可选）
export GITHUB_TOKEN="ghp-your-github-token"

# 启动 CodeCoder
bun dev
```

## 具体配置步骤

### 步骤 1：获取 API 密钥

#### DeepSeek（OpenAI 兼容）

1. 访问 https://platform.deepseek.com/
2. 注册/登录账号
3. 进入 API Keys 页面
4. 创建新密钥，保存为 `sk-xxx`

#### 阿里云百炼（Claude 兼容）

1. 访问 https://dashscope.aliyuncs.com/
2. 注册/登录账号
3. 进入 API-KEY 管理页面
4. 创建 API-KEY，选择 `dashscope-compatible` 模式

#### 火山引擎 ByteDance（OpenAI 兼容）

1. 访问 https://console.volcengine.com/
2. 创建 API Key
3. 使用 `https://ark.cn/v1/v1` 作为 base URL

#### 智谱 AI / ChatGLM（OpenAI 兼容）

1. 访问 https://open.bigmodel.cn/
2. 获取 API 密钥
3. 使用 `https://open.bigmodel.cn/api/paas/v4` 作为 base URL

### 步骤 2：创建配置文件

```bash
# 创建全局配置目录
mkdir -p ~/.codecoder

# 创建配置文件
cat > ~/.codecoder/config.json << 'EOF'
{
  "$schema": "https://codecoder.ai/config.json",
  "provider": {
    "openai-compatible": {
      "options": {
        "baseURL": "https://api.deepseek.com/v1",
        "apiKey": "sk-your-deepseek-api-key"
      },
      "models": {
        "deepseek-chat": {
          "id": "deepseek-chat",
          "name": "DeepSeek Chat",
          "context": 128000
        }
      }
    },
    "anthropic": {
      "options": {
        "baseURL": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "apiKey": "sk-your-dashscope-api-key"
      },
      "models": {
        "qwen-max": {
          "id": "qwen-max",
          "name": "Qwen Max",
          "context": 32000
        }
      }
    }
  },
  "model": "openai-compatible/deepseek-chat"
}
EOF
```

### 步骤 3：验证配置

```bash
# 启动 CodeCoder 并验证配置
cd /path/to/your/project
bun dev

# 在 CodeCoder 中按 Ctrl+A 打开模型列表
# 确认自定义模型已正确加载
```

## 常用自定义提供商配置

### DeepSeek

```json
{
  "provider": {
    "openai-compatible": {
      "options": {
        "baseURL": "https://api.deepseek.com"
      },
      "models": {
        "deepseek-chat": {
          "id": "deepseek-chat",
          "name": "DeepSeek V3",
          "context": 128000
        },
        "deepseek-coder": {
          "id": "deepseek-coder",
          "name": "DeepSeek Coder V2",
          "context": 128000
        }
      }
    }
  }
}
```

### 阿里云百炼（Qwen）

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "baseURL": "https://dashscope.aliyuncs.com/compatible-mode/v1"
      },
      "models": {
        "qwen-turbo": {
          "id": "qwen-turbo",
          "name": "Qwen Turbo",
          "context": 8000
        },
        "qwen-plus": {
          "id": "qwen-plus",
          "name": "Qwen Plus",
          "context": 32000
        },
        "qwen-max": {
          "id": "qwen-max",
          "name": "Qwen Max",
          "context": 32000
        }
      }
    }
  }
}
```

### 火山引擎

```json
{
  "provider": {
    "openai-compatible": {
      "options": {
        "baseURL": "https://ark.cn/v1"
      },
      "models": {
        "doubao-pro": {
          "id": "ep-20241212121234-abcde",
          "name": "Doubao Pro",
          "context": 32000
        }
      }
    }
  }
}
```

### 智谱 AI（ChatGLM）

```json
{
  "provider": {
    "openai-compatible": {
      "options": {
        "baseURL": "https://open.bigmodel.cn/api/paas/v4"
      },
      "models": {
        "glm-4": {
          "id": "glm-4",
          "name": "GLM-4",
          "context": 128000
        },
        "glm-4-plus": {
          "id": "glm-4-plus",
          "name": "GLM-4 Plus",
          "context": 128000
        }
      }
    }
  }
}
```

### Moonshot（Kimi）

```json
{
  "provider": {
    "openai-compatible": {
      "options": {
        "baseURL": "https://api.moonshot.cn/v1"
      },
      "models": {
        "moonshot-v1-8k": {
          "id": "moonshot-v1-8k",
          "name": "Kimi 8k",
          "context": 32000
        },
        "moonshot-v1-32k": {
          "id": "moonshot-v1-32k",
          "name": "Kimi 32k",
          "context": 128000
        }
      }
    }
  }
}
```

## 高级配置

### 为不同 Agent 分配不同模型

```json
{
  "agent": {
    "plan": {
      "model": "anthropic/qwen-max",
      "description": "使用长上下文模型进行规划"
    },
    "build": {
      "model": "openai-compatible/deepseek-coder",
      "description": "使用代码优化模型进行开发"
    },
    "general": {
      "model": "openai-compatible/deepseek-chat",
      "description": "使用通用模型进行日常任务"
    },
    "explore": {
      "model": "anthropic/qwen-turbo",
      "description": "使用快速模型探索代码库"
    }
  }
}
```

### 配置模型变体

```json
{
  "provider": {
    "openai-compatible": {
      "models": {
        "deepseek-chat": {
          "id": "deepseek-chat",
          "variants": {
            "low": {
              "id": "deepseek-chat-low",
              "temperature": 0.1,
              "topP": 0.1
            },
            "high": {
              "id": "deepseek-chat-high",
              "temperature": 0.8,
              "topP": 0.95
            }
          }
        }
      }
    }
  }
}
```

### 配置超时和重试

```json
{
  "provider": {
    "openai-compatible": {
      "options": {
        "baseURL": "https://api.deepseek.com/v1",
        "apiKey": "sk-your-key",
        "timeout": 120000
      }
    },
    "anthropic": {
      "options": {
        "baseURL": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "apiKey": "sk-your-key",
        "timeout": 180000
      }
    }
  },
  "experimental": {
    "chatMaxRetries": 3
  }
}
```

## 故障排查

### 模型未显示在列表中

1. 检查配置文件语法是否正确
2. 确认 API 密钥已设置
3. 查看日志：`"logLevel": "debug"`
4. 验证 base URL 格式（不应包含尾随斜杠）

### API 调用失败

1. 确认网络连接到提供商服务器
2. 验证 API 密钥有效性
3. 检查是否需要添加请求头
4. 确认模型 ID 与提供商文档一致

### 切换模型

在 CodeCoder TUI 中：
- 按 `Ctrl+A` 打开模型选择器
- 使用 `/` 命令切换：`/model openai-compatible/deepseek-chat`

## 配置文件参考

完整配置 Schema 参考：https://codecoder.ai/config.json
