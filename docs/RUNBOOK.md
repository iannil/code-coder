# 运行手册 (RUNBOOK)

> 文档类型: runbook
> 创建时间: 2026-02-05
> 最后更新: 2026-02-16
> 状态: active

## 部署程序

### 开发环境部署

```bash
# 1. 安装依赖
bun install

# 2. 类型检查
bun typecheck

# 3. 运行测试
cd packages/ccode && bun test

# 4. 启动开发模式
bun dev
```

### 生产构建

```bash
# 构建独立可执行文件
bun run --cwd packages/ccode build

# 输出: packages/ccode/bin/ccode
```

### Docker 部署 (推荐)

```bash
# 1. 构建镜像
docker build -t codecoder:latest .

# 2. 运行容器 (独立网络)
docker network create codecoder-net
docker run --network codecoder-net -v ~/.ccode:/app/.ccode codecoder:latest

# 3. 发布文件夹
# 固定在 /release 文件夹
# Rust 服务: /release/rust
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
- 用户日志: `~/.ccode/logs/` 或 `~/.codecoder/logs/`
- 系统日志: 系统日志服务
- 缓存目录: `~/.ccode/cache/` (包含版本标记)
- 配置目录: `~/.ccode/` 或 `~/.codecoder/`

### 数据目录结构

```
~/.ccode/                    # 主配置目录
~/.ccode/logs/               # 日志文件
~/.local/share/ccode/        # XDG 数据目录
~/.local/share/ccode/bin/    # 可执行文件
~/.local/share/ccode/log/    # 运行时日志
~/.cache/ccode/              # 缓存目录
~/.local/state/ccode/        # 状态文件
```

### 健康检查

```bash
# 检查 CLI 是否可用
ccode --version

# 检查配置
ccode config show

# 检查 Agent 列表
ccode agent list

# 检查缓存版本
cat ~/.cache/ccode/version
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

# 检查 ZeroBot 数据库
ls -la ~/.codecoder/workspace/memory/brain.db
```

**修复**:
1. 确保 `./memory/` 目录存在且可写
2. 检查 `MEMORY.md` 格式是否正确
3. 清除记忆缓存: 调用 `invalidateMemoryCache()`
4. 如需写入 ZeroBot: 使用 `createZeroBotMemory({ readOnly: false })`

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
