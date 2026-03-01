# Workspace Migration Report

**Date**: 2026-02-28

## 迁移概述

将 CodeCoder 运行时数据从分散位置迁移到统一的 workspace 目录。

## 迁移前后对照

| 数据类型 | 迁移前位置 | 迁移后位置 |
|---------|-----------|-----------|
| Hands 定义 | `~/.codecoder/hands/` | `~/.codecoder/workspace/hands/` |
| 持久化存储 | `~/.local/share/ccode/storage/` | `~/.codecoder/workspace/storage/` |
| 日志文件 | `~/.local/share/ccode/log/` | `~/.codecoder/workspace/log/` |
| 工具输出 | `~/.local/share/ccode/tool-output/` | `~/.codecoder/workspace/tool-output/` |
| 快照 | `~/.local/share/ccode/snapshot/` | `~/.codecoder/workspace/storage/snapshot/` |
| 状态文件 | `~/.local/state/ccode/` | `~/.codecoder/workspace/knowledge/` |
| 数据库文件 | `~/.codecoder/*.db` | `~/.codecoder/workspace/storage/` |

## 新的 Workspace 结构

```
~/.codecoder/workspace/
├── hands/          # Hands 定义和输出
├── storage/        # 持久化存储（会话、消息、数据库等）
├── log/            # 日志文件
├── tool-output/    # 工具执行输出
├── knowledge/      # 状态文件（auth.json, kv.json等）
├── tracking/       # 执行追踪数据
├── cache/          # 缓存目录
└── mcp-auth.json   # MCP 认证存储
```

## 配置文件更新

`~/.codecoder/config.json` 已添加 `workspace` 配置节：

```json
{
  "workspace": {
    "path": "~/.codecoder/workspace",
    "subdirs": {
      "hands": "hands",
      "storage": "storage",
      "log": "log",
      "tool_output": "tool-output",
      "knowledge": "knowledge",
      "tracking": "tracking",
      "mcp_auth": "mcp-auth.json",
      "cache": "cache"
    }
  }
}
```

## 备份位置

所有原始数据已备份到：
```
~/.codecoder/backup/before-workspace-migration-2026-02-28T06-36-11/
```

## 环境变量覆盖

可通过环境变量 `CODECODER_WORKSPACE` 覆盖默认路径：

```bash
export CODECODER_WORKSPACE=/custom/workspace/path
```

## 验证步骤

1. ✅ 配置文件已更新
2. ✅ 数据已迁移到 workspace
3. ✅ 原始数据已备份
4. ⏳ 需要重启服务验证

## 回滚方案

如需回滚：

1. 从 `config.json` 中移除 `workspace` 配置节
2. 从备份恢复数据：
   ```bash
   cp -r ~/.codecoder/backup/before-workspace-migration-*/hands ~/.codecoder/
   cp -r ~/.codecoder/backup/before-workspace-migration-*/storage ~/.local/share/ccode/
   cp -r ~/.codecoder/backup/before-workspace-migration-*/log ~/.local/share/ccode/
   cp -r ~/.codecoder/backup/before-workspace-migration-*/tool-output ~/.local/share/ccode/
   cp -r ~/.codecoder/backup/before-workspace-migration-*/state ~/.local/state/ccode/
   ```

## 下一步

1. 重启 CodeCoder 服务验证功能正常
2. 确认 Hands 输出到 `~/.codecoder/workspace/hands/`
3. 确认日志输出到 `~/.codecoder/workspace/log/`
4. 验证一周后移除备份目录
