# 周度宏观经济简报重复发送问题修复

**日期**: 2026-03-02
**状态**: 已完成

## 问题描述

周度宏观经济简报在今早（2026-03-02）被发送了多次：
- 01:21:09 - 第一次发送
- 01:38:16 - 第二次发送（服务重启后）
- 01:42:39 - 第三次发送（服务再次重启后）

## 根本原因

`zero-trading/src/macro_agent/report.rs` 中的 `ReportState` 是纯内存状态：

```rust
struct ReportState {
    last_weekly: Option<DateTime<Utc>>,
    last_monthly: Option<DateTime<Utc>>,
    ...
}
```

服务启动时初始化为 `None`，导致每次重启后都认为从未发送过报告，于是立即触发发送。

## 修复方案

### 1. 新增持久化状态结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentReportState {
    pub last_weekly: Option<String>,       // ISO8601 格式
    pub last_monthly: Option<String>,
    pub last_daily_morning: Option<String>,
    pub last_daily_afternoon: Option<String>,
}
```

### 2. 状态文件位置

```
~/.codecoder/workflow/report_state.json
```

### 3. 关键修改

| 方法 | 修改内容 |
|------|----------|
| `MacroReportGenerator::new()` | 启动时从磁盘加载状态 |
| `update_state()` | 发送成功后同时更新内存和磁盘状态 |

### 4. 修改的文件

- `services/zero-trading/src/macro_agent/report.rs`
  - 新增 `PersistentReportState` 结构
  - 新增 `state_file_path()` 函数
  - 修改 `new()` 加载持久化状态
  - 修改 `update_state()` 保存到磁盘

## 验证

1. 编译通过：`cargo check -p zero-trading` ✓
2. 测试通过：`cargo test -p zero-trading` ✓
3. 创建状态文件防止今天再次发送 ✓

## 状态文件内容

```json
{
  "last_weekly": "2026-03-02T01:43:55.030251+00:00",
  "last_monthly": null,
  "last_daily_morning": "2026-03-02T01:44:32.721098+00:00",
  "last_daily_afternoon": null
}
```

## 后续建议

1. 服务重启后会自动加载状态，不会重复发送
2. 如需手动触发报告，可删除状态文件中对应字段
3. 考虑添加 CLI 命令管理报告状态
