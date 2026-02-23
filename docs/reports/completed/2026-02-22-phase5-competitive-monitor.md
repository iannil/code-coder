# Phase 5: 竞品与数据监控 (Competitive Intelligence Monitor)

**完成时间**: 2026-02-22
**状态**: ✅ 完成

## 实现概述

实现了自动化竞品监控系统，支持：
- 网页内容抓取（支持 CSS 选择器）
- RSS 订阅解析
- LLM 智能总结分析
- 定时生成早报
- IM 群组通知

## 核心文件

| 文件 | 操作 | 描述 |
|------|------|------|
| `services/zero-workflow/src/monitor_bridge.rs` | 新建 | 监控核心逻辑（~850行） |
| `services/zero-workflow/src/lib.rs` | 修改 | 添加模块导出和调度器集成 |
| `services/zero-workflow/src/routes.rs` | 修改 | 添加 Monitor API 端点 |
| `services/zero-common/src/config.rs` | 修改 | 添加 MonitorConfig 配置结构 |
| `services/zero-workflow/Cargo.toml` | 修改 | 添加 scraper, rss, futures 依赖 |

## API 端点

```
GET  /api/v1/monitor/status           # 获取监控状态
GET  /api/v1/monitor/reports          # 获取所有报告
POST /api/v1/monitor/tasks            # 创建监控任务
POST /api/v1/monitor/:task_id/run     # 手动触发监控
GET  /api/v1/monitor/:task_id/reports # 获取任务报告
DELETE /api/v1/monitor/:task_id       # 删除监控任务
```

## 配置示例

```json
{
  "workflow": {
    "monitor": {
      "enabled": true,
      "tasks": [
        {
          "id": "daily-competitive",
          "name": "每日竞品早报",
          "schedule": "0 0 9 * * *",
          "sources": [
            {
              "id": "competitor-a",
              "name": "竞品A官网",
              "url": "https://competitor-a.com/news",
              "source_type": "website",
              "selector": ".news-list"
            },
            {
              "id": "competitor-b-blog",
              "name": "竞品B博客",
              "url": "https://competitor-b.com/blog/feed",
              "source_type": "rss"
            }
          ],
          "notification": {
            "channel_type": "feishu",
            "channel_id": "ops-group-123",
            "template": "daily_brief"
          }
        }
      ]
    }
  }
}
```

## 架构设计

```
Cron Scheduler (每天 9:00)
    │
    ▼
MonitorBridge.run_monitor()
    │
    ├── fetch_source() ─────────────┐
    │   (并行抓取多个源)              │
    │                                │
    │   ┌────────┬────────┬────────┐ │
    │   │ 网页A  │  RSS   │  网页B │ │
    │   └────┬───┴────┬───┴────┬───┘ │
    │        │        │        │      │
    ▼        ▼        ▼        ▼      │
extract_content()                     │
    │                                 │
    ▼                                 │
generate_report() ◄───────────────────┘
    │ (调用 LLM via CodeCoder API)
    ▼
send_report() ──► IM 群组 (飞书/企微/钉钉)
```

## 关键特性

### 1. 多源并行抓取
使用 `futures::future::join_all` 并行抓取所有监控源，提高效率。

### 2. 智能内容提取
- **网页**: 支持 CSS 选择器精确提取内容区域
- **RSS**: 自动解析最新 10 条条目

### 3. LLM 分析报告
调用 CodeCoder API 生成结构化分析：
- 今日要点 (highlights)
- 各竞品动态摘要
- 整体分析
- 行动建议

### 4. 多种报告模板
- `daily_brief`: 简洁早报格式
- `detailed`: 详细分析报告
- `comparison`: 竞品对比表格

### 5. IM 通知集成
复用 Zero Channels 服务，支持：
- 飞书
- 企业微信
- 钉钉

## 测试覆盖

新增测试用例：
- `test_source_type_from_str` - 源类型解析
- `test_extract_json_from_response` - JSON 提取
- `test_truncate_content` - 内容截断
- `test_extract_text_from_html` - HTML 文本提取
- `test_format_daily_brief` - 报告格式化
- `test_llm_response_parsing` - LLM 响应解析
- `test_monitor_status_empty` - API 状态检查
- `test_create_monitor_task` - 任务创建
- `test_list_monitor_reports_empty` - 报告列表
- `test_monitor_config` - 配置解析
- `test_monitor_config_defaults` - 默认值处理

## 验证方法

```bash
# 1. 启动服务
./ops.sh start

# 2. 手动触发监控任务
curl -X POST http://localhost:4406/api/v1/monitor/daily-competitive/run

# 3. 查看最新报告
curl http://localhost:4406/api/v1/monitor/daily-competitive/reports?limit=1

# 4. 检查监控状态
curl http://localhost:4406/api/v1/monitor/status
```

## 依赖版本

```toml
scraper = "0.22"      # HTML 解析和 CSS 选择器
rss = "2.0"           # RSS feed 解析
futures = "0.3"       # 异步并行处理
```

## 后续扩展建议

1. **更多数据源**
   - Twitter API 集成
   - 微博/小红书爬虫
   - Google Alerts RSS

2. **增强分析**
   - 历史对比（本周 vs 上周）
   - 趋势分析图表
   - 关键词情感分析

3. **告警机制**
   - 关键词即时通知
   - 异常检测告警
   - 竞品重大事件推送

4. **数据持久化**
   - 报告存储到数据库
   - 历史报告查询
   - 数据导出功能
