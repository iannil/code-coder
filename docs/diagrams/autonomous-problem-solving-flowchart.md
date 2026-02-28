# CodeCoder 全自动自主解决问题流程图

## 完整系统流程图

```mermaid
flowchart TB
    subgraph UI["用户接入层"]
        CLI[CLI命令行]
        TUI[Terminal TUI]
        WEB[Web界面]
        IM[IM渠道<br/>Telegram/Discord/Slack]
    end

    subgraph GATEWAY["安全边界层 (Zero Gateway :4430)"]
        AUTH[认证<br/>Pairing + JWT]
        RBAC[授权<br/>RBAC权限]
        QUOTA[配额管理]
        AUDIT[审计日志]
    end

    subgraph CORE["核心引擎层 (ccode/TypeScript)"]
        ORCHESTRATOR[任务编排器]
        AGENT_ENGINE[Agent执行引擎]
        LLM_PROVIDER[AI Provider适配层<br/>Claude/OpenAI/Google/MCP]
        MEMORY[记忆系统<br/>流层 + 沉积层]
    end

    subgraph AGENTS["Agent生态系统 (23个Agent)"]
        direction TB
        subgraph MAIN["主模式"]
            BUILD[build<br/>开发模式]
            PLAN[plan<br/>规划模式]
            AUTO[autonomous<br/>自主执行★]
        end

        subgraph ENGR["工程质量"]
            REVIEW[code-reviewer]
            SECURITY[security-reviewer]
            TDD[tdd-guide]
            ARCH[architect]
            EXPLORE[explore]
        end

        subgraph ZRS["祝融说系列"]
            OBS[observer<br/>观察者理论]
            DEC[decision<br/>CLOSE决策]
            MAC[macro<br/>宏观经济]
            TRD[trader<br/>交易分析]
            PK[picker<br/>选品]
            MP[miniproduct<br/>极小产品]
            AIE[ai-engineer<br/>AI工程]
        end

        subgraph CONTENT["内容创作"]
            WRITER[writer<br/>长文写作]
            PROOF[proofreader<br/>校对]
        end

        subgraph REVERSE["逆向工程"]
            CR[code-reverse<br/>网站逆向]
            JR[jar-code-reverse<br/>JAR逆向]
        end

        subgraph UTIL["工具辅助"]
            GEN[general<br/>多任务]
            SYNT[synton-assistant]
        end
    end

    subgraph WORKFLOW["工作流引擎 (Zero Workflow :4432)"]
        TRIGGER[触发器<br/>Webhook/Cron/Git]
        FLOW[流程编排]
        STEP[步骤执行]
        CONDITION[条件分支]
    end

    subgraph SERVICES["Rust微服务层"]
        DAEMON[Zero Daemon<br/>进程编排器 :4402]
        CHANNELS[Zero Channels<br/>IM适配 :4431]
        TRADING[Zero Trading<br/>交易系统 :4434]
        BROWSER[Zero Browser<br/>浏览器 :4433]
    end

    subgraph INFRA["基础设施层"]
        REDIS[(Redis<br/>会话/缓存)]
        DB[(SQLite<br/>持久化)]
        DOCKER[Docker<br/>容器编排]
    end

    %% 连接关系
    CLI -->|命令| AUTH
    TUI -->|交互| AUTH
    WEB -->|HTTP| AUTH
    IM -->|消息| CHANNELS
    CHANNELS --> AUTH

    AUTH --> RBAC --> QUOTA --> AUDIT --> ORCHESTRATOR

    ORCHESTRATOR --> AGENT_ENGINE
    AGENT_ENGINE --> AUTO
    AUTO -->|委派| BUILD
    AUTO -->|委派| PLAN
    AUTO -->|委派| REVIEW
    AUTO -->|委派| SECURITY
    AUTO -->|委派| TDD
    AUTO -->|委派| EXPLORE
    AUTO -->|委派| DEC
    AUTO -->|委派| WRITER
    AUTO -->|委派| GEN
    AUTO -->|协调| WORKFLOW

    AGENT_ENGINE --> LLM_PROVIDER
    AGENT_ENGINE --> MEMORY

    WORKFLOW --> TRIGGER --> FLOW --> STEP --> CONDITION
    STEP -->|调用| AGENT_ENGINE

    ORCHESTRATOR --> DAEMON
    DAEMON --> GATEWAY
    DAEMON --> CHANNELS
    DAEMON --> WORKFLOW
    DAEMON --> TRADING
    DAEMON --> BROWSER

    AGENT_ENGINE --> REDIS
    AGENT_ENGINE --> DB
    DAEMON --> DOCKER

    style AUTO fill:#ff9,stroke:#333,stroke-width:3px
    style DEC fill:#9f9,stroke:#333,stroke-width:2px
    style ORCHESTRATOR fill:#99f,stroke:#333,stroke-width:2px
```

---

## 自主执行模式核心流程

```mermaid
flowchart TB
    START([用户输入问题]) --> INTENT[意图识别<br/>Agent推荐]

    INTENT --> DECISION{是否需要<br/>CLOSE决策?}

    DECISION -->|是| CLOSE[执行 @decision<br/>CLOSE五维评估]
    DECISION -->|否| PLAN_DIRECT[直接规划]

    CLOSE --> CLOSE_RESULT{{CLOSE评估结果}}
    CLOSE_RESULT -->|Convergence≥7<br/>可持续性高| PLAN_DIRECT
    CLOSE_RESULT -->|Surplus≥7<br/>余量充足| PLAN_DIRECT
    CLOSE_RESULT -->|风险过高| CONFIRM[请求用户确认]

    CONFIRM --> PLAN_DIRECT

    PLAN_DIRECT --> PLAN[执行 @plan<br/>生成实施计划]

    PLAN --> PLAN_OUT{{计划输出}}
    PLAN_OUT --> PLAN_FILE[写入IMPLEMENTATION_PLAN.md]
    PLAN_OUT --> TASKS[创建TodoWrite任务列表]

    PLAN_FILE --> TDD_CHECK{需要TDD?}
    TASKS --> TDD_CHECK

    TDD_CHECK -->|是| TDD[执行 @tdd-guide<br/>测试驱动开发]
    TDD_CHECK -->|否| EXECUTE[直接执行]

    TDD --> RED[写测试 → RED]
    RED --> GREEN[实现 → GREEN]
    GREEN --> REFACTOR[重构 → IMPROVE]

    REFACTOR --> COVERAGE{覆盖率≥80%?}
    COVERAGE -->|否| RED
    COVERAGE -->|是| REVIEW

    EXECUTE --> REVIEW[执行 @code-reviewer<br/>代码审查]

    REVIEW --> REVIEW_OUT{{审查结果}}
    REVIEW_OUT -->|CRITICAL/HIGH| FIX[修复问题]
    FIX --> REVIEW

    REVIEW_OUT -->|通过| SECURITY[执行 @security-reviewer<br/>安全审查]

    SECURITY --> SEC_OUT{{安全结果}}
    SEC_OUT -->|有问题| SEC_FIX[修复安全问题]
    SEC_FIX --> SECURITY

    SEC_OUT -->|通过| VERIFY[执行 @verification<br/>验收测试]

    VERIFY --> VERIFY_OUT{{验收结果}}
    VERIFY_OUT -->|失败| ITERATE[迭代优化]
    ITERATE --> REVIEW

    VERIFY_OUT -->|通过| SOLIDIFY[固化技能<br/>写入SKILL.md]

    SOLIDIFY --> COMPLETE([任务完成])

    style CLOSE fill:#9f9,stroke:#333
    style PLAN fill:#99f,stroke:#333
    style TDD fill:#f99,stroke:#333
    style REVIEW fill:#ff9,stroke:#333
    style SECURITY fill:#fc9,stroke:#333
    style COMPLETE fill:#9f9,stroke:#333,stroke-width:3px
```

---

## CLOSE 决策框架评估流程

```mermaid
flowchart TB
    START([决策场景]) --> C[C - Convergence<br/>收敛程度评估]

    C --> C_CHECK{收敛度≥7?}
    C_CHECK -->|否| C_REFINE[重新定义问题<br/>提高收敛度]
    C_REFINE --> C

    C_CHECK -->|是| L[L - Leverage<br/>杠杆效应评估]

    L --> L_CHECK{杠杆效应≥7?}
    L_CHECK -->|否| L_BOOST[寻找高杠杆点<br/>结构调整]
    L_BOOST --> L

    L_CHECK -->|是| O[O - Optionality<br/>选择权保留评估]

    O --> O_CHECK{选择权≥7?}
    O_CHECK -->|否| O_EXPAND[增加选项<br/>避免路径锁定]
    O_EXPAND --> O

    O_CHECK -->|是| S[S - Surplus<br/>余量消耗评估]

    S --> S_CHECK{余量保留≥7?}
    S_CHECK -->|否| S_RESERVE[保留资源<br/>降低消耗]
    S_RESERVE --> S

    S_CHECK -->|是| E[E - Evolution<br/>演化空间评估]

    E --> E_CHECK{演化空间≥7?}
    E_CHECK -->|否| E_FLEX[增强适应性<br/>模块化设计]
    E_FLEX --> E

    E_CHECK -->|是| CALC[计算综合得分]

    CALC --> SCORE[CLOSE总分]

    SCORE --> DECISION{总分≥35?}
    DECISION -->|是| APPROVE[✓ 批准执行]
    DECISION -->|否| MITIGATE[风险缓解<br/>或请求确认]

    MITIGATE --> USER_CONFIRM{用户确认?}
    USER_CONFIRM -->|是| APPROVE
    USER_CONFIRM -->|否| REJECT[✗ 拒绝/重新规划]

    APPROVE --> ACTION([进入执行阶段])

    style APPROVE fill:#9f9,stroke:#333,stroke-width:3px
    style REJECT fill:#f99,stroke:#333,stroke-width:2px
    style MITIGATE fill:#ff9,stroke:#333,stroke-width:2px
```

---

## Agent 委派与协作流程

```mermaid
flowchart TB
    START([Autonomous Agent启动]) --> ANALYZE[分析任务类型]

    ANALYZE --> TYPE{任务类型判断}

    TYPE -->|代码审查| REVIEW[code-reviewer Agent]
    TYPE -->|安全分析| SECURITY[security-reviewer Agent]
    TYPE -->|新功能/修复| TDD[tdd-guide Agent]
    TYPE -->|架构设计| ARCH[architect Agent]
    TYPE -->|代码库探索| EXPLORE[explore Agent]
    TYPE -->|长文写作| WRITER[writer Agent]
    TYPE -->|决策分析| DECISION[decision Agent]
    TYPE -->|多任务并行| GENERAL[general Agent]
    TYPE -->|逆向工程| REVERSE[code-reverse Agent]

    REVIEW --> R_RESULT[审查报告]
    SECURITY --> S_RESULT[安全报告]
    TDD --> T_RESULT[测试+实现]
    ARCH --> A_RESULT[架构设计]
    EXPLORE --> E_RESULT[代码地图]
    WRITER --> W_RESULT[长文内容]
    DECISION --> D_RESULT[CLOSE评估]
    GENERAL --> G_RESULT[并行结果]
    REVERSE --> REV_RESULT[逆向分析]

    R_RESULT --> AGGREGATE[结果聚合]
    S_RESULT --> AGGREGATE
    T_RESULT --> AGGREGATE
    A_RESULT --> AGGREGATE
    E_RESULT --> AGGREGATE
    W_RESULT --> AGGREGATE
    D_RESULT --> AGGREGATE
    G_RESULT --> AGGREGATE
    REV_RESULT --> AGGREGATE

    AGGREGATE --> SYNTHESIS[综合分析]
    SYNTHESIS --> NEXT{需要更多Agent?}
    NEXT -->|是| ANALYZE
    NEXT -->|否| OUTPUT([输出最终结果])

    style REVIEW fill:#9cf,stroke:#333
    style SECURITY fill:#fc9,stroke:#333
    style TDD fill:#f99,stroke:#333
    style GENERAL fill:#ccc,stroke:#333
    style OUTPUT fill:#9f9,stroke:#333,stroke-width:3px
```

---

## 工作流触发与执行流程

```mermaid
flowchart TB
    START([外部事件]) --> ROUTER{触发类型}

    ROUTER -->|Webhook| WEBHOOK[HTTP Webhook]
    ROUTER -->|定时| CRON[Cron表达式]
    ROUTER -->|Git事件| GIT[Git Push/PR]
    ROUTER -->|手动| MANUAL[用户触发]

    WEBHOOK --> PARSE[事件解析]
    CRON --> PARSE
    GIT --> PARSE
    MANUAL --> PARSE

    PARSE --> VALIDATE{参数验证}
    VALIDATE -->|失败| ERROR([返回错误])
    VALIDATE -->|通过| MATCH[工作流匹配]

    MATCH --> FOUND{找到工作流?}
    FOUND -->|否| DEFAULT[默认处理]
    FOUND -->|是| LOAD[加载工作流定义]

    DEFAULT --> ERROR

    LOAD --> CONTEXT[构建执行上下文]
    CONTEXT --> STEP[执行第一步]

    STEP --> STEP_TYPE{步骤类型}
    STEP_TYPE -->|agent| AGENT_CALL[调用Agent]
    STEP_TYPE -->|notify| NOTIFY[发送通知]
    STEP_TYPE -->|condition| CONDITION[条件判断]
    STEP_TYPE -->|parallel| PARALLEL[并行执行]

    AGENT_CALL --> STEP_RESULT[收集结果]
    NOTIFY --> STEP_RESULT
    CONDITION --> COND_RESULT{条件结果}
    PARALLEL --> PARA_RESULT[并行聚合]

    COND_RESULT -->|真| NEXT_STEP[下一步骤]
    COND_RESULT -->|假| BRANCH[分支路径]
    PARA_RESULT --> STEP_RESULT

    NEXT_STEP --> MORE_STEPS{更多步骤?}
    BRANCH --> MORE_STEPS
    STEP_RESULT --> MORE_STEPS

    MORE_STEPS -->|是| STEP
    MORE_STEPS -->|否| PERSIST[持久化结果]

    PERSIST --> CALLBACK[执行回调]
    CALLBACK --> SUCCESS([执行成功])

    style AGENT_CALL fill:#99f,stroke:#333
    style SUCCESS fill:#9f9,stroke:#333,stroke-width:3px
    style ERROR fill:#f99,stroke:#333,stroke-width:2px
```

---

## 工具调用循环 (Tool Calling Loop)

```mermaid
flowchart TB
    START([Agent启动]) --> PROMPT[构建系统提示词<br/>+ 用户消息]

    PROMPT --> LLM_CALL[调用 LLM API]

    LLM_CALL --> LLM_RESPONSE{{LLM响应}}

    LLM_RESPONSE -->|文本| FINAL([返回最终结果])
    LLM_RESPONSE -->|工具调用| PARSE[解析工具调用]

    PARSE --> TOOLS[工具列表]

    TOOLS --> TOOL_LOOP{遍历工具}

    TOOL_LOOP -->|下一个工具| EXEC[执行工具]

    EXEC --> TOOL_TYPE{工具类型}

    TOOL_TYPE -->|Read/Write| FILE_IO[文件操作]
    TOOL_TYPE -->|Bash| CMD[执行命令]
    TOOL_TYPE -->|Agent| SUB_AGENT[子Agent调用]
    TOOL_TYPE -->|Skill| SKILL[技能调用]
    TOOL_TYPE -->|WebSearch| SEARCH[网络搜索]
    TOOL_TYPE -->|Task| PARALLEL[并行任务]

    FILE_IO --> RESULT[收集结果]
    CMD --> RESULT
    SUB_AGENT --> RESULT
    SKILL --> RESULT
    SEARCH --> RESULT
    PARALLEL --> RESULT

    RESULT --> TOOL_LOOP

    TOOL_LOOP -->|全部完成| APPEND[追加到消息历史]

    APPEND --> ITER_CHECK{迭代次数≥10?}
    ITER_CHECK -->|是| LIMIT[达到限制]
    ITER_CHECK -->|否| LOOP_CHECK{循环检测?}

    LOOP_CHECK -->|检测到循环| LIMIT
    LOOP_CHECK -->|否| LLM_CALL

    LIMIT --> FINAL

    style LLM_CALL fill:#ff9,stroke:#333
    style EXEC fill:#9cf,stroke:#333
    style FINAL fill:#9f9,stroke:#333,stroke-width:2px
    style LIMIT fill:#f99,stroke:#333,stroke-width:2px
```

---

## 记忆系统读写流程

```mermaid
flowchart LR
    subgraph INPUT[输入]
        USER[用户消息]
        TOOL[工具结果]
        DECISION[决策记录]
    end

    subgraph STREAM[流层 - Daily]
        DAILY[./memory/daily/<br/>YYYY-MM-DD.md]
    end

    subgraph SEDIMENT[沉积层 - Long-term]
        MEMORY[./memory/MEMORY.md]
        PREF[## 用户偏好]
        CTX[## 项目上下文]
        KEY[## 关键决策]
        LESSON[## 经验教训]
    end

    subgraph OUTPUT[输出]
        SYSTEM[系统提示构建]
        RESPONSE[响应生成]
    end

    INPUT -->|即时追加| STREAM
    STREAM -->|每日| ANALYZE{有意义?}

    ANALYZE -->|否| IGNORE[保持日志]
    ANALYZE -->|是| MERGE[智能合并]

    MERGE -->|整合| PREF
    MERGE -->|整合| CTX
    MERGE -->|整合| KEY
    MERGE -->|整合| LESSON

    OUTPUT -->|加载长期上下文| MEMORY
    OUTPUT -->|加载近期上下文| DAILY

    MEMORY --> SYSTEM
    DAILY --> SYSTEM

    SYSTEM --> RESPONSE

    style MEMORY fill:#9cf,stroke:#333
    style STREAM fill:#ff9,stroke:#333
    style SYSTEM fill:#9f9,stroke:#333
```

---

## 确定性 vs 不确定性任务分流

```mermaid
flowchart TB
    START([任务入口]) --> CLASSIFY{任务分类}

    CLASSIFY -->|协议解析| RUST[Zero-* Rust服务]
    CLASSIFY -->|安全验证| RUST
    CLASSIFY -->|速率限制| RUST
    CLASSIFY -->|消息路由| RUST
    CLASSIFY -->|定时调度| RUST
    CLASSIFY -->|权限校验| RUST

    CLASSIFY -->|意图理解| TS[ccode TypeScript引擎]
    CLASSIFY -->|上下文推理| TS
    CLASSIFY -->|代码生成| TS
    CLASSIFY -->|决策建议| TS
    CLASSIFY -->|自然语言处理| TS
    CLASSIFY -->|多轮对话| TS

    subgraph RUST_SERVICES[Rust微服务层]
        GATEWAY[Zero Gateway<br/>认证/路由]
        CHANNELS[Zero Channels<br/>IM适配]
        WORKFLOW[Zero Workflow<br/>调度执行]
        TRADING[Zero Trading<br/>交易执行]
    end

    subgraph TS_SERVICES[TypeScript引擎层]
        AGENTS[Agent系统<br/>23个Agent]
        LLM[LLM调用<br/>多提供商]
        MEMORY[记忆系统<br/>双层架构]
    end

    RUST --> RUST_SERVICES
    TS --> TS_SERVICES

    RUST_SERVICES --> R_OUTPUT[确定结果<br/>高可靠]
    TS_SERVICES --> T_OUTPUT[智能结果<br/>高灵活]

    R_OUTPUT --> HYBRID[混合决策层]
    T_OUTPUT --> HYBRID

    HYBRID --> RESULT([最终输出])

    style RUST fill:#f99,stroke:#333
    style TS fill:#99f,stroke:#333
    style HYBRID fill:#9f9,stroke:#333,stroke-width:3px
```

---

## 完整端到端执行流程示例

```mermaid
sequenceDiagram
    participant U as 用户
    participant I as UI接入层
    participant G as Zero Gateway
    participant O as Orchestrator
    participant A as Autonomous Agent
    participant D as Decision Agent
    participant P as Plan Agent
    participant T as TDD Guide
    participant R as Code Reviewer
    participant S as Security Reviewer
    participant L as LLM Provider
    participant M as Memory

    U->>I: 输入问题
    I->>G: 认证请求
    G->>G: JWT验证 + RBAC
    G->>O: 转发请求

    O->>A: 启动自主模式

    A->>D: @decision CLOSE评估
    D->>L: 调用LLM
    L-->>D: 返回评估结果
    D-->>A: CLOSE报告

    A->>P: @plan 生成计划
    P->>L: 调用LLM
    L-->>P: 返回计划
    P-->>A: IMPLEMENTATION_PLAN.md

    A->>T: @tdd-guide TDD开发
    T->>T: RED: 写测试
    T->>T: GREEN: 写实现
    T->>T: IMPROVE: 重构
    T->>T: 检查覆盖率≥80%
    T-->>A: 测试+代码

    A->>R: @code-reviewer 审查
    R->>L: 调用LLM
    L-->>R: 审查结果
    R-->>A: 审查报告

    A->>S: @security-reviewer 安全检查
    S->>L: 调用LLM
    L-->>S: 安全报告
    S-->>A: 安全评估

    A->>M: 写入记忆
    M-->>A: 确认

    A->>O: 返回结果
    O->>G: 转发结果
    G->>I: 响应
    I-->>U: 显示结果
```

---

## 关键设计原则

### 1. 双层架构
```
┌─────────────────────────────────────────┐
│   TypeScript (ccode) - 智能层          │
│   • Agent编排 • LLM调用 • 记忆管理      │
├─────────────────────────────────────────┤
│   Rust (zero-*) - 边界层                │
│   • 安全认证 • 消息路由 • 工作流调度    │
└─────────────────────────────────────────┘
```

### 2. 自主执行七步循环
```
觉醒 → 扩张 → 创造 → 固化 → 验证 → 演化 → 觉醒
  ↑                                                    ↓
  └──────────────────────────────────────────────────┘
```

### 3. CLOSE 五维评估
| 维度 | 说明 | 目标 |
|------|------|------|
| C - Convergence | 收敛程度 | ≥7 |
| L - Leverage | 杠杆效应 | ≥7 |
| O - Optionality | 选择权保留 | ≥7 |
| S - Surplus | 余量消耗 | ≥7 |
| E - Evolution | 演化空间 | ≥7 |

### 4. 可持续性优先
- 保持"再来一次"的能力 > 追求"最优解"
- 回滚机制、Plan Mode、Undo 能力
- 人在回路 (HITL) 关键决策确认

### 5. 面向大模型可改写
- 一致命名、显式类型、声明式配置
- 小文件原则 (200-400行典型)
- 透明记忆 (Markdown + Git)
