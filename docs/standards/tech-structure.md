# TECH-STRUCTURE

这套架构的核心目标是实现你提到的“自主研究、编程保底、全域协同”。它不仅要有极其精巧的控制流（Control Flow），还要有强大的沙箱容错与自我进化机制（Self-Healing & Evolution）。

以下是我为你设计的系统架构全景视图：

---

## 🏗️ 智擎工作舱 (Omni-Nexus AI) 总体架构图

整个系统被划分为 “五层一环”。

```mermaid
graph TD
    %% Touchpoint Layer
    subgraph 1. 触点层 Touchpoints
        UI_IM[企业 IM: 飞书/钉钉/Slack]
        UI_Web[ZeroBot Web 门户]
        UI_IDE[CodeCoder IDE 插件/终端]
    end

    %% Routing & Orchestration Layer (ZeroBot Core)
    subgraph 2. 中枢调度层 ZeroBot (轻量网关 <10MB)
        ZB_Gateway[统一 API 网关 / 路由]
        ZB_Sec[安全沙箱 & 数据脱敏]
        ZB_Event[事件总线 Event Bus]
        ZB_Cron[后台定时任务引擎]
    end

    %% Deep Execution Layer (CodeCoder Core)
    subgraph 3. 深度执行层 CodeCoder (重装引擎)
        CC_LSP[LSP 语法树解析器]
        CC_MultiAgent[多智能体编排编排: @macro, @trader等]
        CC_Context[本地上下文汇聚器]
    end

    %% Autonomous Fallback Layer
    subgraph 4. 自主保底与兜底层 (The Fallback Engine)
        FB_Search[主动全网检索引擎]
        FB_Sandbox[Docker/WASM 动态代码沙箱]
        FB_REPL[REPL 闭环自纠错执行器]
    end

    %% Global Memory Layer
    subgraph 5. 全局记忆与进化层 Global Memory
        DB_Vector[(向量数据库 Milvus: 上下文/记忆)]
        DB_Graph[(图数据库 Neo4j: 因果链/代码依赖)]
        DB_Tool[(动态工具库 Tool Registry)]
    end

    %% Connections
    UI_IM <--> ZB_Gateway
    UI_Web <--> ZB_Gateway
    UI_IDE <--> CC_Context

    ZB_Gateway <--> ZB_Sec
    ZB_Sec <--> ZB_Event
    ZB_Cron --> ZB_Event

    ZB_Event <--> CC_MultiAgent
    CC_MultiAgent <--> CC_LSP
    CC_MultiAgent <--> FB_REPL

    FB_REPL <--> FB_Search
    FB_REPL <--> FB_Sandbox
    FB_Sandbox -- "代码报错/重试" --> FB_REPL

    ZB_Gateway <--> DB_Vector
    CC_Context <--> DB_Vector
    CC_LSP <--> DB_Graph
    FB_REPL -- "沉淀成功脚本为新工具" --> DB_Tool
    CC_MultiAgent <--> DB_Tool

    classDef zero fill:#e1f5fe,stroke:#03a9f4,stroke-width:2px;
    classDef code fill:#e8f5e9,stroke:#4caf50,stroke-width:2px;
    classDef fallback fill:#fff3e0,stroke:#ff9800,stroke-width:2px;
    class ZB_Gateway,ZB_Sec,ZB_Event,ZB_Cron zero;
    class CC_LSP,CC_MultiAgent,CC_Context code;
    class FB_Search,FB_Sandbox,FB_REPL fallback;
```

---

## 💻 核心架构层级拆解

### 第一层：触点层 (Touchpoints) - “千人千面”

* ZeroBot 端：监听 Webhook（飞书/钉钉），提供极简的交互。
* CodeCoder 端：嵌在开发者的 VS Code 或 JetBrains 中，独占 Terminal 面板，提供沉浸式的多模态交互（键盘、语音、代码高亮）。

### 第二层：中枢调度层 (ZeroBot Core) - “轻量、路由、安全”

ZeroBot 必须使用 Golang 或 Rust 编写，编译为单一二进制文件，确保极低的内存占用（如运行在树莓派或廉价云服务器上）。

* 统一 API 网关 (LLM Router)：内部员工不管用什么模型，全公司只有一个入口。ZeroBot 根据任务（写代码用 Claude 3.5 Sonnet，日常对话用 GPT-4o mini，机密数据用本地 Llama 3）动态路由，控制成本。
* 数据脱敏器 (DLP Filter)：在 Prompt 发往云端前，利用正则或本地小模型，将敏感 IP、密码、身份证号替换为 `[MASKED_IP]`，返回时再还原。
* 事件总线 (Event Bus)：ZeroBot 和 CodeCoder 之间通过轻量级消息队列（如 NATS 或 Redis Pub/Sub）通信，实现异步唤醒。

### 第三层：深度执行层 (CodeCoder Core) - “重度理解与领域 Agent”

CodeCoder 运行在员工本地电脑或高配构建服务器上。

* AST/LSP 引擎：不仅是文本补全，而是实时维护当前代码库的抽象语法树（AST）和引用图谱（Call Graph）。
* 多智能体注册中心：承载 `@macro`（宏观）、`@decision`（决策）、`@dev`（开发）等领域 Agent，支持 Agent 之间的对话和任务拆解。

### 第四层：自主保底与兜底层 (Fallback Engine) - 🔥 系统的灵魂

这是实现“主动使用线上资源和通过编程保底，确保解决问题”的核心引擎。它是一个 REPL (Read-Eval-Print Loop) 自治循环：

1. 主动研究 (Web Searcher)：集成 Tavily 或 Playwright。当 Agent 遇到盲区（如“如何调用上周刚更新的飞书 API”），自动生成搜索 query 去抓取最新文档，喂给模型。
2. 动态编程保底 (Code as a Fallback)：如果现成的 API 走不通，Agent 会自动生成一段 Python/Node.js 脚本来暴力解决问题（比如通过爬虫抓取数据、用 Pandas 清洗极度脏乱的 Excel 表格）。
3. 安全隔离执行 (Docker/WASM Sandbox)：生成的兜底代码绝不会直接在宿主机运行，而是抛入无网络/受限网络的轻量级容器中执行，防止恶意指令（如 `rm -rf`）。
4. 报错捕获与重试 (Error Recovery)：如果脚本执行报 `TypeError`，系统自动将 `Stderr`（错误日志）抓回，让模型反思（Reasoning）并修改代码，无限循环直到执行成功 (Exit Code 0)。

### 第五层：全局记忆与进化层 (Global Memory) - “系统越用越聪明”

* 向量数据库 (Milvus/Qdrant)：存储非结构化记忆。例如产品经理的 PRD、上一次系统崩溃的排查过程记录。
* 动态工具库 (Tool Registry)：在第四层中，当 AI 通过“编程保底”成功解决了一个难题，这段成功的代码脚本会被抽象化、参数化，作为一种新的“工具 (Tool)”注册到库中。下次遇到同类问题，系统直接调用该工具，完成自主进化。

---

## ⚙️ 架构的绝妙之处：一次“高难度任务”的数据流运转

假设发生这样一个极具挑战的场景：
> 运营在飞书 `@ZeroBot`：“帮我把这份刚收到的 1000 页乱码 PDF 财务报表，提取出所有亏损子公司的名字，并对比我们代码库里现有系统对这些公司的风控评级。”

在这个架构下，系统将发生如下奇妙的化学反应：

1. ZeroBot 接收与路由：飞书触发 ZeroBot。ZeroBot 判断此任务包含“文档处理”和“代码库核对”两个复杂步骤，于是将其派发到事件总线，呼叫 CodeCoder 引擎。
2. 多智能体拆解：CodeCoder 主控节点将任务拆分给 `@general` (负责解析 PDF) 和 `@dev` (负责查代码风控逻辑)。
3. 触发编程保底机制：
    * `@general` 发现自带的 PDF 解析器无法处理这份乱码文档。
    * 触发主动研究：它上网搜索，发现针对这种特殊格式需要用 `pdfplumber` 库配合特定参数。
    * 触发编程保底：它立刻写了一段 Python 脚本，并在 Docker 沙箱中安装依赖并运行。
    * 自纠错：第一次运行内存溢出（OOM），它捕获错误，修改代码加入分块读取（Chunking）逻辑，第二次成功提取出公司名单。
4. 跨组件协同：名单通过内部总线传给 `@dev`。`@dev` 利用 LSP 引擎，在本地代码库精准定位到 `RiskAssessmentService` 类，查出了这些公司的风控评级。
5. 进化与回复：
    * 那段处理乱码 PDF 的完美脚本，被系统打上标签，自动存入 DB_Tool (动态工具库)。
    * ZeroBot 将最终对比表格渲染为 Markdown，发回飞书。

## 🛡️ 企业级架构考量 (Enterprise-Grade Considerations)

* 高可用与灾备：ZeroBot 采用无状态 (Stateless) 设计，可以轻易在 K8s 中横向扩容（HPA）应对早高峰的打卡和报表生成请求。
* 权限控制 (RBAC)：ZeroBot 网关层对接企业的 LDAP / 飞书组织架构。实习生无法呼叫调用昂贵 O1 模型的 `@decision` 代理，只能使用本地开源模型，从底层卡死越权和超支。

## 总结

这套架构设计的核心哲学是：“广度交给 ZeroBot，深度交给 CodeCoder，变数交给 Fallback 引擎，记忆交给全局数据库。”

它不再是一个简单的“对话框”，而是一个具备感知、推理、试错、执行和记忆能力的企业级数字生命体。这种设计不仅完美承载了你的所有产品构想，在技术实现上也具备极高的工程可行性和护城河。
