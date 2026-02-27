# ops.sh tail all 日志格式优化

## 修改时间

2026-02-27

## 修改内容

优化 `tail_all_logs()` 函数，实现统一的日志输出格式。

### 新增函数 (位于 `get_service_color()` 之后)

1. **strip_ansi()**: 清理 ANSI 转义序列
2. **get_level_color()**: 获取日志级别颜色
3. **extract_log_level()**: 从日志行提取日志级别
4. **extract_timestamp()**: 从日志行提取或添加时间戳
5. **format_json_fields()**: 格式化 JSON 日志为 key=value 格式
6. **format_log_line()**: 格式化单行日志输出

### 输出格式

统一格式: `MM-DD HH:MM:SS | service | LEVEL | message`

示例:
```
02-27 10:30:45 |         api | INFO  | server_started port=4400
02-27 10:30:45 |        web | INFO  | Server starting on port 4400
02-27 10:30:46 | zero-daemon | WARN  | Health check timeout
02-27 10:30:47 | zero-gateway | ERROR | Connection failed
```

### 颜色方案

**服务颜色** (保持现有):
- api: 绿色
- web: 蓝色
- zero-daemon: 紫色
- zero-gateway: 黄色
- zero-channels: 亮红色
- zero-workflow: 亮蓝色
- whisper: 青色
- redis: 红色

**日志级别颜色**:
- ERROR: 红色
- WARN: 黄色
- INFO: 绿色
- DEBUG: 灰色
- TRACE: 暗灰色

### JSON 解析

自动解析 JSON 格式日志，提取以下字段:
- `timestamp`: 转换为 `MM-DD HH:MM:SS` 格式
- `level`/`severity`: 日志级别
- `event`/`message`/`fields.message`: 主要消息
- 其他字段: 转换为 `key=value` 格式

### 技术要点

1. **macOS sed 兼容性**: 使用字面空格而非 `\s` (BSD sed 不支持)
2. **jq 可选**: JSON 解析优先使用 jq，降级为 grep/sed
3. **进程替换**: 使用 `done < <(command)` 避免管道信号问题

## 验证

运行以下命令验证:

```bash
# 测试单个函数
source ops.sh
echo "" | format_log_line '{"timestamp":"2026-02-27T10:30:45.123Z","level":"INFO","event":"test"}' "api" "\033[0;32m"

# 实时测试 (需要服务运行)
./ops.sh tail all
```

## 相关文件

- `/Users/iannil/Code/zproducts/code-coder/ops.sh`
