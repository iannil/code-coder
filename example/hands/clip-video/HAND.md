---
# Clip - 视频内容创作者 Hand
# 专门用于短视频脚本生成、内容策划和创意开发

id: "clip-video-creator"
name: "Clip Video Creator"
description: "自动化短视频内容创作流程，包括选题、脚本生成和内容优化"
version: "1.0.0"
author: "CodeCoder"

# 调度配置 - 每6小时生成一次新内容
schedule: "0 */6 * * *"

# 多 Agent Pipeline - 顺序执行
pipeline:
  mode: "sequential"
  agents:
    - name: "explore"
      role: "content_explorer"
      params:
        topics: ["AI技术", "编程技巧", "科技趋势"]
        sources: ["twitter", "reddit", "hackernews"]
    - name: "writer"
      role: "script_writer"
      params:
        format: "short_video"
        duration: "60s"
        style: "engaging"
    - name: "proofreader"
      role: "content_refiner"
      params:
        tone: "professional"
        platform: "tiktok"

# 自治级别 - 中等自主
autonomy:
  level: "wild"
  score_threshold: 50
  approval_threshold: 6.5

# CLOSE 框架集成
decision:
  use_close: true
  auto_continue: true
  web_search: true
  evolution: true

# 风险控制
risk_control:
  max_tokens: 10000
  max_cost_usd: 0.50
  max_duration_sec: 300

# 记忆存储路径
memory_path: "hands/clip/{date}.md"

# 输出配置
output:
  format: "markdown"
  include_sources: true
  include_timestamps: true

# 启用状态
enabled: true
---

# Clip Video Creator Hand

## 概述

此 Hand 专注于短视频内容的自动化创作，通过多 Agent Pipeline 实现从选题探索到脚本生成的完整流程。

## 工作流程

1. **探索阶段** (`explore` agent)
   - 扫描热门话题和趋势
   - 收集创意素材
   - 分析目标受众偏好

2. **创作阶段** (`writer` agent)
   - 基于探索结果生成脚本
   - 优化标题和钩子
   - 设计节奏结构

3. **优化阶段** (`proofreader` agent)
   - 内容润色和校对
   - 平台适配优化
   - 添加行动号召

## 输出内容

每个执行周期将生成：
- 标题建议（3-5个选项）
- 完整脚本（60秒版本）
- 关键视觉提示
- 发布建议（最佳时间、标签）

## 使用场景

- TikTok/Reels/Shorts 日常内容更新
- 知识科普类账号运营
- 技术博主内容储备
