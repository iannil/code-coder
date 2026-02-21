# Zero Services 部署指南

本文档介绍如何部署 Zero Services (zero-gateway, zero-channels, zero-workflow)。

## 目录

1. [前置要求](#1-前置要求)
2. [快速开始](#2-快速开始)
3. [配置](#3-配置)
4. [单服务部署](#4-单服务部署)
5. [Docker 部署](#5-docker-部署)
6. [生产环境](#6-生产环境)
7. [故障排除](#7-故障排除)

---

## 1. 前置要求

### 1.1 系统要求

- **操作系统**: Linux (推荐), macOS, Windows (WSL2)
- **内存**: 最低 512MB，推荐 2GB+
- **磁盘**: 500MB+ 可用空间

### 1.2 依赖

**开发环境:**
- Rust 1.75+ (`rustup`)
- Bun 1.3+ (TypeScript 服务)

**生产环境:**
- Docker 24+ (可选)
- SQLite 或 PostgreSQL (配额存储)

### 1.3 安装 Rust

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 验证安装
rustc --version
cargo --version
```

---

## 2. 快速开始

### 2.1 克隆并构建

```bash
# 克隆仓库
git clone https://github.com/your-org/code-coder.git
cd code-coder

# 构建 Rust 服务
cd services
cargo build --workspace --release

# 二进制文件位于:
# - target/release/zero-gateway
# - target/release/zero-channels
# - target/release/zero-workflow
```

### 2.2 创建配置

```bash
# 创建配置目录
mkdir -p ~/.codecoder

# 创建配置文件
cat > ~/.codecoder/config.json << 'EOF'
{
  "gateway": {
    "port": 4402,
    "jwt_secret": "your-secure-jwt-secret-here",
    "token_expiry_hours": 24
  },
  "channels": {
    "port": 4404,
    "telegram": {
      "enabled": false
    }
  },
  "workflow": {
    "webhook": {
      "port": 4405
    },
    "cron": {
      "enabled": true,
      "tasks": []
    }
  },
  "codecoder": {
    "endpoint": "http://localhost:4400"
  },
  "logging": {
    "level": "info",
    "format": "pretty"
  }
}
EOF
```

### 2.3 启动服务

```bash
# 启动 CodeCoder (TypeScript)
cd packages/ccode && bun dev serve &

# 启动 Rust 服务
./target/release/zero-gateway &
./target/release/zero-channels &
./target/release/zero-workflow &
```

### 2.4 验证

```bash
# 健康检查
curl http://localhost:4402/health  # Gateway
curl http://localhost:4404/health  # Channels
curl http://localhost:4405/health  # Workflow

# 测试登录
curl -X POST http://localhost:4402/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}'
```

---

## 3. 配置

### 3.1 配置文件位置

配置文件默认位于 `~/.codecoder/config.json`。

也可通过环境变量指定:
```bash
export CODECODER_CONFIG_PATH=/path/to/config.json
```

### 3.2 完整配置示例

```json
{
  "gateway": {
    "port": 4402,
    "host": "127.0.0.1",
    "jwt_secret": "your-secure-jwt-secret-minimum-32-chars",
    "token_expiry_hours": 24,
    "cors_origins": ["http://localhost:4401"],
    "rate_limit": {
      "requests_per_minute": 100
    }
  },
  "channels": {
    "port": 4404,
    "host": "127.0.0.1",
    "telegram": {
      "enabled": true,
      "bot_token": "123456789:ABCdefGHIjklMNOpqrSTUvwxYZ"
    },
    "discord": {
      "enabled": false,
      "bot_token": ""
    },
    "slack": {
      "enabled": false,
      "bot_token": "",
      "signing_secret": ""
    },
    "feishu": {
      "enabled": false,
      "app_id": "",
      "app_secret": "",
      "verification_token": ""
    }
  },
  "workflow": {
    "webhook": {
      "port": 4405,
      "host": "127.0.0.1",
      "secret": "webhook-signing-secret"
    },
    "cron": {
      "enabled": true,
      "tasks": [
        {
          "id": "daily-backup",
          "expression": "0 0 2 * * *",
          "command": "/opt/scripts/backup.sh",
          "description": "Daily backup at 2 AM"
        }
      ]
    },
    "git": {
      "github_secret": "github-webhook-secret",
      "gitlab_token": "gitlab-webhook-token"
    }
  },
  "codecoder": {
    "endpoint": "http://localhost:4400",
    "timeout_secs": 300
  },
  "logging": {
    "level": "info",
    "format": "json"
  },
  "memory": {
    "backend": "sqlite",
    "sqlite_path": "~/.codecoder/memory.db",
    "postgres_url": ""
  }
}
```

### 3.3 环境变量

可以通过环境变量覆盖配置:

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `CODECODER_CONFIG_PATH` | 配置文件路径 | `~/.codecoder/config.json` |
| `GATEWAY_PORT` | 网关端口 | 4402 |
| `CHANNELS_PORT` | 渠道端口 | 4404 |
| `WORKFLOW_PORT` | 工作流端口 | 4405 |
| `JWT_SECRET` | JWT 密钥 | - |
| `LOG_LEVEL` | 日志级别 | info |
| `LOG_FORMAT` | 日志格式 | pretty |

---

## 4. 单服务部署

### 4.1 Gateway (网关)

```bash
# 启动
./zero-gateway

# 或指定端口
GATEWAY_PORT=8080 ./zero-gateway
```

**功能:**
- JWT 认证/授权
- 用户管理
- 配额管理
- 代理到 CodeCoder

**依赖:**
- CodeCoder API (http://localhost:4400)

### 4.2 Channels (渠道)

```bash
# 启动
./zero-channels

# 或指定端口
CHANNELS_PORT=8081 ./zero-channels
```

**功能:**
- Telegram/Discord/Slack 等 Webhook
- 消息格式转换
- STT/TTS 集成

**依赖:**
- Gateway (可选，用于认证)
- CodeCoder API

### 4.3 Workflow (工作流)

```bash
# 启动
./zero-workflow

# 或指定端口
WORKFLOW_PORT=8082 ./zero-workflow
```

**功能:**
- Cron 调度
- GitHub/GitLab Webhook
- 工作流编排

**依赖:**
- CodeCoder API

---

## 5. Docker 部署

### 5.1 构建镜像

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY services/ ./services/

RUN cd services && cargo build --workspace --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/services/target/release/zero-gateway /usr/local/bin/
COPY --from=builder /app/services/target/release/zero-channels /usr/local/bin/
COPY --from=builder /app/services/target/release/zero-workflow /usr/local/bin/

EXPOSE 4402 4404 4405

CMD ["zero-gateway"]
```

```bash
# 构建
docker build -t zero-services .
```

### 5.2 Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  codecoder:
    build:
      context: .
      dockerfile: packages/ccode/Dockerfile
    ports:
      - "4400:4400"
    volumes:
      - codecoder-data:/root/.codecoder

  gateway:
    image: zero-services
    command: zero-gateway
    ports:
      - "4402:4402"
    environment:
      - CODECODER_ENDPOINT=http://codecoder:4400
      - JWT_SECRET=${JWT_SECRET}
    volumes:
      - ./config.json:/root/.codecoder/config.json:ro
    depends_on:
      - codecoder

  channels:
    image: zero-services
    command: zero-channels
    ports:
      - "4404:4404"
    environment:
      - CODECODER_ENDPOINT=http://codecoder:4400
    volumes:
      - ./config.json:/root/.codecoder/config.json:ro
    depends_on:
      - codecoder

  workflow:
    image: zero-services
    command: zero-workflow
    ports:
      - "4405:4405"
    environment:
      - CODECODER_ENDPOINT=http://codecoder:4400
    volumes:
      - ./config.json:/root/.codecoder/config.json:ro
    depends_on:
      - codecoder

volumes:
  codecoder-data:
```

```bash
# 启动
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止
docker-compose down
```

---

## 6. 生产环境

### 6.1 安全检查清单

- [ ] 更改默认 JWT Secret (至少 32 字符)
- [ ] 更改默认管理员密码
- [ ] 配置 HTTPS (使用反向代理)
- [ ] 限制 CORS 来源
- [ ] 启用速率限制
- [ ] 配置防火墙规则
- [ ] 定期轮换 Webhook Secret

### 6.2 反向代理 (Nginx)

```nginx
# /etc/nginx/sites-available/zero-services
upstream gateway {
    server 127.0.0.1:4402;
}

upstream channels {
    server 127.0.0.1:4404;
}

upstream workflow {
    server 127.0.0.1:4405;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    # Gateway
    location /api/ {
        proxy_pass http://gateway;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Channels Webhook
    location /webhook/ {
        proxy_pass http://channels;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # Workflow Webhook
    location /workflow/ {
        proxy_pass http://workflow;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 6.3 Systemd 服务

```ini
# /etc/systemd/system/zero-gateway.service
[Unit]
Description=Zero Gateway Service
After=network.target

[Service]
Type=simple
User=zero
Group=zero
ExecStart=/opt/zero/bin/zero-gateway
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=CODECODER_CONFIG_PATH=/etc/zero/config.json

[Install]
WantedBy=multi-user.target
```

```bash
# 启用并启动
sudo systemctl enable zero-gateway
sudo systemctl start zero-gateway

# 查看状态
sudo systemctl status zero-gateway

# 查看日志
sudo journalctl -u zero-gateway -f
```

### 6.4 日志管理

生产环境推荐使用 JSON 格式日志:

```json
{
  "logging": {
    "level": "info",
    "format": "json"
  }
}
```

配合日志收集工具 (Loki, Elasticsearch):

```bash
# 使用 vector 收集日志
[sources.zero_services]
type = "journald"
include_units = ["zero-gateway", "zero-channels", "zero-workflow"]

[sinks.loki]
type = "loki"
inputs = ["zero_services"]
endpoint = "http://loki:3100"
```

### 6.5 监控

健康检查端点:

```bash
# 用于负载均衡器健康检查
GET /health   # 返回 {"status": "healthy"}
GET /ready    # 返回 {"status": "ready"}
```

Prometheus 指标 (规划中):

```
zero_gateway_requests_total
zero_gateway_request_duration_seconds
zero_channels_messages_processed
zero_workflow_executions_total
```

---

## 7. 故障排除

### 7.1 常见问题

**Q: 服务无法启动**
```bash
# 检查配置文件
cat ~/.codecoder/config.json | jq .

# 检查端口占用
lsof -i :4402
```

**Q: JWT 认证失败**
```bash
# 验证 JWT Secret 配置
# 确保 jwt_secret 至少 32 字符
```

**Q: Webhook 签名验证失败**
```bash
# 确认 Secret 配置正确
# GitHub: X-Hub-Signature-256 使用 sha256=<hex>
# GitLab: X-Gitlab-Token 使用明文 token
```

**Q: 无法连接 CodeCoder**
```bash
# 检查 CodeCoder 是否运行
curl http://localhost:4400/health

# 检查配置中的 endpoint
```

### 7.2 调试模式

```bash
# 启用调试日志
LOG_LEVEL=debug ./zero-gateway

# 或在配置中:
{
  "logging": {
    "level": "debug"
  }
}
```

### 7.3 获取帮助

- GitHub Issues: https://github.com/your-org/code-coder/issues
- 文档: https://docs.example.com

---

*文档版本: 1.0*
*更新日期: 2026-02-21*
