# 清理清单 (2026-03-01)

**生成时间**: 2026-03-01
**目的**: 记录待清理的 TODO、废弃配置和冗余文档

---

## 一、待清理 TODO/FIXME

### 高优先级 - Web Store API 占位符

这些 TODO 是 Web 前端 store 中的占位实现，需要在 API 端点就绪后替换。

| 文件 | 行号 | 内容 | 建议 |
|------|------|------|------|
| `packages/web/src/stores/gateway.ts` | 75 | TODO: Replace with actual API call | 实现 `fetchGatewayStatus` |
| `packages/web/src/stores/gateway.ts` | 90 | TODO: Replace with actual API call | 实现 `fetchMeteringData` |
| `packages/web/src/stores/gateway.ts` | 117 | TODO: Replace with actual API call | 实现 `updateMeteringConfig` |
| `packages/web/src/stores/gateway.ts` | 133 | TODO: Replace with actual API call | 实现 `resetMeteringData` |
| `packages/web/src/stores/tunnel.ts` | 63 | TODO: Replace with actual API call | 实现 `fetchTunnels` |
| `packages/web/src/stores/tunnel.ts` | 77 | TODO: Replace with actual API call | 实现 `createTunnel` |
| `packages/web/src/stores/tunnel.ts` | 110 | TODO: Replace with actual API call | 实现 `deleteTunnel` |
| `packages/web/src/stores/cron.ts` | 113 | TODO: Replace with actual API call | 实现 `createCronJob` |
| `packages/web/src/stores/cron.ts` | 127 | TODO: Replace with actual API call | 实现 `updateCronJob` |
| `packages/web/src/stores/cron.ts` | 149 | TODO: Replace with actual API call | 实现 `deleteCronJob` |
| `packages/web/src/stores/cron.ts` | 164 | TODO: Replace with actual API call | 实现 `toggleCronJob` |
| `packages/web/src/stores/cron.ts` | 187 | TODO: Replace with actual API call | 实现 `runCronJob` |
| `packages/web/src/stores/cron.ts` | 224 | TODO: Replace with actual API call | 实现 `fetchCronLogs` |

### 低优先级 - 其他 TODO

| 文件 | 行号 | 内容 | 建议 |
|------|------|------|------|
| `packages/ccode/parsers-config.ts` | 240 | TODO: Replace with official tree-sitter-nix WASM | 等待上游发布 |

**总计**: 14 个 TODO 待处理

---

## 二、已废弃配置字段

以下配置字段已标记为 `@deprecated`，建议在未来版本移除：

| 文件 | 行号 | 字段 | 替代方案 |
|------|------|------|----------|
| `config.ts` | 489 | `tools` | 使用 `permission` 字段 |
| `config.ts` | 509 | `maxSteps` | 使用 `steps` 字段 |
| `config.ts` | 1069 | (内部) | 使用 `provider._settings` |
| `config.ts` | 1258 | `mode` | 使用 `agent` 字段 |
| `config.ts` | 1338 | `layout` | 已固定为 stretch 布局 |

**建议**:
- 保留向后兼容至 v0.1.0
- 在 v0.1.0 发布说明中标注废弃警告
- 在 v0.2.0 移除废弃字段

---

## 三、文档清理建议

### 3.1 重复/重叠文档

| 文件 A | 文件 B | 情况 | 建议 |
|--------|--------|------|------|
| `docs/ARCHITECTURE-CURRENT.md` (381 行) | `docs/architecture/ARCHITECTURE.md` (427 行) | 内容重叠 | 合并到 `architecture/ARCHITECTURE.md`，删除 `ARCHITECTURE-CURRENT.md` |

### 3.2 已归档计划文档 (保留)

以下 10 个计划文档已归档在 `docs/archive/plans/`，内容已实现，保留作为历史参考：

| 文件 | 日期 | 内容 |
|------|------|------|
| `2026-02-24-omni-nexus-design.md` | 2026-02-24 | Omni-Nexus 设计 |
| `2026-02-24-omni-nexus-implementation.md` | 2026-02-24 | Omni-Nexus 实现 |
| `2026-02-24-omni-nexus-hitl-design.md` | 2026-02-24 | HITL 设计 |
| `2026-02-24-omni-nexus-hitl-implementation.md` | 2026-02-24 | HITL 实现 |
| `2026-02-25-conversation-store-redis-design.md` | 2026-02-25 | Redis 存储设计 |
| `2026-02-25-conversation-store-redis-impl.md` | 2026-02-25 | Redis 存储实现 |
| `2026-02-25-telegram-execution-time-design.md` | 2026-02-25 | Telegram 执行时间设计 |
| `2026-02-25-telegram-execution-time.md` | 2026-02-25 | Telegram 执行时间实现 |
| `2026-02-25-zero-browser-design.md` | 2026-02-25 | Zero Browser 设计 |
| `2026-02-25-zero-browser.md` | 2026-02-25 | Zero Browser 实现 |

**状态**: 已归档，无需操作

### 3.3 文档结构现状

```
docs/
├── ARCHITECTURE-CURRENT.md  ← 待删除（重复）
├── CODEBASE.md              ← 保留
├── DEBT.md                  ← 保留（技术债务跟踪）
├── developer-guide.md       ← 保留
├── FEATURES.md              ← 保留
├── PROJECT-OVERVIEW.md      ← 保留
├── RUNBOOK.md               ← 保留
├── Skills.md                ← 保留
├── architecture/            ← 规范架构文档位置
├── archive/plans/           ← 已归档计划（保留）
├── guides/                  ← 使用指南
├── progress/                ← 1 个活跃项 (ai-sdk-migration-tech-debt.md)
├── reports/
│   ├── completed/           ← 178 份完成报告
│   └── *.md                 ← 状态报告
├── standards/               ← 文档标准
└── templates/               ← 文档模板
```

---

## 四、清理优先级

### 立即执行 (本周)

1. ~~移动 8 个 progress 文档到 completed~~ ✅
2. 合并 `ARCHITECTURE-CURRENT.md` 到 `architecture/ARCHITECTURE.md`

### 短期 (本月)

1. 实现 Web store API 占位符 (13 个 TODO)
2. 更新 `PROJECT-OVERVIEW.md` 反映最新状态

### 中期 (下个版本)

1. 在 v0.1.0 发布说明中标注废弃字段
2. 评估移除废弃字段的影响

---

## 五、清理检查清单

- [x] `docs/progress/` 已完成项移动 (8 个文件)
- [x] `docs/progress/` 保留 1 个活跃项 (ai-sdk-migration-tech-debt.md)
- [ ] `ARCHITECTURE-CURRENT.md` 合并删除
- [ ] Web store API TODO 清理
- [ ] 废弃配置字段文档化
- [ ] `PROJECT-OVERVIEW.md` 更新

---

## 六、验证命令

```bash
# 检查 progress 目录状态
ls docs/progress/
# 应输出: ai-sdk-migration-tech-debt.md (1 个活跃项)

# 检查 TODO 数量
grep -r "TODO.*Replace" packages/web/src/stores/ | wc -l
# 当前: 13

# 检查废弃字段
grep -c "@deprecated" packages/ccode/src/config/config.ts
# 当前: 5
```
