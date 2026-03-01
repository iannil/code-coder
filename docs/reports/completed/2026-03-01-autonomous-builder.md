# CodeCoder 自主构建能力实现报告

**日期**: 2026-03-01
**状态**: 已完成

## 概述

本次实现为 CodeCoder 添加了自主构建能力（Meta-capability），使系统能够检测能力缺口并自动构建新概念。

## 实现内容

### 新增模块

```
packages/ccode/src/autonomous/builder/
├── index.ts                 # 模块导出
├── types.ts                 # 核心类型定义
├── gap-detector.ts          # 能力缺口检测
├── concept-inventory.ts     # 现有概念清单
├── meta-builder.ts          # 元构建器编排
├── validation.ts            # 概念验证器
├── registration.ts          # 概念注册器
└── generators/              # 概念生成器
    ├── index.ts             # 生成器注册表
    ├── agent-generator.ts   # Agent 生成
    ├── prompt-generator.ts  # Prompt 生成
    ├── skill-generator.ts   # Skill 生成
    ├── tool-generator.ts    # Tool 生成
    ├── hand-generator.ts    # Hand 生成
    ├── memory-generator.ts  # Memory 生成
    └── workflow-generator.ts # Workflow 生成
```

### 核心组件

#### 1. 类型系统 (`types.ts`)

- `ConceptType`: 7 种概念类型 (AGENT, PROMPT, SKILL, TOOL, HAND, MEMORY, WORKFLOW)
- `GapDetectionResult`: 能力缺口检测结果
- `BuildRequest/BuildResult`: 构建请求和结果
- `AUTONOMY_CONCEPT_GATES`: 自治等级门控

#### 2. 概念清单 (`concept-inventory.ts`)

- 统一的概念发现接口
- 整合 Agent.list(), Skill.all(), DynamicToolRegistry
- 支持全文搜索和类型过滤
- 1 分钟 TTL 缓存

#### 3. 缺口检测 (`gap-detector.ts`)

- `detectFromFailure()`: 从任务失败检测
- `detectFromQuery()`: 从搜索无结果检测
- `analyzePatterns()`: 模式分析检测
- CLOSE 框架评估集成

#### 4. 生成器 (generators/)

| 生成器 | 风险等级 | 可自动批准 |
|--------|---------|-----------|
| ToolGenerator | Low | Yes |
| PromptGenerator | Low | Yes |
| SkillGenerator | Low | Yes |
| AgentGenerator | Medium | No |
| MemoryGenerator | Medium | Yes |
| HandGenerator | High | No |
| WorkflowGenerator | High | No |

#### 5. 验证器 (`validation.ts`)

每种概念类型都有专门的验证规则：
- TOOL: 语法检查、重复检测
- SKILL: frontmatter 验证
- AGENT: JSON 有效性、必填字段
- HAND/WORKFLOW: 必须默认禁用

#### 6. 注册器 (`registration.ts`)

- 自动创建目标目录
- 覆盖前创建备份
- 与现有注册表集成

#### 7. 元构建器 (`meta-builder.ts`)

5 阶段构建流程：
1. Evaluation - CLOSE 评估
2. Generation - 概念生成
3. Validation - 验证检查
4. Approval - 批准检查
5. Registration - 注册存储

### 集成修改

1. `autonomous/index.ts`: 导出 builder 模块
2. `decision/criteria.ts`: 添加 `selfBuildConcept` 决策模板

## 设计决策

### CLOSE 框架集成

每个构建决策都通过 CLOSE 框架评估：
- Convergence: 低风险概念更可逆
- Leverage: 构建一次可多次复用
- Optionality: 不会锁定选择
- Surplus: 基于检测置信度
- Evolution: 构建新概念有学习价值

### 自治等级门控

```typescript
AUTONOMY_CONCEPT_GATES = {
  lunatic: ["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY", "WORKFLOW"],
  insane:  ["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY"],
  crazy:   ["PROMPT", "SKILL", "TOOL", "HAND"],
  wild:    ["PROMPT", "SKILL", "TOOL"],
  bold:    ["PROMPT", "TOOL"],
  timid:   ["TOOL"],
}
```

### 安全设计

1. 高风险概念 (HAND, WORKFLOW) 始终需要人工批准
2. 新建的 HAND/WORKFLOW 默认禁用
3. 覆盖前自动备份
4. TOOL 在沙箱中语法验证

## 使用示例

```typescript
import { getMetaBuilder, createGapDetector } from "@/autonomous/builder"

// 从任务失败自动检测并构建
const builder = getMetaBuilder()
await builder.initialize()

const result = await builder.buildFromFailure({
  sessionId: "session-123",
  description: "需要分析 CSV 文件中的异常值",
  technology: "python",
  attempts: 3,
  webSearchUsed: true,
  toolSearchUsed: true,
})

if (result?.success) {
  console.log(`Built ${result.concept.type}: ${result.concept.identifier}`)
}
```

## 待办事项

1. [x] 添加单元测试覆盖 ✅ (2026-03-01, 201 tests)
2. [x] 集成到 evolution-loop.ts 的失败处理中 ✅ (2026-03-01)
3. [x] 添加配置文件支持 ✅ (2026-03-01)
4. [ ] 添加 LLM 增强的概念类型推断
5. [ ] 实现批量模式分析
6. [ ] 添加概念版本控制

## Phase 6: Evolution Loop 集成

**完成日期**: 2026-03-01

### 修改内容

#### 1. EvolutionConfig 扩展

```typescript
// 新增配置项
enableAutoBuilder: boolean          // 启用缺口检测 (默认 true)
enableAutoMetaBuilder: boolean      // 启用自动构建 (默认 false, 保守设计)
autoBuilderMinAttempts: number      // 最小尝试次数 (默认 2)
autoBuilderCloseThreshold: number   // CLOSE 分数阈值 (默认 5.5)
```

#### 2. EvolutionResult 扩展

```typescript
// 新增字段
gapDetected?: GapDetectionResult    // 检测到的能力缺口
buildAttempted?: boolean            // 是否尝试构建
buildResult?: BuildResult           // 构建结果
```

#### 3. 集成逻辑

在任务失败后自动触发：
1. 检查是否满足最小尝试次数
2. 使用 GapDetector 分析失败模式
3. 如果检测到缺口，记录到结果中
4. 如果启用 enableAutoMetaBuilder 且 CLOSE 分数达标，触发构建
5. 构建成功则记录新概念信息

#### 4. 新增私有方法

```typescript
// 从失败中检测能力缺口
private async detectGapFromFailure(
  problem: AutonomousProblem,
  attempts: SolutionAttempt[],
): Promise<GapDetectionResult | null>

// 尝试自动构建新概念
private async attemptAutoBuild(
  gap: GapDetectionResult,
  problem: AutonomousProblem,
): Promise<BuildResult | null>
```

### 设计原则

1. **观察优先**: 默认只检测缺口 (`enableAutoBuilder: true`)，不自动构建
2. **明确启用**: 自动构建需要显式开启 (`enableAutoMetaBuilder: false`)
3. **CLOSE 门控**: 只有 CLOSE 评分达到阈值才会触发构建
4. **可追溯**: 所有检测和构建结果都记录在 EvolutionResult 中

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 生成质量差 | 多重验证 + 人工批准 |
| 无限循环 | 构建频率限制 + 重复检测 |
| 资源消耗 | CLOSE 评估 + 自治门控 |
| 安全风险 | 沙箱执行 + 默认禁用 |

## 相关文件

- 实现计划: `/docs/progress/2026-03-01-self-building-plan.md` (如果存在)
- 架构文档: `/docs/architecture/CORE_CONCEPTS.md`
- 单元测试报告: `/docs/reports/completed/2026-03-01-autonomous-builder-unit-tests.md`

## Phase 8: 配置文件更新

**完成日期**: 2026-03-01

### 修改内容

在 `~/.codecoder/config.json` 添加 `autonomous` 配置节：

```json
"autonomous": {
  "builder": {
    "enabled": true,
    "enableAutoBuilder": true,
    "enableAutoMetaBuilder": true,
    "autoBuilderMinAttempts": 1,
    "autoBuilderCloseThreshold": 4.5,
    "autonomyLevel": "crazy",
    "dryRun": false,
    "maxBuildAttempts": 3
  },
  "storage": {
    "global": "~/.codecoder/data",
    "tools": "~/.codecoder/data/tools",
    "agents": "~/.codecoder/data/agents",
    "prompts": "~/.codecoder/data/prompts",
    "memory": "~/.codecoder/data/memory/schemas"
  },
  "approval": {
    "autoApproveTypes": ["TOOL", "PROMPT", "SKILL"],
    "requireApprovalTypes": ["AGENT", "HAND", "WORKFLOW", "MEMORY"],
    "notifyOnBuild": true,
    "notifyChannel": "telegram"
  }
}
```

### 配置说明

| 字段 | 值 | 说明 |
|------|-----|------|
| `enabled` | `true` | 启用自主构建模块 |
| `enableAutoBuilder` | `true` | 启用缺口检测 |
| `enableAutoMetaBuilder` | `true` | 启用自动构建（IM 场景激进策略） |
| `autoBuilderMinAttempts` | `1` | 失败 1 次即触发（IM 用户期望快速响应） |
| `autoBuilderCloseThreshold` | `4.5` | 降低 CLOSE 阈值，更容易触发构建 |
| `autonomyLevel` | `"crazy"` | 允许 PROMPT/SKILL/TOOL/HAND |
| `autoApproveTypes` | 低风险类型 | TOOL/PROMPT/SKILL 自动批准 |
| `requireApprovalTypes` | 高风险类型 | AGENT/HAND/WORKFLOW/MEMORY 需人工批准 |
| `notifyOnBuild` | `true` | 构建完成后发送通知 |
| `notifyChannel` | `"telegram"` | 通知渠道 |

### 创建的目录

```
~/.codecoder/data/
├── tools/           # 动态工具存储
├── agents/          # 自动生成的 agent 配置
├── prompts/         # 自动生成的 prompt 模板
└── memory/
    └── schemas/     # 内存 schema 定义
```
