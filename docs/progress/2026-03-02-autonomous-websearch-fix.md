# Autonomous Agent WebSearch 修复

## 问题描述

Autonomous agent 无法获取实时数据（如黄金价格），即使 prompt 中明确要求使用 websearch 工具。

## 根因分析

**三层问题**:

1. **权限缺失**: `autonomous` agent 的权限配置中没有 `websearch: "allow"`
2. **实现占位符**: `web-search.ts` 的 `performSearch` 方法是模拟代码，没有真正调用 API
3. **工具过滤**: `registry.ts` 中 websearch 工具被 `CCODE_ENABLE_EXA` 环境变量控制，未启用时不会注册

## 修复内容

### 2026-03-02 15:30 - 第一轮修复

#### 1. 修复 web-search.ts (packages/ccode/src/autonomous/execution/web-search.ts)

**修改 `performSearch` 方法**:
- 从模拟代码改为真正调用 Exa MCP API (`mcp.exa.ai`)
- 添加 SSE 响应解析
- 实现结果解析（JSON 和纯文本两种格式）
- 自动检测来源类型（stackoverflow/github/documentation）

**修改 `fetchAndAnalyze` 方法**:
- 真正通过 fetch 获取 URL 内容
- 使用 Turndown 将 HTML 转为 Markdown
- 实现关键词匹配的章节提取
- 基于内容质量动态计算置信度

#### 2. 修复 agent.ts (packages/ccode/src/agent/agent.ts)

为 `autonomous` agent 添加权限:
```typescript
websearch: "allow",
webfetch: "allow",
```

### 2026-03-02 16:00 - 第二轮修复

#### 3. 修复 registry.ts (packages/ccode/src/tool/registry.ts)

**问题**: WebSearch 工具只在 `CCODE_ENABLE_EXA=true` 环境变量下注册

**修复**: 为 autonomous agent 始终启用 websearch 工具:
```typescript
if (t.id === "codesearch" || t.id === "websearch") {
  // Always enable for autonomous agent (needs real-time data)
  if (agent?.name === "autonomous") {
    return true
  }
  return model.providerID === "ccode" || Flag.CCODE_ENABLE_EXA
}
```

## 验证状态

- [x] TypeScript 类型检查通过
- [ ] 待验证：重新启动 autonomous agent 测试实时搜索功能

## 技术细节

### Exa MCP API 配置

```typescript
const EXA_API_CONFIG = {
  BASE_URL: "https://mcp.exa.ai",
  ENDPOINTS: { SEARCH: "/mcp" },
  DEFAULT_NUM_RESULTS: 5,
  TIMEOUT_MS: 25000,
}
```

### 新增类型定义

- `ExaMcpSearchRequest` - API 请求格式
- `ExaMcpSearchResponse` - API 响应格式
- `ExaSearchResultItem` - 搜索结果项

### 依赖

- `turndown` - HTML 转 Markdown
