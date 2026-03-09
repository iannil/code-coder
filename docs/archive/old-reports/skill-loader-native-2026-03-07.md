# Skill Loader Native Migration

**日期**: 2026-03-07
**状态**: ✅ 已完成

---

## 概述

将 Skill 加载器从 JavaScript 实现 (`gray-matter` npm 包) 迁移到原生 Rust 实现，通过 NAPI-RS 绑定提供更快的 YAML frontmatter 解析。

## 实现详情

### 发现: Rust 实现已存在

在分析代码库后发现，Rust 实现实际上已经完成：

- **Rust 技能模块**: `services/zero-core/src/skill/mod.rs` (441 行)
- **NAPI 绑定**: `services/zero-core/src/napi/skill.rs` (173 行)
- **TypeScript 类型**: `packages/core/src/binding.d.ts` (行 2685-2753)

### 缺失部分

唯一缺失的是 TypeScript 导出和 skill.ts 的集成：

1. `packages/core/src/index.ts` - 未导出技能解析函数
2. `packages/ccode/src/skill/skill.ts` - 未使用原生解析

### 实施的变更

#### 1. 添加 TypeScript 导出 (`packages/core/src/index.ts`)

```typescript
// Phase 11: Skill Parser (native YAML frontmatter parsing)
export const parseSkillContent = nativeBindings?.parseSkillContent
export const parseSkillFromFile = nativeBindings?.parseSkillFromFile
export const parseSkillMetadataOnly = nativeBindings?.parseSkillMetadataOnly
export const validateSkillContent = nativeBindings?.validateSkillContent
export const parseSkillsBatch = nativeBindings?.parseSkillsBatch
export const extractSkillFrontmatter = nativeBindings?.extractSkillFrontmatter
export const stripSkillFrontmatter = nativeBindings?.stripSkillFrontmatter

// 类型导出
export type {
  NapiSkillMetadata,
  NapiParsedSkill,
  NapiSkillParseError,
  NapiParsedSkillResult,
} from './binding.d.ts'
```

#### 2. 更新技能加载器 (`packages/ccode/src/skill/skill.ts`)

- 新增 `parseSkill` 辅助函数，优先使用原生解析
- 当 `isNative && parseSkillFromFile` 可用时使用原生 Rust 实现
- 不可用时回退到 `ConfigMarkdown.parse()` (gray-matter)
- 保持完整的错误处理和日志记录

```typescript
const parseSkill = async (filePath: string): Promise<{ name: string; description: string } | undefined> => {
  // Use native parsing when available (faster, SIMD-accelerated)
  if (isNative && parseSkillFromFile) {
    try {
      const parsed = parseSkillFromFile(filePath)
      return {
        name: parsed.metadata.name,
        description: parsed.metadata.description,
      }
    } catch (err) {
      // 错误处理...
    }
  }
  // Fallback to JS implementation (gray-matter)
  // ...
}
```

## 技术亮点

### 原生实现优势

1. **YAML 预处理**: Rust 实现包含智能 frontmatter 预处理，能正确处理值中包含冒号的情况 (转换为 YAML 块标量)
2. **单次 FFI 调用**: `parseSkillFromFile` 在 Rust 中完成文件 I/O 和 YAML 解析，减少 FFI 边界跨越
3. **完整验证**: 验证必填字段 (name, description)，支持可选字段和自定义字段

### 可用的原生函数

| 函数 | 用途 |
|------|------|
| `parseSkillContent(content)` | 从内容字符串解析技能 |
| `parseSkillFromFile(path)` | 从文件路径解析技能 |
| `parseSkillMetadataOnly(content)` | 仅解析元数据 (更快) |
| `validateSkillContent(content)` | 验证技能内容 |
| `parseSkillsBatch(contents)` | 批量解析多个技能 |
| `extractSkillFrontmatter(content)` | 提取 frontmatter YAML |
| `stripSkillFrontmatter(content)` | 移除 frontmatter |

## 验证

### Rust 测试
```bash
cd services/zero-core && cargo test skill --quiet
# running 10 tests
# test result: ok. 10 passed; 0 failed
```

### 测试用例覆盖
- 基本解析
- 触发器模式
- 描述中的冒号处理
- 缺失 frontmatter 错误
- 缺失必填字段错误
- 自定义字段 (extra)
- Frontmatter 预处理
- 引号值保持
- 仅元数据解析

## 文件变更摘要

| 文件 | 变更 |
|------|------|
| `packages/core/src/index.ts` | +10 行 (函数导出 + 类型导出) |
| `packages/ccode/src/skill/skill.ts` | +30 行 (原生解析集成) |

## 后续建议

1. ~~添加 TypeScript 集成测试~~ (可选)
2. ~~性能基准测试~~ (可选 - 技能解析不是热路径)
3. 考虑批量解析优化 (当技能数量大时)

---

*完成时间: 2026-03-07*
