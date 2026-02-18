# Telegram 文档接收支持实现报告

**日期**: 2026-02-18
**状态**: 已完成

## 概述

为 ZeroBot 实现了 Telegram 文档接收功能，使用户可以通过发送文档让 ZeroBot 分析内容。

## 实现内容

### 1. 依赖添加

在 `Cargo.toml` 中添加 PDF 解析依赖：
```toml
pdf-extract = "0.7"
```

### 2. 文档处理常量

在 `telegram.rs` 中定义：
- `MAX_INLINE_SIZE`: 32KB - 小于此大小的文件直接注入消息内容
- `TEXT_MIME_TYPES`: 支持的文本 MIME 类型列表
- `TEXT_EXTENSIONS`: 支持的文本文件扩展名列表

### 3. 核心函数实现

| 函数 | 功能 |
|------|------|
| `is_text_file()` | 根据 MIME 类型或扩展名判断是否为文本文件 |
| `extract_document_content()` | 从文档提取文本内容 |
| `extract_pdf_content()` | 使用 pdf-extract 提取 PDF 文本 |
| `format_document_message()` | 格式化文档消息供 Agent 使用 |
| `format_file_size()` | 格式化文件大小显示 |
| `save_to_workspace()` | 保存文件到工作区（预留功能） |

### 4. 消息处理流程

在 `listen()` 方法中添加 document 消息类型处理：
1. 检查文件大小（不超过 20MB）
2. 下载文档内容
3. 根据文件类型提取文本
4. 格式化消息并发送给 Agent

### 5. 支持的文件类型

| 类型 | 扩展名 | 处理方式 |
|------|--------|----------|
| 纯文本 | .txt, .md, .log | UTF-8 解码 |
| JSON | .json | UTF-8 解码 |
| CSV | .csv | UTF-8 解码 |
| XML | .xml | UTF-8 解码 |
| 代码文件 | .py, .rs, .js, .ts, .java, .go... | UTF-8 解码 |
| PDF | .pdf | pdf-extract 提取 |

### 6. 消息格式

小文件 (<32KB)：
```
[Document: report.txt (2.3KB)]
--- Content Start ---
文件内容...
--- Content End ---

用户附加消息（如果有）
```

大文件 (≥32KB)：
```
[Document: report.txt (50.0KB)]
--- Content Preview ---
文件内容预览（前2000字符）...
[Content truncated, 50.0KB total]
--- End Preview ---
```

## 测试覆盖

新增 13 个单元测试：
- `is_text_file_by_mime_type`
- `is_text_file_by_extension`
- `extract_text_content`
- `extract_text_content_from_json`
- `extract_unsupported_format_returns_none`
- `extract_non_utf8_text_uses_lossy_conversion`
- `format_file_size_bytes`
- `format_file_size_kilobytes`
- `format_file_size_megabytes`
- `format_document_message_small_text_file`
- `format_document_message_with_caption`
- `format_document_message_unsupported_format`
- `format_document_message_large_file_truncated`

## 验证结果

- `cargo build`: 通过
- `cargo clippy -- -D warnings`: 通过
- `cargo test`: 全部 986 个测试通过

## 修改的文件

| 文件 | 修改类型 |
|------|----------|
| `services/zero-bot/Cargo.toml` | 添加 pdf-extract 依赖 |
| `services/zero-bot/src/channels/telegram.rs` | 添加文档处理逻辑和测试 |
| `services/zero-bot/src/stt/compatible.rs` | 修复 clippy 警告 |
| `services/zero-bot/src/stt/mod.rs` | 配合 compatible.rs 修改 |
| `services/zero-bot/src/tools/codecoder.rs` | 修复 clippy 警告 |
| `services/zero-bot/src/agent/confirmation.rs` | 修复 clippy 警告 |

## 后续可扩展

1. 添加配置选项控制文档处理行为
2. 支持更多文件格式（Word、Excel 等）
3. 大文件保存到工作区功能（预留了 `save_to_workspace` 函数）
