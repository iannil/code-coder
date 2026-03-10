# ~/.codecoder/ 配置文件审计报告

**日期**: 2026-03-10
**状态**: 已完成

---

## 执行摘要

对 `~/.codecoder/` 目录进行了全面审计，识别并清理了过期配置、测试残留和冗余文件。

## 已执行的变更

### 1. config.json - 服务端口配置更新

**变更前:**
```json
"services": {
  "codecoder": { "port": 4400 },
  "gateway": { "port": 4430 },
  "channels": { "port": 4431 },
  "workflow": { "port": 4432 },
  "trading": { "port": 4434 }
}
```

**变更后:**
```json
"services": {
  "codecoder": { "port": 4400 },
  "daemon": {
    "port": 4402,
    "_comment": "Unified entry point for gateway/channels/workflow/trading (previously 4430-4434)"
  }
}
```

**原因:** 根据 CLAUDE.md 文档，端口 4430-4439 已整合到 zero-hub，通过 zero-cli daemon (:4402) 统一管理。旧端口定义为死配置。

### 2. alerts.json - channels_endpoint 更新

**变更前:**
```json
"channels_endpoint": "http://127.0.0.1:4431"
```

**变更后:**
```json
"channels_endpoint": "http://127.0.0.1:4402/api/v1/channels"
```

**原因:** channels 服务已整合到 daemon，需通过统一入口访问。

### 3. bootstrap/candidates.json - 清理测试数据

**变更前:** 包含 4 个测试候选项（名称如 "test-1771681423333"）

**变更后:** 清空为空数组 `[]`

**原因:** 所有候选项均为测试数据，usageCount 为 0，非生产数据。

### 4. 删除冗余文件

- `~/.codecoder/node_modules/` - 空目录
- `~/.codecoder/package.json` - 空对象 `{}`

**原因:** 无实际内容，为历史残留。

---

## 文件状态总览

| 文件 | 状态 | 备注 |
|------|------|------|
| config.json | ✅ 已更新 | 移除过期端口配置 |
| alerts.json | ✅ 已更新 | 更新 channels_endpoint |
| bootstrap/candidates.json | ✅ 已清理 | 移除测试数据 |
| node_modules/ | ✅ 已删除 | 空目录 |
| package.json | ✅ 已删除 | 空文件 |
| secrets.json | ✅ 有效 | 无需变更 |
| credentials.json | ✅ 有效 | 无需变更 |
| channels.json | ✅ 有效 | 无需变更 |
| providers.json | ✅ 有效 | 无需变更 |
| trading.json | ✅ 有效 | 无需变更 |
| routing.json | ✅ 有效 | 无需变更 |
| keywords.json | ✅ 有效 | 无需变更 |
| messages.json | ✅ 有效 | 无需变更 |
| daemon_state.json | ✅ 有效 | 无需变更 |
| .secret_key | ✅ 有效 | 无需变更 |
| financial.db | ✅ 有效 | 16 MB 交易数据 |
| gateway.db | ✅ 有效 | 认证数据 |
| metering.db | ✅ 有效 | 配额追踪 |
| state/* | ✅ 有效 | 运行时状态 |
| workflow/* | ✅ 有效 | 工作流数据 |
| data/* | ✅ 有效 | 存储数据 |

---

## 待跟进事项

### 文档建议

在 CLAUDE.md 中添加以下说明：

1. **credentials.json vs secrets.json 区别**
   - `secrets.json`: 静态 API 密钥配置，由配置加载器直接读取
   - `credentials.json`: 运行时加密的动态凭证，由 ChaCha20-Poly1305 加密

2. **端口配置说明**
   - 明确标注 `services.daemon.port: 4402` 是统一入口
   - 标注 4430-4439 已废弃

---

## 验证

```bash
# 验证 config.json 更新
cat ~/.codecoder/config.json | jq '.services'

# 验证 alerts.json 更新
cat ~/.codecoder/alerts.json | jq '.channels_endpoint'

# 验证 candidates.json 已清理
cat ~/.codecoder/bootstrap/candidates.json | jq '.candidates | length'
```
