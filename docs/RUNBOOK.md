# 运行手册 (RUNBOOK)

> 文档类型: runbook
> 创建时间: 2026-02-05
> 最后更新: 2026-02-22
> 状态: active

## 端口配置

| 服务 | 端口 | 说明 |
|------|------|------|
| CodeCoder API | 4400 | 核心 AI 引擎 API |
| Web Frontend | 4401 | Vite 开发服务器 |
| Zero CLI Daemon | 4402 | 组合服务 (gateway + channels + scheduler) |
| Faster Whisper | 4403 | 本地 STT 服务 |
| Zero Gateway | 4410 | 认证、代理、配额 (独立) |
| Zero Channels | 4411 | IM 渠道 Webhook (独立) |
| Zero Workflow | 4412 | 工作流自动化 (独立) |
| MCP Server | 4420 | Model Context Protocol (独立) |

## 部署程序

### 开发环境部署

```bash
# 1. 安装依赖
bun install
cd services && cargo build --workspace

# 2. 类型检查
bun typecheck

# 3. 运行测试
cd packages/ccode && bun test
cd services && cargo test --workspace

# 4. 启动开发模式
bun dev          # TUI 模式
bun dev:web      # Web 模式
bun dev:serve    # API 服务器模式
```

### Rust 服务部署

```bash
# 构建所有服务
cd services && cargo build --release --workspace

# 构建特定服务
cargo build --release -p zero-gateway
cargo build --release -p zero-channels
cargo build --release -p zero-workflow

# 发布优化构建 (最小体积)
cargo build --profile dist --workspace

# 输出目录: services/target/release/
```

### 生产构建

```bash
# TypeScript: 构建独立可执行文件
bun run --cwd packages/ccode build

# 输出: packages/ccode/bin/ccode

# Rust: 发布构建
cd services && cargo build --profile dist --workspace

# 输出: services/target/dist/
```

### Docker 部署 (推荐)

```bash
# 1. 构建镜像
docker build -t codecoder:latest .

# 2. 运行容器 (独立网络)
docker network create codecoder-net
docker run --network codecoder-net \
  -v ~/.codecoder:/app/.codecoder \
  -p 4400:4400 \
  -p 4401:4401 \
  codecoder:latest

# 3. Rust 服务容器
docker run --network codecoder-net \
  -p 4402:4402 \
  -p 4410:4410 \
  -p 4411:4411 \
  -p 4412:4412 \
  codecoder-services:latest

# 4. 发布文件夹
# 固定在 /release 文件夹
# Rust 服务: /release/rust
# Docker 配置: /release/docker
```

### 多服务启动

```bash
# 启动所有 Rust 服务 (推荐使用 tmux 或 supervisor)

# 方式 1: 分别启动
./target/release/zero-gateway &
./target/release/zero-channels &
./target/release/zero-workflow &

# 方式 2: Docker Compose
docker compose up -d
```

### 发布准备

```bash
# 1. 更新版本号
# 编辑 packages/ccode/package.json

# 2. 运行测试
cd packages/ccode && bun test

# 3. 构建验证
bun run --cwd packages/ccode build

# 4. 创建发布标签
git tag v0.0.1
git push origin v0.0.1
```

## 监控和告警

### 日志位置

- 开发日志: 控制台输出
- 用户日志: `~/.codecoder/logs/` 或 `~/.codecoder/logs/`
- 系统日志: 系统日志服务
- 缓存目录: `~/.codecoder/cache/` (包含版本标记)
- 配置目录: `~/.codecoder/` 或 `~/.codecoder/`

### 数据目录结构

```
~/.codecoder/                    # 主配置目录
~/.codecoder/logs/               # 日志文件
~/.local/share/ccode/        # XDG 数据目录
~/.local/share/ccode/bin/    # 可执行文件
~/.local/share/ccode/log/    # 运行时日志
~/.cache/ccode/              # 缓存目录
~/.local/state/ccode/        # 状态文件
```

### 健康检查

```bash
# TypeScript 服务
ccode --version
ccode config show
ccode agent list

# Rust 服务健康端点
curl http://localhost:4402/health  # Zero CLI Daemon
curl http://localhost:4410/health  # Zero Gateway
curl http://localhost:4411/health  # Zero Channels
curl http://localhost:4412/health  # Zero Workflow

# 检查缓存版本
cat ~/.cache/ccode/version
```

### Rust 服务监控

```bash
# 查看服务日志
journalctl -u zero-gateway -f
journalctl -u zero-channels -f
journalctl -u zero-workflow -f

# 或使用 Docker
docker logs -f zero-gateway
docker logs -f zero-channels
docker logs -f zero-workflow
```

### 性能指标

- 启动时间: <1 秒
- 内存使用: <500MB (基线)
- 测试通过率: >95%
- 缓存版本: 18 (当前)

## 常见问题和修复

### 问题 1: 构建失败

**症状**: `bun build` 报错

**诊断**:
```bash
# 检查类型错误
bun typecheck
```

**修复**:
1. 修复类型错误
2. 清理缓存: `rm -rf node_modules/.cache`
3. 重新安装: `bun install`

### 问题 2: 测试失败

**症状**: `bun test` 报告失败

**诊断**:
```bash
# 运行特定测试
bun test test/unit/failed-test.test.ts

# 查看详细输出
bun test --verbose
```

**修复**:
1. 检查测试代码和被测代码
2. 确保测试数据正确
3. 更新快照 (如需要): `bun test --update-snapshots`

### 问题 3: TypeScript 类型错误

**症状**: `tsc` 或 `tsgo` 报告类型错误

**诊断**:
```bash
# 查看详细错误
bun typecheck
```

**修复**:
1. 添加缺失的类型注解
2. 检查导入路径
3. 确保 `@types/*` 包已安装

### 问题 4: 依赖问题

**症状**: 模块未找到或版本冲突

**诊断**:
```bash
# 检查依赖
bun pm ls

# 检查 workspace 依赖
bun pm ls --filter @codecoder-ai/util
```

**修复**:
1. 清理并重新安装: `rm -rf node_modules bun.lock && bun install`
2. 检查 workspace 配置
3. 更新依赖版本

### 问题 5: Git 钩子失败

**症状**: 提交被 Husky 钩子拒绝

**诊断**:
```bash
# 手动运行钩子
npx husky install
```

**修复**:
1. 检查代码格式化: `bun prettier --check .`
2. 修复格式问题: `bun prettier --write .`
3. 重新提交

### 问题 6: 记忆系统问题

**症状**: Agent 未能加载用户偏好或历史决策

**诊断**:
```bash
# 检查记忆文件
ls -la ./memory/
cat ./memory/MEMORY.md

# 检查数据库
ls -la ~/.codecoder/workspace/memory/brain.db
```

**修复**:
1. 确保 `./memory/` 目录存在且可写
2. 检查 `MEMORY.md` 格式是否正确
3. 清除记忆缓存: 调用 `invalidateMemoryCache()`
4. 如需写入: 使用 `createMemory({ readOnly: false })`

### 问题 7: Rust 服务构建失败

**症状**: `cargo build` 报错

**诊断**:
```bash
# 检查 Rust 版本
rustc --version

# 检查依赖
cargo check --workspace
```

**修复**:
1. 更新 Rust: `rustup update stable`
2. 清理缓存: `cargo clean`
3. 重新构建: `cargo build --workspace`

### 问题 8: Rust 服务连接失败

**症状**: 服务启动但无法连接

**诊断**:
```bash
# 检查端口占用
lsof -i :4402  # Zero CLI Daemon
lsof -i :4410  # Zero Gateway
lsof -i :4411  # Zero Channels
lsof -i :4412  # Zero Workflow

# 检查服务状态
curl -v http://localhost:4410/health  # Zero Gateway
```

**修复**:
1. 检查端口是否被占用
2. 检查配置文件 `~/.codecoder/config.json`
3. 检查防火墙设置
4. 查看服务日志排查错误

## 回滚程序

### 代码回滚

```bash
# 回滚到上一个提交
git reset --hard HEAD~1

# 回滚到特定标签
git checkout tags/v0.0.1

# 撤销已推送的提交 (谨慎使用)
git revert <commit-hash>
```

### 发布回滚

```bash
# 1. 删除有问题的标签
git tag -d v0.0.1
git push origin :refs/tags/v0.0.1

# 2. 创建新版本
git tag v0.0.2
git push origin v0.0.2
```

## 维护任务

### 日常维护

- [ ] 检查 CI/CD 状态
- [ ] 审查和合并 PR
- [ ] 更新依赖版本

### 周期性维护

- [ ] 清理未使用的依赖
- [ ] 更新文档
- [ ] 审查技术债务
- [ ] 性能基准测试

### 依赖更新

```bash
# 检查过期依赖
bun update

# 交互式更新
bun upgrade

# 更新特定依赖
bun update <package-name>
```

## 故障排查清单

- [ ] 检查 Bun 版本 (`bun --version`)
- [ ] 检查依赖安装 (`bun pm ls`)
- [ ] 运行类型检查 (`bun typecheck`)
- [ ] 运行测试 (`bun test`)
- [ ] 检查 Git 状态 (`git status`)
- [ ] 查看日志文件
- [ ] 检查配置文件

## 紧急联系

- GitHub Issues: https://github.com/iannil/code-coder/issues
- 文档: https://code-coder.com/docs

## 相关文档

- [贡献指南](./CONTRIB.md)
- [架构指南](./Architecture-Guide.md)
- [技术债务清单](./DEBT.md)
