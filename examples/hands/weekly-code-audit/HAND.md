---
id: "weekly-code-audit"
name: "周代码审计"
version: "1.0.0"
schedule: "0 0 10 * * 1"
agent: "security-reviewer"
enabled: true
memory_path: "hands/code-audit/{date}.md"
autonomy:
  level: "bold"
  unattended: true
  max_iterations: 3
decision:
  use_close: true
  web_search: false
  evolution: false
  auto_continue: true
resources:
  max_tokens: 100000
  max_cost_usd: 5.0
  max_duration_sec: 1800
params:
  directories:
    - "src"
    - "services"
    - "packages"
  exclude_patterns:
    - "**/node_modules/**"
    - "**/dist/**"
    - "**/*.test.ts"
  focus_areas:
    - "authentication"
    - "input_validation"
    - "sql_injection"
    - "xss"
    - "secrets_exposure"
---

# 周代码审计

每周一 10:00 进行自动化安全代码审计，检查潜在的安全漏洞。

## 职责

1. **代码扫描**: 扫描指定目录的代码变更
2. **漏洞检测**: 识别常见安全漏洞
3. **风险评估**: 评估发现问题的严重程度
4. **报告生成**: 生成详细的审计报告

## 检查项

### 认证与授权
- [ ] JWT/Session 处理是否安全
- [ ] 密码存储是否使用强哈希
- [ ] 权限检查是否完整
- [ ] CSRF 保护是否到位

### 输入验证
- [ ] 用户输入是否验证
- [ ] SQL 参数是否参数化
- [ ] XSS 防护是否实施
- [ ] 路径遍历防护

### 敏感信息
- [ ] 是否有硬编码密钥
- [ ] 日志是否泄露敏感数据
- [ ] 错误消息是否过于详细
- [ ] 配置文件是否安全

### 依赖安全
- [ ] 是否有已知漏洞依赖
- [ ] 依赖版本是否最新
- [ ] 锁文件是否完整

## 输出格式

```markdown
# 安全审计报告 - {date}

## 摘要
- 扫描文件数: {count}
- 发现问题: {issues}
- 严重程度分布: {critical}/{high}/{medium}/{low}

## 关键问题

### [CRITICAL] {问题标题}
- **文件**: {file_path}:{line}
- **描述**: {description}
- **建议**: {recommendation}
- **参考**: {reference_link}

## 统计

| 类别 | 数量 | 趋势 |
|------|------|------|
| 认证问题 | ... | ... |
| 注入漏洞 | ... | ... |
| 敏感信息 | ... | ... |

## 建议优先级
1. {最紧急的修复}
2. {次要修复}
3. ...
```

## 自主级别说明

使用 `bold` 级别（谨慎自主），因为：
- 安全审计需要准确性，不能有误报
- 代码修改建议需要人工确认
- 仅执行分析，不进行自动修复

## 与 HITL 集成

当发现 Critical 级别问题时，自动创建审批请求：

```yaml
approval_type:
  type: "risk_operation"
  description: "Critical security issue found"
  risk_level: "Critical"
```

审批后会发送通知到配置的 IM 渠道。

## 使用说明

1. 复制到 `~/.codecoder/hands/weekly-code-audit/`
2. 根据项目结构调整 `params.directories`
3. 配置 `params.focus_areas` 关注特定领域
4. 可选：连接到 Slack/Telegram 接收通知
