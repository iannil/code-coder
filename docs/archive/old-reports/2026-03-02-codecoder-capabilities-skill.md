# CodeCoder 内置能力 Skill 实现

**日期**: 2026-03-02
**状态**: 已完成

## 问题背景

通过分析 Trace `1ae6634f-a624-4536-9f3c-11c5d0ee9f70` 和 `9aaaa727-122c-4560-bf66-8cfbfb2c1a28`，发现 autonomous agent 在回答系统能力相关问题时存在以下问题：

1. **Trace 1ae6634f**: Agent 使用 glob/grep/read 搜索文件系统，而不是调用 API
2. **Trace 9aaaa727**: Agent 完全没有使用任何工具 (`tools_used=[]`)，直接从"记忆"回答

### 根本原因

- Agent 不知道 Hands API (`http://localhost:4432/api/v1/hands`) 的存在
- 没有专门的 hands 查询工具
- System prompt 中没有说明如何查询系统能力

## 解决方案

### 1. 创建 `codecoder-capabilities` Skill

**位置**: `packages/ccode/src/skill/builtin/codecoder-capabilities/SKILL.md`

**内容概述**:
- Hands 系统完整说明和 API
- Scheduler 系统完整说明和工具
- 服务架构和端口配置
- 可用 Agent 列表
- 内置工具列表
- 常见查询场景示例
- 故障排除指南

### 2. 更新 Autonomous Agent Prompt

**文件**: `packages/ccode/src/agent/prompt/autonomous.txt`

**新增内容**: "System Capabilities Query (系统能力查询)" 部分

**规则**:
- 当询问 hands/定时任务/workflow/系统能力时，必须调用 API
- 提供具体的 curl 命令示例
- 列出检测模式 (detection patterns)
- 禁止从记忆回答或猜测

## 修改的文件

1. **新增**: `packages/ccode/src/skill/builtin/codecoder-capabilities/SKILL.md`
2. **修改**: `packages/ccode/src/agent/prompt/autonomous.txt` (添加 System Capabilities Query 部分)

## 验证方法

1. 重启 CodeCoder 服务
2. 通过 Telegram 发送: "有哪些定时任务"
3. 预期行为: Agent 应该调用 `curl http://localhost:4432/api/v1/hands` 和/或 `scheduler_list_tasks`

## API 端点参考

| 服务 | 端口 | 端点 | 说明 |
|------|------|------|------|
| Workflow | 4432 | `/api/v1/hands` | 列出 Hands |
| CodeCoder API | 4400 | `/api/v1/scheduler/tasks` | 列出定时任务 |

## 后续建议

1. 考虑添加专门的 `hands_list` 工具 (类似 `scheduler_list_tasks`)
2. 在 Hands API 返回中包含 `enabled: false` 的 hands (可选参数)
3. 添加更多 trace 日志来记录 Agent 的响应内容
