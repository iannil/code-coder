# Skills 系统技术文档

## 概述

CodeCoder Skills 是一个可扩展的技能系统，允许用户通过三种方式获取和创建自定义技能：

1. 从市场搜索现成的 Skills
2. 从 GitHub 项目学习并生成 Skills
3. 从专家方法论创建新 Skills

## 核心组件

### 基础层：Skills 三件套

#### github-to-skills

**功能**: 将 GitHub 项目转换为 CodeCoder Skill

**适用项目类型**:

- 命令行工具 (CLI)
- Python 库
- 有 API 的服务

**不适用项目类型**:

- 纯 GUI 应用
- 需要复杂环境的项目
- 文档不全的项目

**输出结构**:

```
{project-name}/
├── SKILL.md          # 技能说明文档
│   ├── github_url    # 项目地址
│   ├── github_hash   # 版本哈希
│   └── version       # 版本号
└── scripts/
    └── wrapper.{ext} # 调用脚本
```

#### skill-manager

**功能**: 管理 Skills 列表

**常用命令**:

- 列出所有 Skills
- 删除 Skills
- 查看 Skill 详情

#### skill-evolution-manager

**功能**: 记录 Skill 使用经验

**数据存储**: 外挂 `evolution.json` 文件

**记录内容**:

- 参数使用经验
- 最佳实践
- 常见问题

## 进阶层：Skill-from-Masters

### 路径 1: search-skill (搜索现成 Skills)

**信任层级**:

| Tier | 描述                     | 示例                          |
| ---- | ------------------------ | ----------------------------- |
| 1    | 官方/高信任              | anthropics/skills, ComposioHQ |
| 2    | 社区精选                | travisvn, skills.sh           |
| 3    | 聚合站 (严格过滤)        | skillsmp.com                  |

**过滤条件**:

- Stars 数量阈值
- 最后更新时间
- 标准格式要求

### 路径 2: skill-from-github (学习 GitHub 项目)

**与 github-to-skills 的差异**:

| 特性           | github-to-skills      | skill-from-github      |
| -------------- | --------------------- | ---------------------- |
| 实现方式       | Wrapper (封装调用)     | Implementation (理解实现) |
| 适用类型       | 工具、库              | 算法、数据结构         |
| 知识深度       | 调用接口              | 学习核心原理           |

**工作流程**:

1. 搜索相关开源项目
2. 阅读 README 和源代码
3. 提取核心算法/逻辑
4. 编码为 Skill

**示例**: ASCII 艺术生成

- 学习像素亮度到字符的映射逻辑
- 学习宽高比处理
- 学习颜色转换

### 路径 3: 主 Skill (从专家方法论创建)

**工作流程**:

1. 三层搜索
   - 本地方法论数据库
   - 网络搜索专家和框架
   - 一手资源 (论文、书籍、博客)

2. 找黄金案例
   - 优秀输出示例
   - 定义"做好"的标准

3. 识别反模式
   - 搜索常见错误
   - 标注避坑指南

4. 交叉验证
   - 比较多个专家观点
   - 找出共识和分歧

**覆盖领域**:

- 产品管理
- 写作
- 销售
- 招聘
- 用户研究
- 工程
- 领导力
- 谈判
- 创业
- 决策制定

## 使用指南

### 命令映射表

| 操作           | 触发短语示例                          |
| -------------- | ------------------------------------- |
| 列出 Skills    | "列出所有 Skills"                     |
| 添加项目       | "把这个项目打包成 Skill: <url>"       |
| 检查更新       | "检查所有 Skills 的更新"              |
| 经验管理       | "Skill 经验管理"                      |

### 最佳实践

1. **按需安装**: 只安装当前需要的 Skills
2. **定期更新**: 检查并更新 Skills
3. **记录经验**: 使用 evolution.json 记录使用心得
4. **质量优先**: 优先选择高信任源的 Skills

## 技术架构

### Skill 元数据

```typescript
interface SkillMetadata {
  name: string
  type: 'Standard' | 'GitHub'
  description: string
  version: string
  github_url?: string
  github_hash?: string
}
```

### 经验数据结构

```typescript
interface EvolutionData {
  skillName: string
  experiences: {
    parameter: string
    recommendation: string
    timestamp: string
  }[]
}
```

## 参考资源

- [Claude Code Skills 官方文档](https://docs.anthropic.com)
- [ComposioHQ Skills 库](https://github.com/ComposioHQ)
- [skills.sh 目录](https://skills.sh)
