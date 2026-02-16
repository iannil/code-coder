# Storage 层数据完整性增强

## 完成日期
2026-02-16

## 概述
增强 Storage 层的健壮性，确保 session/message/part 数据的可靠性。

## 修改文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `packages/ccode/src/storage/storage.ts` | 修改 | 添加错误处理、备份、恢复、健康检查功能 |
| `packages/ccode/src/util/filesystem.ts` | 修改 | 添加原子写入工具 |

## 实现内容

### Phase 1: JSON 解析错误处理 + 自动恢复 ✅

1. **CorruptedError 错误类型**：新增专用错误类型，包含 path、message、originalError、recovered 字段

2. **isolateCorrupted() 函数**：将损坏文件移动到 `_corrupted` 目录，保留原始内容便于分析

3. **read() 自动恢复**：检测到 JSON 解析错误时，自动尝试从备份恢复

### Phase 2: 原子写入 ✅

1. **atomicWrite() 函数**（filesystem.ts）：
   - 先写入临时文件 `.{timestamp}.tmp`
   - 再使用 rename 操作替换目标文件
   - 失败时清理临时文件

2. **write() 和 update() 改用原子写入**：防止写入过程中断导致数据损坏

### Phase 3: 备份机制 ✅

1. **backup() 函数**：
   - 写入前自动备份到 `_backup/{key_path}/{filename}.{timestamp}.json`
   - 保留最多 3 份备份，超过 7 天自动清理

2. **cleanupBackups() 函数**：按数量和时间清理旧备份

3. **restore() 函数**：从最新的有效备份恢复数据

### Phase 4: 健康检查 ✅

1. **HealthReport 接口**：包含 total、healthy、corrupted、orphaned 统计

2. **healthCheck() 函数**：扫描指定前缀下的所有文件，检测 JSON 有效性

3. **listCorrupted() 函数**：列出 `_corrupted` 目录中的所有隔离文件

4. **clearCorrupted() 函数**：清理已隔离的损坏文件

## 存储目录结构

```
~/.local/share/ccode/storage/
├── _backup/           # 备份文件目录
│   ├── session/
│   │   └── {projectID}/
│   │       └── {sessionID}.{timestamp}.json
│   └── message/
│       └── ...
├── _corrupted/        # 损坏文件隔离目录
│   └── {filename}.{timestamp}
├── session/           # 原始数据
├── message/
└── part/
```

## 测试建议

1. **单元测试**：
   - 写入损坏 JSON 后读取，验证抛出 CorruptedError
   - 验证损坏文件被移动到 `_corrupted` 目录
   - 模拟写入中断，验证原子写入安全性
   - 创建备份后恢复，验证功能正常

2. **手动测试**：
   ```bash
   # 在 packages/ccode 目录下运行测试
   bun test test/storage/

   # 手动测试损坏恢复流程
   # 1. 创建一个 session
   # 2. 手动损坏 ~/.ccode/storage/session/{projectID}/{sessionID}.json
   # 3. 重新加载，验证错误处理和隔离
   # 4. 调用 restore，验证恢复
   ```

## 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 原子写入增加 I/O | 性能略降 | rename 操作很快，影响可忽略 |
| 备份占用空间 | 磁盘使用增加 | 限制备份数量（3份）和保留时间（7天） |
| 恢复逻辑复杂 | 可能引入新 bug | 充分测试边界情况 |

## 验收状态

- [x] Phase 1: JSON 解析错误处理
- [x] Phase 2: 原子写入
- [x] Phase 3: 备份机制
- [x] Phase 4: 健康检查
