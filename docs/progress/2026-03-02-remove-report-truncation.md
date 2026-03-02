# 移除宏观报告截断逻辑

**日期**: 2026-03-02
**状态**: 已完成

## 问题描述

周度宏观经济简报在发送时被截断，显示 `[报告已截断]`，但 `zero-channels` 已经实现了更好的长消息处理逻辑。

## 根因分析

在 `services/zero-trading/src/macro_agent/report.rs` 的 `format_telegram_message` 函数中存在硬编码的 3500 字符截断逻辑：

```rust
// 旧代码
let content = if report.content.len() > 3500 {
    format!("{}...\n\n_[报告已截断]_", &report.content[..3500])
} else {
    report.content.clone()
};
```

这与 `zero-channels` 中已实现的长消息处理逻辑冲突：
- 4096字符以下：正常发送
- 4096-20000字符：自动拆分为多条消息
- 20000字符以上：自动转换为 Markdown 文件附件

## 解决方案

移除 `format_telegram_message` 中的截断逻辑，让 `zero-channels` 的专门消息发送层处理长度限制。

## 修改内容

### 修改的文件

1. `services/zero-trading/src/macro_agent/report.rs`
   - 移除 3500 字符截断逻辑
   - 添加文档说明长消息由 zero-channels 处理
   - 添加新测试 `test_format_telegram_message_no_truncation`

## 数据流

```
MacroReportGenerator::generate_and_send
  └── notification.send_alert(&report.title, &message)
        └── NotificationClient::send_message (HTTP POST)
              └── /api/v1/send (routes.rs)
                    └── OutboundRouter::send_direct
                          └── TelegramChannel::send
                                ├── > 20000 chars → Markdown 文件附件
                                ├── 4096-20000 chars → 多条消息
                                └── < 4096 chars → 正常发送
```

## 测试验证

```bash
cargo test test_format_telegram -- --nocapture
# test macro_agent::report::tests::test_format_telegram_message ... ok
# test macro_agent::report::tests::test_format_telegram_message_no_truncation ... ok
```

## 相关常量

- `FILE_THRESHOLD = 20000` (services/zero-channels/src/lib.rs)
- `TELEGRAM_MAX_MESSAGE_LEN = 4096` (services/zero-channels/src/lib.rs)
