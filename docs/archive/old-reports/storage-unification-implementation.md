# 运行时文件存储统一到 ~/.codecoder/ 实施报告

**状态**: ✅ 已完成
**实施时间**: 2026-03-01

## 变更概述

本次修改将所有运行时文件存储路径统一到 `~/.codecoder/` 目录下，替代之前分散在多个 XDG 目录的存储方式。

## 实施内容

### 1. TypeScript 路径定义 (`packages/ccode/src/global/index.ts`)

**变更前**:
- 使用 `xdg-basedir` 库分散存储
- `~/.local/share/ccode/` - 数据目录
- `~/.cache/ccode/` - 缓存目录
- `~/.local/state/ccode/` - 状态目录

**变更后**:
```
~/.codecoder/
├── data/       # 存储、内存、快照
├── cache/      # 模型定义缓存、node_modules
├── state/      # KV 状态、提示历史
├── logs/       # 统一日志目录
└── bin/        # LSP 二进制
```

**关键代码变更**:
- 移除 `xdg-basedir` 依赖
- 所有路径改为 getter，从 `configDir()` 派生
- 保留 `CCODE_TEST_HOME` 环境变量支持测试隔离

### 2. Rust Daemon 日志路径 (`services/zero-cli/src/daemon/mod.rs`)

**变更前**: 使用相对路径 `../.logs`

**变更后**: 使用 `zero_common::config::config_dir().join("logs")`

### 3. 测试预加载 (`packages/ccode/test/preload.ts`)

**变更前**: 设置 `XDG_*_HOME` 环境变量

**变更后**: 仅设置 `CCODE_TEST_HOME`，预创建 `.codecoder/` 目录结构

### 4. 依赖移除 (`packages/ccode/package.json`)

移除 `xdg-basedir: "5.1.0"` 依赖

### 5. 迁移脚本 (`packages/ccode/src/migration/storage-unification.ts`)

创建迁移脚本用于将旧位置数据迁移到新位置：

```bash
# 检查迁移状态
bun run src/migration/storage-unification.ts

# 预览迁移（不执行）
bun run src/migration/storage-unification.ts --migrate --dry-run

# 执行迁移
bun run src/migration/storage-unification.ts --migrate
```

## 验证结果

- ✅ TypeScript 类型检查通过
- ✅ Rust 编译通过
- ✅ 存储和配置相关测试全部通过 (77 tests)
- ✅ 核心单元测试通过 (1274+ tests)
- ✅ 迁移脚本功能验证通过

## 新目录结构

```
~/.codecoder/
├── config.json           # 核心配置
├── secrets.json          # 凭证文件
├── trading.json          # 交易模块配置
├── channels.json         # IM 渠道配置
├── providers.json        # LLM 提供商配置
├── data/
│   ├── storage/          # 会话、消息、项目存储
│   ├── memory/           # SQLite 内存数据库
│   └── snapshot/         # 快照
├── cache/
│   ├── models.json       # 模型定义缓存
│   ├── package.json      # NPM 包追踪
│   ├── node_modules/     # 动态安装的包
│   └── version           # 缓存版本标记
├── state/
│   ├── kv.json           # 键值状态
│   └── prompt-history.jsonl  # 提示历史
├── logs/
│   ├── trace-*.jsonl     # TypeScript 跟踪日志
│   ├── *.log             # 传统日志文件
│   ├── zero-gateway.log  # Rust 服务日志
│   ├── zero-channels.log
│   ├── zero-workflow.log
│   └── zero-trading.log
├── bin/
│   ├── rg               # ripgrep
│   ├── gopls            # Go LSP
│   └── ...
└── workspace/           # 工作空间
```

## 后续建议

1. **用户通知**: 首次启动时检测旧数据并提示用户运行迁移脚本
2. **清理旧目录**: 2 个版本后可考虑自动删除旧 XDG 目录
3. **文档更新**: 更新用户文档说明新的存储位置

## 修改文件列表

| 文件 | 变更类型 |
|------|---------|
| `packages/ccode/src/global/index.ts` | 修改 |
| `packages/ccode/test/preload.ts` | 修改 |
| `packages/ccode/package.json` | 修改 |
| `services/zero-cli/src/daemon/mod.rs` | 修改 |
| `packages/ccode/src/migration/storage-unification.ts` | 新增 |
