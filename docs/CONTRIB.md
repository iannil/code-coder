# 贡献指南 (CONTRIBUTING)

> 文档类型: guide
> 创建时间: 2026-02-05
> 最后更新: 2026-02-16
> 状态: active

## 开发环境设置

### 前置要求

- **Bun** 1.3+ - 运行时和包管理器
- **Git** - 版本控制
- **Node.js** 兼容环境 (通过 Bun)

### 安装步骤

```bash
# 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd codecoder

# 安装依赖
bun install
```

### 验证安装

```bash
# 类型检查
bun typecheck

# 运行测试
cd packages/ccode && bun test

# 启动开发环境
bun dev
```

## 完整脚本参考

### 根目录脚本 (package.json)

| 脚本 | 说明 |
|------|------|
| `bun dev` | 启动 CodeCoder TUI 开发模式 |
| `bun typecheck` | 运行所有包的 TypeScript 类型检查 (Turborepo) |
| `bun test` | *禁止使用* - 必须在特定包内运行 |
| `bun prepare` | 初始化 Husky Git 钩子 |

### ccode 包脚本 (`packages/ccode/`)

#### 开发与构建

| 脚本 | 说明 |
|------|------|
| `bun dev` | 启动 CLI/TUI 开发模式 (浏览器条件) |
| `bun build` | 构建独立可执行文件 |
| `bun build:single` | 构建单一可执行文件 |
| `bun build:baseline` | 构建单一可执行文件 (基线模式) |
| `bun build:skip-install` | 跳过安装步骤构建 |
| `bun typecheck` | 运行 tsgo TypeScript 类型检查 |
| `bun lint` | 运行 tsgo 代码检查 |
| `bun format` | 检查代码格式 (Prettier) |
| `bun format:write` | 自动修复代码格式 |

#### 测试命令

| 脚本 | 说明 |
|------|------|
| `bun test` | 运行所有测试 |
| `bun test:tui` | 运行 TUI 相关测试 (单元+集成+E2E) |
| `bun test:tui:unit` | 仅运行 TUI 单元测试 |
| `bun test:tui:integration` | 仅运行 TUI 集成测试 |
| `bun test:tui:e2e` | 仅运行 TUI E2E 测试 |
| `bun test:tui:coverage` | 运行 TUI 测试覆盖率报告 |
| `bun test:tui:visual` | 运行 TUI 视觉回归测试 |
| `bun test:tui:perf` | 运行 TUI 性能测试 |
| `bun test:tui:a11y` | 运行 TUI 无障碍测试 |
| `bun test:tui:all` | 运行所有 TUI 测试 (单元+集成+E2E+性能+无障碍) |
| `bun test:coverage` | 运行全局测试覆盖率报告 |
| `bun test:coverage:report` | 生成覆盖率报告到 ./coverage 目录 |
| `bun test:verify` | 验证测试覆盖率是否达标 |

### util 包脚本 (`packages/util/`)

| 脚本 | 说明 |
|------|------|
| `bun typecheck` | 运行 util 包的 TypeScript 类型检查 |

## 开发工作流

### 1. 创建功能分支

```bash
git checkout -b feature/your-feature-name
```

### 2. 进行开发

- 遵循项目代码风格 (Prettier, 120 字符宽度)
- 保持文件小而专注 (<300 行)
- 编写测试覆盖新功能

### 3. 测试

```bash
# 类型检查
bun typecheck

# 运行相关测试
cd packages/ccode && bun test test/unit/your-module
```

### 4. 提交代码

使用约定式提交格式:

```
feat: 添加新功能
fix: 修复 bug
docs: 更新文档
test: 添加测试
refactor: 重构代码
```

### 5. 创建 Pull Request

- PR 标题遵循约定式提交规范
- 引用相关 Issue
- 提供测试截图 (UI 变更)
- 保持 PR 小而专注

## 测试指南

### 测试结构

```
packages/ccode/test/
├── unit/           # 单元测试
├── integration/    # 集成测试
├── e2e/           # 端到端测试
├── performance/   # 性能测试
└── accessibility/ # 无障碍测试
```

### 运行测试

```bash
# 从包目录运行
cd packages/ccode

# 所有测试
bun test

# 特定类型测试
bun test test/unit/memory-markdown

# 带覆盖率
bun test --coverage
```

### 测试覆盖率目标

- 目标覆盖率: 80%+
- 核心模块: 90%+

## 代码风格

### Prettier 配置

```json
{
  "semi": false,
  "printWidth": 120
}
```

### TypeScript 规范

- 使用显式类型注解 (公共 API)
- 避免使用 `any` 类型
- 使用 `readonly` 标记不可变数据

### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| 文件 | kebab-case | `memory-markdown.ts` |
| 类 | PascalCase | `MemoryManager` |
| 函数 | camelCase | `loadDailyNotes` |
| 常量 | UPPER_SNAKE_CASE | `MAX_ENTRIES` |
| 接口 | PascalCase | `DailyEntry` |

## 项目结构

```
codecoder/
├── packages/
│   ├── ccode/          # 核心 CLI 工具
│   └── util/           # 共享工具库
├── docs/               # 项目文档
├── script/             # 构建脚本
└── CLAUDE.md           # Claude Code 指导
```

## LLM 友好设计原则

1. **小文件原则**: 每个文件 <300 行
2. **清晰命名**: 使用描述性名称
3. **显式类型**: 完整的 TypeScript 定义
4. **单一职责**: 每个文件/函数专注一件事
5. **可搜索性**: 统一命名模式

## 环境变量

### 内部使用变量

以下环境变量在代码内部使用，通常不需要手动设置:

| 变量 | 用途 | 默认值 |
|------|------|--------|
| `AGENT` | 标识 Agent 模式 | `1` |
| `CODECODER` | 标识 CodeCoder 环境 | `1` |
| `CCODE_TEST_HOME` | 测试时覆盖用户主目录 | - |
| `HOME` / `USERPROFILE` | 用户主目录 (跨平台) | - |
| `EDITOR` / `VISUAL` | 默认文本编辑器 | `vi` |
| `SHELL` | Shell 路径 (Unix) | - |
| `COMSPEC` | Shell 路径 (Windows) | - |
| `HTTP_PROXY` / `HTTPS_PROXY` | HTTP 代理配置 | - |
| `http_proxy` / `https_proxy` | HTTP 代理配置 (小写) | - |
| `PWD` | 当前工作目录 | - |

### 配置方式

项目不使用 `.env` 文件。配置通过:

- **CLI 参数**: 命令行传入
- **配置文件**: `~/.codecoder/config.json` 或 `~/.codecoder/`
- **交互式输入**: 首次使用时的配置向导

## 获取帮助

- 查看 [Architecture Guide](./Architecture-Guide.md)
- 查看 [开发指南](./guides/development.md)
- 查看 [测试指南](./guides/testing.md)
- 提交 Issue 到 GitHub

## 相关文档

- [项目 README](../README.md)
- [架构指南](./Architecture-Guide.md)
- [文档标准](./standards/document-structure.md)
