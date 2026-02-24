# 端口重新分配实施报告

> 文档类型: progress
> 创建时间: 2026-02-23
> 状态: completed

## 概述

完成端口分配重新设计，解决 MCP Server 与 Zero Channels 的端口冲突（两者均使用 4405），并建立清晰的端口分组逻辑。

## 新端口分配方案

### 端口分组

| 端口段 | 用途 | 使用中 |
|--------|------|--------|
| 4400-4409 | 核心服务 | 4400-4403 |
| 4410-4419 | 独立 Rust 服务 | 4410-4412 |
| 4420-4429 | 协议/工具服务 | 4420 |
| 4430-4439 | 第三方集成 | 预留 |
| 4440-4449 | 测试端口 | 预留 |

### 详细端口分配

| 服务 | 旧端口 | 新端口 | 变更说明 |
|------|--------|--------|----------|
| CodeCoder API Server | 4400 | 4400 | 不变 |
| Web Frontend (Vite) | 4401 | 4401 | 不变 |
| Zero CLI Daemon | 4402 | 4402 | 不变 |
| Faster Whisper STT | 4403 | 4403 | 不变 |
| Zero Gateway | 4404 | 4410 | 移动到 Rust 服务端口段 |
| Zero Channels | 4405 | 4411 | 移动到 Rust 服务端口段 |
| Zero Workflow | 4406 | 4412 | 移动到 Rust 服务端口段 |
| MCP Server (HTTP) | 4405 | 4420 | 解决冲突，移动到协议端口段 |

## 修改的文件

### 运维脚本
- `ops.sh` - 更新端口定义和帮助文本

### TypeScript 代码
- `packages/util/src/config.ts` - 更新默认端口配置
- `packages/ccode/src/mcp/server.ts` - MCP 服务端口 4405→4420
- `packages/ccode/src/cli/cmd/mcp.ts` - CLI 默认端口参数
- `packages/ccode/test/mcp/integration.test.ts` - 测试端口

### Rust 代码
- `services/zero-common/src/config.rs` - 更新默认端口和测试
- `services/zero-cli/src/main.rs` - Gateway CLI 默认端口
- `services/zero-cli/src/client.rs` - 客户端端点常量和注释
- `services/zero-workflow/src/lib.rs` - Workflow 服务端口
- `services/zero-workflow/src/review_bridge.rs` - 注释更新
- `services/zero-gateway/tests/integration_test.rs` - 测试配置

### 文档
- `CLAUDE.md` - 端口配置部分
- `README.md` - 端口表格
- `README_CN.md` - 端口表格
- `docs/RUNBOOK.md` - 端口表格和命令示例
- `docs/guides/DEPLOYMENT.md` - 完整部署配置
- `docs/standards/mcp-guide.md` - MCP 服务端口

## 验证结果

### 代码编译检查
- ✅ TypeScript 类型检查通过（无新增错误）
- ✅ Rust 编译检查通过（仅有预存在的 warnings）

### 端口引用检查
```bash
# 确认无遗漏的旧端口引用
grep -r "4404\|4405\|4406" --include="*.ts" --include="*.rs" --include="*.md"
# 结果: 无匹配
```

### ops.sh 输出验证
```
独立 Rust 服务 (可选，用于模块化部署):
  zero-gateway       独立网关服务 (端口 4410)
  zero-channels      独立频道服务 (端口 4411)
  zero-workflow      独立工作流服务 (端口 4412)
```

## 迁移注意事项

### 需要用户更新的配置

如果用户有现存的 `~/.codecoder/config.json`，需要更新端口配置：

```json
{
  "gateway": {
    "port": 4410  // 原 4404
  },
  "channels": {
    "port": 4411  // 原 4405
  },
  "workflow": {
    "webhook": {
      "port": 4412  // 原 4406
    }
  }
}
```

### Docker/Nginx 配置更新

如果使用 Docker Compose 或 Nginx 反向代理，需要更新端口映射：
- `4404:4404` → `4410:4410`
- `4405:4405` → `4411:4411`
- `4406:4406` → `4412:4412`

## 后续工作

- [ ] 更新 CI/CD 配置中的端口引用（如有）
- [ ] 更新 Kubernetes 部署文件（如有）
- [ ] 通知团队成员端口变更
