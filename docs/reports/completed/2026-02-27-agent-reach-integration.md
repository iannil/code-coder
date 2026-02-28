# Agent Reach 集成进度

## 状态: 已完成 ✅

**实施日期**: 2026-02-27

## 概述

将 Agent-Reach 的互联网访问能力集成到 CodeCoder 的 packages/ccode 中，使 agent 能够自动调用工具访问各类互联网平台。

## 实施内容

### 新增文件

| 文件 | 用途 | 行数 |
|------|------|------|
| `src/tool/reach/types.ts` | 类型定义：ChannelStatus, ChannelInfo, ReachConfig 等 | ~145 |
| `src/tool/reach/config.ts` | 配置管理：读写 ~/.codecoder/reach.json | ~95 |
| `src/tool/reach/utils.ts` | 工具函数：命令执行、URL 解析、格式化 | ~200 |
| `src/tool/reach/doctor.ts` | 依赖诊断：检测 yt-dlp, bird CLI, MCP 状态 | ~250 |
| `src/tool/reach/youtube.ts` | YouTube 工具：视频信息、字幕、搜索 | ~280 |
| `src/tool/reach/bilibili.ts` | Bilibili 工具：视频信息、字幕 | ~220 |
| `src/tool/reach/rss.ts` | RSS 工具：RSS/Atom 解析 | ~200 |
| `src/tool/reach/twitter.ts` | Twitter 工具：推文、搜索、时间线 | ~300 |
| `src/tool/reach/reddit.ts` | Reddit 工具：帖子、评论、搜索 | ~310 |
| `src/tool/reach/xiaohongshu.ts` | 小红书工具：笔记、搜索 (MCP) | ~150 |
| `src/tool/reach/douyin.ts` | 抖音工具：视频、搜索 (MCP) | ~160 |
| `src/tool/reach/linkedin.ts` | LinkedIn 工具：简介、帖子 (MCP) | ~165 |
| `src/tool/reach/bosszhipin.ts` | Boss直聘工具：职位搜索 (MCP) | ~175 |
| `src/tool/reach/index.ts` | 统一导出 + ReachTools 数组 | ~65 |

### 修改文件

| 文件 | 修改内容 |
|------|----------|
| `src/tool/registry.ts` | 导入 ReachTools，添加到工具注册列表 |

## 架构设计

### 工具层级

```
Tier 0 (零配置)
├── reach_youtube   - YouTube 视频/字幕/搜索 (yt-dlp)
├── reach_bilibili  - B站视频/字幕 (yt-dlp)
└── reach_rss       - RSS/Atom 解析 (内置)

Tier 1 (需配置)
├── reach_twitter   - 推文/搜索/时间线 (bird CLI + cookies)
└── reach_reddit    - 帖子/评论/搜索 (JSON API + 可选代理)

Tier 2 (需 MCP)
├── reach_xiaohongshu - 小红书笔记/搜索
├── reach_douyin      - 抖音视频/搜索
├── reach_linkedin    - LinkedIn 简介/帖子
└── reach_bosszhipin  - Boss直聘职位搜索
```

### 配置文件

```json
// ~/.codecoder/reach.json
{
  "proxy": "http://127.0.0.1:7890",
  "twitter": {
    "cookies": "/path/to/cookies.txt"
  },
  "xiaohongshu": {
    "mcpName": "xiaohongshu-mcp"
  }
}
```

### 依赖检测

使用 `doctor.ts` 模块检测各平台可用性：

```
Agent Reach - Dependency Diagnostics
========================================

Tier 0 (Zero Config)
--------------------
  ✓ YouTube       yt-dlp found at /usr/local/bin/yt-dlp
  ✓ Bilibili      yt-dlp found at /usr/local/bin/yt-dlp
  ✓ RSS/Atom      Built-in parser ready

Tier 1 (Needs Config)
---------------------
  ! Twitter       bird CLI found, but no cookies configured
  ✓ Reddit        Reddit API accessible

Tier 2 (Needs MCP)
------------------
  ✗ 小红书        No MCP server configured for xiaohongshu
  ✗ 抖音          No MCP server configured for douyin
  ✗ LinkedIn      No MCP server configured for linkedin
  ✗ Boss直聘      No MCP server configured for bosszhipin

Summary
-------
  3/9 channels ready
  2 channels need attention
  4 channels unavailable
```

## 技术要点

1. **工具定义模式**: 使用 `Tool.define(id, { description, parameters, execute })` 与现有工具保持一致
2. **命令执行**: 使用 `spawn` + `Shell.killTree` 支持超时和中止
3. **MCP 集成**: 复用 `MCP.tools()` 获取 AI SDK 动态工具，使用 `tool.execute()` 调用
4. **配置隔离**: reach.json 与主配置分离，文件权限 600 保护敏感信息
5. **依赖检测**: 渐进式检测，友好的安装提示

## 验证步骤

```bash
# 1. 类型检查
cd packages/ccode && bun run typecheck

# 2. 手动测试 YouTube
# 在 CodeCoder 中输入：
# 使用 reach_youtube 工具获取 https://youtube.com/watch?v=dQw4w9WgXcQ 的视频信息

# 3. 手动测试 RSS
# 使用 reach_rss 工具读取 https://hnrss.org/frontpage 的最新 5 条内容
```

## 后续工作

- [ ] 添加 CLI 命令 `codecoder reach doctor` 用于诊断
- [ ] 添加 CLI 命令 `codecoder reach configure` 用于配置
- [ ] 为 Tier 2 工具添加更多错误处理和重试逻辑
- [ ] 考虑添加缓存机制减少重复请求
