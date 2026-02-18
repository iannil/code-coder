# CLI 命令参考

## 概览

```bash
zero-bot <command> [options]
```

## 命令列表

| 命令 | 说明 |
|------|------|
| `onboard` | 设置向导 |
| `agent` | 与 AI 对话 |
| `gateway` | 启动 HTTP 服务器 |
| `daemon` | 启动守护进程 |
| `status` | 系统状态 |
| `doctor` | 诊断检查 |
| `channel` | 通道管理 |
| `skill` | 技能管理 |
| `service` | 服务管理 |
| `migrate` | 数据迁移 |
| `cron` | 定时任务 |
| `integration` | 集成管理 |

---

## onboard

设置向导，用于首次配置。

```bash
# 快速设置
zero-bot onboard

# 交互式向导
zero-bot onboard --interactive
```

**功能**:
- 创建配置目录
- 检测 API key
- 生成 config.toml
- 加密敏感信息

---

## agent

与 AI 进行对话。

```bash
# 单条消息
zero-bot agent -m "你好"
zero-bot agent --message "你好"

# 交互式聊天
zero-bot agent
```

**选项**:
| 选项 | 说明 |
|------|------|
| `-m, --message <MSG>` | 发送单条消息 |

---

## gateway

启动 HTTP webhook 服务器。

```bash
zero-bot gateway
```

**功能**:
- 接收外部 webhook
- 处理 Telegram/Slack/Discord 回调
- 与 tunnel 配合使用

---

## daemon

启动完整自主运行时。

```bash
zero-bot daemon
```

**功能**:
- 后台运行
- 监听所有通道
- 执行定时任务
- 心跳监控

---

## status

显示系统状态。

```bash
zero-bot status
```

**输出**:
- 配置状态
- Provider 状态
- Memory 状态
- 通道状态

---

## doctor

运行诊断检查。

```bash
zero-bot doctor
```

**检查项目**:
- 配置文件有效性
- API key 可用性
- 依赖程序检测
- 网络连接测试

---

## channel

通道管理命令。

```bash
# 列出通道
zero-bot channel list

# 添加通道
zero-bot channel add telegram '{"token": "..."}'

# 移除通道
zero-bot channel remove my-channel

# 启动通道
zero-bot channel start

# 健康检查
zero-bot channel doctor
```

**子命令**:
| 子命令 | 说明 |
|--------|------|
| `list` | 列出所有配置的通道 |
| `add <type> <config>` | 添加通道 |
| `remove <name>` | 移除通道 |
| `start` | 启动所有通道 |
| `doctor` | 运行通道健康检查 |

---

## skill

技能管理命令。

```bash
# 列出技能
zero-bot skill list

# 安装技能
zero-bot skill install https://example.com/skill.toml
zero-bot skill install ./local-skill/

# 移除技能
zero-bot skill remove my-skill
```

**子命令**:
| 子命令 | 说明 |
|--------|------|
| `list` | 列出已安装技能 |
| `install <source>` | 安装技能 |
| `remove <name>` | 移除技能 |

---

## service

系统服务管理。

```bash
# 安装服务
zero-bot service install

# 启动服务
zero-bot service start

# 停止服务
zero-bot service stop

# 查看状态
zero-bot service status

# 卸载服务
zero-bot service uninstall
```

**子命令**:
| 子命令 | 说明 |
|--------|------|
| `install` | 安装为系统服务 |
| `start` | 启动服务 |
| `stop` | 停止服务 |
| `status` | 查看服务状态 |
| `uninstall` | 卸载服务 |

**支持的平台**:
- macOS: launchd
- Linux: systemd

---

## migrate

数据迁移命令。

```bash
# 从 OpenClaw 迁移
zero-bot migrate openclaw

# 指定源目录
zero-bot migrate openclaw --source ~/.openclaw/workspace

# 预览迁移（不写入）
zero-bot migrate openclaw --dry-run
```

**子命令**:
| 子命令 | 说明 |
|--------|------|
| `openclaw` | 从 OpenClaw 导入数据 |

**选项**:
| 选项 | 说明 |
|------|------|
| `--source <path>` | 源目录路径 |
| `--dry-run` | 仅预览，不执行 |

---

## cron

定时任务管理。

```bash
# 列出任务
zero-bot cron list

# 添加任务
zero-bot cron add "0 * * * *" "echo hello"

# 移除任务
zero-bot cron remove task-id
```

**子命令**:
| 子命令 | 说明 |
|--------|------|
| `list` | 列出所有定时任务 |
| `add <expr> <cmd>` | 添加任务 |
| `remove <id>` | 移除任务 |

---

## integration

外部集成管理。

**子命令**:
| 子命令 | 说明 |
|--------|------|
| `info <name>` | 显示集成详情 |

---

## 全局选项

| 选项 | 说明 |
|------|------|
| `-h, --help` | 显示帮助 |
| `-V, --version` | 显示版本 |

---

## 配置文件

默认配置目录: `~/.codecoder/`

```
~/.codecoder/
├── config.toml     # 主配置
├── .secret_key     # 加密密钥
└── memory/
    └── brain.db    # SQLite 数据库
```

---

## 环境变量

| 变量 | 说明 |
|------|------|
| `ZEROBOT_CONFIG` | 自定义配置文件路径 |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `OPENROUTER_API_KEY` | OpenRouter API key |
| `GEMINI_API_KEY` | Gemini API key |
| ... | ... |
