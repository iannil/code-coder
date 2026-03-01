# IM 长消息完整发送实施报告

## 概述

**日期**: 2026-03-01
**状态**: 已完成
**影响范围**: zero-channels, zero-workflow

## 问题描述

用户反馈 HAND 执行结果（如周度价值分析报告）发送到 Telegram 时内容不完整。

**根本原因**:
1. `zero-workflow/notification_bridge.rs` - Brief 模板截断到 500 字符，Detailed 截断到 3000 字符
2. `zero-channels/progress.rs` - 工具输出截断到 2000 字符
3. Telegram 层的分片机制本身正常，但上游已截断内容

## 解决方案

采用**智能分发策略**：根据内容长度选择最佳发送方式

| 内容长度 | 发送方式 |
|---------|---------|
| ≤ 4096 字符 | 单条消息 |
| 4097 - 20000 字符 | 多条消息（自动分片） |
| > 20000 字符 | 转为 Markdown 文件发送 |

## 修改清单

### 1. `services/zero-channels/src/lib.rs`
新增常量:
```rust
/// Telegram single message maximum length (characters).
pub const TELEGRAM_MAX_MESSAGE_LEN: usize = 4096;

/// Threshold for converting long messages to file attachments.
pub const FILE_THRESHOLD: usize = 20000;
```

### 2. `services/zero-channels/src/telegram/mod.rs`
修改 `send()` 方法，添加文件转换逻辑:
- 导入 `FILE_THRESHOLD` 常量
- 超过 20000 字符的消息转为 Markdown 文件发送
- 添加日志记录文件转换事件

### 3. `services/zero-workflow/src/hands/notification_bridge.rs`
移除截断逻辑:
- `format_brief()`: 移除 500 字符截断
- `format_detailed()`: 移除 3000 字符截断
- 添加注释说明由 Telegram 层处理分片/文件转换

### 4. `services/zero-channels/src/progress.rs`
调整常量:
- `MAX_OUTPUT_LENGTH`: 从 2000 增加到 50000
- 更新注释说明下游处理消息大小限制

## 测试验证

### 单元测试
新增测试用例:
- `file_threshold_constant`: 验证 FILE_THRESHOLD 值
- `split_message_at_file_threshold`: 验证阈值边界分片
- `split_message_above_file_threshold`: 验证超阈值处理

### 测试结果
```
running 34 tests (telegram module)... test result: ok
running 6 tests (notification_bridge module)... test result: ok
running 24 tests (progress module)... test result: ok
```

### 编译验证
```bash
cargo build --release -p zero-channels -p zero-workflow
# Finished `release` profile [optimized] target(s)
```

## 架构说明

```
应用层 (notification_bridge.rs, progress.rs)
    │ 传递完整内容
    ▼
传输层 (telegram/mod.rs)
    │ 智能判断
    ├── ≤4096: 单条发送
    ├── 4096-20000: 分片发送
    └── >20000: 文件发送
```

## 验证方法

1. **手动验证**:
   ```bash
   # 触发 HAND
   curl -X POST http://localhost:4432/api/v1/hands/weekly-value-analysis/trigger

   # 检查 Telegram 消息
   # - 中等长度 (<20000): 应收到多条消息
   # - 超长 (>20000): 应收到 .md 文件
   ```

2. **日志检查**:
   - 查找 "Converting long message to file attachment" 日志

## 后续建议

1. 考虑为其他 IM 渠道（Feishu、WeChat Work）实现类似的文件转换逻辑
2. 监控文件转换的使用频率，必要时调整阈值
3. 考虑添加配置项允许用户自定义阈值
