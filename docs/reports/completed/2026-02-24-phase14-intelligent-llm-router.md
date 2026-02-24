# Phase 14: 智能 LLM 路由 (Intelligent LLM Router) - 完成

## 实现时间

2026-02-24

## 背景

根据 `tech-structure.md` 中枢调度层的核心能力：

> "统一 API 网关 (LLM Router)：ZeroBot 根据任务（写代码用 Claude 3.5 Sonnet，日常对话用 GPT-4o mini，机密数据用本地 Llama 3）动态路由，控制成本。"
>
> "权限控制 (RBAC)：实习生无法呼叫调用昂贵 O1 模型的 `@decision` 代理，只能使用本地开源模型，从底层卡死越权和超支。"

## 实现内容

### 文件清单

| 文件 | 功能 |
|------|------|
| `src/provider/routing-rules.ts` | 路由规则配置（任务分类、角色权限、模型定义） |
| `src/api/server/handlers/llm-router.ts` | LLM 路由核心逻辑（分类器、路由器、权限检查） |
| `src/api/server/router.ts` | 注册 15 个新 API 路由 |
| `test/unit/api/llm-router.test.ts` | 72 个单元测试 |

### 核心 API

```typescript
// 路由请求到最优模型
POST /api/v1/router/route
{ content: string, userId?: string, userRole?: UserRole, preferredModel?: string, agentName?: string }
→ RoutingDecision { modelId, modelName, provider, taskType, userRole, isFallback, reason, warnings }

// 配置管理
GET  /api/v1/router/config        // 获取配置
PUT  /api/v1/router/config        // 更新配置
GET  /api/v1/router/roles         // 列出角色权限
GET  /api/v1/router/roles/:role   // 获取角色权限
PUT  /api/v1/router/roles/:role   // 更新角色权限
GET  /api/v1/router/models        // 列出模型
PUT  /api/v1/router/models/:id    // 更新模型配置
GET  /api/v1/router/rules         // 列出分类规则
POST /api/v1/router/rules         // 添加分类规则
DELETE /api/v1/router/rules/:id   // 删除分类规则

// 监控与调试
GET  /api/v1/router/stats         // 路由统计
GET  /api/v1/router/history       // 路由历史
POST /api/v1/router/classify      // 分类测试（不执行路由）
GET  /api/v1/router/health        // 健康检查
```

### 任务分类规则

| 任务类型 | 特征 | 推荐模型 |
|---------|------|---------|
| `coding` | 代码块、编程关键词、@dev/@code agent | Claude 3.5 Sonnet |
| `analysis` | @macro/@decision agent、分析关键词 | Claude 3 Opus |
| `chat` | 简单问答、闲聊 | GPT-4o Mini / Haiku |
| `sensitive` | SSN、信用卡、API Key、密码 | 本地模型 (Ollama) |

### 角色权限 (RBAC)

| 角色 | 允许的模型层级 | 日限额 | 月限额 |
|------|--------------|--------|--------|
| admin | premium, standard, budget, local | 100M | 1B |
| developer | standard, budget, local (禁止 O1) | 10M | 100M |
| intern | budget, local (禁止 Opus, O1, GPT-4o) | 1M | 10M |
| guest | local (允许 gpt-4o-mini) | 100K | 1M |

### 路由流程

```
请求 → [任务分类器] → 任务类型
                          ↓
                    [敏感内容检测] → 强制本地模型?
                          ↓
                    [权限检查] → 用户角色允许的模型列表
                          ↓
                    [模型选择] → 最优模型 (按偏好 + 可用性)
                          ↓
                    [记录决策] → 路由历史
                          ↓
                    [返回决策] → RoutingDecision
```

### 关键设计决策

1. **优先级分类**: 敏感内容 (priority=0) > 代码 (1-2) > 分析 (1-2) > 聊天 (100)
2. **三层权限检查**: 显式拒绝 → 显式允许 → 层级检查
3. **降级策略**: 首选模型不可用时，自动选择次优模型
4. **本地优先敏感数据**: 敏感内容强制路由到本地 Ollama 模型

## 测试结果

```
72 pass
 0 fail
158 expect() calls
```

测试覆盖：
- 模式验证 (Zod schemas)
- 默认配置验证
- 辅助函数 (getTierCostRank, canRoleAccessTier, canRoleAccessModel, findBestModel)
- 任务分类准确性
- RBAC 权限检查
- 边缘情况 (空内容、超长内容、特殊字符、Unicode)

## 架构覆盖率变化

| 层级 | 之前 | 之后 |
|------|------|------|
| 中枢调度层 | 97% | 99% |

## 后续优化方向

1. **分类器增强**: 引入轻量 ML 模型提升分类准确率
2. **动态定价**: 根据实时 API 定价调整路由策略
3. **A/B 测试**: 支持模型 A/B 测试以优化选择
4. **负载均衡**: 多实例模型时的负载分配
5. **缓存**: 相似请求的路由决策缓存

## 使用示例

```typescript
// 发送代码请求
POST /api/v1/router/route
{
  "content": "Help me implement a function to sort an array",
  "userRole": "developer"
}
// → { modelId: "claude-3-5-sonnet", taskType: "coding", ... }

// 发送敏感内容
POST /api/v1/router/route
{
  "content": "My API key is sk-abcdefghijklmnopqrstuvwxyz123456",
  "userRole": "developer"
}
// → { modelId: "ollama-llama3", taskType: "sensitive", warnings: ["Sensitive content detected..."] }

// 实习生请求（被限制）
POST /api/v1/router/route
{
  "content": "Complex analysis task",
  "userRole": "intern",
  "preferredModel": "claude-3-opus"
}
// → { modelId: "claude-3-haiku", warnings: ["Preferred model ... not allowed for role intern"] }
```
