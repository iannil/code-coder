# ZeroBot-CodeCoder 鉴权方案实现进度

## 实现日期: 2026-02-18

## 实施状态: 已完成 ✅

## 验证状态

- [x] Rust 代码编译通过 (cargo check 成功)
- [x] TypeScript 依赖安装 (proper-lockfile)
- [x] 凭证库实现完成 (Rust + TypeScript)
- [x] API 鉴权实现完成
- [x] 自动登录工具实现完成
- [x] CLI 命令实现完成

## 概述

实现了 ZeroBot 和 CodeCoder 之间的统一凭证管理系统，支持 API Key、OAuth、用户名密码登录等多种鉴权类型。

## 已完成的工作

### Phase 1: 凭证存储基础设施 ✅

**新建文件:**
- `services/zero-bot/src/security/vault.rs` - Rust 凭证库核心实现
  - ChaCha20-Poly1305 加密存储
  - 文件锁定确保并发安全
  - URL 模式匹配
  - CRUD 操作
  - 单元测试覆盖

- `packages/ccode/src/credential/vault.ts` - TypeScript 凭证库
  - 与 Rust 版本兼容的加密格式
  - 使用 proper-lockfile 进行文件锁定
  - 完整的 CRUD API

- `packages/ccode/src/credential/resolver.ts` - 凭证解析器
  - URL 模式匹配
  - OAuth token 自动刷新
  - HTTP header 注入

- `packages/ccode/src/credential/index.ts` - 模块导出

**修改文件:**
- `services/zero-bot/src/security/mod.rs` - 添加 vault 模块导出
- `services/zero-bot/src/config/schema.rs` - 添加 VaultConfig 配置
- `packages/ccode/src/config/config.ts` - 添加 Server.apiKey 和 Vault 配置

### Phase 2: CodeCoder API 鉴权 ✅

**修改文件:**
- `services/zero-bot/src/tools/codecoder.rs`
  - CodeCoderTool 现在接受可选的 api_key 参数
  - 所有 HTTP 请求都添加 Authorization header
  - 测试用例更新

- `services/zero-bot/src/tools/mod.rs`
  - 更新 CodeCoderTool 实例化以传递 api_key

- `services/zero-bot/src/config/schema.rs`
  - CodeCoderConfig 添加 api_key 字段

### Phase 3: 外部服务凭证注入 ✅

**新建文件:**
- `services/zero-bot/src/tools/auto_login.rs` - 自动登录工具
  - 浏览器自动化登录
  - TOTP 自动生成
  - 交互式 2FA 验证码请求
  - 常见登录页面模式识别

**修改文件:**
- `services/zero-bot/src/agent/confirmation.rs`
  - 添加 request_text_input 函数用于 2FA
  - 添加 TextInputRegistry

- `services/zero-bot/src/tools/mod.rs`
  - 添加 AutoLoginTool 导出
  - all_tools 函数添加 vault_config 参数

- `services/zero-bot/src/channels/mod.rs`
  - 更新 all_tools 调用

- `services/zero-bot/src/agent/loop_.rs`
  - 更新 all_tools 调用

### Phase 4: CLI 凭证管理命令 ✅

**新建文件:**
- `services/zero-bot/src/credential.rs` - CLI 命令处理
  - `zero-bot credential list` - 列出所有凭证
  - `zero-bot credential add` - 添加凭证
  - `zero-bot credential remove` - 删除凭证
  - `zero-bot credential show` - 显示凭证详情

**修改文件:**
- `services/zero-bot/src/main.rs`
  - 添加 CredentialCommands 枚举
  - 添加 credential 子命令

- `services/zero-bot/Cargo.toml`
  - 添加依赖: fs4, zeroize, rand, rpassword, totp-rs

## 架构说明

```
┌─────────────────────────────────────────────────────────────────┐
│                        Credential Vault                          │
│                 ~/.codecoder/credentials.json                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  API Keys    │  │    OAuth     │  │  Login Creds │          │
│  │   Provider   │  │   Provider   │  │   Provider   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────────┐
        │ ZeroBot  │   │ CodeCoder│   │ Browser Auto │
        │ (Rust)   │   │ (TS/Bun) │   │ (Playwright) │
        └──────────┘   └──────────┘   └──────────────┘
```

## 使用示例

### 添加 API Key

```bash
zero-bot credential add --type api_key --service github --key ghp_xxx --patterns "*.github.com"
```

### 添加 OAuth

```bash
zero-bot credential add --type oauth --service google --client-id xxx --client-secret yyy
```

### 添加登录凭证

```bash
zero-bot credential add --type login --service example --username user@example.com --patterns "login.example.com"
# 会提示输入密码
```

### 列出凭证

```bash
zero-bot credential list
```

## 安全特性

1. **加密存储**: 所有凭证使用 ChaCha20-Poly1305 加密
2. **最小权限**: 凭证文件权限设为 0600
3. **内存安全**: 凭证使用后通过 zeroize 清理
4. **域名白名单**: 凭证只能用于匹配的 URL 模式
5. **文件锁定**: 使用 fs4/proper-lockfile 确保并发安全

## 待验证项目

- [x] Rust 代码编译通过
- [x] TypeScript 代码编译通过 (web 包)
- [ ] 凭证加密/解密测试
- [ ] URL 模式匹配测试
- [ ] ZeroBot → CodeCoder API Key 鉴权测试
- [x] 前端组件 TypeScript 编译通过

## Phase 5: 前端凭证管理页面 ✅

**实施日期**: 2026-02-18

### 新建文件

1. **`packages/ccode/src/api/server/handlers/credential.ts`** - API 处理器
   - `GET /api/credentials` - 列出所有凭证（不含敏感数据）
   - `GET /api/credentials/:id` - 获取单个凭证详情
   - `POST /api/credentials` - 添加新凭证
   - `PUT /api/credentials/:id` - 更新凭证
   - `DELETE /api/credentials/:id` - 删除凭证
   - `GET /api/credentials/resolve` - 根据 URL 或服务名解析凭证

2. **`packages/web/src/stores/credential.ts`** - 状态管理
   - 使用 Zustand + Immer
   - 完整的 CRUD 操作
   - 加载/错误状态管理

3. **`packages/web/src/components/credentials/CredentialPanel.tsx`** - 主面板
   - 凭证列表展示（按类型分组）
   - 类型统计卡片
   - 添加/删除功能
   - 删除确认对话框

4. **`packages/web/src/components/credentials/CredentialForm.tsx`** - 添加表单
   - 类型选择（API Key / OAuth / Login / Bearer Token）
   - 动态字段显示
   - URL 模式输入（支持多个模式）
   - 敏感字段使用密码输入

5. **`packages/web/src/components/credentials/index.ts`** - 模块导出

### 修改文件

1. **`packages/ccode/src/api/server/router.ts`**
   - 导入凭证处理器
   - 注册凭证路由

2. **`packages/web/src/lib/types.ts`**
   - 添加凭证类型定义 (CredentialType, OAuthCredential, LoginCredential, CredentialEntry, CredentialSummary, CredentialCreateInput)

3. **`packages/web/src/lib/api.ts`**
   - 添加凭证 API 方法 (listCredentials, getCredential, addCredential, updateCredential, deleteCredential)

4. **`packages/web/src/stores/index.ts`**
   - 导出凭证 Store

5. **`packages/web/src/pages/Settings.tsx`**
   - 添加 "Credentials" 导航项到 Security 分组
   - 使用 Vault 图标
   - 渲染 CredentialPanel 组件

### UI 功能特性

- 按类型分组展示凭证
- 类型统计卡片（API Key / OAuth / Login / Bearer Token）
- 添加新凭证表单（类型动态切换）
- URL 模式管理（支持多个模式，回车添加）
- 删除确认对话框
- 敏感字段使用密码输入
- 加载和错误状态处理
- Toast 通知反馈

## 下一步

1. 运行完整的编译验证
2. 编写集成测试
3. 添加 E2E 测试用例
