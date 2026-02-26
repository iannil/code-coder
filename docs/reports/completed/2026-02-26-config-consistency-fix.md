# Config Consistency Fix: ccode (TypeScript) vs zero-* (Rust)

完成时间: 2026-02-26 19:07

## 问题描述

ccode (TypeScript) 和 zero-* (Rust) 服务读取 `~/.codecoder/config.json` 时使用不同的配置路径：

| 配置项 | TypeScript (修复前) | Rust |
|--------|-------------------|------|
| 端口 | `server.port` | `services.codecoder.port` |
| 绑定地址 | `server.hostname` | `network.bind` |

由于当前 JSON 配置文件只有 `network.bind` 和 `services.codecoder.port`，没有 `server` 字段，导致 ccode serve 命令使用默认值而非配置值。

## 解决方案

在 TypeScript 中添加统一配置支持，使 ccode 能够读取 Rust 风格的配置作为回退：

### 1. 添加配置 Schema (`packages/ccode/src/config/config.ts`)

新增 `Network` 和 `Services` 配置类型以匹配 Rust 结构：

```typescript
export const Network = z
  .object({
    bind: z.string().optional(),
    public_url: z.string().nullable().optional(),
  })
  .strict()

export const Services = z
  .object({
    codecoder: ServicePortConfig.optional(),
    gateway: ServicePortConfig.optional(),
    channels: ServicePortConfig.optional(),
    workflow: ServicePortConfig.optional(),
    trading: ServicePortConfig.optional(),
  })
  .strict()
```

### 2. 更新配置解析逻辑 (`packages/ccode/src/cli/network.ts`)

修改 `resolveNetworkOptions` 函数，添加统一配置路径的回退支持：

```typescript
// Port: CLI flag > server.port > services.codecoder.port > default
const port = portExplicitlySet
  ? args.port
  : (config?.server?.port ?? config?.services?.codecoder?.port ?? args.port)

// Hostname: CLI flag > server.hostname > network.bind > default
const hostname = hostnameExplicitlySet
  ? args.hostname
  : mdns && !config?.server?.hostname
    ? "0.0.0.0"
    : (config?.server?.hostname ?? config?.network?.bind ?? args.hostname)
```

## 配置解析优先级

修复后的优先级：

**端口解析:**
1. CLI flag `--port`
2. `server.port` (TypeScript 原有路径)
3. `services.codecoder.port` (Rust 统一路径)
4. 默认值 4400

**主机名解析:**
1. CLI flag `--hostname`
2. mDNS 模式自动设置 "0.0.0.0"
3. `server.hostname` (TypeScript 原有路径)
4. `network.bind` (Rust 统一路径)
5. 默认值 "::"

## 验证结果

### TypeScript 类型检查
```
bun turbo typecheck --filter=ccode
✅ 通过
```

### 单元测试
```
bun test config
✅ 86 tests passed
```

### 配置读取验证
```
network.bind: 127.0.0.1
services.codecoder.port: 4400
server.port: (not set)
server.hostname: (not set)

结果: ccode serve 将正确使用 port=4400, hostname=127.0.0.1
```

## 向后兼容性

- 现有的 `server.*` 配置仍然优先
- 新增的 `network.*` 和 `services.*` 作为回退
- 不需要修改现有的 config.json 文件

## 修改的文件

1. `packages/ccode/src/config/config.ts` - 添加 Network/Services schema
2. `packages/ccode/src/cli/network.ts` - 更新解析逻辑

## 相关代码位置

- TypeScript config: `packages/ccode/src/config/config.ts:982-1030`
- TypeScript network: `packages/ccode/src/cli/network.ts:34-57`
- Rust config: `services/zero-common/src/config.rs:46-125`
