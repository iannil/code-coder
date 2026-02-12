# 开发指南

本指南介绍如何设置开发环境以及如何为 CodeCoder 做出贡献。

## 前置要求

- **Bun** 1.3 或更高版本
- **Node.js**（某些依赖需要）
- **Git**

## 安装

```bash
# 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd codecoder

# 安装依赖
bun install
```

## 开发命令

### 运行 CodeCoder

```bash
# 在当前目录运行 TUI
bun dev

# 在指定目录运行
bun dev <path>

# 启动无头 API 服务器（默认端口 4096）
bun dev serve
bun dev serve --port 8080

# 启动服务器 + 打开 Web 界面
bun dev web
```

### 构建

```bash
# 构建独立可执行文件
bun run --cwd packages/ccode build

# 类型检查所有包
bun turbo typecheck
```

### 测试

```bash
# 运行测试（必须从特定包目录运行）
cd packages/ccode && bun test
```

### SDK 生成

SDK 生成功能已被移除。`packages/sdk/` 目录不再存在。

## 架构概览

### Monorepo 结构

| 包                     | 说明                                             |
| ---------------------- | ----------------------------------------------- |
| `packages/ccode/`      | 核心 CLI 工具和业务逻辑。入口点：`src/index.ts` |
| `packages/ccode/src/cli/cmd/tui/` | 终端 UI（SolidJS + OpenTUI）                   |
| `packages/util/`       | 共享工具                                         |
| `script/`              | 构建和发布脚本                                   |
| `scripts/`             | 项目级工具脚本                                   |

### 核心概念

1. **客户端/服务器架构**: CodeCoder 作为服务器运行，可由多个客户端驱动（TUI、Web、桌面、移动端）
2. **Agent 系统**: 多个内置 Agent（build、plan、general）用于不同场景
3. **LSP 集成**: 原生语言服务器协议支持，提供代码智能
4. **MCP 协议**: Model Context Protocol，支持可扩展的工具生态

## 代码风格

- **Prettier**: 120 字符行宽，无分号
- **EditorConfig**: 2 空格缩进，最大 80 字符行，LF 换行
- 优先使用 `const` 和三元运算符，而非 `let` 和 `else` 语句
- 尽可能使用 Bun API（例如 `Bun.file()`）

## 常用资源

- [项目 README](../../README.md)
- [架构指南](../Architecture-Guide.md)
- [产品文档](https://code-coder.com/docs)
