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

## 深度推进计划2.0

原有的待办事项（To-Do List）混合了**“项目管理流程”**、**“底层核心技术”**和**“极客探索型功能”**，如果直接按列表平铺执行，很容易导致项目失控或在某个技术难点（如App逆向）上卡死。

为了确保这套 S 级架构能够顺利落地并获得资源支持，我为你重新梳理了一份**【敏捷迭代与分期交付项目计划】**。

整个计划按照 **“立项审批 -> 核心基座 -> Web深度自愈 -> 移动与极客拓展”** 分为 4 个里程碑（Phase），从易到难，逐步验证价值。

### Phase 0: 商业闭环与项目立项 (商务/管理阶段)

**目标：** 明确边界，算清投入产出，完成内部 OA 流程，拿到开发资源（时间、服务器、API预算）。
**预计耗时：** 1-2 周

* [x] **架构设计 (Architecture V2)**：在现有架构上补充“逆向引擎”、“移动端Agent”和“资产调度器”。
* [ ] **需求分析 (PRD)**：
  * 圈定 Phase 1-3 具体要接管哪些内部系统、逆向哪些外部网站、控制什么特定的 App。
  * 梳理合规风险（尤其是白嫖额度和逆向爬取的红线边界）。
* [ ] **成本核算与报价 (Budgeting)**：
  * 硬件成本：高配沙箱服务器、移动端测试机/云手机。
  * 软件成本：大模型 Token 预估开销。
  * 人力成本：开发总工时估算。
* [ ] **编写商业落地方案 (Proposal)**：将技术架构包装为面向业务/老板的《降本增效解决方案PPT》。
* [ ] **OA 会议预定 (Kick-off Meeting)**：预约立项评审会，拉通技术、产品、业务负责人。
* [ ] **OA 流程申请 (Approval)**：发起项目立项审批，正式获批预算与排期。

---

### Phase 1: 核心基座与“对话即执行” (MVP 最小可行性产品)

**目标：** 把 ZeroBot 中枢调度层跑通，实现基础的 LLM 对话、意图路由，以及简单的工具调用。
**预计耗时：** 3-4 周

* [ ] **基础设施搭建**：
  * 部署 ZeroBot 网关（Golang/Rust），打通 飞书/钉钉 Webhook。
  * 搭建全局记忆层（Milvus 向量库 + Neo4j 图数据库）。
* [ ] **中枢调度开发**：
  * 实现 LLM Router（根据 Prompt 路由到不同模型）。
  * 实现基础的安全数据脱敏（DLP Filter）。
* [ ] **标准工具集闭环**：
  * 开发简单的 API 工具（如天气、基础数据库查询），验证【AI -> Event Bus -> 执行 -> 返回】的全链路。

---

### Phase 2: Web 深度解析与记忆演化 (硬核攻坚 - 网页端)

**目标：** 攻克你在待办中提到的“自动接口记忆”、“前端JS分析”等痛点，让系统具备**举一反三**的能力，不再每次重新推理。
**预计耗时：** 4-6 周

* [ ] **动态接口拦截与嗅探 (Web Reverse Engine)**：
  * 集成 Playwright 拦截浏览器请求，自动提取未公开的 API 路径、Header 和 Token。
* [ ] **前端 JS 逆向与 AST 解析**：
  * 开发 JS 解析器，识别前端的数据加密/计算逻辑（如各类 Sign 签名算法），提取为 Python/Node 工具函数。
* [ ] **认知与获取方式缓存 (Memory Cache)**：
  * 建立 `Schema 记忆库`。当 AI 成功跑通一次数据抓取（网页或接口），自动将其请求体结构和解析规则（XPath/JSONPath）作为“经验”存入缓存。
  * **自纠错机制**：如果下次使用缓存报错（网页改版），触发 Sandbox 重新抓包分析。
* [ ] **自动提交与流转引擎**：基于记忆，自动拼接参数并提交给目标接口。

---

### Phase 3: 移动端接管与“羊毛党”资产自治 (终极极客形态)

**目标：** 跨越屏幕边界控制 App，并实现系统运行成本的“极致压缩”（白嫖与自动找Key）。
**预计耗时：** 4-6 周

* [ ] **App 视觉与物理控制 (Mobile Agent)**：
  * 集成 Appium 或 ADB 底层驱动。
  * 接入视觉大模型 (VLM)，实现屏幕 UI 元素的坐标识别与点击。
  * 封装原子操作：实现 **自动打开APP、自动注册、自动登录、抓取信息、自动回复、表单提交**。
* [ ] **“薅羊毛/找Key” 自动化 (Hustler Agent)**：
  * 开发针对开发者平台（如 Notion, GitHub 等）的自动化注册流程。
  * 实现自动接收验证邮件、自动在网页生成并提取 API Key。
* [ ] **动态配额与资产管理 (Quota Manager)**：
  * 建立本地 `Secret Vault (密钥保险箱)`。
  * 开发监控器：轮询各类免费额度账户，一旦 A 账号额度耗尽，自动切换到系统刚刚注册的 B 账号。

---

### 💡 给你的执行建议

1. **先过 Phase 0 再写代码**：待办里的黑客技术（逆向、白嫖）非常费时间。如果没有立项和老板的支持，很容易变成个人自嗨但无法落地的烂尾项目。
2. **Phase 2 和 3 的防坑指南**：
   * **JS 逆向**是不可能 100% 自动化的（遇到混淆极度变态的 Webpack/WASM 还是得人工兜底）。你的架构里必须留出 **“人工介入 (Human-in-the-loop)”** 接口：当系统尝试 3 次依然逆向失败时，自动在飞书发消息：“大佬，这个 JS 签名太变态了，求手工介入提取逻辑”。
   * **App 控制**建议优先使用“视觉方案 (VLM + 截图点选)”，比传统的“扒 UI 树结构 (Uiautomator)”更鲁棒，因为 App 更新频繁，UI 树容易变，但视觉表现通常不变。

按照这个计划，你可以直接把 **Phase 0** 作为你这周的任务，先拉会、写方案、过审批！
