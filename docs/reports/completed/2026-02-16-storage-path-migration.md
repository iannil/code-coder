# 存储路径迁移报告：~/.zero-bot → ~/.codecoder

## 完成时间
2026-02-16

## 变更概述

将 ZeroBot 服务的用户数据存储路径从 `~/.zero-bot/` 迁移到 `~/.codecoder/`，统一项目命名。

## 修改范围

### Rust 核心代码 (services/zero-bot/src/)

| 文件 | 修改内容 |
|------|----------|
| `runtime/native.rs` | `storage_path()` 返回值: `.zero-bot` → `.codecoder` |
| `runtime/native.rs` | 测试断言: `contains("zero-bot")` → `contains("codecoder")` |
| `config/schema.rs` | `Config::default()` 默认路径 |
| `config/schema.rs` | `load_from_codecoder()` 工作区路径 |
| `config/schema.rs` | `load_or_init()` 目录创建路径和错误消息 |
| `security/secrets.rs` | 注释中的路径说明 |
| `channels/mod.rs` | 用户提示消息 |
| `skills/mod.rs` | 文档注释和帮助信息 |
| `skills/mod.rs` | 测试断言 |
| `onboard/wizard.rs` | 目录创建路径和用户提示 |
| `integrations/mod.rs` | 用户提示消息 |

### TypeScript 代码 (packages/)

| 文件 | 修改内容 |
|------|----------|
| `ccode/src/memory-zerobot/provider.ts` | `DEFAULT_WORKSPACE` 常量 |
| `ccode/src/memory-zerobot/types.ts` | 类型注释 |
| `ccode/src/agent/memory-bridge.ts` | `dbPath` 默认值 |
| `memory/src/types.ts` | `SqliteConfig.dbPath` 默认值 |
| `memory/src/types.ts` | `DEFAULT_CONFIG` 常量 |
| `memory/test/factory.test.ts` | 测试断言 |

### 文档

| 文件 | 修改内容 |
|------|----------|
| `README.md` | 配置示例 |
| `README_CN.md` | 配置示例 |
| `services/zero-bot/examples/config.toml.example` | 示例配置路径 |
| `services/zero-bot/CLAUDE.md` | 项目说明 |
| `services/zero-bot/docs/reference/CLI.md` | CLI 文档 |
| `services/zero-bot/docs/guides/QUICK_START.md` | 快速开始 |
| `services/zero-bot/docs/guides/CONFIGURATION.md` | 配置说明 |
| `services/zero-bot/docs/architecture/DATA_FLOW.md` | 架构图 |
| `services/zero-bot/docs/development/CHANGELOG.md` | 变更日志 |
| `docs/RUNBOOK.md` | 运维手册 |
| `docs/guides/memory-architecture.md` | 记忆架构 |
| `docs/diagrams/memory-access-matrix.mmd` | Mermaid 图 |
| `docs/diagrams/memory-access-matrix.svg` | 重新生成的 SVG 图 |
| `docs/reports/completed/*.md` | 多个完成报告 |

### 保持不变的内容

- CLI 命令：`zero-bot onboard`, `zero-bot agent` 等
- 服务标签：`com.zero-bot.daemon`
- systemd 服务：`zero-bot.service`
- 二进制文件名：`zero-bot`
- skills 同步标记：`.zero-bot-open-skills-sync`

## 验证结果

### Rust 验证
```
cargo check ✅ (1 unrelated warning)
cargo test  ✅ (所有测试通过)
```

### TypeScript 验证
```
packages/memory: bun test ✅ (49 tests passed)
packages/memory: typecheck ✅
```

### 路径检查
```
grep "\.zero-bot" ✅ (无残留引用)
```

## 迁移说明

现有用户需要手动迁移数据：
```bash
mv ~/.zero-bot ~/.codecoder
```

此为破坏性变更，用户升级后需要执行上述迁移命令。
