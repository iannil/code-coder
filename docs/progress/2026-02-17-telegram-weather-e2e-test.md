# Telegram → ZeroBot → CodeCoder 天气查询端到端测试进展

## 测试日期：2026-02-17

## 当前状态：Ready for Manual Testing

---

## 零、Channel Store Mock 数据问题修复

### 问题描述
Channel Store 之前使用硬编码的 `MOCK_CHANNELS` 数据，导致 Infrastructure → Channels 页面显示的是占位数据而非真实的 ZeroBot 通道状态。

### 修复方案
1. **创建 Channel API Handler** (`packages/ccode/src/api/server/handlers/channel.ts`)
   - 读取 `~/.codecoder/config.json` 中的 `zerobot.channels` 配置
   - 提供 `/api/channels` 端点返回通道状态

2. **注册 API 路由** (`packages/ccode/src/api/server/router.ts`)
   - `GET /api/channels` - 列出所有通道
   - `GET /api/channels/:name` - 获取特定通道状态
   - `POST /api/channels/:name/health` - 检查通道健康状态

3. **更新 API 客户端** (`packages/web/src/lib/api.ts`)
   - 添加 `listChannels()`, `getChannel()`, `checkChannelHealth()` 方法

4. **更新 Channel Store** (`packages/web/src/stores/channel.ts`)
   - 移除 `MOCK_CHANNELS` 硬编码数据
   - 使用真实 API 调用获取通道状态
   - 添加 `zeroBotRunning` 状态追踪

### 修改文件清单
| 文件 | 操作 |
|------|------|
| `packages/ccode/src/api/server/handlers/channel.ts` | 新建 |
| `packages/ccode/src/api/server/router.ts` | 修改 - 添加路由注册 |
| `packages/web/src/lib/types.ts` | 无需修改 - 类型已存在 |
| `packages/web/src/lib/api.ts` | 修改 - 添加 API 方法 |
| `packages/web/src/stores/channel.ts` | 重写 - 使用真实 API |
| `packages/web/src/stores/index.ts` | 修改 - 导出新 hook |

### 验证状态
- [x] Web 包 TypeScript 检查通过
- [x] Channel handler 编译成功
- [ ] 运行时测试待验证

---

## 一、基础设施审查完成

### 1. Web UI (packages/web)

| 组件 | 路径 | 状态 |
|------|------|------|
| Tasks 页面 | `src/pages/Tasks.tsx` | ✅ 已实现 |
| TaskCreate 组件 | `src/components/task/TaskCreate.tsx` | ✅ 已实现 |
| TaskList 组件 | `src/components/task/TaskList.tsx` | ✅ 已实现 |
| TaskDetail 组件 | `src/components/task/TaskDetail.tsx` | ✅ 已实现 |
| Task Store | `src/stores/task.ts` | ✅ 已实现 (含 SSE) |
| Infrastructure 页面 | `src/pages/Infrastructure.tsx` | ✅ 已实现 |
| ChannelPanel 组件 | `src/components/channel/ChannelPanel.tsx` | ✅ 已实现 |
| Channel Store | `src/stores/channel.ts` | ⚠️ 使用 Mock 数据 |
| API 客户端 | `src/lib/api.ts` | ✅ 已实现 |
| 路由配置 | `src/router.ts` | ✅ 已配置 /tasks, /infrastructure |
| 侧边栏导航 | `src/App.tsx` | ✅ 已添加 Tasks, Infrastructure 链接 |

### 2. CodeCoder API (packages/ccode)

| 端点 | 路径 | 状态 |
|------|------|------|
| POST /api/v1/tasks | `src/api/server/handlers/task.ts` | ✅ 创建任务 |
| GET /api/v1/tasks | `src/api/server/handlers/task.ts` | ✅ 列出任务 |
| GET /api/v1/tasks/:id | `src/api/server/handlers/task.ts` | ✅ 获取任务 |
| GET /api/v1/tasks/:id/events | `src/api/server/handlers/task.ts` | ✅ SSE 事件流 |
| POST /api/v1/tasks/:id/interact | `src/api/server/handlers/task.ts` | ✅ 批准/拒绝 |
| DELETE /api/v1/tasks/:id | `src/api/server/handlers/task.ts` | ✅ 删除任务 |

### 3. ZeroBot (services/zero-bot)

| 组件 | 路径 | 状态 |
|------|------|------|
| Telegram Channel | `src/channels/telegram.rs` | ✅ 长轮询实现 |
| CodeCoder Tool | `src/tools/codecoder.rs` | ✅ SSE 桥接实现 |
| 配置 Schema | `src/config/schema.rs` | ✅ 支持 codecoder 配置 |

---

## 二、测试环境准备

### 2.1 Telegram Bot 创建步骤

1. 打开 Telegram，搜索 `@BotFather`
2. 发送 `/newbot` 创建新 bot
3. 按提示设置 bot 名称和用户名
4. 记录返回的 token（格式：`123456:ABC-DEF...`）1905582852:AAG1sQ3c3usDVyL1BoNvrIwx1ij9SzDV4zw

### 2.2 配置文件

编辑 `~/.codecoder/config.json`，添加 `zerobot.channels` 段：

```json
{
  "zerobot": {
    "channels": {
      "cli": true,
      "telegram": {
        "bot_token": "YOUR_BOT_TOKEN_HERE",
        "allowed_users": ["your_username", "*"]
      }
    }
  }
}
```

**注意**: 将上述配置合并到你现有的 config.json 中，保留 provider 等其他配置。

### 2.3 启动命令

**终端 1 - CodeCoder API**:
```bash
cd /Users/iannil/Code/zproducts/code-coder
bun dev serve
```

**终端 2 - Web UI**:
```bash
cd /Users/iannil/Code/zproducts/code-coder/packages/web
bun dev
```

**终端 3 - ZeroBot daemon**:
```bash
cd /Users/iannil/Code/zproducts/code-coder/services/zero-bot
cargo run -- daemon
```

---

## 三、测试阶段

### 阶段 1：Web UI 预验证

| 步骤 | 操作 | 预期结果 | 实际结果 |
|------|------|----------|----------|
| 1.1 | 访问 http://localhost:5173 | 页面正常加载 | [ ] 待测试 |
| 1.2 | 点击侧边栏 "Agents" | 显示 agent 列表 | [ ] 待测试 |
| 1.3 | 点击侧边栏 "Tasks" | 显示 Tasks 页面 | [ ] 待测试 |
| 1.4 | 点击 "New Task" 按钮 | 弹出创建对话框 | [ ] 待测试 |
| 1.5 | 选择 agent: general | Agent 可选择 | [ ] 待测试 |
| 1.6 | 输入: "查询今天北京的天气" | 文本可输入 | [ ] 待测试 |
| 1.7 | 点击 Create Task | 任务创建成功 | [ ] 待测试 |
| 1.8 | 观察任务状态变化 | pending→running→completed | [ ] 待测试 |
| 1.9 | 查看任务输出 | 包含天气信息 | [ ] 待测试 |

### 阶段 2：Infrastructure 页面验证

| 步骤 | 操作 | 预期结果 | 实际结果 |
|------|------|----------|----------|
| 2.1 | 点击侧边栏 "Infrastructure" | 显示 Infrastructure 页面 | [ ] 待测试 |
| 2.2 | 点击 "Channels" 标签 | 显示 Channel 列表 | [ ] 待测试 |
| 2.3 | 查看 Telegram 状态 | 显示配置状态 | [ ] 待测试 |

**注意**: Channel Store 当前使用 Mock 数据，显示的是预设数据而非实时 ZeroBot 状态。

### 阶段 3：Telegram E2E 测试

| 步骤 | 操作 | 预期结果 | 实际结果 |
|------|------|----------|----------|
| 3.1 | 在 Telegram 搜索 bot | 找到创建的 bot | [ ] 待测试 |
| 3.2 | 发送 "查询今天北京天气" | 消息发送成功 | [ ] 待测试 |
| 3.3 | 查看 ZeroBot 日志 | 显示收到消息 | [ ] 待测试 |
| 3.4 | 查看 Web UI Tasks | 自动创建新任务 | [ ] 待测试 |
| 3.5 | 等待任务完成 | 状态变为 completed | [ ] 待测试 |
| 3.6 | 查看 Telegram 回复 | 收到天气信息 | [ ] 待测试 |

---

## 四、当前配置状态

### Provider 配置
```json
{
  "zerobot": {
    "default_provider": "deepseek",
    "default_model": "deepseek-chat",
    "channels": { ... },
    "codecoder": {
      "enabled": true,
      "endpoint": "http://localhost:4096"
    }
  }
}
```

ZeroBot 将自动解析 DeepSeek provider:
- baseURL: `https://api.deepseek.com`
- API Key: 从 `provider.deepseek.options.apiKey` 读取

### 模型变更历史
1. **uniapi/gemini-2.5-pro** → 返回空响应
2. **deepseek/deepseek-chat** ← 当前配置

---

## 五、已知限制

1. **Channel 配置修改**: 添加/删除/启用/禁用通道需要直接编辑 `~/.codecoder/config.toml` 并重启 ZeroBot daemon。Web UI 目前只能查看状态，不能修改配置。

2. **Provider 配置**: 确保 `default_provider` 和相关 API key 已正确配置，否则 agent 无法执行 WebSearch。

3. **网络依赖**: `general` agent 使用 WebSearch 查询天气，需要网络连接和 provider API 可用。

---

## 五、下一步行动

1. [ ] 创建 Telegram Bot 并获取 token
2. [ ] 配置 `~/.codecoder/config.toml`
3. [ ] 启动三个服务（CodeCoder API、Web UI、ZeroBot daemon）
4. [ ] 执行 Web UI 预验证测试
5. [ ] 执行 Telegram E2E 测试
6. [ ] 记录测试结果和截图
7. [ ] 将完成的测试报告移至 `docs/reports/completed/`

---

## 更新日志

| 时间 | 操作 | 执行人 |
|------|------|--------|
| 2026-02-17 10:00 | 创建测试进展文档，完成基础设施审查 | Claude |
| 2026-02-17 10:30 | 修复 Channel Store Mock 数据问题，实现真实 API (TOML) | Claude |
| 2026-02-17 10:45 | 改用 JSON 格式配置，路径 `zerobot.channels` | Claude |
| 2026-02-17 10:50 | 修复配置 schema 验证错误，API 端点测试通过 | Claude |
| 2026-02-17 11:30 | ZeroBot 支持从 CodeCoder config.json 解析 provider 配置 | Claude |
| 2026-02-17 12:00 | 修改系统提示词，默认使用 CodeCoder tool | Claude |
| 2026-02-17 12:30 | 切换模型为 DeepSeek chat (gemini-2.5-pro 返回空响应) | Claude |
| 2026-02-17 13:00 | 实现 AgentExecutor - agentic tool-calling loop | Claude |
| 2026-02-17 13:30 | 修复 CodeCoder API context 缺少 userID/platform 字段 | Claude |
| 2026-02-17 14:00 | 实现 Telegram 授权通知 - 当 CodeCoder 需要确认时发送通知 | Claude |
| 2026-02-17 15:00 | 修复 SSE 流处理 - 添加每块超时防止无限阻塞，超时后回退到轮询 | Claude |
