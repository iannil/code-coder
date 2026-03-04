# LLM Routing Configuration

配置智能 LLM 路由规则，包括任务分类、模型选择和角色权限控制。

## 配置文件位置

- **默认配置**：`packages/ccode/src/config/routing.default.json`
- **用户配置**：`~/.codecoder/routing.json`

用户配置会覆盖默认配置中的同名字段，允许选择性自定义。

## 配置结构

```json
{
  "$schema": "./schemas/routing.schema.json",
  "version": "1.0.0",
  "enabled": true,
  "defaultModelId": "claude-3-5-sonnet",
  "defaultRole": "guest",
  "enableDlpIntegration": true,
  "forceLocalForSensitive": true,
  "rules": [...],
  "models": [...],
  "rolePermissions": [...],
  "taskModelPreferences": {...}
}
```

### 全局设置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | `boolean` | `true` | 是否启用智能路由 |
| `defaultModelId` | `string` | `claude-3-5-sonnet` | 未分类任务的默认模型 |
| `defaultRole` | `string` | `guest` | 未知用户的默认角色 |
| `enableDlpIntegration` | `boolean` | `true` | 启用 DLP 敏感内容检测 |
| `forceLocalForSensitive` | `boolean` | `true` | 敏感内容强制使用本地模型 |

## 任务分类规则 (rules)

分类规则用于检测消息的任务类型，从而选择最优模型。

```json
{
  "rules": [
    {
      "id": "rule-coding-keywords",
      "taskType": "coding",
      "priority": 2,
      "patterns": ["\\bfunction\\s+\\w+"],
      "keywords": ["code", "implement", "debug"],
      "agents": ["@dev", "@code"],
      "enabled": true
    }
  ]
}
```

### 规则字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `string` | 规则唯一标识 |
| `taskType` | `enum` | 任务类型：`coding`/`analysis`/`chat`/`sensitive` |
| `priority` | `integer` | 优先级（数字越小优先级越高） |
| `patterns` | `array` | 正则表达式模式列表 |
| `keywords` | `array` | 关键词列表（不区分大小写） |
| `agents` | `array` | 触发该任务类型的 Agent 名称 |
| `enabled` | `boolean` | 是否启用此规则 |

### 任务类型

| 类型 | 说明 | 推荐模型 |
|------|------|---------|
| `coding` | 代码相关任务 | claude-3-5-sonnet, gpt-4o |
| `analysis` | 深度分析任务 | claude-3-opus, o1 |
| `chat` | 简单对话 | gpt-4o-mini, claude-3-haiku |
| `sensitive` | 敏感内容 | 本地模型 (ollama-*) |

## 模型定义 (models)

定义可用的 LLM 模型及其属性。

```json
{
  "models": [
    {
      "id": "claude-3-5-sonnet",
      "name": "Claude 3.5 Sonnet",
      "provider": "anthropic",
      "tier": "standard",
      "optimizedFor": ["coding"],
      "costPer1M": 3,
      "available": true,
      "isLocal": false
    }
  ]
}
```

### 模型字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `string` | 模型唯一标识 |
| `name` | `string` | 显示名称 |
| `provider` | `string` | 提供商：`anthropic`/`openai`/`google`/`ollama` |
| `tier` | `enum` | 模型层级：`premium`/`standard`/`budget`/`local` |
| `optimizedFor` | `array` | 该模型优化的任务类型 |
| `costPer1M` | `number` | 每百万 token 成本（美元） |
| `available` | `boolean` | 是否可用 |
| `isLocal` | `boolean` | 是否为本地模型 |

### 模型层级

| 层级 | 说明 | 示例 |
|------|------|------|
| `premium` | 最强性能，高成本 | claude-3-opus, o1 |
| `standard` | 均衡性能 | claude-3-5-sonnet, gpt-4o |
| `budget` | 经济实惠 | gpt-4o-mini, claude-3-haiku |
| `local` | 本地运行，零成本 | ollama-llama3, ollama-codellama |

## 角色权限 (rolePermissions)

基于角色的访问控制（RBAC），限制不同角色可使用的模型。

```json
{
  "rolePermissions": [
    {
      "role": "developer",
      "allowedTiers": ["standard", "budget", "local"],
      "allowedModels": [],
      "deniedModels": ["o1"],
      "dailyTokenLimit": 10000000,
      "monthlyTokenLimit": 100000000
    }
  ]
}
```

### 权限字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `role` | `enum` | 角色：`admin`/`developer`/`intern`/`guest` |
| `allowedTiers` | `array` | 允许的模型层级 |
| `allowedModels` | `array` | 额外允许的特定模型（覆盖层级限制） |
| `deniedModels` | `array` | 拒绝的特定模型（覆盖层级允许） |
| `dailyTokenLimit` | `integer` | 每日 token 配额 |
| `monthlyTokenLimit` | `integer` | 每月 token 配额 |

### 默认角色权限

| 角色 | 允许层级 | 每日配额 | 每月配额 |
|------|---------|---------|---------|
| `admin` | 全部 | 100M | 1B |
| `developer` | standard, budget, local | 10M | 100M |
| `intern` | budget, local | 1M | 10M |
| `guest` | local + gpt-4o-mini | 100K | 1M |

## 任务-模型偏好 (taskModelPreferences)

指定每种任务类型的首选模型（按优先级排序）。

```json
{
  "taskModelPreferences": {
    "coding": ["claude-3-5-sonnet", "gpt-4o", "ollama-codellama"],
    "analysis": ["claude-3-opus", "o1", "gpt-4o"],
    "chat": ["gpt-4o-mini", "claude-3-haiku", "ollama-llama3"],
    "sensitive": ["ollama-llama3", "ollama-codellama"]
  }
}
```

路由器会按顺序尝试列表中的模型，选择第一个用户有权限访问且可用的模型。

## 使用示例

### 添加自定义模型

创建 `~/.codecoder/routing.json`：

```json
{
  "models": [
    {
      "id": "deepseek-v3",
      "name": "DeepSeek V3",
      "provider": "deepseek",
      "tier": "standard",
      "optimizedFor": ["coding"],
      "costPer1M": 0.5,
      "available": true,
      "isLocal": false
    }
  ],
  "taskModelPreferences": {
    "coding": ["deepseek-v3", "claude-3-5-sonnet", "gpt-4o"]
  }
}
```

### 修改角色权限

允许 intern 使用 gpt-4o：

```json
{
  "rolePermissions": [
    {
      "role": "intern",
      "allowedTiers": ["budget", "local"],
      "allowedModels": ["gpt-4o"],
      "deniedModels": [],
      "dailyTokenLimit": 2000000,
      "monthlyTokenLimit": 20000000
    }
  ]
}
```

### 添加自定义分类规则

为特定领域添加分类规则：

```json
{
  "rules": [
    {
      "id": "rule-financial-analysis",
      "taskType": "analysis",
      "priority": 1,
      "patterns": [],
      "keywords": ["stock", "market", "trading", "investment"],
      "agents": ["@trader", "@picker"],
      "enabled": true
    }
  ]
}
```

### 禁用敏感内容检测

```json
{
  "enableDlpIntegration": false,
  "forceLocalForSensitive": false
}
```

**警告**：禁用 DLP 可能导致敏感数据（如 API 密钥）被发送到云端模型。

## TypeScript API

```typescript
import {
  getRoutingConfig,
  getRoutingConfigSync,
  reloadRoutingConfig,
  findBestModel,
  type RoutingConfig
} from "@/provider/routing-rules"

// 异步加载配置（首次调用）
const config = await getRoutingConfig()

// 同步获取缓存的配置
const syncConfig = getRoutingConfigSync()

// 重新加载配置
const reloaded = await reloadRoutingConfig()

// 为任务找到最佳模型
const model = findBestModel(
  "coding",           // 任务类型
  "developer",        // 用户角色
  config.models,      // 可用模型
  config.rolePermissions  // 角色权限
)
```

## 路由决策流程

```
┌─────────────────────┐
│   接收用户消息      │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ 1. 任务类型分类     │
│   - 检查 Agent 名   │
│   - 匹配正则模式    │
│   - 匹配关键词      │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ 2. DLP 敏感检测     │
│   - SSN 模式        │
│   - 信用卡模式      │
│   - API 密钥模式    │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ 3. 模型选择         │
│   - 检查用户权限    │
│   - 按偏好排序      │
│   - 选择首个可用    │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ 4. 返回路由决策     │
│   - 模型 ID         │
│   - 任务类型        │
│   - 决策原因        │
└─────────────────────┘
```

## Schema 验证

配置文件支持 JSON Schema 验证。在 VS Code 中添加 `$schema` 字段可获得自动补全和错误提示：

```json
{
  "$schema": "./schemas/routing.schema.json"
}
```

## 常见问题

### Q: 为什么选择了非预期的模型？

1. 检查用户角色是否有权限访问预期模型
2. 确认模型的 `available` 字段为 `true`
3. 检查 `taskModelPreferences` 中的优先级顺序

### Q: 如何查看路由决策历史？

通过 API 端点：
```bash
curl http://localhost:4400/api/v1/router/history?limit=10
```

### Q: 配置修改后需要重启吗？

是的，配置在启动时加载并缓存。修改后需要重启 CodeCoder，或调用 `reloadRoutingConfig()` 重新加载。

### Q: 如何测试分类规则？

使用分类 API：
```bash
curl -X POST http://localhost:4400/api/v1/router/classify \
  -H "Content-Type: application/json" \
  -d '{"content": "帮我实现一个排序算法", "agentName": "@dev"}'
```
