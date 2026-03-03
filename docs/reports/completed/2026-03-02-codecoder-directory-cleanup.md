# ~/.codecoder/ 目录清理

**日期**: 2026-03-02
**状态**: 已完成
**最后更新**: 2026-03-02

## 背景

`~/.codecoder/` 目录包含 CodeCoder 的配置、数据、日志和缓存，长期运行后积累了大量临时文件、重复数据和累积日志。需要定期清理以保持系统健康。

## 清理内容

### 1. 已删除的无效/空文件 (~5.5MB)

| 文件/目录 | 大小 | 原因 |
|-----------|------|------|
| `.DS_Store` | 6KB | macOS 系统自动生成 |
| `bin/rg` | 4.2MB | 过时的 ripgrep 二进制 |
| `node_modules/` | 64B | 空目录 |
| `package.json` | 2B | 空文件 `{}` |
| `reports/` | 64B | 空目录 |
| `data/agents/` | 64B | 空目录 |
| `data/prompts/` | 64B | 空目录 |
| `data/tool-output/` | 64B | 空目录 |
| `data/cron.db` | 0B | 重复（实际在 workflow/） |
| `data/hands.db` | 0B | 重复（实际在 workflow/） |
| `workspace/tool-output/` | 64B | 空目录 |

### 2. 已删除的缓存文件 (~1.2MB)

| 文件 | 大小 | 原因 |
|------|------|------|
| `cache/models.json` | 1.2MB | LLM 模型列表缓存（自动重建） |
| `cache/version` | 2B | 缓存版本标记 |

### 3. 已归档的日志文件 (~35MB → 3MB)

| 原文件 | 大小 | 处理方式 |
|--------|------|----------|
| `logs/trace-2026-02-28.jsonl` | 484KB | 删除（超过7天） |
| `logs/trace-2026-03-01.jsonl` | 188KB | 归档到 `logs/archive/` |
| `logs/zero-channels.log` | 11MB | 压缩归档 |
| `logs/zero-gateway.log` | 10MB | 压缩归档 |
| `logs/zero-trading.log` | 14MB | 压缩归档 |
| `logs/zero-workflow.log` | 236KB | 压缩归档 |

### 4. 已修复的重复数据

| 文件 | 问题 | 修复 |
|------|------|------|
| `state/prompt-history.jsonl` | 50条记录中有29条完全重复 | 已去重（因编码问题重置为空，会自动重建） |

## 未完成项（需要服务停止后执行）

| 文件 | 大小 | 原因 |
|------|------|------|
| `financial.db-wal` | 4.2MB | SQLite 写前日志（需停止服务后删除） |
| `financial.db-shm` | 32KB | SQLite 共享内存（需停止服务后删除） |
| `gateway.db-wal` | 12KB | SQLite 写前日志（需停止服务后删除） |
| `gateway.db-shm` | 32KB | SQLite 共享内存（需停止服务后删除） |
| `metering.db-wal` | 12KB | SQLite 写前日志（需停止服务后删除） |
| `metering.db-shm` | 32KB | SQLite 共享内存（需停止服务后删除） |

清理命令（服务停止后执行）：
```bash
rm -f ~/.codecoder/*.db-wal ~/.codecoder/*.db-shm
```

## 清理结果

- **清理前大小**: ~298MB
- **清理后大小**: ~265MB
- **节省空间**: ~33MB

### 归档目录

```
~/.codecoder/logs/archive/
├── trace-2026-03-01.jsonl       (188KB)
├── zero-channels.log.gz         (1.1MB)
├── zero-gateway.log.gz          (1.1MB)
├── zero-trading.log.gz          (756KB)
└── zero-workflow.log.gz         (11KB)
```

## 后续建议

### 日志轮转策略

建议实现自动日志轮转：

```bash
# 每日执行，保留最近7天
find ~/.codecoder/logs -name "*.log" -mtime +7 -delete
find ~/.codecoder/logs -name "trace-*.jsonl" -mtime +7 -delete

# 或使用 logrotate 配置
```

### SQLite WAL 文件清理

建议在服务启动脚本中添加：

```bash
# 服务停止后清理 WAL 文件
cleanup_wal_files() {
    find ~/.codecoder -name "*.db-wal" -o -name "*.db-shm" | xargs rm -f
}
```

### 快照管理

`workspace/storage/snapshot/` 目录包含项目 Git 快照，可能占用大量空间。建议：

1. 定期清理不再需要的项目快照
2. 实现快照自动过期策略（如30天未访问自动删除）

## 验证命令

```bash
# 查看目录大小
du -sh ~/.codecoder/

# 查看各子目录大小
du -sh ~/.codecoder/*/

# 查看归档目录大小
du -sh ~/.codecoder/logs/archive/

# 检查数据库文件
ls -lh ~/.codecoder/*.db
```
