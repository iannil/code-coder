# TypeScript → Rust 迁移 Wave 4 评估报告

**日期**: 2026-03-05
**状态**: 部分完成

---

## 执行摘要

Wave 4 迁移计划过于乐观。经过详细代码分析，发现大部分计划中的迁移需要**先扩展 Rust 实现**才能进行。

### 已完成

| Phase | 模块 | 删除 TS 行数 | 状态 |
|-------|------|------------|------|
| W | provider/transform.ts | 276 | ✅ 完成 |

### 阻塞中（需要 Rust 开发）

| Phase | 模块 | 阻塞原因 |
|-------|------|---------|
| T | MCP 客户端 | Rust 缺少 OAuth PKCE 流程 |
| V | 凭证管理 | Rust 缺少 URL 模式匹配、PKCE 支持 |
| P | LSP 服务器 | Rust 缺少语言检测/安装逻辑 |
| U | 会话存储 | Rust 缺少 Part-based 消息 schema |

---

## Phase W: Provider Transform 清理

### 改动内容

**之前 (868 行)**:
- 使用 try/catch 加载可选的 native bindings
- 每个函数都有 TypeScript fallback 代码
- `normalizeMessages()`, `applyCaching()` 作为内部函数

**之后 (592 行)**:
- 直接 import native bindings（必需）
- 移除所有 fallback 代码
- 删除内部 `normalizeMessages()` 和 `applyCaching()` 函数
- 简化 `sdkKey()`, `temperature()`, `topP()`, `topK()` 为单行

### 具体删除的代码

1. **导入逻辑** (~15 行): 从 try/catch 改为直接 import
2. **sdkKey() fallback** (~18 行): switch 语句
3. **normalizeMessages()** (~120 行): 完整函数
4. **applyCaching()** (~35 行): 完整函数
5. **message() fallback** (~40 行): TypeScript 实现路径
6. **temperature() fallback** (~12 行): switch 逻辑
7. **topP() fallback** (~7 行): switch 逻辑
8. **topK() fallback** (~8 行): switch 逻辑
9. **未使用的 import**: `unique` from remeda

**总计删除**: 276 行 (31.8% 减少)

---

## 阻塞分析

### Phase T: MCP 客户端

**TypeScript 实现** (`mcp/index.ts` 948 行):
- 完整 OAuth 2.0 PKCE 流程
- 支持 startAuth, finishAuth, authenticate
- 使用 @modelcontextprotocol/sdk

**Rust 实现** (`McpClientManagerHandle`):
- 基础传输层: stdio/http/sse
- 工具调用: add, status, list_tools, call_tool
- **缺少**: OAuth 流程、PKCE、动态客户端注册

### Phase V: 凭证管理

**TypeScript 实现** (`credential/vault.ts` 506 行):
- AES-256-GCM 加密
- 文件锁定 (proper-lockfile)
- URL 模式匹配
- OAuth token 管理

**Rust 实现** (`CredentialManagerHandle`):
- 基础 CRUD 操作
- **缺少**: URL 模式匹配、加密存储、文件锁定

### Phase P: LSP 服务器

**TypeScript 实现** (`lsp/server.ts` 2046 行):
- 12+ 语言支持 (Deno, TS, Vue, ESLint, Go, Python, Ruby...)
- 二进制检测 (`Bun.which()`, `Bun.resolve()`)
- 自动安装逻辑
- 根目录检测

**Rust 实现** (`LspServerManagerHandle`):
- 基础进程管理
- JSON-RPC 通信
- **缺少**: 语言配置、二进制检测、安装逻辑

### Phase U: 会话存储

**TypeScript 实现** (`session/message-v2.ts` 765 行):
- 丰富的消息类型: SnapshotPart, PatchPart, TextPart, ReasoningPart, FilePart, AgentPart, DecisionPart, SubtaskPart...
- UI 消息转换
- 复杂类型验证 (Zod)

**Rust 实现** (`MessageStoreHandle`, `SessionStoreHandle`):
- 基础消息类型: role, content, tokens
- **缺少**: Part-based schema、类型验证

---

## 累计迁移成果

| 阶段 | 删除 TS 行数 | 累计 |
|------|------------|------|
| Wave 1-3 (之前) | ~4,318 | 4,318 |
| Wave 4 Phase W | 276 | 4,594 |

---

## 下一步建议

### 短期（无需 Rust 开发）

1. 审查其他 TypeScript 文件，寻找可以简化的 fallback 代码
2. 统一 native binding 加载模式

### 中期（需要 Rust 开发）

1. **优先**: 扩展 `McpAuthStoreHandle` 支持 PKCE
2. 扩展 `CredentialManagerHandle` 支持 URL 模式匹配
3. 在 Rust 实现 `ConfigLoader` (JSONC 解析)
4. 在 Rust 实现 `GitOpsHandle` (基于 git2)

### 长期

1. LSP 语言配置数据驱动化，简化 TypeScript 代码
2. 重新评估是否值得将所有逻辑迁移到 Rust

---

## 结论

Wave 4 的原计划过于激进。实际可行的迁移受限于 Rust 实现的完整度。建议在下一阶段：

1. 优先扩展 Rust NAPI 绑定
2. 采用渐进式迁移策略
3. 保持 TypeScript 代码作为 reference implementation
