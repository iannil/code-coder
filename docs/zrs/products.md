# 产品

## SYNTON-DB

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](./LICENSE)

[English](README.md)

---

### 项目概述

SYNTON-DB 是一个专门为大语言模型设计的记忆数据库，通过结合知识图谱和向量检索，提供语义关联、逻辑推理和动态记忆能力。

与传统数据库（SQL、NoSQL、Vector）专注于 CRUD 操作不同，SYNTON-DB 基于三大核心原则构建：

- 入库即理解 - 自动从输入中提取知识图谱
- 查询即推理 - 混合向量相似度 + 图遍历
- 输出即上下文 - 返回预处理的上下文包，而非原始数据

#### 解决什么问题？

传统数据库存储和检索数据但缺乏语义理解。SYNTON-DB：

1. 理解实体之间的关系，而不仅是内容相似度
2. 通过记忆衰减和强化维持时间上下文
3. 通过图遍历进行多跳推理
4. 为 LLM 优化合成上下文

#### 核心差异

| 特性 | 传统数据库 | SYNTON-DB |
| ------ | ----------- | ----------- |
| 存储 | 表/文档/向量 | 张量图（带向量的节点 + 带关系的边） |
| 查询 | SQL/向量搜索 | PaQL（提示即查询语言） |
| 检索 | 基于相似度 | Graph-RAG（向量 + 图遍历） |
| 记忆 | 静态 | 动态（基于访问的衰减/强化） |
| 输出 | 原始行/列 | 合成的上下文包 |

---

### 核心特性

#### 张量图存储（Tensor-Graph）

- 节点包含内容与可选向量嵌入
- 边代表逻辑关系（is_a、causes、contradicts 等）
- 支持 4 种节点类型：`entity`（实体）、`concept`（概念）、`fact`（事实）、`raw_chunk`（原始片段）
- 支持 7 种关系类型：`is_a`（是）、`is_part_of`（属于）、`causes`（导致）、`similar_to`（相似）、`contradicts`（矛盾）、`happened_after`（发生于）、`belongs_to`（归属于）

#### Graph-RAG 混合检索

- 结合向量相似度搜索与多跳图遍历
- 可配置向量与图评分的权重
- 返回带置信度分数的排序结果
- 可配置遍历深度和结果限制

#### PaQL（提示即查询语言）

- 自然语言查询解析器
- 支持逻辑运算符（AND、OR、NOT）
- 支持过滤器和图遍历查询
- 为 LLM 生成的查询优化

#### 记忆衰减机制

- 艾宾浩斯遗忘曲线实现
- 基于访问分数的保留策略（0.0-10.0 分）
- 周期性衰减计算
- 可配置的保留阈值

#### ML 嵌入服务

- 多后端支持：本地（Candle）、OpenAI、Ollama
- 嵌入缓存提升性能
- 可配置模型选择
- 支持 CPU/GPU 设备

#### 双协议 API

- REST API（端口 8080）- 基于 HTTP 的 JSON
- gRPC API（端口 50051）- 高性能二进制协议
- 为 Web 客户端启用 CORS

---

### 快速开始

#### Docker Compose（推荐）

```bash
## 克隆仓库
git clone https://github.com/synton-db/synton-db.git
cd synton-db

## 启动所有服务（数据库 + 监控）
docker-compose up -d

## 检查服务状态
docker-compose ps

## 查看日志
docker-compose logs -f synton-db
```

暴露的服务：

- `8080` - REST API
- `50051` - gRPC API
- `9090` - Prometheus 指标
- `3000` - Grafana 仪表板

#### 从源码构建

```bash
## 前置要求：Rust 1.75+、Git

## 构建服务器
cargo build --release -p synton-db-server

## 构建 CLI 工具
cargo build --release -p synton-cli

## 运行服务器
./target/release/synton-db-server --config config.toml
```

#### 验证

```bash
## 健康检查
curl http://localhost:8080/health

## 获取统计信息
curl http://localhost:8080/stats
```

---

### CLI 使用

`synton-cli` 工具提供全面的命令行界面。

#### 连接选项

```bash
synton-cli --host <主机> --port <端口> --format <text|json> [命令]
```

#### 节点操作

```bash
## 创建节点
synton-cli node create "巴黎是法国的首都" --node-type fact

## 通过 ID 获取节点
synton-cli node get <uuid>

## 删除节点（带确认）
synton-cli node delete <uuid>

## 强制删除（跳过确认）
synton-cli node delete <uuid> --force

## 列出所有节点
synton-cli node list --limit 100
```

#### 边操作

```bash
## 在节点间创建边
synton-cli edge create <源节点ID> <目标节点ID> --relation is_part_of --weight 0.9

## 列出节点的边
synton-cli edge list <节点ID> --limit 100
```

#### 查询操作

```bash
## 执行 PaQL 查询
synton-cli query execute "首都城市" --limit 10
```

#### 系统操作

```bash
## 获取数据库统计
synton-cli stats

## 获取详细统计
synton-cli stats --detailed

## 导出数据为 JSON
synton-cli export --format json --output backup.json

## 从 JSON 导入数据
synton-cli import --format json --input backup.json

## 导入时遇到错误继续
synton-cli import --format json --input backup.json --continue-on-error
```

---

### API 端点

#### REST API（端口 8080）

| 端点 | 方法 | 描述 |
| ------ | ------ | ------ |
| `/health` | GET | 健康检查 |
| `/stats` | GET | 数据库统计 |
| `/nodes` | GET | 列出所有节点 |
| `/nodes` | POST | 创建新节点 |
| `/nodes/:id` | GET | 按 ID 获取节点 |
| `/nodes/:id` | DELETE | 按 ID 删除节点 |
| `/edges` | POST | 创建新边 |
| `/query` | POST | 执行 PaQL 查询 |
| `/traverse` | POST | 图遍历 |
| `/bulk` | POST | 批量操作 |

##### 请求/响应示例

健康检查

```bash
curl http://localhost:8080/health
```

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_secs": 0
}
```

创建节点

```bash
curl -X POST http://localhost:8080/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "content": "巴黎是法国的首都",
    "node_type": "fact"
  }'
```

```json
{
  "node": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "content": "巴黎是法国的首都",
    "node_type": "fact",
    "embedding": null,
    "meta": {
      "created_at": "2025-02-05T10:00:00Z",
      "access_score": 5.0
    }
  },
  "created": true
}
```

执行查询

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "首都",
    "limit": 10,
    "include_metadata": false
  }'
```

```json
{
  "nodes": [...],
  "total_count": 5,
  "execution_time_ms": 12,
  "truncated": false
}
```

创建边

```bash
curl -X POST http://localhost:8080/edges \
  -H "Content-Type: application/json" \
  -d '{
    "source": "<uuid-1>",
    "target": "<uuid-2>",
    "relation": "is_part_of",
    "weight": 0.9
  }'
```

批量操作

```bash
curl -X POST http://localhost:8080/bulk \
  -H "Content-Type: application/json" \
  -d '{
    "nodes": [
      {"content": "节点1", "node_type": "entity"},
      {"content": "节点2", "node_type": "concept"}
    ],
    "edges": []
  }'
```

#### gRPC API（端口 50051）

gRPC API 提供相同功能，在高吞吐量场景下性能更佳。请参阅 `crates/api/src/grpc.rs` 了解 Protocol Buffers 定义。

---

### 项目结构

```text
synton-db/
├── crates/
#│   ├── bin/          # 服务器二进制 ✅
#│   ├── cli/          # 命令行工具 ✅
#│   ├── core/         # 核心类型（Node、Edge、Relation）✅
#│   ├── storage/      # RocksDB + Lance 存储 ✅
#│   ├── vector/       # 向量索引 ✅
#│   ├── graph/        # 图遍历算法 ✅
#│   ├── graphrag/     # 混合搜索实现 ✅
#│   ├── paql/         # 查询语言解析器 ✅
#│   ├── memory/       # 记忆衰减管理 ✅
#│   ├── ml/           # ML 嵌入服务 ✅
#│   └── api/          # REST + gRPC API 层 ✅
#├── e2e/              # 端到端测试 ✅
#├── release/          # 发布文件
#│   └── docker/       # Docker 配置文件
#├── docs/             # 文档
#│   ├── progress/     # 进行中的文档
#│   └── reports/      # 已完成的报告
├── docker-compose.yml
├── Dockerfile
└── Cargo.toml
```

#### 架构概览

```text
┌─────────────────────────────────────────────────────────────┐
│                      接口层                                  │
│  ┌──────────────────┐        ┌──────────────────┐          │
│  │   REST API       │        │    gRPC API      │          │
│  │   (Axum)         │        │    (Tonic)       │          │
│  └──────────────────┘        └──────────────────┘          │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    认知计算层                               │
│  ┌──────────────────┐  ┌─────────────┐  ┌───────────────┐  │
│  │     PaQL         │  │  Graph-RAG  │  │  记忆管理     │  │
│  │   解析器         │  │   搜索      │  │   (衰减)      │  │
│  └──────────────────┘  └─────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                 张量图存储层                                │
│  ┌──────────────────┐        ┌──────────────────┐          │
│  │    RocksDB       │        │     Lance        │          │
│  │  (图存储)        │        │  (向量存储)      │          │
│  └──────────────────┘        └──────────────────┘          │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    基础设施层                               │
│                   Rust + Tokio 运行时                       │
└─────────────────────────────────────────────────────────────┘
```

---

### 配置

#### 配置文件

创建 `config.toml` 文件或使用 `release/docker/config.toml` 中的默认配置：

```toml
[server]
## 绑定地址
host = "0.0.0.0"

## gRPC 服务器端口
grpc_port = 50051

## REST API 服务器端口
rest_port = 8080

## 启用/禁用服务器
grpc_enabled = true
rest_enabled = true

[storage]
## RocksDB 数据目录
rocksdb_path = "./data/rocksdb"

## Lance 数据目录
lance_path = "./data/lance"

## RocksDB 最大打开文件数
max_open_files = 5000

## RocksDB 缓存大小（MB）
cache_size_mb = 256

## 启用预写日志
wal_enabled = true

[memory]
## 遗忘曲线衰减尺度（天）
decay_scale = 20.0

## 保留阈值（0.0-1.0）
retention_threshold = 0.1

## 新节点初始访问分数
initial_access_score = 5.0

## 每次访问的分数提升
access_boost = 0.5

## 启用周期性衰减计算
periodic_decay_enabled = false

## 衰减计算间隔（秒）
decay_interval_secs = 3600

[logging]
## 日志级别：trace、debug、info、warn、error
level = "info"

## 启用 JSON 格式日志
json_format = false

## 启用追踪输出
tracing_enabled = true

[graphrag]
## 图遍历最大深度
max_traversal_depth = 3

## 混合搜索返回的最大节点数
max_results = 10

## 向量相似度权重（0.0-1.0）
vector_weight = 0.7

## 图邻近度权重（0.0-1.0）
graph_weight = 0.3

## 启用置信度评分
confidence_scoring = true

[ml]
## 启用 ML 功能
enabled = true

## 后端类型：local、openai、ollama
backend = "local"

## 本地模型配置
local_model = "sentence-transformers/all-MiniLM-L6-v2"
device = "cpu"
max_length = 512

## API 配置（用于 openai/ollama 后端）
api_endpoint = "https://api.openai.com/v1"
api_model = "text-embedding-3-small"
timeout_secs = 30

## 嵌入缓存
cache_enabled = true
cache_size = 10000
```

#### 环境变量

可通过环境变量覆盖配置：

| 变量 | 描述 | 默认值 |
| ------ | ------ | -------- |
| `SYNTON_SERVER_HOST` | 服务器绑定地址 | `0.0.0.0` |
| `SYNTON_SERVER_GRPC_PORT` | gRPC 端口 | `50051` |
| `SYNTON_SERVER_REST_PORT` | REST API 端口 | `8080` |
| `SYNTON_STORAGE_ROCKSDB_PATH` | RocksDB 数据路径 | `./data/rocksdb` |
| `SYNTON_STORAGE_LANCE_PATH` | Lance 数据路径 | `./data/lance` |
| `SYNTON_LOG_LEVEL` | 日志级别 | `info` |

---

### 开发

#### 前置要求

- Rust 1.75+
- Node.js 18+（用于 E2E 测试）
- Docker & Docker Compose（用于容器化测试）

#### 运行测试

```bash
## 单元测试
cargo test

## 单元测试（带输出）
cargo test -- --nocapture

## 运行特定测试
cargo test test_add_node

## E2E 测试
cd e2e
npm install
npx playwright install
npm test

## E2E 测试（可见浏览器）
npm run test:headed

## E2E 测试报告
npm run test:report
```

#### 代码质量

```bash
## 格式化代码
cargo fmt

## 检查格式
cargo fmt --check

## Clint 检查
cargo clippy

## 将警告视为错误
cargo clippy -- -D warnings

## 生成文档
cargo doc --open

## 生成所有 crate 的文档
cargo doc --document-private-items --open
```

#### 构建

```bash
## Debug 构建
cargo build

## Release 构建（优化）
cargo build --release

## 构建特定 crate
cargo build -p synton-db-server

## 使用特性构建
cargo build --features all
```

#### Docker 开发

```bash
## 构建 Docker 镜像
docker build -t synton-db:dev .

## 运行容器
docker run -p 8080:8080 -p 50051:50051 synton-db:dev

## 使用自定义配置运行
docker run -v $(pwd)/config.toml:/etc/synton-db/config.toml synton-db:dev
```

---

### 设计理念

> 传统数据库的核心是 CRUD，追求 ACID 或 CAP。
> 认知数据库的核心是：感知、关联、回忆和进化。

#### 入库即理解

传统数据库按原样存储数据。SYNTON-DB 自动：

- 提取实体和关系
- 构建知识图谱
- 创建语义嵌入
- 建立时间上下文

#### 查询即推理

传统数据库匹配模式。SYNTON-DB：

- 结合向量相似度与图遍历
- 通过连接节点跟踪逻辑链
- 按置信度和相关性加权结果
- 返回上下文相关的信息

#### 输出即上下文

传统数据库返回原始行。SYNTON-DB：

- 合成相关信息
- 压缩和优先排序上下文
- 为 LLM 消费格式化输出
- 维护出处和置信度

---

### 路线图

#### 已完成 ✅

- [x] 核心数据模型（Node、Edge、Relation）
- [x] 存储层（RocksDB + Lance 后端）
- [x] 向量索引（Lance 集成）
- [x] 图遍历（BFS/DFS 算法）
- [x] Graph-RAG 混合检索
- [x] PaQL 查询解析器
- [x] 记忆衰减管理
- [x] REST + gRPC 双 API
- [x] 全功能 CLI 工具
- [x] Docker 部署
- [x] E2E 测试套件
- [x] Prometheus + Grafana 监控
- [x] 配置管理
- [x] ML 嵌入服务（本地/OpenAI/Ollama）

#### 进行中 🚧

- [ ] 高级 PaQL 语法特性
- [ ] 查询缓存层

#### 计划中 📋

- [ ] WebUI 控制台
- [ ] 备份/恢复工具
- [ ] 访问控制和身份验证
- [ ] 分布式存储支持
- [ ] 高级告警系统

---

### 贡献

欢迎贡献！请遵循以下准则：

1. 代码风格：遵循 Rust 约定并使用 `cargo fmt`
2. 测试：为新功能编写测试（目标覆盖率 80%）
3. 提交：使用约定式提交格式（`feat:`、`fix:`、`docs:` 等）
4. 文档：更新相关文档以反映变更
5. PR：提供清晰的描述并链接相关 issue

#### 开发流程

```bash
## 1. Fork 并克隆仓库
git clone https://github.com/synton-db/synton-db.git

## 2. 创建功能分支
git checkout -b feat/your-feature

## 3. 进行更改并测试
cargo test
cargo clippy

## 4. 使用约定格式提交
git commit -m "feat: 添加功能描述"

## 5. 推送并创建 PR
git push origin feat/your-feature
```

---

### 许可证

Apache License 2.0

---

### 链接

- 仓库：[https://github.com/synton-db/synton-db](https://github.com/synton-db/synton-db)
- 文档：[docs/](./docs/)
- 问题：[https://github.com/synton-db/synton-db/issues](https://github.com/synton-db/synton-db/issues)
- 讨论：[https://github.com/synton-db/synton-db/discussions](https://github.com/synton-db/synton-db/discussions)

## CodeCoder

<h3 align="center">面向终端的 AI 编程代理</h3>

<p align="center">
  <a href="https://github.com/iannil/code-coder/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/iannil/code-coder?style=flat-square&logo=github" /></a>
  <a href="https://github.com/iannil/code-coder/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/npm/l/ccode?style=flat-square" /></a>
  <a href="https://github.com/iannil/code-coder/actions/workflows/ci.yml"><img alt="Build status" src="https://img.shields.io/github/actions/workflow/status/iannil/code-coder/ci.yml?style=flat-square&branch=master" /></a>
  <a href="https://github.com/iannil/code-coder/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/iannil/code-coder?style=flat-square" /></a>
</p>

<p align="center">
  #<a href="#概述">概述</a> •
  #<a href="#为什么选择-codecoder">特性</a> •
  #<a href="#核心功能">功能</a> •
  #<a href="#快速开始">快速开始</a> •
  #<a href="#安装">安装</a> •
  #<a href="#使用">使用</a> •
  #<a href="#配置">配置</a>
</p>

<p align="center">
  <a href="README.md">English</a>
</p>

---

### 概述

CodeCoder 是一个运行在终端中的开源 AI 编程代理。通过整合 20+ AI 提供商的支持与强大的代码分析工具和专用代理，它帮助你以前所未有的效率编写、理解和重构代码。

与基于浏览器的 AI 助手不同，CodeCoder 直接在你的开发环境中工作，消除了编辑器、终端和 AI 聊天之间的上下文切换。凭借透明的双层记忆系统和细粒度权限控制，你完全掌控 AI 可以执行的操作。

---

### 为什么选择 CodeCoder?

| 特性 | CodeCoder | Cursor | Copilot | Aider |
|------|-----------|--------|---------|-------|
| **终端原生** | ✅ | ❌ | ❌ | ✅ |
| **多提供商 AI** | 20+ | 1 | 1 | 1 |
| **专用代理** | 14+ | ❌ | ❌ | ❌ |
| **开源** | ✅ | ❌ | ❌ | ✅ |
| **本地模型** | ✅ | ✅ | ❌ | ❌ |
| **记忆系统** | 透明 | 专有 | 专有 | ❌ |
| **权限控制** | 细粒度 | 基础 | 基础 | ❌ |
| **MCP 支持** | ✅ | ❌ | ❌ | ❌ |

---

### 核心功能

#### 🤖 多 AI 提供商支持

支持 20+ AI 提供商：

- **云端服务**: Anthropic (Claude), OpenAI (GPT-4o, o1), Google (Gemini 2.0)
- **企业级**: Azure OpenAI, AWS Bedrock, Google Vertex AI
- **专业服务**: Groq, Mistral, Cohere, Perplexity, xAI, Cerebras
- **本地部署**: Ollama, vLLM, LM Studio 通过 OpenAI 兼容 API
- **灵活接入**: 任何 OpenAI 兼容的端点

#### 🎯 专用代理

按 `Tab` 键切换专用代理模式：

| 代理 | 用途 |
|------|------|
| `build` | 具备完整写入权限的开发代理 |
| `plan` | 只读模式的代码探索和分析 |
| `explore` | 快速代码库搜索和导航 |
| `code-reviewer` | 自动化代码质量审查 |
| `security-reviewer` | 安全漏洞分析 |
| `tdd-guide` | 测试驱动开发流程助手 |
| `architect` | 系统设计和架构决策 |
| `code-reverse` | 网站逆向工程 |
| `jar-code-reverse` | JAR 文件逆向工程 |

#### 🛠️ 强大工具集

- **文件操作**: 智能读取、写入、编辑，支持冲突检测
- **代码搜索**: 基于 ripgrep 的快速搜索，具有语义理解能力
- **LSP 集成**: 原生语言服务器协议支持代码智能
- **Git 集成**: 无缝 GitHub 操作，PR 创建/管理
- **Bash 访问**: 带权限提示的终端命令执行
- **Web 工具**: WebFetch、WebSearch 用于收集上下文
- **MCP 协议**: 通过模型上下文协议服务器扩展功能

#### 🧠 智能记忆系统

透明的双层记忆架构：

- **流层** (`memory/daily/`): 所有交互的每日追加日志
- **沉积层** (`memory/MEMORY.md`): 整合的知识，包括用户偏好、关键决策和经验教训

所有记忆都是人类可读的 Markdown 文件并由 Git 跟踪——没有不透明的嵌入或向量数据库。

#### 🔒 安全优先

- **细粒度权限**: 文件写入、bash 命令和 Web 请求需要明确批准
- **审计追踪**: 所有 AI 操作的完整历史记录
- **无数据收集**: 一切在本地运行
- **开源**: 完全可审计的代码库

#### 🔌 可扩展

- **MCP 服务器**: 连接外部工具和服务
- **自定义代理**: 定义你自己的专用代理
- **技能系统**: 创建和共享可重用技能
- **自定义提供商**: 添加任何 OpenAI 兼容的提供商

---

### 快速开始

5 分钟内即可上手：

```bash
## 1. 安装前置要求 (Bun 1.3+)
curl -fsSL https://bun.sh/install | bash

## 2. 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd code-coder

## 3. 安装依赖
bun install

## 4. 设置 API 密钥
export ANTHROPIC_API_KEY=你的密钥

## 5. 运行 CodeCoder
bun dev
```

---

### 安装

#### 前置要求

- **Bun** 1.3+ ([安装指南](https://bun.sh/docs/installation))
- **Node.js** 22+ (可选，用于兼容性)

#### 从源码安装

```bash
## 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd code-coder

## 安装依赖
bun install

## 构建 CLI
cd packages/ccode && bun run build

## 链接二进制文件（可选，用于全局访问）
bun link
```

#### 开发模式

```bash
## 直接从源码运行（无需构建）
bun dev

## 在指定目录运行
bun dev /path/to/project
```

#### Docker (即将推出)

```bash
## 在容器中运行
docker run -it ghcr.io/iannil/code-coder:latest
```

---

### 使用

#### 基本命令

```bash
## 在当前目录运行 CodeCoder
ccode

## 在指定目录运行
ccode /path/to/project

## 启动无头 API 服务器（默认端口 4096）
ccode serve
ccode serve --port 8080

## 显示所有可用命令
ccode --help

## 列出可用模型
ccode models

## 管理 MCP 服务器
ccode mcp list
ccode mcp add <服务器名称>
```

#### 代理模式

在 TUI 中按 `Tab` 循环切换代理模式：

- **build**: 完整开发能力，带写入权限
- **plan**: 只读探索和分析
- **@代理名称**: 使用专用子代理

#### 常见工作流

**代码探索**

```
> "探索认证系统的实现"
```

使用 `explore` 代理快速理解代码结构。

**功能实现**

```
> "添加用户登出功能"
```

使用 `build` 代理按 TDD 流程实现。

**代码审查**

```
> "审查 api/routes.ts 的最近更改"
```

使用 `code-reviewer` 代理进行质量分析。

**安全审计**

```
> "检查 auth.ts 的安全漏洞"
```

使用 `security-reviewer` 代理发现潜在问题。

**网站逆向工程**

```
> "逆向工程这个网站: https://example.com"
```

使用 `code-reverse` 代理分析并克隆网站功能。

---

### 配置

#### 配置文件

CodeCoder 将配置存储在 `~/.ccode/config.json`：

```json
{
  "defaultModel": "anthropic:claude-sonnet-4-20250514",
  "theme": "dark",
  "editor": "code",
  "autoConfirm": false,
  "maxTokens": 200000
}
```

#### API 密钥

通过环境变量配置 API 密钥：

```bash
## Anthropic (Claude)
export ANTHROPIC_API_KEY=你的密钥

## OpenAI
export OPENAI_API_KEY=你的密钥

## Google
export GOOGLE_API_KEY=你的密钥

## Groq
export GROQ_API_KEY=你的密钥

## 或使用 .env 文件
echo "ANTHROPIC_API_KEY=你的密钥" > ~/.ccode/.env
```

#### 自定义代理

在 `~/.ccode/agents.json` 中定义自定义代理：

```json
{
  "agents": [
    {
      "name": "my-specialist",
      "description": "我的框架专家",
      "permission": "read",
      "systemPrompt": "你是...方面的专家"
    }
  ]
}
```

#### MCP 服务器

添加 MCP 服务器以扩展功能：

```bash
## 添加 MCP 服务器
ccode mcp add github
ccode mcp add filesystem

## 列出可用服务器
ccode mcp list
```

详见 [MCP 指南](docs/standards/mcp-guide.md)。

---

### 架构

#### 技术栈

**运行时 & 构建**

- Bun 1.3+ - 快速 JavaScript 运行时和包管理器
- Turborepo - 高性能 monorepo 构建系统

**前端**

- Solid.js 1.9+ - 具有细粒度响应性的响应式 UI 框架
- OpenTUI 0.1+ - 基于 Solid 的终端 UI 框架
- TailwindCSS 4.1+ - 原子化 CSS 框架

**后端**

- Hono 4.10+ - 轻量级快速 Web 框架
- TypeScript 5.8+ - 类型安全的 JavaScript

**AI/ML**

- Vercel AI SDK - 20+ AI 提供商的统一接口
- MCP SDK - 模型上下文协议集成
- Agent Client Protocol - 代理通信标准

#### 项目结构

```
code-coder/
├── packages/
#│   ├── ccode/              # 核心 CLI 和业务逻辑
│   │   ├── src/
#│   │   │   ├── agent/      # 代理实现
#│   │   │   ├── cli/        # CLI 命令和 TUI
#│   │   │   │   └── cmd/tui/ # 终端 UI (SolidJS)
#│   │   │   ├── lsp/        # LSP 集成
#│   │   │   ├── mcp/        # MCP 协议
#│   │   │   ├── permission/ # 权限系统
#│   │   │   ├── provider/   # AI 提供商适配器
#│   │   │   ├── session/    # 会话管理
#│   │   │   ├── tool/       # 工具实现
#│   │   │   └── util/       # 内部工具
#│   │   └── test/           # 测试（单元、集成、端到端）
#│   ├── sdk/                # JavaScript/TypeScript SDK
#│   └── util/               # 共享工具
#├── docs/                   # 项目文档
#│   ├── guides/             # 用户指南
#│   ├── standards/          # 技术标准
#│   └── templates/          # 文档模板
#├── script/                 # 构建和工具脚本
#└── memory/                 # 双层记忆系统
    #├── daily/              # 每日流日志
    #└── MEMORY.md           # 长期沉积
```

---

### 开发

#### 环境准备

```bash
## 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd code-coder

## 安装依赖
bun install

## 以开发模式运行 TUI
bun dev
```

#### 开发命令

```bash
## 类型检查所有包
bun turbo typecheck

## 运行测试（从特定包目录）
cd packages/ccode && bun test

## 运行特定测试套件
bun test:tui:unit
bun test:tui:integration
bun test:tui:e2e

## 构建独立可执行文件
cd packages/ccode && bun run build

## 从 OpenAPI 规范生成 SDK
./script/generate.ts
```

#### 代码风格

- **Prettier**: 120 字符宽度，无分号
- **EditorConfig**: 2 空格缩进，最大 80 字符行宽，LF 换行
- **TypeScript**: 启用严格模式

#### 测试

测试覆盖率目标：**80%+**

```bash
## 运行所有测试
bun test

## 运行并生成覆盖率报告
bun test:coverage

## 验证覆盖率达标
bun test:verify
```

#### 贡献

我们欢迎贡献！提交 PR 前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

**适合入手的问题：**

- [Bug 报告](https://github.com/iannil/code-coder/issues?q=is%3Aissue+state%3Aopen+label%3Abug)
- [功能请求](https://github.com/iannil/code-coder/issues?q=is%3Aissue+state%3Aopen+label%3Aenhancement)

**欢迎贡献的领域：**

- Bug 修复
- 额外的 LSP / 格式化工具
- LLM 性能改进
- 新提供商支持
- 文档改进
- 测试覆盖率提升

---

### 文档

- [开发指南](docs/developer-guide.md) - 环境准备和开发说明
- [架构指南](docs/Architecture-Guide.md) - 系统设计和模式
- [代码库导航](docs/CODEBASE.md) - 面向 LLM 的代码库理解
- [新手指南](docs/guides/beginners-guide.md) - CodeCoder 入门
- [API 密钥管理](docs/guides/api-key-management.md) - 配置 AI 提供商
- [MCP 指南](docs/standards/mcp-guide.md) - 模型上下文协议
- [技能文档](docs/Skills.md) - 技能系统参考

---

### 故障排除

#### 常见问题

**"Command not found: ccode"（命令未找到）**

- 确保已构建二进制文件：`cd packages/ccode && bun run build`
- 或链接它：`bun link`
- 或直接运行：`bun run packages/ccode/src/index.ts`

**"API key not found"（未找到 API 密钥）**

- 设置相应的环境变量
- 示例：`export ANTHROPIC_API_KEY=你的密钥`
- 或添加到 `~/.ccode/.env`

**TUI 显示问题**

- 确保终端支持真彩色
- 尝试：`export TERM=xterm-256color`

**大型项目的内存问题**

- 使用 `plan` 代理进行只读探索
- 在配置中设置 `maxTokens`
- 使用聚焦的目录路径

---

### 许可证

MIT 许可证 - 详情见 [LICENSE](LICENSE)。

---

### 致谢

基于 [CodeCoder](https://github.com/iannil/code-coder) - 原始开源 AI 编程代理。

构建工具：

- [Bun](https://bun.sh) - 快速 JavaScript 运行时
- [Solid.js](https://solidjs.com) - 响应式 UI 框架
- [OpenTUI](https://github.com/sst/opentui) - 终端 UI 框架
- [Vercel AI SDK](https://sdk.vercel.ai) - AI 提供商抽象
- [Hono](https://hono.dev) - Web 框架

<p align="center">
  <sub>由开源社区用热情构建</sub>
</p>

## Trade Trader

[English](README.md) | [中文](README_CN.md)

一个面向中国期货市场的交易系统，集成五大期货交易所：上期所、大商所、郑商所、中金所和广期所。系统采用事件驱动架构，使用 Redis 发布订阅进行实时消息传递，Django ORM 进行数据持久化。

### 功能特性

- **多交易所支持**：连接 5 大中国期货交易所
  - SHFE（上海期货交易所）
  - DCE（大连商品交易所）
  - CZCE（郑州商品交易所）
  - CFFEX（中国金融期货交易所）
  - GFEX（广州期货交易所）

- **事件驱动架构**：基于 async/await 异步模式，使用 Redis pub/sub 处理实时消息

- **策略框架**：可扩展的策略基类（`BaseModule`），支持：
  - 通过装饰器注册 Redis 频道回调
  - Cron 定时任务支持
  - 异步消息处理

- **数据持久化**：Django ORM 集成 MySQL 后端

- **日志系统**：三层日志（文件、控制台、Redis 发布订阅）

- **技术分析**：集成 TA-Lib 进行行情数据分析

### 系统架构

```mermaid
graph TB
    A[main.py] --> B[TradeStrategy]
    B --> C[BaseModule]
    C --> D[Redis Pub/Sub]
    C --> E[Crontab Scheduler]
    D --> F[CTP Gateway]
    F --> G[Exchanges]
    G --> G1[SHFE]
    G --> G2[DCE]
    G --> G3[CZCE]
    G --> G4[CFFEX]
    G --> G5[GFEX]
    B --> H[Django ORM]
    H --> I[MySQL]
```

### 环境要求

- Python 3.7+
- Redis 服务器
- MySQL 数据库
- TA-Lib C 库

### 安装

1. 先安装 TA-Lib C 库，再安装 Python 依赖：

```bash
pip install -r requirements.txt
```

2. 配置 MySQL 超时设置，在 `/etc/my.cnf.d/server.cnf` 中添加：

```ini
[mysqld]
wait_timeout=31536000
interactive_timeout=31536000
```

3. 首次运行时，配置文件会在 `~/.config/trade-trader/config.ini` 自动生成。编辑该文件配置：

- `[DASHBOARD]` - Django Dashboard 项目路径（必需）
- `[REDIS]` - Redis 连接设置
- `[MYSQL]` - MySQL 数据库连接
- `[MSG_CHANNEL]` - Redis 发布订阅频道模式
- `[TRADE]` - 交易参数
- `[LOG]` - 日志配置

### 运行

启动交易系统：

```bash
python -m trade_trader.main
```

入口点会初始化 Django、设置日志（文件、控制台和 Redis pub/sub）、写入 PID 文件，并运行 `trade_trader/strategy/brother2.py` 中的 `TradeStrategy`。

### 创建自定义策略

继承 `BaseModule` 类创建自定义交易策略：

```python
from trade_trader.strategy import BaseModule
from trade_trader.utils.func_container import RegisterCallback

class MyStrategy(BaseModule):
    @RegisterCallback(channel='MSG:CTP:REQ:*')
    async def on_request(self, channel, data):
        """处理 Redis 频道消息"""
        pass

    @RegisterCallback(crontab='*/5 * * * *')
    async def periodic_task(self):
        """每 5 分钟执行一次"""
        pass
```

### 配置说明

配置文件位置：`~/.config/trade-trader/config.ini`

| 配置节 | 描述 |
|--------|------|
| `[REDIS]` | Redis 连接设置 |
| `[MYSQL]` | 数据库连接 |
| `[MSG_CHANNEL]` | Redis pub/sub 频道模式 |
| `[DASHBOARD]` | Django Dashboard 项目路径 |
| `[TRADE]` | 命令超时和忽略的合约 |
| `[LOG]` | 日志级别和格式 |
| `[QuantDL]`、`[Tushare]` | 第三方 API 密钥 |

### 文档

详细技术文档位于 `docs/` 目录：

- [docs/README.md](docs/README.md) - 文档索引
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - 系统架构（含 Mermaid 图）
- [docs/MODULES.md](docs/MODULES.md) - 模块详解
- [docs/DATA_MODELS.md](docs/DATA_MODELS.md) - Django 数据模型说明
- [docs/CODEBASE_MAP.md](docs/CODEBASE_MAP.md) - 代码库映射

### 项目结构

| 组件 | 文件 | 说明 |
|------|------|------|
| 入口点 | `trade_trader/main.py` | 系统启动 |
| 策略基类 | `trade_trader/strategy/__init__.py` | BaseModule |
| 主策略 | `trade_trader/strategy/brother2.py` | TradeStrategy |
| 配置 | `trade_trader/utils/read_config.py` | 配置管理 |
| 工具 | `trade_trader/utils/__init__.py` | 交易所数据 |
| 模型 | `panel/models.py` | Django ORM 模型 |

### 代码规范

- 大量中文注释和日志消息
- 使用 `ujson` 进行 JSON 解析（比标准 `json` 更快）
- 所有金融计算使用 `Decimal`
- 使用 `price_round()` 函数处理合约特定的价格精度
- 全面使用 async/await 模式配合 `asyncio`
- 错误码从 XML 加载的 `ctp_errors` 字典获取

### 许可证

Apache License 2.0

```
Copyright 2016 timercrack

Licensed under the Apache License, Version 2.0 (the "License"); you may
not use this file except in compliance with the License. You may obtain
a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
License for the specific language governing permissions and limitations
under the License.
```

## CODE-LINT-X

> 别再猜测 `status == 3` 是什么意思了。

[![Crates.io](https://img.shields.io/crates/v/code-lint-x)](https://crates.io/crates/code-lint-x)
[![Documentation](https://docs.rs/code-lint-x/badge.svg)](https://docs.rs/code-lint-x)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**CODE-LINT-X** 是一款静态代码分析工具，用于检测软件架构中的**"概念压缩"（Concept Compression）**—— 当数据字段（尤其是枚举和整型字段）承载了过多职责时，会导致复杂的条件逻辑和可维护性问题。

基于 *"数据冗余 > 控制复杂度"* 的原则，CODE-LINT-X 帮助团队识别并重构那些悄然侵蚀代码质量的"全局枚举"（God Enums）。

---

### 问题所在

```java
// 反模式：概念压缩
if (order.type == 4) { ... }  // type 4 是什么？

// 这个枚举有 15 个值，在 12 个包中的 47 处被使用
enum OrderType {
    NORMAL, PREORDER, VIRTUAL, PRESALE, GROUP, FLASH_SALE, ...
}
```

**这会导致：**

- :hourglass: 阅读代码时的认知负担
- :bug: 因误解含义而产生的隐藏 bug
- :snail: 难以维护和测试
- :x: 跨模块的高耦合

---

### 解决方案

```java
// 显式、自文档化的数据
if (order.is_virtual) { ... }
if (order.requires_payment) { ... }
if (order.is_group_order) { ... }
```

> *"多存一个字段的存储成本，远小于维护一坨 if/else 的认知成本。"*

---

### 核心特性

#### 检测器

| 检测器 | 描述 |
|--------|------|
| **全局枚举检测器** | 使用压缩指数识别职责过多的枚举 |
| **注释异味检测器** | 发现指示设计不良的解释性注释（如 `// special case`、`// TODO` 等） |
| **上下文感知过滤** | 通过识别合法模式（状态机、工厂模式）减少误报 |

#### 多语言支持

| 语言 | 状态 | 特性 |
|------|------|------|
| Java | :white_check_mark: 完整 | 枚举、常量、switch/if 语句 |
| Go | :white_check_mark: 完整 | iota 常量、接口 |
| TypeScript | :white_check_mark: 完整 | 枚举、联合类型、字面量类型 |
| Python | :white_check_mark: 完整 | Enum 类、老式常量 |

#### 团队协作

- **趋势追踪** — 跟踪复杂度随时间的演变
- **团队仪表板** — 模块健康评分、Top 10 高危字段
- **重构建议** — 无 AI 的启发式推荐，包含重构前后代码示例
- **影响分析** — 变更范围、风险等级、工作量估算、迁移计划

#### CI/CD 集成

- Git hooks（pre-commit、pre-push）
- 兼容 GitHub Actions
- JSON/HTML 报告格式
- 通过 `.codelintrc` 配置阈值

---

### 安装

#### 从 Crates.io 安装（推荐）

```bash
cargo install code-lint-x
```

#### 从源码安装

```bash
git clone https://github.com/iannil/code-lint-x.git
cd code-lint-x
cargo install --path .
```

---

### 快速开始

#### 基本扫描

```bash
## 扫描项目
code-lint-x scan ./src

## 生成 HTML 报告
code-lint-x scan ./src --format html --output report.html

## JSON 输出（用于 CI/CD）
code-lint-x scan ./src --format json --output results.json
```

#### 配置文件

在项目根目录创建 `.codelintrc`：

```toml
[thresholds]
compression_index = "warning:30, error:60"

[detector.god_enum]
enabled = true
min_enum_values = 3

[detector.comment_smell]
enabled = true
keywords = ["special case", "except", "temporary", "hack", "TODO"]

[ignore]
paths = ["generated/", "vendor/", "*_test.go"]

[output]
format = "json"
sort_by = "severity"
```

#### Git Hooks

```bash
## 安装 pre-commit hook
code-lint-x hook install

## 卸载 hooks
code-lint-x hook uninstall
```

#### 趋势分析

```bash
## 查看时间趋势
code-lint-x trend --since "1 month ago"

## 按模块分组
code-lint-x trend --by-module

## 按作者分组
code-lint-x trend --by-author
```

---

### 压缩指数

**压缩指数**衡量单个字段承载了多少职责：

```
压缩指数 = (控制流引用次数 × 涉及包数量) / 枚举值数量
```

| 指数 | 等级 | 建议操作 |
|------|------|----------|
| 0-20 | :white_check_mark: 健康 | 无需操作 |
| 20-40 | :warning: 警告 | 考虑重构 |
| 40+ | :x: 严重 | 建议重构 |

---

### 示例输出

```
正在扫描: /path/to/project
├── 发现 23 个可疑枚举
├── 发现 47 条注释异味

┌─ 全局枚举检测 ─────────────────────────────────────────┐
│                                                      │
│  OrderType (order/Order.java:42)                     │
│  ├── 压缩指数: 87.3  :x: 严重                          │
│  ├── 15 个枚举值                                      │
│  ├── 47 处控制流引用                                   │
│  ├── 用于 12 个包                                     │
│  │                                                   │
│  重构建议:                                            │
│  ├─ 拆分为: is_virtual, is_preorder, is_group         │
│  ├─ 新枚举: FulfillmentMode, PaymentMethod            │
│  └─ 预期压缩指数: 87.3 → 12.4                          │
│                                                      │
└──────────────────────────────────────────────────────┘
```

---

### 命令行参考

```bash
code-lint-x <命令> [选项]

命令:
    scan        扫描源码中的概念压缩
    trend       显示趋势分析
    dashboard   生成团队仪表板
    hook        管理 git hooks
    config      配置管理
    init        初始化配置文件
    help        显示帮助信息

扫描选项:
    -p, --path <PATH>           扫描路径 [默认: .]
    -f, --format <FORMAT>       输出格式: text|json|html [默认: text]
    -o, --output <FILE>         输出到文件
    -c, --config <FILE>         配置文件 [默认: .codelintrc]
    --since <COMMIT>            增量扫描（从指定提交开始）
```

---

### 架构

```
src/
#├── analyzer/      # 智能重构建议
#│   ├── suggester.rs    # 生成可操作的建议
#│   ├── impact.rs       # 分析变更影响和范围
#│   └── context.rs      # 上下文感知过滤
#├── cli/           # 命令行接口
#├── config/        # 配置文件解析
#├── core/          # AST 定义、追踪器、指标
#├── detector/      # 检测算法
#│   ├── god_enum.rs     # 压缩指数计算器
#│   └── comment_smell.rs # 注释模式匹配
#├── integration/   # Git hooks、CI/CD
#├── lang/          # 语言特定解析器
│   ├── java/
│   ├── go/
│   ├── typescript/
│   └── python/
#├── report/        # 报告生成
#│   ├── trend.rs        # 历史趋势追踪
#│   └── dashboard.rs    # 团队仪表板 HTML
#└── scanner/       # 项目扫描协调器
```

---

### 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

---

### 开发

```bash
## 克隆仓库
git clone https://github.com/iannil/code-lint-x.git
cd code-lint-x

## 运行测试
cargo test

## 带调试输出运行
cargo run -- scan ./tests/fixtures --format json

## 格式化代码
cargo fmt

## 运行 linter
cargo clippy
```

---

### 设计哲学

> *"你的代码正试图告诉你一些东西。你在听吗？"*

我们习惯用 Linter 检查语法错误。但谁来检查**设计的腐坏**？

CODE-LINT-X 是首个关注**概念压缩**的静态分析工具——软件架构的无声杀手。

别让一个 `Integer` 毁了你的架构。**解压它。**

---

### 许可证

[MIT License](LICENSE)

---

### 链接

- [文档](docs/index.md)
- [项目状态](docs/project/status.md)
- [架构设计](docs/project/architecture.md)
- [书籍](https://zhurongshuo.com/practices/season-4/data-as-the-boundary/) — 《数据即边界：重构软件复杂度》

---

### 致谢

灵感来源：

- *Yoni Goldberg* 的 [Node.js Best Practices](https://github.com/goldbergyoni/nodebestpractices)
- *Martin Fowler* 的 [Refactoring](https://refactoring.com/)
- *Michael Feathers* 的《修改代码的艺术》

## CODE-LEDGE-X

> 数据设计优于代码逻辑 —— 通过增加数据冗余，消除代码复杂度

[English](README.md) | [中文](README.zh-CN.md)

---

### 项目简介

CODE-LEDGE-X 是一个将架构哲学与开源实践相结合的项目，包含两部分：

1. 《[数据即边界：重构软件复杂度](https://zhurongshuo.com/practices/season-4/data-as-the-boundary/)》 —— 系统架构实战书籍
2. CODE-LEDGE-X —— 基于书中理论的通用分布式业务对账引擎

---

### CODE-LEDGE-X MVP 快速开始

CODE-LEDGE-X 是一个通用的分布式业务对账引擎，支持多种数据源（CSV、PostgreSQL 等）的自动对账和差异报告生成。

#### 安装

```bash
## 克隆仓库
git clone https://github.com/iannil/code-ledge-x.git
cd code-ledge-x

## 编译
make build

## 或使用 Go 直接编译
go build -o bin/ledgex ./cmd/ledgex
```

#### 使用示例

```bash
## 使用示例配置运行对账
./bin/ledgex run --config ./configs/example.yaml

## 查看版本信息
./bin/ledgex version

## 初始化配置文件
./bin/ledgex init
```

#### 配置文件示例

```yaml
job: "order_vs_payment"
output: "./report.html"

source_left:
  type: "csv"
  path: "./testdata/left.csv"
  mapping:
    biz_id: "order_id"
    amount: "total_fee"
    state: "status"

source_right:
  type: "csv"
  path: "./testdata/right.csv"
  mapping:
    biz_id: "payment_id"
    amount: "amount"
    state: "status"

rules:
  - type: "existence"
  - type: "amount_match"
  - type: "state_match"
```

#### 对账结果

运行后会生成 HTML 差异报告，包含：

- 记录统计（左/右侧记录数、匹配数、差异数）
- 差异明细（仅左侧存在、仅右侧存在、金额不一致、状态不一致）

#### 开发

```bash
## 运行测试
make test

## 代码格式化
make fmt
```

---

### 核心哲学

> "多存一个字段的存储成本，远小于维护 if/else 逻辑的认知成本。"

本书提出的概念解压理论指出：

| 层次 | 含义 | 示例 |
|------|------|------|
| 微观 | 字段拆解 | `order_type` → `fulfillment_type` + `promotion_type` + `payment_terms` |
| 中观 | 表结构分离 | 业务表 vs 审计表（快照模式） |
| 宏观 | 系统边界 | Intent（入站事件）vs Result（业务状态）双重账本 |

---

### 快速导航

#### LedgeX 开源项目

- [项目提案](docs/ledgex/proposal.md) —— 产品定位与设计哲学
- [架构设计](docs/ledgex/architecture.md) —— 四层架构详解
- [技术栈](docs/ledgex/tech-stack.md) —— 技术选型与理由
- [实施阶段](docs/ledgex/phases.md) —— 三阶段路线图
- [API 文档](docs/ledgex/api.md) —— 核心接口定义

#### 开发相关

- [开发指南](docs/development.md) —— 贡献者指南
- [进度追踪](docs/progress/status.md) —— 开发进度与待办
- [路线图](docs/progress/roadmap.md) —— 项目里程碑

---

### 目标读者

- 高级开发工程师 / 技术专家
- 系统架构师
- 技术管理者 / CTO

---

### 许可证

本项目采用 [Apache 2.0](LICENSE) 开源协议。

<div align="center">

## PII-AIRLOCK

#### 让公有 LLM 变私有 — 面向 LLM API 的 PII 保护中间件

[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.1.0-blue.svg)](https://github.com/pii-airlock/pii-airlock/releases)
[![Tests](https://img.shields.io/badge/tests-600%20passed-brightgreen.svg)](https://github.com/pii-airlock/pii-airlock/actions)
[![Coverage](https://img.shields.io/badge/coverage-73%25-green.svg)](https://github.com/pii-airlock/pii-airlock/actions)
[![Code style: ruff](https://img.shields.io/badge/code%20style-ruff-000000.svg)](https://github.com/astral-sh/ruff)

[English](README.md) | [文档](docs/) | [更新日志](docs/progress/changelog.md)

---

PII-AIRLOCK 是一个开源中间件/反向代理，用于在使用公有 LLM API 时保护敏感个人信息。将其部署在您的应用程序和 LLM 提供商（OpenAI、Claude 等）之间，可自动实时检测、脱敏和还原 PII。

</div>

---

### 概述

```
┌─────────────────┐     ┌─────────────────────────────────────────┐     ┌─────────────────┐
│                 │     │           PII-AIRLOCK (v1.1)            │     │                 │
│  您的应用       │────▶│  ┌─────────┐    ┌─────────────────┐     │────▶│   OpenAI API    │
│  (Dify/Flowise) │     │  │  脱敏   │────│   映射存储      │     │     │   Claude API    │
│                 │◀────│  └─────────┘    │   + 缓存        │     │◀────│   Azure OpenAI  │
└─────────────────┘     └─────────────────────────────────────────┘     └─────────────────┘
                              ▲
                   多租户 │ 合规 │ 审计 │ 管理界面
```

### 核心特性

#### 基础能力

| 特性 | 说明 |
| ------- | ----------- |
| 零代码接入 | 只需修改 `base_url` - 完全兼容 OpenAI API 格式 |
| 智能脱敏 | 语义化占位符（`<PERSON_1>`），LLM 可自然理解 |
| 流式支持 | 处理 SSE 流式响应，智能缓冲区处理分割的占位符 |
| 模糊恢复 | 即使 LLM 修改了占位符格式也能恢复 PII |
| 自定义规则 | 通过 YAML 配置定义企业专属 PII 模式 |
| 意图检测 | 智能上下文感知检测，在询问语境中跳过脱敏 |
| 密钥扫描 | 在发送到 LLM 之前检测 API 密钥、令牌等秘密信息 |

#### 企业级特性

| 特性 | 说明 |
| ------- | ----------- |
| 多租户 | 租户隔离，独立配置和限流 |
| 响应缓存 | LLM 响应缓存，降低 API 成本和延迟 |
| 配额管理 | 支持按小时/天/月的请求/令牌配额限制 |
| API 密钥管理 | 安全的 API 密钥创建和生命周期管理 |
| RBAC | 基于角色的访问控制（Admin/Operator/Viewer/User） |
| 合规预设 | 预配置的 GDPR、CCPA、PIPL、金融合规规则 |
| 白名单 | 公众人物、地名等安全实体白名单 |
| 审计日志 | 完整的审计追踪，支持查询和导出 |
| Web 管理控制台 | 全功能管理界面 |
| 生产就绪 | 结构化日志、Prometheus 指标、API 限流 |

#### 脱敏策略

| 策略 | 说明 | 示例 | 适用场景 |
| -------- | ----------- | ------- | -------- |
| placeholder | 类型化占位符 | `张三` → `<PERSON_1>` | LLM 处理（默认） |
| hash | SHA256 哈希 | `张三` → `a1b2c3d4...` | 日志分析、去重 |
| mask | 部分掩码 | `13800138000` → `138****8000` | 界面显示 |
| redact | 完全替换 | `test@example.com` → `[REDACTED]` | 最高隐私 |
| synthetic | 仿真数据替换 | `张三` → `李明` | 测试、演示 |

### 支持的 PII 类型

| 类型 | 占位符 | 示例 |
| ---- | ----------- | ------- |
| 姓名 | `<PERSON_N>` | 张三 → `<PERSON_1>` |
| 手机号 | `<PHONE_N>` | 13800138000 → `<PHONE_1>` |
| 邮箱 | `<EMAIL_N>` | test@example.com → `<EMAIL_1>` |
| 身份证 | `<ID_CARD_N>` | 110101199003077758 → `<ID_CARD_1>` |
| 银行卡 | `<CREDIT_CARD_N>` | 6222021234567890 → `<CREDIT_CARD_1>` |
| IP 地址 | `<IP_N>` | 192.168.1.1 → `<IP_1>` |
| 自定义 | 可配置 | PROJ-2024-AB → `<PROJECT_CODE_1>` |

### 快速开始

#### 安装

```bash
## 克隆仓库
git clone https://github.com/pii-airlock/pii-airlock.git
cd pii-airlock

## 创建虚拟环境
python -m venv .venv
#source .venv/bin/activate  # Windows: .venv\Scripts\activate

## 安装依赖
pip install -e .

## 下载中文 NLP 模型（中文 PII 检测必需）
python -m spacy download zh_core_web_trf
```

#### 启动服务

```bash
## 设置 OpenAI API 密钥
export OPENAI_API_KEY=sk-your-api-key

## 启动代理服务
python -m pii_airlock.main

## 服务运行在 http://localhost:8000
## API 文档: http://localhost:8000/docs
## Web UI: http://localhost:8000/ui
## 调试控制台: http://localhost:8000/debug
## 管理控制台: http://localhost:8000/admin
```

#### 使用 OpenAI Python 客户端

```python
from openai import OpenAI

## 只需将 base_url 指向 PII-AIRLOCK
client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="sk-your-api-key"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "给张三（john@example.com）写一封关于会议的邮件。"}
    ]
)

print(response.choices[0].message.content)
## PII 会在发送到 OpenAI 之前自动脱敏，
## 并在响应中自动还原
```

#### 流式支持

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="sk-your-api-key"
)

## 流式响应同样支持
stream = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "介绍一下张三（电话：13800138000）"}
    ],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="", flush=True)
## 即使占位符被分割到多个 chunk，也能正确处理
```

#### Docker 部署

```bash
## 使用 docker-compose（推荐）
docker-compose up -d

## 或手动构建运行
docker build -t pii-airlock .
docker run -p 8000:8000 -e OPENAI_API_KEY=sk-xxx pii-airlock
```

### Web 界面

#### 测试界面 (`/ui`)
简单的 Web 界面，用于测试 PII 检测和脱敏，无需调用 LLM。

#### 调试控制台 (`/debug`)
可视化调试界面，具备：
- 原始文本与脱敏文本并排对比
- PII 高亮显示，不同类型颜色区分
- 交互式工具提示显示详细信息
- 导出映射数据为 JSON

#### 管理控制台 (`/admin`)
全功能管理界面，包括：
- **仪表盘**：系统统计、最近活动、实时状态
- **合规配置**：激活/切换合规预设（GDPR/CCPA/PIPL/金融）
- **白名单管理**：添加/删除白名单条目、批量导入
- **审计日志**：查询、过滤、导出审计记录

### 配置

#### 环境变量

| 变量 | 说明 | 默认值 |
| -------- | ----------- | ------- |
| **基础配置** |
| `OPENAI_API_KEY` | OpenAI API 密钥 | - |
| `PII_AIRLOCK_UPSTREAM_URL` | 上游 LLM API 地址 | `https://api.openai.com` |
| `PII_AIRLOCK_PORT` | 服务端口 | `8000` |
| `PII_AIRLOCK_MAPPING_TTL` | 映射过期时间（秒） | `300` |
| `PII_AIRLOCK_INJECT_PROMPT` | 注入防幻觉提示 | `true` |
| **多租户** |
| `PII_AIRLOCK_MULTI_TENANT_ENABLED` | 启用多租户模式 | `false` |
| `PII_AIRLOCK_TENANT_CONFIG_PATH` | tenants.yaml 路径 | - |
| **缓存** |
| `PII_AIRLOCK_CACHE_ENABLED` | 启用响应缓存 | `false` |
| `PII_AIRLOCK_CACHE_TTL` | 缓存 TTL（秒） | `3600` |
| `PII_AIRLOCK_CACHE_MAX_SIZE` | 最大缓存条目数 | `10000` |
| **安全** |
| `PII_AIRLOCK_SECRET_SCAN_ENABLED` | 启用密钥扫描 | `true` |
| **日志** |
| `PII_AIRLOCK_LOG_LEVEL` | 日志级别 | `INFO` |
| `PII_AIRLOCK_LOG_FORMAT` | 日志格式 (json/text) | `json` |
| **限流** |
| `PII_AIRLOCK_RATE_LIMIT` | 限流配置 | `60/minute` |
| `PII_AIRLOCK_RATE_LIMIT_ENABLED` | 启用限流 | `true` |

#### 自定义 PII 规则

创建 `config/custom_patterns.yaml`：

```yaml
patterns:
  - name: employee_id
    entity_type: EMPLOYEE_ID
    regex: "EMP[A-Z]\\d{6}"
    score: 0.85
    context:
      - 员工
      - 工号
      - employee

  - name: project_code
    entity_type: PROJECT_CODE
    regex: "PROJ-\\d{4}-[A-Z]{2}"
    score: 0.9
    context:
      - 项目
      - 编号
```

设置配置路径：

```bash
export PII_AIRLOCK_CONFIG_PATH=./config/custom_patterns.yaml
```

#### 合规预设

预配置的合规预设位于 `config/compliance_presets/`：

- **GDPR** (`gdpr.yaml`)：欧洲数据保护规则
- **CCPA** (`ccpa.yaml`)：加州消费者隐私法案
- **PIPL** (`pipl.yaml`)：中国个人信息保护法
- **Financial** (`financial.yaml`)：金融行业合规规则

### API 端点

#### OpenAI 兼容 API

| 端点 | 方法 | 说明 |
| -------- | ------ | ----------- |
| `/v1/chat/completions` | POST | 带 PII 保护的聊天补全 |
| `/v1/models` | GET | 列出可用模型 |

#### 管理 API

| 端点 | 方法 | 说明 |
| -------- | ------ | ----------- |
| **合规** |
| `/api/v1/compliance/presets` | GET | 列出可用预设 |
| `/api/v1/compliance/active` | GET/POST | 获取/设置活跃预设 |
| **白名单** |
| `/api/v1/allowlist` | GET | 列出所有白名单 |
| `/api/v1/allowlist/{name}/entries` | GET/POST/DELETE | 管理白名单条目 |
| **审计** |
| `/api/v1/audit/events` | GET | 查询审计事件 |
| `/api/v1/audit/export` | GET | 导出审计日志 |
| **意图检测** |
| `/api/v1/intent/patterns` | GET/POST | 管理意图模式 |

#### 监控与测试

| 端点 | 说明 |
| -------- | ----------- |
| `/health` | 健康检查 |
| `/metrics` | Prometheus 指标 |
| `/ui` | Web 测试界面 |
| `/debug` | 可视化调试控制台 |
| `/admin` | 管理控制台 |
| `/api/test/anonymize` | 测试脱敏 |
| `/api/test/deanonymize` | 测试还原 |

### 编程接口

```python
from pii_airlock import Anonymizer, Deanonymizer
from pii_airlock.core.strategies import StrategyConfig, StrategyType

## 基础脱敏
anonymizer = Anonymizer()
result = anonymizer.anonymize("联系张三，邮箱 john@example.com")
#print(result.text)  # 联系 <PERSON_1>，邮箱 <EMAIL_1>
#print(result.mapping.get_original("<PERSON_1>"))  # 张三

## 还原
deanonymizer = Deanonymizer()
restored = deanonymizer.deanonymize(result.text, result.mapping)
#print(restored.text)  # 联系张三，邮箱 john@example.com

## 使用自定义策略
strategy_config = StrategyConfig({
    "PERSON": StrategyType.MASK,
    "PHONE_NUMBER": StrategyType.REDACT,
})
anonymizer = Anonymizer(strategy_config=strategy_config)
result = anonymizer.anonymize("张三的电话是13800138000")
#print(result.text)  # 张*的电话是[REDACTED]

## 禁用意图检测实现严格脱敏
anonymizer = Anonymizer(enable_intent_detection=False)
#result = anonymizer.anonymize("谁是张三？")  # 张三会被脱敏
```

### 工作原理

```
1. 拦截    → 捕获入站请求
2. 密钥扫描 → 检查 API 密钥/令牌（发现则阻止）
3. 脱敏    → 使用 NLP 检测 PII，替换为占位符
4. 检查缓存 → 如有缓存响应则返回
5. 检查配额 → 验证配额限制
6. 映射    → 存储占位符到原始值的映射
7. 转发    → 将脱敏后的提示发送给上游 LLM
8. 缓存    → 存储响应供后续请求使用
9. 还原    → 替换响应中的占位符
10. 审计   → 记录事务日志
11. 返回   → 将还原后的响应返回给客户端
```

#### 处理 LLM 幻觉

LLM 可能会修改占位符（例如 `<PERSON_1>` → `<Person 1>`）。PII-AIRLOCK 通过以下方式处理：

1. **系统提示注入**：指示 LLM 原样保留占位符
2. **模糊匹配**：使用灵活的正则模式匹配修改后的占位符

### 开发

```bash
## 安装开发依赖
pip install -e ".[dev]"

## 运行测试
pytest

## 运行测试（带覆盖率）
pytest --cov=pii_airlock --cov-report=term-missing

## 代码检查
ruff check src/ tests/

## 类型检查
mypy src/
```

### 项目结构

```
pii-airlock/
├── src/pii_airlock/
#│   ├── core/                   # 核心脱敏引擎
#│   │   ├── anonymizer.py       # 主要脱敏逻辑
#│   │   ├── deanonymizer.py     # 带模糊匹配的还原
#│   │   ├── mapping.py          # PII 映射管理
#│   │   ├── strategies.py       # 脱敏策略（5种）
#│   │   ├── stream_buffer.py    # SSE 流式缓冲
#│   │   ├── intent_detector.py  # 上下文感知意图检测
#│   │   ├── synthetic/          # 仿真数据生成
#│   │   ├── fuzzy/              # 模糊匹配引擎
#│   │   └── secret_scanner/     # 密钥检测
#│   ├── api/                    # FastAPI 路由和代理
#│   │   ├── routes.py           # API 端点（含 Web UI）
#│   │   ├── proxy.py            # 代理服务逻辑
#│   │   ├── compliance_api.py   # 合规管理 API
#│   │   ├── allowlist_api.py    # 白名单管理 API
#│   │   ├── audit_api.py        # 审计日志 API
#│   │   └── intent_api.py       # 意图检测 API
#│   ├── auth/                   # 认证与授权
#│   │   ├── tenant.py           # 多租户支持
#│   │   ├── api_key.py          # API 密钥管理
#│   │   ├── rbac.py             # 基于角色的访问控制
#│   │   └── quota.py            # 配额管理
#│   ├── audit/                  # 审计日志
#│   │   ├── models.py           # 审计事件模型
#│   │   ├── store.py            # 审计存储
#│   │   └── logger.py           # 审计日志器
#│   ├── cache/                  # 响应缓存
#│   │   └── llm_cache.py        # LLM 响应缓存
#│   ├── config/                 # 配置加载
#│   │   ├── pattern_loader.py   # 自定义规则加载器
#│   │   └── compliance_loader.py# 合规预设加载器
#│   ├── recognizers/            # PII 识别器
#│   │   ├── zh_phone.py         # 中国手机号识别
#│   │   ├── zh_id_card.py       # 中国身份证识别
#│   │   ├── zh_person.py        # 中文姓名识别
#│   │   ├── allowlist.py        # 白名单识别器
#│   │   ├── entropy_detector.py # 高熵值检测器
#│   │   └── registry.py         # 识别器注册表
#│   ├── static/                 # 静态文件
#│   │   ├── debug.html          # 调试控制台
#│   │   └── admin.html          # 管理控制台
#│   ├── storage/                # 存储后端
#│   ├── logging/                # 结构化日志
#│   └── metrics/                # Prometheus 指标
#├── config/                     # 配置文件
#│   ├── compliance_presets/     # GDPR, CCPA, PIPL, 金融
#│   └── allowlists/             # 白名单文件
#├── tests/                      # 测试套件（600+ 测试）
#├── docs/                       # 文档
├── Dockerfile
├── docker-compose.yml
└── pyproject.toml
```

### 应用场景

- **企业合规**：使用 GPT-4/Claude 同时满足 GDPR、CCPA、PIPL 要求
- **低代码平台**：为 Dify、Flowise、LangFlow 添加网关
- **医疗/金融**：使用云端 LLM 安全处理敏感数据
- **开发测试**：无需暴露真实 PII 测试 LLM 应用
- **多团队协作**：共享基础设施，隔离配置和配额

### 贡献

我们欢迎贡献！请查看我们的 [贡献指南](CONTRIBUTING.md) 了解详情。

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

### 致谢

- [Microsoft Presidio](https://github.com/microsoft/presidio) - PII 检测引擎
- [spaCy](https://spacy.io/) - NLP 框架
- [FastAPI](https://fastapi.tiangolo.com/) - Web 框架
- [OpenAI](https://openai.com/) - LLM API

---

<div align="center">

由 PII-AIRLOCK 团队用心打造

[在 GitHub 上给我们 Star](https://github.com/pii-airlock/pii-airlock) — 这对我们很重要！

</div>

## Open-Retro-Sight

> 非侵入式工业边缘AI网关 - 给老机器装上"数字眼睛"

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.9+](https://img.shields.io/badge/python-3.9+-blue.svg)](https://www.python.org/downloads/)

[English](README.md)

### 简介

Open-RetroSight 是一款非侵入式的工业边缘AI网关软件，通过计算机视觉技术将传统"哑设备"的数据数字化。

**不拆机、不停产、不改线** —— 用几百元成本，几分钟部署，实现老旧设备的数字化改造。

### 核心能力

- **七段数码管/LCD屏幕识别** - 读取温度、压力、计数等数字
- **指针式仪表读取** - 通过指针角度检测映射为数值
- **状态指示灯识别** - Andon灯识别，计算OEE
- **开关/旋钮位置检测** - 识别档位状态

### 快速开始

```bash
## 克隆项目
git clone https://github.com/open-retrosight/open-retrosight.git
cd open-retrosight

## 安装依赖
pip install -r requirements.txt

## 或使用 pip 安装
pip install -e .
```

### 文档

详细文档请查看 [docs/](docs/index.md)：

- [项目介绍](docs/getting-started/introduction.md) - 背景、价值、适用场景
- [技术架构](docs/architecture/overview.md) - 系统设计与技术栈
- [应用场景](docs/use-cases/) - 各类设备的识别方案
- [产品规划](docs/roadmap/) - 愿景、MVP功能、商业化

### 硬件要求

- Raspberry Pi Zero 2W / 香橙派 + USB摄像头
- 或 退役Android手机
- 或 ESP32-CAM + 边缘服务器

### 技术栈

`Python` `OpenCV` `PaddleOCR` `YOLOv8` `MQTT` `Modbus TCP` `Streamlit`

### 项目状态

当前阶段：**MVP v0.1.0 完成**

#### 已完成里程碑

- [x] 产品规划与设计
- [x] 文档体系建立
- [x] 项目结构初始化
- [x] Phase 1: 基础能力（视频流、OCR、MQTT、Web界面）
- [x] Phase 2: 核心功能（指针识别、透视校正、Modbus、断网续传）
- [x] Phase 3: 扩展功能（指示灯、开关识别、图像增强）

#### 发展路线

| 阶段 | 重点 | 状态 |
|------|------|------|
| **v0.1** | MVP 核心功能 | ✅ 完成 |
| **v0.2** | 社区建设 | 计划中 |
| **v0.3** | 硬件套件与产品打磨 | 计划中 |
| **v0.4** | 云平台上线（SaaS） | 计划中 |
| **v1.0** | 生态完善与算法市场 | 计划中 |

##### 近期计划 (v0.2)
- [ ] 建立开发者社区
- [ ] 完善文档和教程
- [ ] 收集用户反馈、持续迭代
- [ ] 增加更多设备识别模板

##### 未来规划
- **硬件套件**：开箱即用的摄像头、补光灯、防护外壳套装
- **高级算法**：复杂仪表识别、单摄像头多目标检测
- **云平台**：多设备管理、数据存储、报警推送
- **行业解决方案**：制造业、能源、化工、物流等垂直领域

#### Phase 1 已实现功能

- **视频流采集** (`src/retrosight/capture/camera.py`)
  - USB/CSI/RTSP 摄像头支持
  - 多线程帧缓冲

- **数字 OCR 识别** (`src/retrosight/recognition/ocr.py`)
  - 七段数码管识别 (PaddleOCR)
  - 图像预处理优化

- **数据平滑滤波** (`src/retrosight/preprocessing/filter.py`)
  - 卡尔曼滤波、滑动平均、指数平滑
  - 异常值过滤

- **MQTT 数据发布** (`src/retrosight/output/mqtt.py`)
  - 断线重连、离线缓存
  - 结构化 JSON 数据

- **Web 配置界面** (`src/retrosight/ui/app.py`)
  - Streamlit 实时预览
  - ROI 区域配置

#### Phase 2 已实现功能

- **透视变换校正** (`src/retrosight/preprocessing/transform.py`)
  - 四点透视变换
  - 图像配准（抗振动）
  - 镜头畸变校正

- **指针识别算法** (`src/retrosight/recognition/pointer.py`)
  - 霍夫线变换检测指针
  - 自动表盘中心检测
  - 角度到数值映射

- **Modbus TCP 输出** (`src/retrosight/output/modbus.py`)
  - Modbus TCP Server（伪装 PLC）
  - 多数据类型支持（INT16/FLOAT32等）
  - 寄存器自动分配

- **断网续传缓存** (`src/retrosight/output/buffer.py`)
  - SQLite 持久化存储
  - 优先级队列
  - 自动重试与过期清理

#### Phase 3 已实现功能

- **指示灯识别** (`src/retrosight/recognition/light.py`)
  - HSV 颜色检测（红/黄/绿/蓝/白）
  - Andon 三色灯塔状态监控
  - 闪烁检测与频率估算
  - OEE 可用率计算

- **开关/旋钮识别** (`src/retrosight/recognition/switch.py`)
  - 拨动开关 ON/OFF 检测
  - 旋钮档位识别（多档位支持）
  - 按钮/滑动开关状态
  - 模板匹配与位置检测

- **图像增强** (`src/retrosight/preprocessing/enhancement.py`)
  - 自适应对比度增强（CLAHE）
  - 去反光处理（inpainting）
  - 多帧融合降噪
  - 低光照增强

### 许可证

MIT License

### 相关链接

- [文档中心](docs/index.md)
- [产品愿景](docs/roadmap/vision.md)
- [技术挑战](docs/roadmap/challenges.md)

## V-Waitlist

[English](./README.md)

<div align="center">

> 面向独立开发者的开源、无服务器病毒式等待名单系统

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/iannil/v-waitlist&env=UPSTASH_REDIS_REST_URL,UPSTASH_REDIS_REST_TOKEN,ADMIN_SECRET_KEY&project-name=v-waitlist&repository-name=v-waitlist)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/iannil/v-waitlist)

</div>

---

### 什么是 V-Waitlist？

**V-Waitlist** 是 Viral Loops 等付费服务的零成本替代方案。构建带有推荐系统、排行榜和社交分享的病毒式等待名单。

- **$0/月** - 使用 Vercel + Upstash Redis 的免费版
- **3 行代码集成** - 放入 `<v-waitlist>` 即可
- **极致性能** - Edge Functions + Redis，响应 <100ms
- **数据自主** - 存储在你自己的 Redis 实例中

---

### 功能特性

| 功能 | 说明 |
|------|------|
| 🎯 **病毒式推荐** | 用户通过邀请他人提升排名 |
| 📊 **实时排名** | Redis 驱动，毫秒级响应 |
| 🛡️ **防刷保护** | Cloudflare Turnstile + 速率限制 + 邮箱过滤 |
| 📤 **数据导出** | 导出 CSV 用于邮件营销 |
| 🌙 **深色模式** | 内置亮色和深色主题 |
| 🎨 **可定制** | 通过属性自定义颜色 |
| ⚡ **Edge Runtime** | 全球分发，冷启动亚秒级 |

---

### 快速开始

#### 1. 部署到 Vercel

点击上方的 "Deploy with Vercel" 按钮。你需要：

- 一个 [Upstash Redis](https://upstash.com) 账号（免费版即可）
- 你的 Redis REST URL 和 Token

#### 2. 添加到你的网站

```html
<script src="https://your-app.vercel.app/sdk.js"></script>
<v-waitlist project-id="your-project-id"></v-waitlist>
```

#### 3. 完成

你的等待名单现在已上线。用户可以注册并开始邀请他人。

---

### 配置选项

#### 属性

| 属性 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `project-id` | string | 是 | - | 你的项目唯一标识符 |
| `mode` | string | 否 | `input` | `input` 或 `modal` |
| `theme` | string | 否 | `light` | `light` 或 `dark` |
#| `primary-color` | string | 否 | `#000000` | 主色调（十六进制） |
| `api-base-url` | string | 否 | (当前域名) | 自定义 API 基础 URL |

#### 示例

```html
<v-waitlist
  project-id="my-product"
  mode="modal"
  theme="dark"
  #primary-color="#6366f1"
></v-waitlist>
```

---

### 自托管

```bash
## 克隆仓库
git clone https://github.com/iannil/v-waitlist.git
cd v-waitlist

## 安装依赖
pnpm install

## 配置环境变量
cp .env.example apps/api/.env.local
```

编辑 `apps/api/.env.local` 填入你的凭据：

```bash
UPSTASH_REDIS_REST_URL=https://xxx.upstash.io
UPSTASH_REDIS_REST_TOKEN=AXXXX...
TURNSTILE_SITE_KEY=0x...
TURNSTILE_SECRET_KEY=0x...
ADMIN_SECRET_KEY=your-secret-key
```

```bash
## 运行开发服务器
pnpm dev

## 构建生产版本
pnpm build
```

---

### 开发

```bash
## 安装依赖
pnpm install

## 运行 API 开发服务器 (http://localhost:3000)
pnpm --filter @v-waitlist/api dev

## 运行 SDK 开发服务器 (http://localhost:5173)
pnpm --filter @v-waitlist/sdk dev

## 运行 Web 开发服务器 (http://localhost:3001)
pnpm --filter @v-waitlist/web dev

## 运行所有开发服务器
pnpm dev

## 构建所有包
pnpm build

## 运行测试
pnpm test
```

---

### 项目结构

```
v-waitlist/
├── apps/
#│   ├── api/              # Next.js API (Edge Runtime)
│   │   ├── app/api/
#│   │   │   ├── join/     # POST /api/join
#│   │   │   ├── status/   # GET /api/status
#│   │   │   └── export/   # GET /api/export
│   │   └── lib/
│   │       ├── redis.ts
│   │       ├── redis-scripts.ts
│   │       ├── turnstile.ts
│   │       └── utils.ts
#│   └── web/              # 落地页
├── packages/
#│   └── sdk/              # 前端组件 (Preact + Vite)
│       └── dist/
#│           └── v-waitlist.min.js  # 19KB (7.89KB gzipped)
#├── docs/                 # 文档
#├── LICENSE               # MIT 许可证
#└── vercel.json           # 部署配置
```

---

### API 文档

#### POST /api/join

注册新用户到等待名单。

**请求：**

```json
{
  "email": "user@example.com",
  "projectId": "my-project",
  "referrerCode": "abc12345",
  "turnstileToken": "0x..."
}
```

**响应：**

```json
{
  "success": true,
  "refCode": "def67890",
  "rank": 543,
  "total": 1002,
  "shareUrl": "?ref=def67890"
}
```

#### GET /api/status

获取用户当前排名和统计信息。

**请求：**

```
GET /api/status?email=user@example.com&projectId=my-project
```

**响应：**

```json
{
  "success": true,
  "rank": 543,
  "total": 1002,
  "aheadOf": 459,
  "refCode": "abc12345",
  "referralCount": 3,
  "shareUrl": "?ref=abc12345"
}
```

#### GET /api/export

导出所有用户为 CSV（需要管理员密钥）。

**请求：**

```
GET /api/export?projectId=my-project
Authorization: Bearer YOUR_ADMIN_SECRET_KEY
```

**响应：**

```
email,ref_code,referred_by,referral_count,created_at,rank
user1@example.com,abc12345,,5,2025-01-19T10:30:00Z,10
```

完整 API 文档请参阅 [`docs/02-api-design.md`](./docs/02-api-design.md)。

---

### 环境变量

| 变量 | 必填 | 说明 |
|------|------|------|
| `UPSTASH_REDIS_REST_URL` | 是 | Upstash Redis REST API URL |
| `UPSTASH_REDIS_REST_TOKEN` | 是 | Upstash Redis 认证令牌 |
| `TURNSTILE_SITE_KEY` | 否 | Cloudflare Turnstile Site Key |
| `TURNSTILE_SECRET_KEY` | 否 | Cloudflare Turnstile Secret Key |
| `ADMIN_SECRET_KEY` | 是 | `/api/export` 的管理密钥 |

---

### 文档

- [架构设计](./docs/01-architecture.md) - 系统设计和技术栈
- [API 设计](./docs/02-api-design.md) - API 端点和数据模型
- [前端 SDK](./docs/03-frontend-sdk.md) - 组件集成指南
- [安全防护](./docs/04-security.md) - 防刷措施
- [开发路线图](./docs/05-roadmap.md) - 开发时间线
- [进度追踪](./docs/00-progress.md) - 当前开发状态
- [验收报告](./docs/08-acceptance-report.md) - 项目验收报告

---

### 技术栈

- **计算层**: Next.js with Edge Runtime (Vercel)
- **存储层**: Upstash Redis (无服务器 Redis)
- **前端**: Preact + Vite
- **安全层**: Cloudflare Turnstile
- **UI**: Web Components with Shadow DOM

---

### 对比

| | V-Waitlist | Viral Loops |
|---|---|---|
| **价格** | $0 | $99/月起 |
| **性能** | <100ms | 较慢 |
| **数据** | 你自己的 Redis | 他们的平台 |
| **集成难度** | 3 行代码 | 复杂 |

---

### 贡献

欢迎贡献！随时提交 Pull Request。

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

### 许可证

[MIT](./LICENSE) - Copyright (c) 2025 V-Waitlist Contributors

---

**Built with** - [Next.js](https://nextjs.org) · [Upstash Redis](https://upstash.com) · [Preact](https://preactjs.com) · [Cloudflare](https://cloudflare.com)

## Open Remote ID Parser (ORIP)

<p align="center">
  <strong>高性能、跨平台的无人机 Remote ID 信号解析库</strong>
</p>

<p align="center">
  #<a href="#特性">特性</a> •
  #<a href="#安装">安装</a> •
  #<a href="#快速开始">快速开始</a> •
  #<a href="#api-参考">API</a> •
  #<a href="#从源码构建">构建</a> •
  #<a href="#贡献">贡献</a>
</p>

<p align="center">
  <a href="README.md">English</a> | <b>中文</b>
</p>

---

**Open Remote ID Parser** 是一个轻量级 C++ 库，用于解码无人机 Remote ID 广播。它支持多种协议（ASTM F3411、ASD-STAN）和传输层（蓝牙 Legacy/Extended、WiFi Beacon/NAN），非常适合在移动设备、嵌入式系统或桌面平台上构建无人机侦测应用。

> Remote ID 是无人机的"电子牌照"，受全球各地法规强制要求（美国 FAA、欧洲 EASA）。本库使任何人都能利用智能手机或树莓派等普通硬件构建无人机侦测解决方案。

### 特性

- **多协议支持**
  - ASTM F3411-22a（美国/国际）
  - ASD-STAN EN 4709-002（欧盟）
  - GB/T（中国）- 接口预留

- **多传输方式支持**
  - 蓝牙 4.x Legacy 广播
  - 蓝牙 5.x Extended 广播 / 长距离（Coded PHY）
  - WiFi Beacon（802.11 厂商自定义 IE）
  - WiFi NAN（邻居感知网络）

- **高级分析**
  - 异常检测（欺骗、重放攻击、不可能的速度）
  - 轨迹分析（平滑、预测、模式分类）
  - 会话管理（去重、超时处理）

- **跨平台绑定**
  - C++（核心库）
  - C API（用于 FFI 集成）
  - Android/Kotlin（通过 JNI）
  - Python（通过 ctypes）

- **性能**
  - 使用位域的零拷贝解析
  - 最小化内存分配
  - 适合移动设备上的实时处理

### 支持的消息类型

| 消息类型 | 描述 |
| --------- | ------ |
| Basic ID (0x0) | 无人机序列号、注册 ID |
| Location (0x1) | 经纬度、高度、速度、航向 |
| Authentication (0x2) | 加密认证数据 |
| Self-ID (0x3) | 操作员定义的描述文本 |
| System (0x4) | 操作员位置、作业区域 |
| Operator ID (0x5) | 操作员注册号 |
| Message Pack (0xF) | 单次广播中的多条消息 |

### 安装

#### C++ (CMake)

```cmake
include(FetchContent)
FetchContent_Declare(
    orip
    GIT_REPOSITORY https://github.com/iannil/open-remote-id-parser.git
    GIT_TAG v0.1.0
)
FetchContent_MakeAvailable(orip)

target_link_libraries(your_target PRIVATE orip)
```

#### Python

```bash
cd python
pip install .

## 或用于开发
pip install -e ".[dev]"
```

#### Android (Gradle)

```kotlin
// settings.gradle.kts
include(":orip")
project(":orip").projectDir = file("path/to/open-remote-id-parser/android/orip")

// app/build.gradle.kts
dependencies {
    implementation(project(":orip"))
}
```

### 快速开始

#### C++

```cpp
##include <orip/orip.h>

int main() {
    orip::RemoteIDParser parser;
    parser.init();

    // 设置回调
    parser.setOnNewUAV([](const orip::UAVObject& uav) {
        std::cout << "发现新无人机: " << uav.id << std::endl;
    });

    // 解析传入的 BLE 广播
    std::vector<uint8_t> ble_data = /* 来自扫描器 */;
    auto result = parser.parse(ble_data, rssi, orip::TransportType::BT_LEGACY);

    if (result.success) {
        std::cout << "无人机 ID: " << result.uav.id << std::endl;
        std::cout << "位置: " << result.uav.location.latitude
                  << ", " << result.uav.location.longitude << std::endl;
    }

    return 0;
}
```

#### Python

```python
from orip import RemoteIDParser, TransportType

with RemoteIDParser() as parser:
    parser.set_on_new_uav(lambda uav: print(f"发现新无人机: {uav.id}"))

    ## 解析 BLE 广播数据
    result = parser.parse(ble_data, rssi=-70, transport=TransportType.BT_LEGACY)

    if result.success:
        print(f"无人机: {result.uav.id}")
        print(f"位置: {result.uav.location.latitude}, {result.uav.location.longitude}")
```

#### Kotlin (Android)

```kotlin
import com.orip.RemoteIDParser
import com.orip.TransportType

class DroneScanner {
    private val parser = RemoteIDParser()

    init {
        parser.setOnNewUAV { uav ->
            Log.d("DroneScanner", "发现新无人机: ${uav.id}")
        }
    }

    // 在 BLE 扫描回调中
    fun onScanResult(result: ScanResult) {
        val scanRecord = result.scanRecord ?: return

        val parseResult = parser.parse(
            scanRecord.bytes,
            result.rssi,
            TransportType.BT_LEGACY
        )

        if (parseResult.success) {
            updateMap(parseResult.uav)
        }
    }

    fun cleanup() {
        parser.close()
    }
}
```

#### C API

```c
##include <orip/orip_c.h>

int main() {
    orip_parser_t* parser = orip_create();
    orip_result_t result;

    uint8_t payload[] = { /* BLE 数据 */ };

    orip_parse(parser, payload, sizeof(payload), -70,
               ORIP_TRANSPORT_BT_LEGACY, &result);

    if (result.success) {
        printf("无人机: %s\n", result.uav.id);
        printf("纬度: %f, 经度: %f\n",
               result.uav.location.latitude,
               result.uav.location.longitude);
    }

    orip_destroy(parser);
    return 0;
}
```

### 高级功能

#### 异常检测

检测欺骗尝试和不可能的飞行模式：

```cpp
##include <orip/anomaly_detector.h>

orip::analysis::AnomalyDetector detector;

// 分析每次无人机更新
auto anomalies = detector.analyze(uav, rssi);

for (const auto& anomaly : anomalies) {
    switch (anomaly.type) {
        case AnomalyType::REPLAY_ATTACK:
            std::cerr << "警告: 检测到可能的重放攻击！" << std::endl;
            break;
        case AnomalyType::SPEED_IMPOSSIBLE:
            std::cerr << "警告: 检测到不可能的速度！" << std::endl;
            break;
        // ...
    }
}
```

#### 轨迹分析

追踪飞行路径并预测未来位置：

```cpp
##include <orip/trajectory_analyzer.h>

orip::analysis::TrajectoryAnalyzer analyzer;

// 添加位置更新
analyzer.addPosition(uav.id, uav.location);

// 获取飞行模式
auto pattern = analyzer.classifyPattern(uav.id);
// 返回: LINEAR（直线）, CIRCULAR（环形）, PATROL（巡逻）, STATIONARY（悬停）等

// 预测 5 秒后的位置
auto prediction = analyzer.predictPosition(uav.id, 5000);
std::cout << "预测位置: " << prediction.latitude
          << ", " << prediction.longitude << std::endl;

// 获取轨迹统计
auto stats = analyzer.getStats(uav.id);
std::cout << "总距离: " << stats.total_distance_m << " 米" << std::endl;
std::cout << "最大速度: " << stats.max_speed_mps << " 米/秒" << std::endl;
```

### API 参考

#### 核心类

| 类 | 描述 |
| --- | ------ |
| `RemoteIDParser` | 主解析器类，处理所有协议 |
| `UAVObject` | 完整的无人机数据（ID、位置、操作员信息） |
| `ParseResult` | 解析操作的结果 |
| `LocationVector` | 位置、高度、速度、航向 |
| `SystemInfo` | 操作员位置、作业区域 |

#### 分析类

| 类 | 描述 |
| --- | ------ |
| `AnomalyDetector` | 检测欺骗和不可能的模式 |
| `TrajectoryAnalyzer` | 追踪飞行路径、预测位置 |

#### 协议解码器

| 类 | 描述 |
| --- | ------ |
| `ASTM_F3411_Decoder` | ASTM F3411-22a（美国/国际） |
| `ASD_STAN_Decoder` | ASD-STAN EN 4709-002（欧盟） |
| `WiFiDecoder` | WiFi Beacon 和 NAN 帧 |
| `CN_RID_Decoder` | GB/T 中国标准（预留） |

### 从源码构建

#### 环境要求

- CMake 3.16+
- C++17 兼容编译器（GCC 8+、Clang 7+、MSVC 2019+）
- （可选）Android NDK 用于 Android 构建
- （可选）Python 3.8+ 用于 Python 绑定

#### 构建步骤

```bash
## 克隆仓库
git clone https://github.com/iannil/open-remote-id-parser.git
cd open-remote-id-parser

## 创建构建目录
mkdir build && cd build

## 配置
cmake .. -DCMAKE_BUILD_TYPE=Release

## 构建
cmake --build . --parallel

## 运行测试
ctest --output-on-failure

## 安装（可选）
sudo cmake --install .
```

#### 构建选项

| 选项 | 默认值 | 描述 |
| ----- | ------- | ------ |
| `ORIP_BUILD_TESTS` | ON | 构建单元测试 |
| `ORIP_BUILD_EXAMPLES` | ON | 构建示例程序 |
| `ORIP_BUILD_SHARED` | OFF | 构建共享库（.so/.dll） |

#### Android 构建

```bash
cd android
./gradlew :orip:assembleRelease
```

AAR 将生成在 `android/orip/build/outputs/aar/`。

#### Python 构建

```bash
cd python
pip install build
python -m build
```

### 项目结构

```
open-remote-id-parser/
#├── include/orip/           # 公共 C++ 头文件
#│   ├── orip.h              # 主包含文件
#│   ├── parser.h            # RemoteIDParser 类
#│   ├── types.h             # 数据结构
#│   ├── astm_f3411.h        # ASTM 解码器
#│   ├── asd_stan.h          # ASD-STAN 解码器
#│   ├── wifi_decoder.h      # WiFi 解码器
#│   ├── anomaly_detector.h  # 异常检测
#│   ├── trajectory_analyzer.h # 轨迹分析
#│   └── orip_c.h            # C API
├── src/
#│   ├── core/               # 核心实现
#│   ├── protocols/          # 协议解码器
#│   ├── analysis/           # 分析模块
#│   └── utils/              # 工具类
#├── tests/                  # 单元测试
#├── examples/               # 示例程序
#├── android/                # Android 库
│   └── orip/
#│       ├── src/main/java/  # Kotlin 类
#│       └── src/main/cpp/   # JNI 封装
#├── python/                 # Python 绑定
#│   ├── orip/               # Python 包
#│   ├── tests/              # Python 测试
#│   └── examples/           # Python 示例
#└── docs/                   # 文档
```

### 硬件推荐

#### 入门级（移动侦测）

- 任何支持蓝牙 5.0+ 的 Android 手机
- 侦测距离：300-800米（取决于环境条件）

#### 专业级（固定站点）

- 树莓派 4 + ESP32-C3（BLE 嗅探器）
- 外置高增益天线
- 侦测距离：2-5公里

#### 企业级

- 软件定义无线电（SDR）方案
- 多接收器用于三角定位
- 与现有安防系统集成

### 贡献

欢迎贡献！请在提交 PR 前阅读我们的贡献指南。

1. Fork 本仓库
2. 创建功能分支（`git checkout -b feature/amazing-feature`）
3. 提交更改（`git commit -m 'Add amazing feature'`）
4. 推送到分支（`git push origin feature/amazing-feature`）
5. 开启 Pull Request

#### 开发环境设置

```bash
## 安装开发依赖
pip install -e "python/.[dev]"

## 运行 C++ 测试
cd build && ctest

## 运行 Python 测试
cd python && pytest

## 代码格式化（如果安装了 clang-format）
find src include -name "*.cpp" -o -name "*.h" | xargs clang-format -i
```

### 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

### 开发路线图

#### 已完成

- [x] **核心引擎**: 零拷贝设计的 C++ 解析库
- [x] **ASTM F3411**: 完整支持全部 7 种消息类型（Basic ID、Location、Authentication、Self-ID、System、Operator ID、Message Pack）
- [x] **ASD-STAN EN 4709-002**: 欧洲标准，含 EU Operator ID 验证
- [x] **GB/T 接口**: 中国国标预留（等待规范发布）
- [x] **多传输方式**: 蓝牙 4.x Legacy、蓝牙 5.x Extended/Long Range、WiFi Beacon、WiFi NAN
- [x] **C API**: 完整的 FFI 接口，支持回调
- [x] **Android 绑定**: Kotlin/JNI 封装，AAR 打包
- [x] **Python 绑定**: 基于 ctypes，支持上下文管理器
- [x] **会话管理器**: 去重、超时处理、事件回调
- [x] **异常检测**: 8 种检测类型（速度、位置、重放攻击、信号等）
- [x] **轨迹分析**: 平滑、预测、模式分类
- [x] **单元测试**: 70+ 测试用例覆盖所有模块
- [x] **文档**: README（中/英）、CONTRIBUTING、CHANGELOG

#### 进行中

- [ ] **发布产物**: 构建 `/release` 目录（静态库/共享库）
- [ ] **CI/CD 完善**: GitHub Actions 构建验证
- [ ] **性能基准测试**: 解析延迟、内存占用分析

#### 计划中

- [ ] **v0.1.0 发布**: 首个正式版本
- [ ] **Android 示例应用**: 完整的集成示例
- [ ] **真机测试**: 使用真实无人机抓包数据验证
- [ ] **API 文档**: 自动生成参考文档（Doxygen）
- [ ] **iOS 绑定**: 通过 C API 的 Swift 封装

### 致谢

- [ASTM F3411](https://www.astm.org/f3411-22a.html) - Remote ID 标准规范
- [ASD-STAN EN 4709-002](https://asd-stan.org/) - 欧洲 Remote ID 标准
- [OpenDroneID](https://github.com/opendroneid) - 参考实现

## Geofence-Updater-Lite (GUL)

[English](README.md) | [简体中文](README.zh-CN.md)

<div align="center">

![Go Version](https://img.shields.io/badge/Go-1.25+-00ADD8?style=flat&logo=go)
![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)
![Tests](https://img.shields.io/badge/tests-passing-brightgreen.svg)
![Coverage](https://img.shields.io/badge/coverage-90%25-brightgreen.svg)

**一个轻量级、高可靠的地理围栏数据同步系统**

专为无人机/无人驾驶飞行器在低带宽、不稳定网络环境下运行而设计

#[功能特性](#功能特性) • [快速开始](#快速开始) • [使用指南](#使用指南) • [API 文档](#api-文档) • [协议规范](#协议规范)

</div>

---

### 项目简介

Geofence-Updater-Lite (GUL) 是一个去中心化的地理围栏数据同步系统，核心设计理念是通过 **Merkkle Tree 实现增量更新**，将版本差异压缩至几 KB，使其能够在 GPRS 级别的网络环境中稳定运行。

**核心特性：**

- **极低带宽** - 使用 Merkle Tree + 二进制差分，增量更新仅需几 KB
- **去中心化分发** - 纯静态文件，可部署在任意 CDN/OSS，零服务器成本
- **安全优先** - Ed25519 数字签名，离线验证，防篡改
- **高性能查询** - 基于 R-Tree 空间索引，毫秒级围栏检查
- **跨平台** - 纯 Go 实现，支持 Linux/macOS/Windows

---

### 功能特性

| 特性 | 说明 |
| ------ | ------ |
| **极低带宽** | Merkle Tree 实现增量更新，版本差异可能只有几 KB |
| **去中心化分发** | 核心数据为静态文件，可部署在 CDN/OSS/IPFS |
| **数字签名** | Ed25519 签名 + KeyID 机制，防篡改验证 |
| **高性能查询** | 基于 R-Tree 空间索引，毫秒级围栏检查 |
| **离线验签** | 内置公钥验证，不依赖数据来源 |
| **版本回滚保护** | 拒绝应用旧版本数据 |
| **进度回调** | 大文件下载支持进度报告 |
| **易集成** | 纯 Go 实现，跨平台编译支持 |

---

### 架构设计

#### 核心原则

```
┌─────────────────────────────────────────────────────────────────┐
│                     Git 思想                                    │
│              Merkle Tree 管理版本，只下载差异                    │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│                     CDN 友好                                    │
│              纯静态文件，可部署在任意 CDN/OSS                    │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│                     安全优先                                    │
│              Ed25519 签名，离线验证，防篡改                      │
└─────────────────────────────────────────────────────────────────┘
```

#### 双组件架构

```
┌──────────────────────────────────────────────────────────────────────┐
│                                                                       │
│                        服务端 (Publisher)                            │
│                      CLI 工具 / Web 后台                             │
│                                                                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │ 围栏数据  │  │ Merkle   │  │  Delta   │  │ Snapshot │  │  签名   │ │
│  │  输入    │  │  Tree    │  │  Patch   │  │   文件   │  │  生成   │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬────┘ │
│       │           │           │           │              │        │   │
│       ▼           ▼           ▼           ▼              ▼        ▼   │
│   ┌─────────────────────────────────────────────────────────────────┐ │
│   │              静态文件存储 (CDN/OSS/IPFS)                         │ │
│   │  manifest.json │  v1.bin  │  v1_v2.delta  │  v2.snapshot.bin   │ │
│   └─────────────────────────────────────────────────────────────────┘ │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTP 轮询
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        客户端 (Drone SDK)                            │
│                      运行在无人机 / 遥控器 APP                        │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  HTTP 客户端   │  同步逻辑  │  SQLite  │  R-Tree  │  签名验证  │  │
│  │  (下载/重试)   │  (轮询)   │  (持久化) │ (查询)  │  (离线)    │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                       │
│                         API: Check(lat, lon) → Allowed?              │
└──────────────────────────────────────────────────────────────────────┘
```

---

### 快速开始

#### 前置要求

- **Go 1.25+** （推荐使用最新稳定版本）
- **Make** （可选，用于便捷构建）
- **Docker** （可选，用于容器化部署）

#### 安装

##### 方式一：从源码构建

```bash
## 克隆仓库
git clone https://github.com/iannil/geofence-updater-lite.git
cd geofence-updater-lite

## 下载依赖
go mod download

## 构建所有二进制文件
make build-all

## 或仅构建发布工具
make build
```

构建产物位于 `bin/` 目录：

- `publisher` - 发布工具（服务端）
- `sdk-example` - SDK 使用示例（客户端）

##### 方式二：Docker 构建

```bash
## 构建镜像
docker build -t gul-publisher .

## 运行容器
docker run -it --rm -v $(pwd)/data:/data gul-publisher
```

##### 方式三：交叉编译

```bash
## 为多个平台构建
make cross-compile
```

#### 基本使用流程

**1. 生成密钥对**

```bash
$ ./bin/publisher keys

生成的密钥对：
  私钥: 0x7c3a9f2e... (请妥善保管)
  公钥: 0x8d4b1c5a... (用于客户端验证)
  KeyID: k1_20240118
```

**2. 初始化数据库**

```bash
$ ./bin/publisher init

初始化完成：
  数据库: ./data/fences.db
  版本: v1
```

**3. 添加围栏**

创建围栏数据文件 `fence.json`：

```json
{
  "id": "fence-20240118-001",
  "type": "TEMP_RESTRICTION",
  "geometry": {
    "polygon": [
      {"lat": 39.9042, "lon": 116.4074},
      {"lat": 39.9142, "lon": 116.4074},
      {"lat": 39.9142, "lon": 116.4174},
      {"lat": 39.9042, "lon": 116.4174}
    ]
  },
  "start_ts": 1709880000,
  "end_ts": 1709990000,
  "priority": 10,
  "name": "北京三环临时管控区",
  "description": "临时活动禁飞区"
}
```

添加到数据库：

```bash
$ ./bin/publisher add fence.json

添加成功：fence-20240118-001 (类型: TEMP_RESTRICTION)
```

**4. 发布更新**

```bash
$ ./bin/publisher publish --output ./output

发布完成：
  版本: v2
  增量包: ./output/v1_v2.delta (2.3 KB)
  快照: ./output/v2.snapshot.bin (15.6 KB)
  清单: ./output/manifest.json
```

**5. 部署到 CDN**

将 `output/` 目录上传到你的 CDN/OSS：

```bash
## 示例：使用 AWS CLI
aws s3 sync ./output s3://your-bucket/geofence/
```

**6. 客户端使用**

```bash
$ ./bin/sdk-example \
  -manifest https://cdn.example.com/geofence/manifest.json \
  -public-key 0x8d4b1c5a... \
  -store ./geofence.db

启动同步...
  当前版本: v0
  远程版本: v2
  下载增量包: 2.3 KB
  应用更新完成: v0 → v2
  验签通过

开始围栏检查...
  检查 (39.9042, 116.4074): 禁止飞行 - 北京三环临时管控区
```

---

### 使用指南

#### 发布工具 (Publisher Tool)

发布工具用于管理和发布地理围栏更新。

##### 命令说明

```bash
## 生成 Ed25519 密钥对
$ publisher keys

## 初始化围栏数据库
$ publisher init [--db-path ./data/fences.db]

## 添加新围栏
$ publisher add <fence.json>

## 批量添加围栏
$ publisher add --batch <fences-dir>

## 列出所有围栏
$ publisher list [--type TEMP_RESTRICTION]

## 删除围栏
$ publisher remove <fence-id>

## 发布新版本
$ publisher publish [--output ./output] [--message "更新说明"]

## 查看版本历史
$ publisher history
```

##### 支持的围栏类型

| 类型 | 说明 | 优先级建议 |
| ------ | ------ | ----------- |
| `TEMP_RESTRICTION` | 临时管控区 | 10-50 |
| `PERMANENT_NO_FLY` | 永久禁飞区 | 100 |
| `ALTITUDE_LIMIT` | 高度限制区 | 20-40 |
| `ALTITUDE_MINIMUM` | 最低高度要求 | 20-40 |
| `SPEED_LIMIT` | 速度限制区 | 10-30 |

##### 几何形状支持

```json
// 多边形（Polygon）
{
  "geometry": {
    "polygon": [
      {"lat": 39.9042, "lon": 116.4074},
      {"lat": 39.9142, "lon": 116.4074},
      {"lat": 39.9142, "lon": 116.4174},
      {"lat": 39.9042, "lon": 116.4174}
    ]
  }
}

// 圆形（Circle）
{
  "geometry": {
    "circle": {
      "center": {"lat": 39.9042, "lon": 116.4074},
      "radius_m": 5000
    }
  }
}

// 矩形（Rectangle）
{
  "geometry": {
    "rectangle": {
      "min": {"lat": 39.9000, "lon": 116.4000},
      "max": {"lat": 39.9200, "lon": 116.4200}
    }
  }
}
```

---

#### 客户端 SDK (Drone SDK)

SDK 提供地理围栏查询和自动同步功能。

##### Go SDK 集成

```go
package main

import (
    "context"
    "log"
    "time"

    "github.com/iannil/geofence-updater-lite/pkg/config"
    "github.com/iannil/geofence-updater-lite/pkg/sync"
)

func main() {
    ctx := context.Background()

    // 创建配置
    cfg := &config.ClientConfig{
        ManifestURL:    "https://cdn.example.com/geofence/manifest.json",
        PublicKeyHex:   "8d4b1c5a...", // 公钥十六进制
        StorePath:      "./geofence.db",
        SyncInterval:   1 * time.Minute,
        HTTPTimeout:    30 * time.Second,
    }

    // 创建同步器
    syncer, err := sync.NewSyncer(ctx, cfg)
    if err != nil {
        log.Fatal(err)
    }
    defer syncer.Close()

    // 启动自动同步
    results := syncer.StartAutoSync(ctx, 1*time.Minute)

    // 处理同步结果
    go func() {
        for result := range results {
            if result.Error != nil {
                log.Printf("同步错误: %v", result.Error)
                continue
            }
            if result.UpToDate {
                log.Printf("已是最新版本 (v%d)", result.CurrentVer)
            } else {
                log.Printf("更新完成: v%d → v%d，耗时 %v",
                    result.PreviousVer, result.CurrentVer, result.Duration)
            }
        }
    }()

    // 围栏检查
    allowed, restriction, err := syncer.Check(ctx, 39.9042, 116.4074)
    if err != nil {
        log.Fatal(err)
    }

    if !allowed {
        log.Printf("禁止飞行: %s - %s", restriction.Name, restriction.Description)
        // 执行禁飞逻辑...
    }
}
```

##### SDK API 参考

| 方法 | 说明 | 返回值 |
| ------ | ------ | -------- |
| `NewSyncer(ctx, cfg)` | 创建同步器 | `(*Syncer, error)` |
| `StartAutoSync(ctx, interval)` | 启动自动同步 | `<-chan SyncResult` |
| `CheckForUpdates(ctx)` | 检查更新 | `(*Manifest, error)` |
| `Sync(ctx)` | 执行同步 | `(*SyncResult, error)` |
| `Check(ctx, lat, lon)` | 围栏检查 | `(allowed, restriction, error)` |
| `Close()` | 关闭同步器 | `error` |

---

### API 文档

#### pkg/crypto - 密码学模块

```go
// 生成 Ed25519 密钥对
keyPair, err := crypto.GenerateKeyPair()

// 对数据签名
signature := crypto.Sign(privateKey, data)

// 验证签名
valid := crypto.Verify(publicKey, data, signature)

// 计算密钥 ID（用于密钥轮换）
keyID := crypto.PublicKeyToKeyID(publicKey)
```

#### pkg/merkle - Merkle Tree 模块

```go
// 从围栏项构建 Merkle Tree
tree, err := merkle.NewTree(fences)

// 获取根哈希
rootHash := tree.RootHash()

// 生成 Merkle 证明
proof, err := tree.GetProof(fenceID)

// 验证 Merkle 证明
valid := merkle.VerifyProof(fenceID, fenceData, proof, rootHash)
```

#### pkg/storage - 存储模块

```go
// 打开数据库
store, err := storage.Open(ctx, &storage.Config{Path: "./geofence.db"})

// 添加围栏
store.AddFence(ctx, &fence)

// 点查询（使用 R-Tree）
fences, err := store.QueryAtPoint(ctx, lat, lon)

// 版本管理
version, _ := store.GetVersion(ctx)
store.SetVersion(ctx, newVersion)
```

#### pkg/sync - 同步模块

```go
// 创建同步器
syncer, _ := sync.NewSyncer(ctx, cfg)

// 检查更新
manifest, _ := syncer.CheckForUpdates(ctx)

// 同步数据
result, _ := syncer.Sync(ctx)

// 自动同步
results := syncer.StartAutoSync(ctx, interval)
```

#### pkg/binarydiff - 二进制差分模块

```go
// 计算差异
delta, err := binarydiff.Diff(oldFences, newFences)

// 应用差异
newFences, err := binarydiff.PatchFences(oldFences, delta)
```

---

### 性能指标

| 操作 | 性能 | 说明 |
| ------ | ------ | ------ |
| 围栏检查 | < 1ms | 1000 次查询，R-Tree 索引 |
| Merkle Tree 构建 | < 100ms | 1000 个围栏 |
| Delta 计算 | < 50ms | 1000 个围栏对比 |
| 增量包大小 | ~2-5 KB | 典型 100 个围栏的变更 |
| 全量快照 | ~15 KB | 100 个围栏（Protobuf 编码） |

---

### 协议规范

#### 围栏项 (Fence Item)

| 字段 | 类型 | 说明 |
| ------ | ------ | ------ |
| `id` | string | 唯一标识符 |
| `type` | FenceType | 围栏类型 |
| `geometry` | Geometry | 几何形状（多边形/圆形/矩形） |
| `start_ts` | int64 | 生效时间戳 |
| `end_ts` | int64 | 失效时间戳，0 表示永不过期 |
| `priority` | uint32 | 优先级，高优先级覆盖低优先级 |
| `max_alt_m` | uint32 | 最大高度限制（米），0 表示无限制 |
| `max_speed_mps` | uint32 | 最大速度限制（米/秒），0 表示无限制 |
| `name` | string | 围栏名称 |
| `description` | string | 围栏描述 |
| `signature` | []byte | Ed25519 签名 |
| `key_id` | string | 密钥 ID |

#### 清单文件 (Manifest)

| 字段 | 类型 | 说明 |
| ------ | ------ | ------ |
| `version` | uint64 | 全局版本号（递增） |
| `timestamp` | int64 | 发布时间戳 |
| `root_hash` | []byte | Merkle Tree 根哈希 |
| `delta_url` | string | 增量包下载地址 |
| `snapshot_url` | string | 全量快照下载地址 |
| `delta_size` | uint64 | 增量包大小（字节） |
| `snapshot_size` | uint64 | 快照大小（字节） |
| `delta_hash` | []byte | 增量包哈希（SHA-256） |
| `snapshot_hash` | []byte | 快照哈希（SHA-256） |
| `message` | string | 版本消息 |

---

### 项目结构

```
geofence-updater-lite/
#├── cmd/                          # 命令行工具
#│   ├── publisher/                # 发布工具（服务端）
#│   └── sdk-example/               # SDK 使用示例（客户端）
#├── pkg/                          # 核心包
#│   ├── binarydiff/               # 二进制差分算法
#│   ├── client/                   # HTTP 客户端
#│   ├── config/                   # 配置管理
#│   ├── converter/                # 数据格式转换
#│   ├── crypto/                   # Ed25519 密码学
#│   ├── geofence/                 # 地理围栏核心逻辑
#│   ├── merkle/                   # Merkle Tree 实现
#│   ├── protocol/protobuf/        # Protocol Buffers 定义
#│   ├── publisher/                # 发布逻辑
#│   ├── storage/                  # SQLite 存储层
#│   ├── sync/                     # 同步逻辑
#│   └── version/                  # 版本管理
#├── internal/                     # 内部包
#│   ├── testutil/                 # 测试工具
#│   └── version/                  # 内部版本信息
#├── docs/                         # 文档
#│   ├── spec/                     # 技术规范
#│   ├── progress/                 # 进度文档
#│   └── planning/                 # 计划文档
#├── scripts/                      # 构建脚本
#├── test/                         # 测试数据
#├── bin/                          # 构建输出
#├── Makefile                      # 构建系统
#├── go.mod                        # Go 模块定义
#├── go.sum                        # 依赖锁定
#├── Dockerfile                    # Docker 定义
#├── LICENSE                       # Apache 2.0 许可证
#├── README.md                     # 英文版（默认）
#├── README.zh-CN.md               # 中文版
#├── CONTRIBUTING.md               # 贡献指南
#├── CHANGELOG.md                  # 变更日志
#├── CLAUDE.md                     # Claude Code 项目指导
#└── SECURITY.md                   # 安全政策
```

---

### 开发指南

#### 运行测试

```bash
## 运行所有测试
make test

## 运行带覆盖率的测试
make test-coverage

## 运行基准测试
make test-bench
```

#### 代码规范

```bash
## 代码格式化
make fmt

## 静态检查
make vet

## 代码检查（需要 golangci-lint）
make lint
```

#### 构建

```bash
## 构建所有二进制文件
make build-all

## 交叉编译
make cross-compile

## 构建 Docker 镜像
make docker-build
```

---

### 常见问题 (FAQ)

**Q: 为什么选择 Ed25519 而不是 RSA/ECDSA？**

A: Ed25519 提供更高的安全性和性能：

- 签名大小仅 64 字节（RSA-2048 需要 256 字节）
- 验证速度比 ECDSA 快约 5 倍
- 内置抗侧信道攻击保护

**Q: 如何处理密钥轮换？**

A: 使用 KeyID 机制，每个签名包含密钥 ID，客户端可以支持多个公钥：

```go
syncer.PublicKeys = map[string]*crypto.PublicKey{
    "k1_2024": oldPublicKey,
    "k2_2024": newPublicKey,
}
```

**Q: 支持多少个围栏？**

A: 理论上无上限。实测：

- 10,000 个围栏：全量快照约 1.5 MB，查询 < 2ms
- 100,000 个围栏：全量快照约 15 MB，查询 < 5ms

**Q: 能否在没有网络的情况下使用？**

A: 可以。SDK 会使用本地缓存的围栏数据继续工作，网络恢复后自动同步。

---

### 路线图

- [x] 核心数据结构
- [x] Ed25519 签名验证
- [x] Merkle Tree 实现
- [x] R-Tree 空间索引
- [x] 二进制差分
- [x] HTTP 同步
- [x] 发布工具
- [x] SDK 示例
- [x] CI/CD 流水线 (GitHub Actions)
- [ ] C++ SDK
- [ ] 性能基准测试
- [ ] Web 管理界面

---

### 许可证

本项目采用 [Apache License 2.0](LICENSE) 许可证。

```
Copyright 2024-2025 Geofence-Updater-Lite Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

### 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何参与贡献。

---

### 致谢

本项目借鉴了以下开源项目的设计思路：

- [Git](https://git-scm.com/) - Merkle Tree 版本管理思想
- [bsdiff](https://www.daemonology.net/bsdiff/) - 二进制差分算法
- [go-polyline](https://github.com/twpayne/go-polyline) - 坐标压缩算法

## Hetero-Compute-Router (HCS)

[![CI](https://github.com/iannil/hetero-compute-router/workflows/CI/badge.svg)](https://github.com/iannil/hetero-compute-router/actions)
[![Helm](https://github.com/iannil/hetero-compute-router/workflows/Helm%20Lint/badge.svg)](https://github.com/iannil/hetero-compute-router/actions)
[![Go Report Card](https://goreportcard.com/badge/github.com/iannil/hetero-compute-router)](https://goreportcard.com/report/github.com/iannil/hetero-compute-router)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Kubernetes](https://img.shields.io/badge/kubernetes-%3E%3D%201.24-blue)](https://kubernetes.io/)

HCS (Hetero-Compute-Router) 是一个基于 Kubernetes 的异构算力虚拟化与适配层。它抽象屏蔽不同厂商（NVIDIA、华为昇腾、海光、寒武纪）的硬件差异，实现 AI 工作负载的 "一次编写，随处运行"。

[English](README.md) | [中文](README_CN.md)

---

### 目录

#- [问题背景](#问题背景)
#- [解决方案：软硬解耦](#解决方案软硬解耦)
#- [核心特性](#核心特性)
#- [系统架构](#系统架构)
#- [支持的硬件](#支持的硬件)
#- [快速开始](#快速开始)
#- [使用示例](#使用示例)
#- [配置说明](#配置说明)
#- [开发路线图](#开发路线图)
#- [参与贡献](#参与贡献)
#- [开源许可](#开源许可)

---

### 问题背景

现代 AI 基础设施面临重大挑战：

| 挑战 | 描述 |
| ------ | ------ |
| 厂商锁定 | 应用与特定硬件紧密耦合（如 `nvidia.com/gpu`） |
| 硬件碎片化 | GPU/NPU 生态多样（NVIDIA、华为、海光、寒武纪），API 互不兼容 |
| 资源粒度粗 | 即使只需要部分资源，也必须分配整张 GPU |
| 拓扑无感知 | 标准调度器忽略互联拓扑（NVLink、HCCS、PCIe） |
| 容错能力弱 | 缺乏国产芯片的亚健康检测和自动故障转移 |

传统方式：

```
用户申请 nvidia.com/gpu → 只能调度到 NVIDIA 节点
```

HCS 方式：

```
用户申请 ai.compute/vram: 16Gi → HCS 分析集群现状 →
动态分配 NVIDIA A100 或 华为昇腾 910B →
自动注入对应的驱动库和环境变量
```

---

### 解决方案：软硬解耦

HCS 实现 三层解耦模型，将用户工作负载与硬件细节分离：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              用户请求层                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Pod 请求: ai.compute/vram: 16Gi, ai.compute/tflops-fp16: "100"      │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         调度与注入层 (HCS)                                    │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐          │
│  │   调度器扩展      │───▶│   Webhook       │───▶│  修改后的 Pod    │          │
│  │   Extension     │    │   注入器         │    │  (含驱动配置)     │          │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘          │
└─────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         资源抽象层 (URA)                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      ComputeNode CRD                                │    │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐                │    │
│  │  │ NVIDIA  │  │  华为    │  │   海光  │  │  寒武纪  │                │    │
│  │  │ A100x8  │  │ 910Bx8  │  │ DCUx8   │  │ MLUx8   │                │    │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### 核心特性

#### 1. 统一资源抽象 (URA)

Node-Agent 部署在每个节点上，使用原生 API（NVML、DSMI、CNMon）采集硬件信息，上报 "算力指纹"：

- 显存 (VRAM)：总量和可用量
- 计算能力：FP16/FP32/INT8 算力 (TFLOPS)
- 互联拓扑：NVLink/HCCS/RoCE/PCIe 带宽和延迟
- 健康评分：实时硬件健康监控（对国产芯片至关重要）

#### 2. 拓扑感知调度

HCS 调度器扩展 Kubernetes 调度框架，实现智能调度插件：

| 插件 | 功能 |
| ------ | ------ |
| Filter | 过滤不满足算力需求的节点 |
| Score | 优先选择高带宽互联和最优碎片化程度的节点 |
| Reserve | 在绑定前预留显存配额 |
| Bind | 附加硬件绑定注解 |

打分因素：

- 互联亲和性（优先选择 NVLink/HCCS 而非 PCIe）
- 装箱优化（最小化碎片）
- 健康加权（降低亚健康节点优先级）
- 算力汇率（跨厂商等效换算）

#### 3. 运行时注入（变更 Webhook）

运行时注入器 根据目标硬件自动配置容器环境：

```yaml
## HCS Webhook 处理前
spec:
  containers:
  - name: pytorch
    image: pytorch/pytorch:latest
    resources:
      requests:
        ai.compute/vram: "16Gi"

## HCS Webhook 处理后（自动注入华为昇腾配置）
spec:
  containers:
  - name: pytorch
    image: pytorch/pytorch:latest
    env:
    - name: ASCEND_VISIBLE_DEVICES
      value: "0,1"
    - name: LD_LIBRARY_PATH
      value: "/usr/local/Ascend/driver/lib64"
    volumeMounts:
    - name: ascend-driver
      mountPath: /usr/local/Ascend
  volumes:
  - name: ascend-driver
    hostPath:
      path: /usr/local/Ascend
```

#### 4. 软件定义显存切分（规划中）

libhcs_interceptor.so 实现无需硬件虚拟化的配额强制执行：

```
┌─────────────────────────────────────────┐
│         应用程序 (PyTorch)               │
└─────────────────┬───────────────────────┘
                  │ cudaMalloc()
                  ▼
┌─────────────────────────────────────────┐
│   libhcs_interceptor.so (LD_PRELOAD)    │
│  - 拦截 cudaMalloc/aclrtMalloc           │
│  - 强制执行显存配额 (HCS_VRAM_QUOTA)       │
│  - 超出配额返回 OOM 错误                   │
└─────────────────┬───────────────────────┘
                  ▼
┌─────────────────────────────────────────┐
│      厂商驱动 (CUDA/Ascend)              │
└─────────────────────────────────────────┘
```

#### 5. 算力汇率换算

支持基于性能等效的跨厂商调度：

```yaml
scheduler:
  exchangeRates:
    #nvidia-a100: 1.0      # 基准
    nvidia-a800: 0.95
    nvidia-h100: 1.5
    huawei-910b: 0.85
    hygon-dcu: 0.6
```

---

### 系统架构

#### 组件概览

| 组件 | 类型 | 描述 |
| ------ | ------ | ------ |
| Node-Agent | DaemonSet | 采集硬件信息，上报 ComputeNode CRD |
| Scheduler | Deployment | 扩展 K8s 调度器，实现算力感知调度 |
| Webhook | Deployment | 变更 Pod，注入运行时环境 |
| Interceptor | 库文件 | （规划中）通过 LD_PRELOAD 实现软件显存切分 |

#### ComputeNode CRD

```yaml
apiVersion: hetero.zrs.io/v1alpha1
kind: ComputeNode
metadata:
  name: gpu-node-01
spec:
  vendor: nvidia
  devices:
    - id: "0"
      model: "A100-80G"
      vram: "80Gi"
      compute:
        fp16: "312"
        fp32: "19.5"
      topology:
        busId: "0000:17:00.0"
        links:
          - type: nvlink
            peers: ["1", "2", "3"]
            bandwidth: "600GB/s"
      healthScore: 100
status:
  phase: Ready
  vramAllocatable: "80Gi"
  vramAllocated: "16Gi"
```

---

### 支持的硬件

| 厂商 | 产品 | 检测 | 调度 | 注入 | 显存切分 |
| ------ | ------ | ------ | ------ | ------ | ---------- |
| NVIDIA | A100/A800/H100/V100 | ✅ | ✅ | ✅ | 🔄 规划中 |
| 华为 | 昇腾 910A/910B | ✅ | ✅ | ✅ | 🔄 规划中 |
| 海光 | DCU Z100 | 🔄 规划中 | 🔄 规划中 | 🔄 规划中 | 🔄 规划中 |
| 寒武纪 | MLU370 | 🔄 规划中 | 🔄 规划中 | 🔄 规划中 | 🔄 规划中 |

---

### 快速开始

#### 前置条件

- Kubernetes 1.24+
- Helm 3.10+
- cert-manager 1.12+（推荐用于 TLS）
- 至少一个 GPU/NPU 节点

#### 安装

方式一：OCI Registry（推荐）

```bash
helm install hcs oci://ghcr.io/iannil/hetero-compute-router/charts/hcs \
  --namespace hcs-system \
  --create-namespace
```

方式二：从源码安装

```bash
## 克隆仓库
git clone https://github.com/iannil/hetero-compute-router.git
cd hetero-compute-router

## 安装 CRD
kubectl apply -f config/crd/

## 使用 Helm 安装
helm install hcs ./chart/hcs \
  --namespace hcs-system \
  --create-namespace
```

#### 验证安装

```bash
## 检查 Pod 状态
kubectl get pods -n hcs-system

## 预期输出：
## NAME                             READY   STATUS    RESTARTS   AGE
## hcs-node-agent-xxxxx             1/1     Running   0          1m
## hcs-scheduler-xxxxxxxxxx-xxxxx   1/1     Running   0          1m
## hcs-webhook-xxxxxxxxxx-xxxxx     1/1     Running   0          1m

## 检查 ComputeNode 资源
kubectl get computenodes

## 预期输出（有 GPU 节点时）：
## NAME          VENDOR   NODE        PHASE   VRAM          AGE
## gpu-node-01   nvidia   gpu-node-01 Ready   85899345920   1m
```

---

### 使用示例

#### 基础用法

请求抽象算力资源，而非厂商特定资源：

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: ai-workload
spec:
  schedulerName: hcs-scheduler
  containers:
  - name: pytorch
    image: pytorch/pytorch:latest
    resources:
      requests:
        ai.compute/vram: "16Gi"
        ai.compute/tflops-fp16: "100"
```

#### PyTorch 训练示例

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: pytorch-training
  labels:
    app: pytorch
spec:
  schedulerName: hcs-scheduler
  containers:
  - name: pytorch
    image: pytorch/pytorch:2.1.0-cuda12.1-cudnn8-runtime
    command: ["python", "train.py"]
    resources:
      requests:
        ai.compute/vram: "32Gi"
      limits:
        ai.compute/vram: "64Gi"
```

#### 多 GPU 作业（拓扑偏好）

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: distributed-training
  annotations:
    hcs.io/topology-preference: "high-bandwidth"
spec:
  schedulerName: hcs-scheduler
  containers:
  - name: pytorch
    image: pytorch/pytorch:latest
    resources:
      requests:
        #ai.compute/vram: "160Gi"  # 2x 80GB GPU
        ai.compute/tflops-fp16: "600"
```

---

### 配置说明

#### Helm Values 概览

| 参数 | 描述 | 默认值 |
| ------ | ------ | -------- |
| `nodeAgent.enabled` | 启用 Node-Agent DaemonSet | `true` |
| `nodeAgent.logLevel` | 日志级别 (debug/info/warn/error) | `info` |
| `nodeAgent.reportInterval` | 心跳间隔（秒） | `30` |
| `scheduler.enabled` | 启用 HCS 调度器 | `true` |
| `scheduler.replicas` | 调度器副本数 | `1` |
| `scheduler.leaderElection.enabled` | 启用 Leader 选举 | `true` |
| `webhook.enabled` | 启用变更 Webhook | `true` |
| `webhook.replicas` | Webhook 副本数 | `2` |
| `webhook.failurePolicy` | Webhook 失败策略 | `Fail` |

#### 环境特定配置

开发环境：

```bash
helm install hcs ./chart/hcs \
  -f ./chart/hcs/values-dev.yaml \
  --namespace hcs-dev \
  --create-namespace
```

生产环境：

```bash
helm install hcs ./chart/hcs \
  -f ./chart/hcs/values-prod.yaml \
  --namespace hcs-system \
  --create-namespace
```

完整配置参考请查看 [配置参数说明](docs/deployment/configuration.md)。

---

### 开发路线图

#### Phase 1: The Observer（全知之眼，MVP）✅

- [x] 支持 NVIDIA/昇腾检测的 Node-Agent
- [x] ComputeNode CRD 定义
- [x] 基础调度逻辑
- [x] Helm Chart 部署

#### Phase 2: The Router（算力路由，当前阶段）

- [x] 算力汇率换算
- [x] 变更准入 Webhook
- [x] 驱动和环境变量注入
- [ ] 跨厂商兼容性测试

#### Phase 3: The Virtualizer（算力虚拟化，规划中）

- [ ] `libhcs_interceptor.so` 显存切分
- [ ] 动态镜像重定
- [ ] 基于 eBPF 的亚健康检测
- [ ] 自动 Checkpoint 恢复

#### 版本计划

| 版本 | 目标时间 | 主要功能 |
| ------ | ---------- | ---------- |
| v0.1.0-alpha | 2026 Q2 | Phase 1 MVP |
| v0.2.0-beta | 2026 Q3 | Phase 2 完成 |
| v0.3.0-beta | 2026 Q4 | Phase 3 完成 |
| v1.0.0 | 2027 Q1 | 生产就绪版本 |

---

### 与现有方案对比

| 功能特性 | K8s 原生 | Volcano/YuniKorn | HCS |
| ---------- | ---------- | ------------------ | --------- |
| 资源粒度 | 整卡 | 静态 MPS | 动态软切分 |
| 异构支持 | 基于 Label | Device Plugin | 统一抽象 |
| 运行时环境 | 手动配置 | 手动配置 | 自动注入 |
| 拓扑感知 | 无 | 仅 NUMA | 跨芯片互联 |
| 容错处理 | Pod 重启 | 任务重试 | 亚健康隔离 + Checkpoint |
| 定位 | 资源调度 | 批处理调度 | 算力虚拟化层 |

---

### 从源码构建

```bash
## 克隆仓库
git clone https://github.com/iannil/hetero-compute-router.git
cd hetero-compute-router

## 构建所有二进制文件
make build

## 运行测试
make test

## 构建 Docker 镜像
make docker-build

## 推送镜像（需要认证）
make docker-push
```

---

### 参与贡献

我们欢迎贡献！请查看 [贡献指南](CONTRIBUTING.md) 了解详情。

#### 开发设置

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/my-feature`
3. 修改代码并添加测试
4. 运行代码检查：`make lint`
5. 提交 Pull Request

#### 代码结构

```
hetero-compute-router/
├── cmd/
#│   ├── node-agent/      # Node-Agent 入口
#│   ├── scheduler/       # 调度器扩展入口
#│   └── webhook/         # Webhook 服务入口
├── pkg/
#│   ├── api/v1alpha1/    # CRD 类型和 deepcopy
#│   ├── agent/           # Node-Agent 逻辑
#│   ├── collectors/      # 硬件采集器
#│   ├── detectors/       # 硬件检测器 (NVML, DSMI)
#│   ├── exchange/        # 算力汇率
#│   ├── interceptor/     # API 劫持库（规划中）
#│   ├── scheduler/       # 调度器插件
#│   └── webhook/         # 准入 Webhook
#├── chart/hcs/           # Helm Chart
#├── config/              # Kubernetes 清单
#├── docs/                # 文档
#└── hack/                # 构建脚本
```

---

### 社区

- GitHub Issues：[报告 Bug 或提出功能建议](https://github.com/iannil/hetero-compute-router/issues)
- Discussions：[提问和分享想法](https://github.com/iannil/hetero-compute-router/discussions)

---

### 开源许可

本项目采用 Apache License 2.0 许可证 - 详见 [LICENSE](LICENSE) 文件。

---

### 致谢

HCS 的诞生源于生产环境中管理异构 AI 基础设施的挑战。特别感谢：

- Kubernetes 调度框架提供的可扩展调度能力
- NVIDIA NVML 和华为 DSMI 提供的硬件检测 API
- 云原生社区的持续创新

---

HCS：消除厂商锁定，释放算力自由。

## Open-UAV-Telemetry-Bridge

[![Go Version](https://img.shields.io/badge/Go-1.21+-00ADD8?style=flat&logo=go)](https://go.dev/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

**协议无关的无人机遥测边缘网关** - 连接无人机协议与云平台的桥梁

[English](README.md)

---

### 项目简介

Open-UAV-Telemetry-Bridge（简称 OUTB）是一个轻量级、高性能的遥测网关，专为低空物联网（IoD, Internet of Drones）设计。它在多种无人机协议（MAVLink、DJI、GB/T 28181）之间进行转换，并通过 MQTT、WebSocket、HTTP 或 gRPC 输出标准化数据。

#### 为什么需要 OUTB？

- **协议碎片化**：PX4 使用 MAVLink，大疆使用私有协议，政府平台要求 GB/T 28181
- **坐标系混乱**：GPS 输出 WGS84，但国内地图需要 GCJ02/BD09 偏移
- **带宽限制**：原始遥测数据 50-100Hz 对 4G 网络来说太多了
- **集成复杂**：每个平台都需要定制适配器

OUTB 通过统一的插件化架构解决所有这些问题。

---

### 功能特性

#### 核心功能

- **多协议支持**：MAVLink（UDP/TCP/串口）、DJI（通过 Android 转发端）、GB/T 28181
- **统一数据模型**：无论源协议如何，都输出标准化 JSON
- **坐标转换**：自动 WGS84 → GCJ02/BD09 转换，适配国内地图
- **频率控制**：可配置降采样（如 50Hz → 1Hz），节省带宽
- **状态缓存**：内存状态存储 + 历史轨迹存储

#### 输出接口

- **MQTT 发布器**：标准 MQTT 3.1.1，支持遗嘱消息（LWT）
- **HTTP REST API**：查询无人机状态、健康检查、网关状态
- **WebSocket**：实时状态推送
- **轨迹存储**：环形缓冲区历史轨迹（可配置保留数量）

#### 运维特性

- **边缘就绪**：可运行在树莓派 4、Jetson Nano 或云服务器
- **零依赖**：单一二进制文件，无需外部运行时
- **热配置**：基于 YAML 的配置文件

---

### 快速开始

#### 环境要求

- Go 1.21 或更高版本
- （可选）MQTT Broker（如 Mosquitto）

#### 安装

```bash
## 克隆仓库
git clone https://github.com/iannil/open-uav-telemetry-bridge.git
cd open-uav-telemetry-bridge

## 构建
make build

## 或交叉编译到树莓派/Jetson
make build-linux-arm64
```

#### 配置

```bash
## 复制示例配置
cp configs/config.example.yaml configs/config.yaml

## 按需编辑
vim configs/config.yaml
```

#### 运行

```bash
## 使用配置文件运行
./bin/outb configs/config.yaml
```

#### 验证

```bash
## 健康检查
curl http://localhost:8080/health

## 获取网关状态
curl http://localhost:8080/api/v1/status

## 列出已连接的无人机
curl http://localhost:8080/api/v1/drones
```

---

### 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                      北向接口层                              │
│  ┌──────────┐  ┌───────────┐  ┌──────────┐  ┌────────────┐  │
│  │   MQTT   │  │ WebSocket │  │   HTTP   │  │    gRPC    │  │
│  └────┬─────┘  └─────┬─────┘  └────┬─────┘  └─────┬──────┘  │
└───────┼──────────────┼─────────────┼──────────────┼─────────┘
        │              │             │              │
        └──────────────┴──────┬──────┴──────────────┘
                              │
┌─────────────────────────────┼───────────────────────────────┐
│                        核心引擎                              │
│  ┌─────────────┐  ┌─────────┴────────┐  ┌────────────────┐  │
│  │  频率控制器  │  │     状态存储      │  │   轨迹存储     │  │
│  └─────────────┘  └──────────────────┘  └────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐│
│  │            坐标转换器 (WGS84→GCJ02/BD09)                ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────┼───────────────────────────────┐
│                      南向适配层                              │
│  ┌──────────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │   MAVLink    │  │   DJI 转发端    │  │   GB/T 28181   │  │
│  │(UDP/TCP/串口)│  │  (TCP 服务器)   │  │  (SIP/NOTIFY)  │  │
│  └──────┬───────┘  └───────┬────────┘  └────────┬────────┘  │
└─────────┼──────────────────┼────────────────────┼───────────┘
          │                  │                    │
     ┌────┴────┐      ┌──────┴──────┐      ┌──────┴──────┐
     │PX4/Ardu │      │   大疆无人机  │      │  政府平台   │
     │ Pilot   │      │  (通过 APP)  │      │            │
     └─────────┘      └─────────────┘      └────────────┘
```

---

### API 参考

#### HTTP 接口

| 方法 | 端点 | 描述 |
| ------ | ------ | ------ |
| GET | `/health` | 健康检查 |
| GET | `/api/v1/status` | 网关状态和统计信息 |
| GET | `/api/v1/drones` | 列出所有已连接的无人机 |
| GET | `/api/v1/drones/{id}` | 获取指定无人机状态 |
| GET | `/api/v1/drones/{id}/track` | 获取历史轨迹点 |
| DELETE | `/api/v1/drones/{id}/track` | 清除轨迹历史 |

#### WebSocket

连接 `ws://localhost:8080/api/v1/ws` 获取实时更新。

**消息类型：**

```json
// 状态更新（服务端 → 客户端）
{
  "type": "state_update",
  "data": { /* DroneState */ }
}

// 订阅特定无人机（客户端 → 服务端）
{
  "type": "subscribe",
  "device_ids": ["drone-001", "drone-002"]
}

// 取消订阅（客户端 → 服务端）
{
  "type": "unsubscribe",
  "device_ids": ["drone-001"]
}
```

#### 统一数据模型（DroneState）

```json
{
  "device_id": "mavlink-001",
  "timestamp": 1709882231000,
  "protocol_source": "mavlink",
  "location": {
    "lat": 39.9042,
    "lon": 116.4074,
    "lat_gcj02": 39.9066,
    "lon_gcj02": 116.4136,
    "alt": 120.5,
    "coordinate_system": "WGS84"
  },
  "attitude": {
    "roll": 0.05,
    "pitch": -0.12,
    "yaw": 180.0
  },
  "velocity": {
    "vx": 10.5,
    "vy": 0.0,
    "vz": -0.5
  },
  "status": {
    "battery_percent": 85,
    "flight_mode": "AUTO",
    "armed": true,
    "signal_quality": 95
  }
}
```

---

### 配置说明

```yaml
## 服务器设置
server:
  #log_level: info  # debug, info, warn, error

## MAVLink 适配器
mavlink:
  enabled: true
  #connection_type: udp   # udp | tcp | serial
  address: "0.0.0.0:14550"

## DJI 转发端适配器
dji:
  enabled: false
  listen_address: "0.0.0.0:14560"
  max_clients: 10

## MQTT 发布器
mqtt:
  enabled: true
  broker: "tcp://localhost:1883"
  client_id: "outb-001"
  topic_prefix: "uav/telemetry"
  qos: 1
  lwt:
    enabled: true
    topic: "uav/status"
    message: "offline"

## HTTP API
http:
  enabled: true
  address: "0.0.0.0:8080"
  cors_enabled: true
  cors_origins: ["*"]

## 频率控制
throttle:
  default_rate_hz: 1.0
  min_rate_hz: 0.5
  max_rate_hz: 10.0

## 坐标转换（国内地图）
coordinate:
  #convert_gcj02: true   # 高德、腾讯、谷歌中国
  #convert_bd09: false   # 百度地图

## 轨迹存储
track:
  enabled: true
  max_points_per_drone: 10000
  sample_interval_ms: 1000
```

---

### 部署场景

#### 1. 机载边缘网关

部署在挂载于无人机上的树莓派或 Jetson。

```
[飞控] --串口--> [树莓派上的 OUTB] --4G--> [云端]
```

#### 2. 地面站网关

与地面站软件一起运行。

```
[无人机] --无线电--> [地面站 + OUTB] --WiFi/4G--> [云端]
```

#### 3. 云端聚合器

用于机队管理的集中式协议转换。

```
[无人机机队] --TCP/UDP--> [云端 OUTB] --> [后端服务]
```

---

### 开发路线图

- [x] **v0.1** - MAVLink → MQTT 基础管道
- [x] **v0.2** - DJI Android 转发端应用
- [x] **v0.3** - 坐标转换 + HTTP API
- [x] **v0.3.1** - WebSocket + 轨迹存储
- [x] **v0.4** - GB/T 28181 国标支持
- [x] **v1.0** - Web 管理界面 (React + TypeScript)

---

### 项目结构

```
#├── cmd/outb/               # 应用程序入口
├── internal/
#│   ├── adapters/           # 南向协议适配器
#│   │   ├── mavlink/        # MAVLink (UDP/TCP/串口)
#│   │   └── dji/            # DJI 转发端 (TCP 服务器)
#│   ├── api/                # HTTP/WebSocket 服务器
#│   ├── config/             # YAML 配置
#│   ├── core/               # 核心引擎
#│   │   ├── coordinator/    # 坐标转换
#│   │   ├── statestore/     # 状态缓存
#│   │   ├── throttler/      # 频率控制
#│   │   └── trackstore/     # 历史轨迹
#│   ├── models/             # 统一数据模型
#│   └── publishers/         # 北向发布器
#│       └── mqtt/           # MQTT 发布器
#├── android/                # DJI Android 转发端 (Kotlin)
#├── configs/                # 配置示例
#├── scripts/                # 测试工具
#└── docs/                   # 文档
```

---

### 技术选型

| 模块 | 技术 | 版本 |
| ------ | ------ | ------ |
| MAVLink 解析 | gomavlib | v3.3.0 |
| MQTT 客户端 | paho.mqtt.golang | v1.5.1 |
| HTTP 路由 | chi | v5.2.4 |
| WebSocket | gorilla/websocket | v1.5.3 |
| 配置管理 | yaml.v3 | v3.0.1 |

---

### 参与贡献

欢迎贡献代码！提交 PR 前请阅读贡献指南。

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

---

### 开源协议

本项目采用 Apache License 2.0 协议 - 详见 [LICENSE](LICENSE) 文件。

---

### 致谢

- [gomavlib](https://github.com/bluenviron/gomavlib) - Go MAVLink 库
- [paho.mqtt.golang](https://github.com/eclipse/paho.mqtt.golang) - Eclipse Paho MQTT 客户端
- [chi](https://github.com/go-chi/chi) - 轻量级 HTTP 路由

## 离线同步引擎

> Local-First 架构的离线同步引擎，针对低带宽环境优化

中文版 | [English](README.md)

完整的离线同步解决方案，采用 Local-First 架构设计。应用可以完全离线运行，以本地存储作为主数据源，同时在后台自动与服务器同步。针对不稳定的网络条件（如非洲地区的 2G/3G 网络）进行了优化，支持数据压缩、断点续传和智能冲突解决。

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.3-blue)
[![License](https://img.shields.io/badge/license-MIT-green)

### ✨ 特性

#### 核心能力

- 🌐 **完全离线支持** - 基于 IndexedDB 本地存储，可完全离线工作
- 🔄 **自动同步** - 检测到网络后自动同步
- ⚡ **增量同步** - 仅传输变更数据，节省带宽
- 🗜️ **Outbox 模式** - 拦截写操作，本地排队，可靠同步
- 🧠 **智能冲突解决** - Last-Write-Wins (LWW) + 向量时钟
- 📱 **跨平台** - 基于 RxDB，支持 Web 和移动端

#### 高级特性

- 📦 **数据压缩** - MessagePack + DEFLATE，减少 40-60% 数据量
- 📤 **断点续传** - 完整的 TUS 协议实现，支持大文件上传
- ⚡ **性能优化** - 批量操作、索引优化、查询缓存
- 🔌 **实时推送** - WebSocket 服务端推送通知
- 🛡️ **类型安全** - 端到端 TypeScript 支持

### 📐 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                        客户端应用                             │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                    UI 层 (React)                       │ │
│  └──────────────────────┬─────────────────────────────────┘ │
│                         │                                   │
│  ┌──────────────────────▼─────────────────────────────────┐ │
│  │                 离线 SDK (@offline-sync/sdk)            │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────┐     │ │
│  │  │ 存储层   │  │ 网络管理 │  │ Outbox  │  │ 同步    │     │ │
│  │  │ (RxDB)  │  │  器     │  │ (队列)   │  │管理器   │     │ │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └───┬────┘     │ │
│  │       │            │            │            │         │ │
│  │  ┌───▼────────────▼────────────▼────────────▼───────┐  │ │
│  │  │           IndexedDB (浏览器本地存储)               │  │ │
│  │  └──────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTPS (压缩传输)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    同步网关服务器                             │
│  ┌──────────────┐  ┌──────────┐  ┌─────────┐  ┌────────┐    │
│  │   网关        │  │ 应用器    │  │ 仲裁器   │  │   TUS  │    │
│  │  (路由)       │  │(应用操作) │  │(冲突解决) │  │(断点续传)│   │
│  └──────┬───────┘  └────┬─────┘  └────┬─────┘  └────┬───┘   │
│         │               │             │             │       │
│  ┌──────▼──────────────▼─────────────▼─────────────▼───┐    │
│  │                 CouchDB (主数据库)                    │   │
│  │  - todos, products, customers, orders                │   │
│  │  - _changes feed 用于增量同步                          │   │
│  │  - Mango Query 支持                                   │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 🚀 快速开始

#### 安装

```bash
## 克隆仓库
git clone https://github.com/iannil/offline-sync-engine.git
cd offline-sync-engine

## 安装依赖
pnpm install
```

#### 运行开发服务器

```bash
## 启动服务端 (端口 3000)
pnpm dev:server

## 启动客户端演示 (端口 5173)
pnpm dev:client
```

#### 构建

```bash
## 构建 SDK
pnpm --filter @offline-sync/sdk build

## 构建服务端
pnpm --filter @offline-sync/server build

## 构建演示应用
pnpm --filter @offline-sync/client-demo build
```

### 💻 使用示例

#### SDK 基础用法

```typescript
import { OfflineClient } from '@offline-sync/sdk';

// 初始化客户端
const client = new OfflineClient({
  database: { name: 'my-app' },
  sync: {
    enabled: true,
    url: 'https://api.example.com/sync',
    interval: 30000,  // 每 30 秒同步一次
    enableCompression: true,
  },
});

// 等待客户端就绪
await client.initialize();

// 获取数据库
const db = client.getDatabase();

// 创建待办事项 (离线 + 自动同步)
const todo = await db.todos.insert({
  id: 'todo-1',
  text: '学习离线同步引擎',
  completed: false,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
});

// 手动触发同步
await client.getSyncManager().triggerSync();

// 监听同步状态
client.getSyncManager().onStateChange((state) => {
  console.log('同步中:', state.isSyncing);
  console.log('待同步数量:', state.pendingCount);
});
```

#### TUS 断点续传

```typescript
import { createTusUpload } from '@offline-sync/sdk/storage';

// 创建文件上传
const uploader = createTusUpload({
  endpoint: 'https://api.example.com/api/tus',
  data: file,
  metadata: {
    filename: file.name,
    type: file.type,
  },
  chunkSize: 5 * 1024 * 1024,  // 5MB 分片
  onProgress: (sent, total) => {
    console.log(`进度: ${(sent / total * 100).toFixed(1)}%`);
  },
});

// 开始上传
const uploadUrl = await uploader.start();

// 暂停上传
uploader.pause();

// 恢复上传 (支持断点续传)
await uploader.resume();
```

#### 服务端 API

```bash
## 推送本地操作到服务器
curl -X POST https://api.example.com/api/sync/push \
  -H "Content-Type: application/msgpack+deflate" \
  -H "Accept: application/msgpack+deflate" \
  --data-binary '@payload.bin'

## 拉取服务器变更
curl "https://api.example.com/api/sync/pull?since=1234567890" \
  -H "Accept: application/msgpack+deflate"

## TUS 创建上传
curl -X POST https://api.example.com/api/tus \
  -H "Tus-Resumable: 1.0.0" \
  -H "Upload-Length: 1024000" \
  -H "Upload-Metadata: filename dGVzdC5qcGc="
```

### 📦 包结构

```
offline-sync-engine/
├── packages/
#│   ├── sdk/              # 客户端 SDK
│   │   ├── src/
#│   │   │   ├── storage/     # 存储模块
#│   │   │   ├── network/     # 网络管理
#│   │   │   ├── outbox/      # 离线队列
#│   │   │   ├── sync/        # 同步管理
#│   │   │   └── client/      # 客户端入口
│   │   └── package.json
│   │
#│   ├── server/           # 同步网关服务器
│   │   ├── src/
#│   │   │   ├── gateway/     # 同步网关
#│   │   │   ├── applier/     # 操作应用器
#│   │   │   ├── arbiter/     # 冲突仲裁器
#│   │   │   ├── database/    # 数据库层
#│   │   │   └── tus/         # TUS 协议
│   │   └── package.json
│   │
#│   └── client-demo/       # 演示应用
│       ├── src/
│       │   ├── components/
│       │   └── db/
│       └── package.json
│
#├── docs/                 # 文档
#├── pnpm-workspace.yaml  # Monorepo 配置
└── package.json
```

### 🔧 配置

#### SDK 配置

```typescript
interface OfflineClientConfig {
  // 数据库配置
  database: {
    name: string;              // 数据库名称
  };

  // 同步配置
  sync?: {
    enabled: boolean;         // 启用同步
    url: string;              // 同步服务器 URL
    interval?: number;        // 同步间隔 (毫秒)
    batchSize?: number;       // 批量大小
    enableCompression?: boolean;  // 启用压缩
    enableWebSocket?: boolean;    // 启用 WebSocket
  };

  // Outbox 配置
  outbox?: {
    maxRetries?: number;      // 最大重试次数
    initialDelay?: number;    // 初始重试延迟 (毫秒)
    maxDelay?: number;        // 最大重试延迟 (毫秒)
  };
}
```

#### 服务端配置

```bash
## 环境变量
COUCHDB_URL=http://localhost:5984
COUCHDB_USERNAME=admin
COUCHDB_PASSWORD=password
COUCHDB_DB_PREFIX=offline-sync
PORT=3000
HOST=0.0.0.0
```

### 📚 API 文档

#### SDK 导出

```typescript
// 客户端
import { OfflineClient } from '@offline-sync/sdk/client';

// 存储
import {
  createDatabase,
  getDatabase,
  todoSchema,
  productSchema,
} from '@offline-sync/sdk/storage';

// 查询
import {
  findAll,
  findById,
  findWhere,
  paginate,
  count,
  QueryBuilder,
} from '@offline-sync/sdk/storage';

// 压缩
import {
  CompressionService,
  compress,
  decompress,
} from '@offline-sync/sdk/storage';

// TUS 协议
import {
  createTusUpload,
  uploadFile,
  TusUploader,
} from '@offline-sync/sdk/storage';

// 测试
import {
  benchmarkWrite,
  benchmarkRead,
  benchmarkQuery,
  testCapacity,
} from '@offline-sync/sdk/testing';

// 类型
import type { Todo, Product, OutboxAction, NetworkStatus } from '@offline-sync/sdk';
```

#### 服务端端点

| 端点 | 方法 | 描述 |
| ---------- | -------- | ------------- |
| `/health` | GET | 健康检查 |
| `/api/sync/push` | POST | 推送本地操作 |
| `/api/sync/pull` | GET | 拉取远程变更 |
| `/api/sync/:collection` | GET | 获取集合数据 |
| `/api/sync/:collection/:id` | GET | 获取单个文档 |
| `/api/applier/apply` | POST | 应用单个操作 |
| `/api/applier/batch` | POST | 批量应用操作 |
| `/api/arbiter/check` | POST | 冲突检测 |
| `/api/arbiter/resolve` | POST | LWW 冲突解决 |
| `/api/arbiter/resolve/merge` | POST | 字段级合并 |
| `/api/tus` | POST | 创建上传 |
| `/api/tus/:id` | PATCH | 上传分片 |
| `/api/stream` | WS | 实时推送 |

### 🧪 开发

#### 环境要求

- Node.js >= 18
- pnpm >= 8
- CouchDB >= 3.0 (可选，用于生产环境)

#### 开发命令

```bash
## 安装依赖
pnpm install

## 启动开发服务器
#pnpm dev:server  # 服务端
#pnpm dev:client  # 客户端

## 运行测试
pnpm test

## 代码检查
pnpm lint
pnpm format
```

#### 本地 CouchDB 开发

```bash
## 使用 Docker 启动 CouchDB
docker run -d \
  --name couchdb \
  -p 5984:5984 \
  -e COUCHDB_USER=admin \
  -e COUCHDB_PASSWORD=password \
  couchdb:3
```

### 📖 文档

| 文档 | 描述 |
| ---------- | ------------- |
| [架构概览](docs/architecture/overview.md) | Local-First 架构设计 |
| [API 文档](docs/api/) | 客户端/服务端 API 定义 |
| [验收报告](docs/VERIFICATION.md) | 功能验证清单 |
| [开发进度](docs/progress/next-steps.md) | 开发路线图 |

### 🤝 贡献

欢迎贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

#### 代码规范

- 使用 TypeScript 编写代码
- 遵循 ESLint 规则
- 为新功能添加单元测试
- 更新相关文档

### 📊 开发进度

```
✅ 第一阶段: 基础离线  [████████████████████████████] 100%
   └─ RxDB 集成、Schema 定义、离线队列、LWW 冲突解决

✅ 第二阶段: 优化       [████████████████████████████] 100%
   └─ 增量同步、MessagePack 压缩

✅ 第三阶段: 高级特性   [████████████████████████████] 100%
   └─ TUS 断点续传、WebSocket 推送、性能优化
```

详见 [开发进度](docs/progress/next-steps.md)。

### 🔗 技术栈

| 类别 | 技术 |
| ---------- | ------------ |
| 前端框架 | React + TypeScript |
| 本地数据库 | RxDB + Dexie (IndexedDB) |
| 后端框架 | Fastify (Node.js) |
| 主数据库 | CouchDB |
| 数据序列化 | MessagePack |
| 数据压缩 | DEFLATE (pako) |
| 断点续传 | TUS 协议 v1.0.0 |
| 实时通信 | WebSocket |
| 包管理器 | pnpm workspaces |
| 构建工具 | tsup (库) + Vite (应用) |
| 测试框架 | Vitest |

### 📄 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

### 🙏 致谢

本项目构建于以下优秀的开源项目之上：

- [RxDB](https://rxdb.info/) - JavaScript NoSQL 数据库
- [Fastify](https://www.fastify.io/) - 高性能 Node.js Web 框架
- [Nano](https://www.npmjs.com/package/nano) - CouchDB 客户端
- [MessagePack](https://msgpack.org/) - 高效的二进制序列化
- [TUS 协议](https://tus.io/) - 断点续传协议
- [Pako](https://github.com/nodeca/pako) - zlib 接口

---

<p align="center">
  <sub>为低带宽环境的离线优先应用而构建 ❤️</sub>
</p>

## License Key Manager

[![Go Version](https://img.shields.io/badge/Go-1.22+-00ADD8?style=flat&logo=go)](https://go.dev/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

面向独立开发者的"设置即遗忘"软件授权管理微服务。

[English](./README.md)

### 特性

- **极低资源占用** - Go + SQLite，<50MB 内存，<20MB Docker 镜像
- **零维护** - 自动处理支付平台 Webhook，自动发码、退款自动吊销
- **数据私有** - 数据完全掌控在自己的服务器上
- **支付闭环** - 直接对接 Gumroad / LemonSqueezy

### 快速开始

#### 使用 Docker（推荐）

```bash
## 克隆仓库
git clone https://github.com/iannil/license-key-manager.git
cd license-key-manager

## 配置环境变量
cp .env.example .env
## 编辑 .env 文件，设置 ADMIN_SECRET

## 使用 Docker Compose 启动
docker-compose up -d
```

#### 从源码构建

```bash
## 前提：Go 1.22+
go build -o license-manager ./cmd/server

## 运行
./license-manager
```

#### 验证安装

```bash
curl http://localhost:8080/health
## {"status":"ok"}
```

### 配置项

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `PORT` | 服务端口 | `8080` |
| `DB_TYPE` | 数据库类型 | `sqlite` |
| `DB_PATH` | SQLite 数据库路径 | `./data/licenses.db` |
| `ADMIN_SECRET` | 管理 API 密钥 | （必填） |
| `LEMONSQUEEZY_SECRET` | LemonSqueezy Webhook 签名密钥 | - |
| `GUMROAD_PRODUCT_PERMALINK` | Gumroad 产品标识，用于验证 | - |

### API 文档

#### 公开接口（无需认证）

##### 激活授权

```http
POST /v1/licenses/activate
Content-Type: application/json

{
  "key": "PRO-XXXX-XXXX-XXXX-XXXX",
  "machine_id": "设备唯一标识",
  "device_name": "MacBook Pro"
}
```

**响应：**
```json
{
  "success": true,
  "data": {
    "valid": true,
    "license_id": 1,
    "product_id": "my-app",
    "is_new_activation": true
  }
}
```

##### 验证授权

```http
POST /v1/licenses/validate
Content-Type: application/json

{
  "key": "PRO-XXXX-XXXX-XXXX-XXXX",
  "machine_id": "设备唯一标识"
}
```

##### 解绑设备

```http
POST /v1/licenses/deactivate
Content-Type: application/json

{
  "key": "PRO-XXXX-XXXX-XXXX-XXXX",
  "machine_id": "设备唯一标识"
}
```

#### 管理接口（需要 `X-Admin-Secret` 请求头）

##### 创建授权码

```http
POST /admin/licenses
X-Admin-Secret: your-admin-secret
Content-Type: application/json

{
  "product_id": "my-app",
  "user_email": "customer@example.com",
  "max_activations": 3,
  "expiration_days": 365,
  "prefix": "PRO"
}
```

##### 查询授权详情

```http
GET /admin/licenses/:key
X-Admin-Secret: your-admin-secret
```

##### 吊销授权

```http
PUT /admin/licenses/:key/revoke
X-Admin-Secret: your-admin-secret
```

#### Webhook 接口

##### LemonSqueezy

```http
POST /webhooks/lemonsqueezy
```

支持的事件：
- `order_created` - 创建新授权码
- `order_refunded` - 吊销授权码
- `subscription_cancelled` - 吊销授权码

##### Gumroad

```http
POST /webhooks/gumroad
```

自动处理销售和退款事件。

### 客户端集成示例

#### Node.js / Electron

```javascript
const crypto = require('crypto');
const axios = require('axios');
const os = require('os');

const LICENSE_SERVER = 'https://license.yourdomain.com';

// 生成设备唯一标识
function getMachineId() {
  const { networkInterfaces, cpus, hostname } = require('os');
  const data = JSON.stringify({
    hostname: hostname(),
    cpus: cpus()[0]?.model,
    network: Object.values(networkInterfaces()).flat().find(i => !i.internal)?.mac
  });
  return crypto.createHash('sha256').update(data).digest('hex').substring(0, 32);
}

async function activateLicense(licenseKey) {
  try {
    const response = await axios.post(`${LICENSE_SERVER}/v1/licenses/activate`, {
      key: licenseKey,
      machine_id: getMachineId(),
      device_name: os.hostname()
    });
    return response.data.success;
  } catch (error) {
    if (error.response?.data?.error?.code === 'MACHINE_LIMIT_EXCEEDED') {
      console.error('激活设备数已达上限');
    }
    return false;
  }
}

async function validateLicense(licenseKey) {
  try {
    const response = await axios.post(`${LICENSE_SERVER}/v1/licenses/validate`, {
      key: licenseKey,
      machine_id: getMachineId()
    });
    return response.data.success;
  } catch (error) {
    return false;
  }
}
```

#### Python

```python
import hashlib
import platform
import requests

LICENSE_SERVER = 'https://license.yourdomain.com'

def get_machine_id():
    data = f"{platform.node()}-{platform.processor()}-{platform.system()}"
    return hashlib.sha256(data.encode()).hexdigest()[:32]

def activate_license(license_key):
    try:
        response = requests.post(f"{LICENSE_SERVER}/v1/licenses/activate", json={
            "key": license_key,
            "machine_id": get_machine_id(),
            "device_name": platform.node()
        })
        return response.json().get("success", False)
    except:
        return False

def validate_license(license_key):
    try:
        response = requests.post(f"{LICENSE_SERVER}/v1/licenses/validate", json={
            "key": license_key,
            "machine_id": get_machine_id()
        })
        return response.json().get("success", False)
    except:
        return False
```

### 错误码

| 错误码 | 说明 |
|--------|------|
| `LICENSE_NOT_FOUND` | 授权码不存在 |
| `LICENSE_INVALID` | 授权码无效或已过期 |
| `LICENSE_REVOKED` | 授权码已被吊销 |
| `MACHINE_LIMIT_EXCEEDED` | 设备激活数已达上限 |
| `MACHINE_NOT_FOUND` | 该设备未激活此授权 |
| `UNAUTHORIZED` | 管理密钥无效 |
| `INVALID_SIGNATURE` | Webhook 签名验证失败 |

### 项目结构

```
license-key-manager/
#├── cmd/server/main.go          # 应用入口
├── internal/
#│   ├── config/                 # 配置管理
#│   ├── database/               # 数据库层
#│   ├── models/                 # 数据模型 (License, Activation)
#│   ├── services/               # 业务逻辑
#│   ├── handlers/               # HTTP 处理器
#│   └── middleware/             # 认证 & Webhook 签名验证
#├── pkg/response/               # 统一响应格式
├── Dockerfile
├── docker-compose.yml
└── docs/
```

### 开发

```bash
## 运行测试
go test -v ./...

## 测试覆盖率
go test -cover ./...

## 构建
go build -o bin/license-manager ./cmd/server
```

### 路线图

#### V1.0（已完成）

- [x] 授权码生成与管理
- [x] 授权激活 / 验证 / 解绑 API
- [x] 管理 API（创建 / 查询 / 吊销授权码）
- [x] Gumroad Webhook 集成
- [x] LemonSqueezy Webhook 集成
- [x] SQLite 数据库支持
- [x] Docker 部署支持
- [x] 单元测试

#### 未来计划

- [ ] Go-Cache 缓存层（高频验证请求优化）
- [ ] 离线验证（RSA 签名许可文件）
- [ ] 浮动授权（基于心跳）
- [ ] PostgreSQL/MySQL 支持
- [ ] 邮件通知集成

### 贡献

欢迎贡献代码！请随时提交 Pull Request。

### 开源协议

本项目基于 MIT 协议开源 - 详见 [LICENSE](LICENSE) 文件。

## LLM Math Calculator (LMC)

[English](README.md) | 中文

**AI 基础设施资源规划工具** — 估算大模型训练和推理所需的计算资源

[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

LMC 是一个面向大模型训练的"科学计算器"，将复杂的资源估算公式封装为极简接口。它能帮助你快速回答：

- 训练一个 70B 模型需要多少张 GPU？
- 用 64 张 A100 训练需要多长时间？
- 显存够不够？该用什么并行策略？

### 功能特性

- **核心计算引擎**: 基于 Megatron-LM 论文的数学模型，支持 FLOPs、显存、训练时间估算
- **并行策略推荐**: 自动推荐 TP/PP/DP/ZeRO 配置
- **硬件数据库**: 预置 A100、H100、H800、昇腾 910B 等主流 GPU 参数
- **模型预设库**: 支持 Llama-3、Mixtral、GPT-3、Qwen、DeepSeek 等模型
- **多种接口**: CLI 命令行 + Python API + Web UI

### 安装

```bash
## 基础安装
pip install lmc

## 包含 Web UI
pip install lmc[web]

## 开发环境
pip install lmc[dev]
```

从源码安装：

```bash
git clone https://github.com/iannil/llm-math-calculator.git
cd llm-math-calculator
pip install -e ".[dev,web]"
```

### 快速开始

#### CLI 命令行

```bash
## 基础估算
lmc train --gpu A100-80G-SXM --params 70B --tokens 400B --num-gpus 64

## 指定目标训练天数，自动计算 GPU 数量
lmc train --gpu A100-80G-SXM --params 70B --tokens 400B --days 30

## 使用模型预设
lmc train --preset Llama-3-70B --tokens 400B --gpu H100-80G-SXM --num-gpus 128

## 交互式模式
lmc train -i

## 查看硬件列表
lmc hardware list

## 查看模型预设
lmc model list

## CI/CD 可行性检查
lmc check --params 70B --gpu A100-80G-SXM --num-gpus 64
```

输出示例：

```
╭──────────────────────────────────────────────────────────────╮
│ LLM Math Calculator Report                                   │
╰──────────────────────────────────────────────────────────────╯
                       [Input]
  Model       70.0B Params
  Data        400.0B Tokens
  Hardware    A100-80G-SXM (FP16 Peak: 312.0 TFLOPS)
  GPUs        64

       [Resources Required]
  Total FLOPs      168.00 ZFLOPs
  Training Time    6.5 months
  GPU Hours        299,145
  GPU Days         12,464.4

        [Memory Breakdown (per GPU)]
  Parameters              4.4 GB
  Gradients               2.2 GB
  Optimizer States        13.1 GB
  Activations             9.2 GB
  Buffer/Frag (~8%)       2.7 GB
  --------------------    ----------
  Total Usage             35.9 GB / 80.0 GB
  Status                  (Safe)

         [Recommended Strategy]
  Framework          Megatron-DeepSpeed
  Parallelism        TP=4, PP=8, DP=2
  ZeRO Stage         Stage-2
  FlashAttn          Enabled (Required)
  Grad Checkpoint    Recommended
  MFU                50%
```

#### Python API

```python
from lmc import (
    estimate_resources,
    TrainingConfig,
    ZeROStage,
    calc_total_flops,
    calc_memory_model_states,
    get_hardware,
    get_preset,
)

## 完整资源估算
config = TrainingConfig(
    params_billion=70,
    tokens_billion=400,
    gpu_name="A100-80G-SXM",
    num_gpus=64,
    zero_stage=ZeROStage.ZERO_2,
    use_recomputation=True,
)
result = estimate_resources(config)

print(f"训练时间: {result.training_time_formatted}")
print(f"GPU Hours: {result.gpu_hours:,.0f}")
print(f"显存/GPU: {result.memory.per_gpu:.1f} GB")
print(f"推荐策略: TP={result.parallelism.tensor_parallel}, PP={result.parallelism.pipeline_parallel}")

## 单独计算
flops = calc_total_flops(params=70e9, tokens=400e9)
memory = calc_memory_model_states(params=70e9, zero_stage=ZeROStage.ZERO_2, num_gpus=8)

## 使用预设
preset = get_preset("Llama-3-70B")
print(f"Hidden size: {preset.hidden_size}, Layers: {preset.num_layers}")
```

#### Web UI

```bash
## 启动 Web 界面
streamlit run src/lmc/web.py
## 或
lmc-web
```

### 核心公式

基于 [Megatron-LM](https://arxiv.org/abs/1909.08053) 论文和业界实践：

| 指标 | 公式 | 说明 |
| ------ | ------ | ------ |
| 训练算力 | `6 × P × D` | P=参数量, D=训练数据量 |
| 显存 (ZeRO-0) | `16 Bytes × P` | 参数+梯度+优化器状态 |
| 激活值 | `s × b × h × L × (34 + 5ah/s)` | s=序列长度, b=batch, h=hidden, L=层数 |
| KV Cache | `2 × b × s × h × L × 2` | 推理场景 |
| 训练时间 | `FLOPs / (GPUs × TFLOPS × MFU)` | MFU: 模型利用率 |

#### ZeRO 优化

| 阶段 | 分区内容 | 显存节省 |
| ------ | ---------- | ---------- |
| ZeRO-0 | 无 | 基准 (16B/param) |
| ZeRO-1 | 优化器状态 | ~4x |
| ZeRO-2 | 优化器+梯度 | ~8x |
| ZeRO-3 | 全部 | ~N (GPU数) |

### 支持的硬件

| 硬件 | 显存 | FP16 TFLOPS | 互联带宽 | 典型 MFU |
| ------ | ------ | ------------- | ---------- | ---------- |
| A100-40G-SXM | 40 GB | 312 | NVLink 600 GB/s | 50% |
| A100-80G-SXM | 80 GB | 312 | NVLink 600 GB/s | 50% |
| A100-80G-PCIe | 80 GB | 312 | - | 45% |
| H100-80G-SXM | 80 GB | 989 | NVLink 900 GB/s | 55% |
| H100-80G-PCIe | 80 GB | 756 | - | 50% |
| H800-80G-SXM | 80 GB | 989 | NVLink 400 GB/s | 50% |
| Ascend 910B | 64 GB | 320 | HCCS 392 GB/s | 45% |
| L40S | 48 GB | 362 | - | 45% |

### 支持的模型预设

| 模型 | 参数量 | 架构 | MoE |
| ------ | -------- | ------ | ----- |
| Llama-3-8B | 8B | Llama | - |
| Llama-3-70B | 70B | Llama | - |
| Llama-3.1-405B | 405B | Llama | - |
| Mixtral-8x7B | 46.7B (12.9B active) | Mixtral | 8×2 |
| Mixtral-8x22B | 141B (39B active) | Mixtral | 8×2 |
| GPT-3-175B | 175B | GPT | - |
| Qwen2-72B | 72B | Qwen | - |
| DeepSeek-V2-236B | 236B (21B active) | DeepSeek | 160×6 |

### 项目结构

```
llm-math-calculator/
#├── pyproject.toml          # 项目配置
├── data/
#│   ├── hardware.json       # 硬件数据库
#│   └── presets.json        # 模型预设库
├── src/lmc/
#│   ├── __init__.py         # API 导出
#│   ├── cli.py              # CLI 命令
#│   ├── engine.py           # 计算引擎
#│   ├── loader.py           # 数据加载器
#│   ├── models.py           # 数据模型
#│   ├── optimizer.py        # 策略推荐器
#│   └── web.py              # Web UI
└── tests/
    #└── test_engine.py      # 单元测试
```

### 开发

```bash
## 克隆仓库
git clone https://github.com/iannil/llm-math-calculator.git
cd llm-math-calculator

## 创建虚拟环境
python -m venv .venv
source .venv/bin/activate

## 安装开发依赖
pip install -e ".[dev,web]"

## 运行测试
pytest tests/ -v

## 运行单个命令测试
lmc train --gpu A100-80G-SXM --params 7B --tokens 100B
```

### 贡献

欢迎贡献！特别是：

- **硬件数据**: 添加新 GPU (如 B200、MI300X) 到 `data/hardware.json`
- **模型预设**: 添加新模型到 `data/presets.json`
- **公式优化**: 改进估算精度
- **文档翻译**: 多语言支持

提交 PR 前请确保：

1. 通过所有测试 (`pytest tests/`)
2. 代码符合项目风格
3. 更新相关文档

### 路线图

#### 已完成 (v1.0)

- [x] **核心计算引擎**：基于 Megatron-LM 公式的 FLOPs、显存、训练时间估算
- [x] **CLI 命令行工具**：`lmc train`、`lmc check`、`lmc calc-gpus`、`lmc hardware`、`lmc model` 命令
- [x] **Python API**：20+ 导出函数和类
- [x] **Web UI**：基于 Streamlit 的可视化界面
- [x] **硬件数据库**：8 种 GPU（A100、H100、H800、昇腾 910B、L40S 等）
- [x] **模型预设库**：8 个模型（Llama-3、Mixtral、GPT-3、Qwen、DeepSeek 等）
- [x] **MoE 支持**：混合专家模型的激活参数追踪
- [x] **ZeRO 优化**：完整支持 ZeRO 0-3 阶段
- [x] **单元测试**：24 个测试用例覆盖核心功能
- [x] **文档**：中英文双语 README

#### 计划中

- [ ] 添加更多 GPU 支持（B200、MI300X 等）
- [ ] 添加推理资源估算功能
- [ ] PyPI 包发布
- [ ] Web UI 交互增强

### 参考文献

- [Megatron-LM: Training Multi-Billion Parameter Language Models Using Model Parallelism](https://arxiv.org/abs/1909.08053)
- [ZeRO: Memory Optimizations Toward Training Trillion Parameter Models](https://arxiv.org/abs/1910.02054)
- [FlashAttention: Fast and Memory-Efficient Exact Attention](https://arxiv.org/abs/2205.14135)

### License

MIT License - 详见 [LICENSE](LICENSE) 文件
