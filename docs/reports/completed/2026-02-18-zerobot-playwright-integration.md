# ZeroBot Playwright MCP 集成

**日期**: 2026-02-18
**状态**: 已完成

## 概述

实现 ZeroBot 通过 CodeCoder 调用 Playwright MCP 执行浏览器操作（如航班查询）的完整支持。

## 问题背景

用户希望通过 ZeroBot（Telegram）调用 CodeCoder 实现需要浏览器操作的任务。当前系统存在以下问题：
1. general agent 执行成功但直接回复"无法查询"，没有实际使用工具
2. MCP 工具权限机制未正确触发

## 实施内容

### Phase 2: 增强 general agent 工具使用指导

**修改文件**:
- `packages/ccode/src/agent/agent.ts` - 添加 PROMPT_GENERAL 引用
- `packages/ccode/src/agent/prompt/general.txt` - 新建 prompt 文件

**变更内容**:
1. 新增 `general.txt` prompt 文件，指导 agent 使用 Playwright MCP 工具
2. 在 agent.ts 中导入并配置 general agent 使用此 prompt

### Phase 3: 优化远程任务的权限流程

**修改文件**:
- `packages/ccode/src/security/remote-policy.ts`

**变更内容**:

1. **扩展 DANGEROUS_OPERATIONS**:
   - 添加 Playwright 浏览器交互操作（navigate, click, type, fill_form, file_upload, evaluate, run_code, select_option, drag, press_key, handle_dialog）

2. **扩展 SAFE_OPERATIONS**:
   - 添加 Playwright 只读操作（snapshot, take_screenshot, console_messages, network_requests, tabs, wait_for, navigate_back, resize, hover, close, install）

3. **更新 shouldRequireApproval**:
   - 添加 MCP 工具前缀匹配逻辑，所有 `mcp__` 前缀工具默认需要审批

4. **更新 describeApprovalReason**:
   - 添加 Playwright 操作的人类可读描述

## 验证步骤

### 本地验证
```bash
# 启动 CodeCoder API 服务器
cd ~/Code/zproducts/code-coder && bun dev serve

# 通过 curl 测试任务创建
curl -X POST http://localhost:4400/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "agent": "general",
    "prompt": "打开 https://www.ctrip.com 并截图",
    "context": {
      "source": "remote",
      "userID": "test-user",
      "platform": "test"
    }
  }'
```

### 端到端验证
1. 启动 ZeroBot daemon
2. 通过 Telegram 发送：`查询明天海口到盐城的航班`
3. 验证收到权限确认请求
4. 点击"批准"或"始终批准"
5. 验证收到航班信息回复

## 关键文件清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `packages/ccode/src/agent/agent.ts` | 修改 | 添加 PROMPT_GENERAL 导入和引用 |
| `packages/ccode/src/agent/prompt/general.txt` | 新增 | general agent 专用 prompt |
| `packages/ccode/src/security/remote-policy.ts` | 修改 | MCP 工具权限处理 |

## 注意事项

1. **安全性**: MCP 工具（尤其是 Playwright）可以执行任意浏览器操作，需要谨慎审批
2. **性能**: 浏览器操作较慢，需要合理设置超时
3. **状态管理**: Playwright 会话状态需要正确管理（登录状态、cookie 等）
4. **错误处理**: 网络问题、页面加载失败等需要优雅处理

## 后续增强

1. 用户级工具白名单持久化（已实现）
2. 操作日志审计
3. 沙箱模式限制 Playwright 只能访问特定域名
4. 会话复用提高效率
