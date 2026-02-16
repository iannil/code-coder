# 变更日志

本文件记录 ZeroBot 的重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

---

## [0.1.0] - 2026-02

### 项目重命名

- **ZeroClaw → ZeroBot**
  - 包名: `zeroclaw` → `zero-bot`
  - 二进制: `zeroclaw` → `zero-bot`
  - 配置目录: `~/.zeroclaw/` → `~/.codecoder/`
  - 详见 [RENAME_HISTORY.md](../archive/RENAME_HISTORY.md)

### 新增

- **Providers**
  - 24 个 LLM 后端支持
  - OpenAI 兼容层 (compatible.rs)
  - 弹性包装器 (reliable.rs)

- **Channels**
  - 8 个消息通道
  - 统一 Channel trait 接口
  - 健康检查支持

- **Tools**
  - 9 个 Agent 工具
  - 沙箱安全执行
  - 符号链接逃逸检测

- **Memory**
  - SQLite + 向量嵌入
  - FTS5 全文搜索
  - 混合搜索 (向量 + 关键词)
  - Markdown 后端

- **Security**
  - ChaCha20-Poly1305 加密存储
  - 沙箱策略 (白名单)
  - 设备配对

- **Tunnel**
  - 5 种隧道类型
  - 自动检测可用隧道

- **Gateway**
  - Axum HTTP 服务器
  - Webhook 处理

- **Daemon**
  - 自主运行时
  - Cron 调度
  - 心跳监控

- **Skills**
  - TOML manifest 解析
  - SKILL.md 加载

### 改进

- 移除全局 `#![allow(dead_code)]`
- 清理未使用代码
- 更新测试数量: 1,017 → 1,811

### 修复

- Gemini provider OAuth 令牌加载
- Windows icacls 命令构建

### 文档

- 新建 `/docs` 目录结构
- 架构文档 (LLM 友好)
- 更新 CLAUDE.md

---

## [Unreleased]

### 计划中

- Streaming 响应支持
- 多模态输入 (图片/音频)
- Function Calling 标准化
- Web UI 管理界面
