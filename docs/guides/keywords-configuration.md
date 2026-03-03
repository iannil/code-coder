# Agent Keywords Configuration

配置 Agent 的触发关键词和路由别名，实现自定义的消息路由规则。

## 配置文件位置

- **默认配置**：`packages/ccode/src/agent/keywords.default.json`
- **用户配置**：`~/.codecoder/keywords.json`

用户配置会覆盖默认配置中的同名字段，允许选择性自定义。

## 配置结构

```json
{
  "$schema": "./schemas/keywords.schema.json",
  "version": "1.0.0",
  "agents": {
    "macro": {
      "triggers": ["macro", "GDP", "economy"],
      "aliases": ["macro", "宏观", "经济"],
      "priority": 6,
      "enabled": true
    }
  },
  "defaults": {
    "agent": "general",
    "cli_agent": "build",
    "im_agent": "autonomous",
    "use_implicit_matching": false
  }
}
```

### 字段说明

#### agents

每个 Agent 的关键词配置：

| 字段 | 类型 | 说明 |
|------|------|------|
| `triggers` | `array` | 触发关键词列表，支持字符串或高级规则对象 |
| `aliases` | `array` | `@mention` 路由时的别名（如 `@宏观`） |
| `priority` | `integer` | 优先级（1-10），用于冲突解决 |
| `enabled` | `boolean` | 是否启用该 Agent 的关键词 |

#### triggers

触发规则支持两种格式：

**简单字符串**：
```json
"triggers": ["macro", "GDP", "economy"]
```

**高级规则对象**：
```json
"triggers": [
  { "type": "keyword", "value": "macro", "priority": 10 },
  { "type": "pattern", "value": "GDP|GNP|CPI", "priority": 8 },
  { "type": "context", "value": "auth|payment|credential", "priority": 7 }
]
```

规则类型：

| 类型 | 说明 |
|------|------|
| `keyword` | 包含匹配（不区分大小写） |
| `pattern` | 正则表达式匹配 |
| `context` | 语义上下文匹配（正则） |
| `event` | 系统事件触发（如 `pr.opened`） |

#### defaults

默认路由设置：

| 字段 | 说明 |
|------|------|
| `agent` | 无匹配时的默认 Agent |
| `cli_agent` | CLI 渠道的默认 Agent |
| `im_agent` | IM 渠道（Telegram、Discord 等）的默认 Agent |
| `use_implicit_matching` | 是否启用隐式关键词匹配 |

## 路由优先级

消息路由按以下顺序尝试匹配：

1. **@mention 别名** - 最高优先级
   - 示例：`@macro 分析PMI` → macro agent
   - 示例：`@宏观 分析PMI` → macro agent

2. **隐式关键词匹配** - 仅当 `use_implicit_matching: true`
   - 示例：`分析GDP数据` → macro agent（如果启用）
   - 默认禁用以避免误路由

3. **默认 Agent** - 根据渠道类型
   - CLI：`build`
   - IM：`autonomous`

## 使用示例

### 添加自定义别名

创建 `~/.codecoder/keywords.json`：

```json
{
  "agents": {
    "macro": {
      "aliases": ["macro", "宏观", "经济", "财经", "finance"]
    }
  }
}
```

现在可以使用 `@财经` 或 `@finance` 触发 macro agent。

### 添加新的触发关键词

```json
{
  "agents": {
    "trader": {
      "triggers": [
        "trade",
        "trading",
        "技术分析",
        { "type": "pattern", "value": "BTC|ETH|股票", "priority": 8 }
      ]
    }
  }
}
```

### 禁用特定 Agent 的关键词

```json
{
  "agents": {
    "explore": {
      "enabled": false
    }
  }
}
```

### 启用隐式匹配

```json
{
  "defaults": {
    "use_implicit_matching": true
  }
}
```

**警告**：启用隐式匹配可能导致意外路由。例如，"分析黄金市场" 可能同时匹配 `trader` 和 `picker`。

## 双语支持

关键词配置支持中英文混合：

```json
{
  "agents": {
    "decision": {
      "triggers": ["decision", "CLOSE", "选择", "决策"],
      "aliases": ["decision", "决策", "close"]
    }
  }
}
```

用户可以使用 `@decision`、`@决策` 或 `@close` 触发同一个 Agent。

## TypeScript API

```typescript
import {
  getKeywords,
  detectAlias,
  detectAgent,
  type KeywordsConfig
} from "@/config/keywords"

// 加载配置
const config = await getKeywords()

// 检测 @mention 别名
const agent = detectAlias("@macro 分析PMI", config)
// => "macro"

// 完整检测（别名 → 触发 → 默认）
const resolved = detectAgent("@macro 分析PMI", "build", config)
// => "macro"
```

## Rust API

```rust
use zero_common::keywords::{keywords, detect_alias, detect_agent};

// 获取配置
let config = keywords();

// 检测 @mention 别名
if let Some(agent) = detect_alias("@macro 分析PMI", config) {
    println!("Routing to: {}", agent);
}

// 完整检测
let agent = detect_agent("@macro 分析PMI", "build", config);
```

## Schema 验证

配置文件支持 JSON Schema 验证。在 VS Code 中添加 `$schema` 字段可获得自动补全和错误提示：

```json
{
  "$schema": "./schemas/keywords.schema.json"
}
```

## 常见问题

### Q: 为什么消息没有路由到预期的 Agent？

1. 检查是否使用了正确的 `@mention` 格式
2. 确认别名在配置中已定义
3. 检查 `enabled` 是否为 `true`

### Q: 如何查看当前加载的配置？

在 CLI 中使用：
```bash
ccode config show keywords
```

### Q: 配置修改后需要重启吗？

是的，配置在启动时加载并缓存。修改后需要重启 CodeCoder。
