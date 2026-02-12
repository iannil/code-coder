import { DocumentSchema } from "./schema"

export interface DocumentTemplate {
  id: string
  name: string
  description: string
  category: "fiction" | "non-fiction" | "technical" | "academic" | "business"
  defaultOutline: {
    title: string
    description?: string
    chapters: DocumentTemplateChapter[]
  }
  styleGuide: DocumentSchema.StyleGuide
}

export interface DocumentTemplateChapter {
  id: string
  title: string
  description: string
  estimatedWords: number
  subsections?: string[]
}

export const TEMPLATES: Record<string, DocumentTemplate> = {
  novel: {
    id: "novel",
    name: "长篇小说",
    description: "适用于长篇小说创作，包含三幕结构",
    category: "fiction",
    defaultOutline: {
      title: "小说标题",
      description: "一部引人入胜的长篇小说",
      chapters: [
        {
          id: "ch1",
          title: "序章/开端",
          description: "介绍背景，设定故事基调，引入主要人物或冲突",
          estimatedWords: 3000,
          subsections: "场景设定,人物介绍,初始事件".split(","),
        },
        {
          id: "ch2",
          title: "发展/上升动作",
          description: "故事发展，人物关系建立，冲突逐步升级",
          estimatedWords: 5000,
          subsections: "情节推进,人物发展,伏笔埋设".split(","),
        },
        {
          id: "ch3",
          title: "转折/中点",
          description: "故事转折点，主人公面临重大选择或危机",
          estimatedWords: 5000,
          subsections: "转折事件,内心冲突,方向转变".split(","),
        },
        {
          id: "ch4",
          title: "高潮前奏",
          description: "为高潮做铺垫，冲突达到顶峰前奏",
          estimatedWords: 5000,
          subsections: "紧张局势,最后准备,命运抉择".split(","),
        },
        {
          id: "ch5",
          title: "高潮",
          description: "全书最高潮，主要矛盾爆发点",
          estimatedWords: 5000,
          subsections: "冲突爆发,真相揭示,决战时刻".split(","),
        },
        {
          id: "ch6",
          title: "结局/尾声",
          description: "故事收尾，人物命运归宿",
          estimatedWords: 4000,
          subsections: "冲突解决,人物成长,主题升华".split(","),
        },
      ],
    },
    styleGuide: {
      tone: "叙事性",
      audience: "普通读者",
      format: "markdown",
      requirements: [
        "保持叙事节奏和张力",
        "人物性格发展连贯",
        "情节推进自然",
        "注意伏笔和呼应",
        "环境描写与情节融合",
      ],
    },
  },

  technical: {
    id: "technical",
    name: "技术书籍",
    description: "适用于编程、技术类书籍",
    category: "technical",
    defaultOutline: {
      title: "技术书籍标题",
      description: "一本深入讲解某项技术的书籍",
      chapters: [
        {
          id: "ch1",
          title: "简介与背景",
          description: "介绍技术背景，为什么要学习这项技术",
          estimatedWords: 4000,
          subsections: "技术概述,应用场景,学习路线".split(","),
        },
        {
          id: "ch2",
          title: "基础入门",
          description: "基础概念和快速上手",
          estimatedWords: 8000,
          subsections: "环境搭建,基础语法,Hello World".split(","),
        },
        {
          id: "ch3",
          title: "核心概念",
          description: "深入讲解核心概念",
          estimatedWords: 10000,
          subsections: "概念详解,原理分析,图解说明".split(","),
        },
        {
          id: "ch4",
          title: "进阶技巧",
          description: "高级用法和最佳实践",
          estimatedWords: 10000,
          subsections: "高级特性,性能优化,设计模式".split(","),
        },
        {
          id: "ch5",
          title: "实战案例",
          description: "完整项目案例",
          estimatedWords: 12000,
          subsections: "项目架构,核心实现,问题解决".split(","),
        },
        {
          id: "ch6",
          title: "部署与运维",
          description: "生产环境部署和运维",
          estimatedWords: 6000,
          subsections: "部署方案,监控告警,故障排查".split(","),
        },
        {
          id: "ch7",
          title: "附录",
          description: "参考资料和工具",
          estimatedWords: 4000,
          subsections: "常用命令,参考资料,问题FAQ".split(","),
        },
      ],
    },
    styleGuide: {
      tone: "专业",
      audience: "开发者",
      format: "markdown",
      requirements: [
        "代码示例完整可运行",
        "概念解释清晰易懂",
        "提供图表辅助理解",
        "注意事项和最佳实践",
        "章节间循序渐进",
      ],
    },
  },

  business: {
    id: "business",
    name: "商业计划书",
    description: "适用于创业项目商业计划",
    category: "business",
    defaultOutline: {
      title: "商业计划书",
      description: "一份完整的商业计划",
      chapters: [
        {
          id: "ch1",
          title: "执行摘要",
          description: "项目概述和核心亮点",
          estimatedWords: 3000,
          subsections: "项目简介,核心价值,融资需求".split(","),
        },
        {
          id: "ch2",
          title: "公司介绍",
          description: "公司背景和团队介绍",
          estimatedWords: 4000,
          subsections: "公司背景,团队介绍,发展历程".split(","),
        },
        {
          id: "ch3",
          title: "市场分析",
          description: "行业分析和竞争格局",
          estimatedWords: 6000,
          subsections: "市场规模,目标用户,竞争分析".split(","),
        },
        {
          id: "ch4",
          title: "产品与服务",
          description: "详细介绍产品或服务",
          estimatedWords: 6000,
          subsections: "产品介绍,核心功能,技术优势".split(","),
        },
        {
          id: "ch5",
          title: "商业模式",
          description: "盈利模式和发展规划",
          estimatedWords: 5000,
          subsections: "盈利模式,营销策略,发展规划".split(","),
        },
        {
          id: "ch6",
          title: "财务规划",
          description: "财务预测和资金使用",
          estimatedWords: 5000,
          subsections: "收入预测,成本分析,资金使用".split(","),
        },
        {
          id: "ch7",
          title: "风险分析",
          description: "风险因素和应对措施",
          estimatedWords: 3000,
          subsections: "市场风险,技术风险,运营风险".split(","),
        },
      ],
    },
    styleGuide: {
      tone: "专业",
      audience: "投资人",
      format: "markdown",
      requirements: [
        "数据支撑论点",
        "逻辑清晰严谨",
        "突出竞争优势",
        "展示投资价值",
        "风险客观评估",
      ],
    },
  },

  course: {
    id: "course",
    name: "在线课程",
    description: "适用于在线教育课程设计",
    category: "non-fiction",
    defaultOutline: {
      title: "课程名称",
      description: "系统性的在线课程",
      chapters: [
        {
          id: "ch1",
          title: "课程介绍",
          description: "课程目标和学习方法",
          estimatedWords: 2000,
          subsections: "课程目标,适用人群,学习方法".split(","),
        },
        {
          id: "ch2",
          title: "模块一：基础知识",
          description: "模块概述和基础概念",
          estimatedWords: 6000,
          subsections: "概念讲解,示例演示,练习题".split(","),
        },
        {
          id: "ch3",
          title: "模块二：核心内容",
          description: "核心知识点深入讲解",
          estimatedWords: 8000,
          subsections: "重点难点,案例分析,实战演练".split(","),
        },
        {
          id: "ch4",
          title: "模块三：应用实践",
          description: "实际应用和项目实战",
          estimatedWords: 8000,
          subsections: "项目实战,问题解决,经验总结".split(","),
        },
        {
          id: "ch5",
          title: "模块四：进阶拓展",
          description: "高级内容和拓展学习",
          estimatedWords: 6000,
          subsections: "高级技巧,拓展阅读,职业发展".split(","),
        },
        {
          id: "ch6",
          title: "课程总结",
          description: "知识回顾和学习建议",
          estimatedWords: 3000,
          subsections: "知识回顾,学习建议,后续学习".split(","),
        },
      ],
    },
    styleGuide: {
      tone: "教学",
      audience: "学习者",
      format: "markdown",
      requirements: [
        "知识点循序渐进",
        "示例通俗易懂",
        "提供练习和实践",
        "重点内容突出",
        "鼓励学习互动",
      ],
    },
  },

  research: {
    id: "research",
    name: "学术论文",
    description: "适用于学术研究报告",
    category: "academic",
    defaultOutline: {
      title: "研究论文标题",
      description: "学术论文或研究报告",
      chapters: [
        {
          id: "ch1",
          title: "摘要与引言",
          description: "研究背景和目的",
          estimatedWords: 3000,
          subsections: "研究背景,研究目的,研究意义".split(","),
        },
        {
          id: "ch2",
          title: "文献综述",
          description: "相关研究回顾",
          estimatedWords: 5000,
          subsections: "国内研究,国外研究,文献评述".split(","),
        },
        {
          id: "ch3",
          title: "研究方法",
          description: "研究设计和方法",
          estimatedWords: 4000,
          subsections: "研究设计,数据来源,分析方法".split(","),
        },
        {
          id: "ch4",
          title: "研究结果",
          description: "数据分析和研究发现",
          estimatedWords: 6000,
          subsections: "数据描述,分析结果,研究发现".split(","),
        },
        {
          id: "ch5",
          title: "讨论与分析",
          description: "结果讨论和理论分析",
          estimatedWords: 5000,
          subsections: "结果讨论,理论分析,实践意义".split(","),
        },
        {
          id: "ch6",
          title: "结论与展望",
          description: "研究总结和未来方向",
          estimatedWords: 3000,
          subsections: "研究结论,研究局限,未来展望".split(","),
        },
      ],
    },
    styleGuide: {
      tone: "学术",
      audience: "研究人员",
      format: "markdown",
      requirements: [
        "遵循学术规范",
        "引用准确规范",
        "论证逻辑严密",
        "数据真实可靠",
        "结论客观审慎",
      ],
    },
  },

  custom: {
    id: "custom",
    name: "自定义模板",
    description: "空白模板，自由创作",
    category: "non-fiction",
    defaultOutline: {
      title: "文档标题",
      description: "文档描述",
      chapters: [],
    },
    styleGuide: {
      format: "markdown",
    },
  },
}

export function listTemplates(): DocumentTemplate[] {
  return Object.values(TEMPLATES)
}

export function getTemplate(id: string): DocumentTemplate | undefined {
  return TEMPLATES[id]
}

export function applyTemplate(
  templateId: string,
  customizations: {
    title?: string
    description?: string
    targetWords?: number
    chapterCount?: number
  },
): {
    outline: DocumentSchema.Outline
    styleGuide: DocumentSchema.StyleGuide
  } {
  const template = getTemplate(templateId)
  if (!template) throw new Error(`Template not found: ${templateId}`)

  const outline: DocumentSchema.Outline = {
    title: customizations.title ?? template.defaultOutline.title,
    description: customizations.description ?? template.defaultOutline.description,
    chapters: template.defaultOutline.chapters.map((ch, i) => ({
      ...ch,
      id: `ch${i + 1}`,
    })),
  }

  // Adjust word counts based on target
  if (customizations.targetWords && customizations.chapterCount) {
    const wordsPerChapter = Math.floor(customizations.targetWords / customizations.chapterCount)
    outline.chapters = outline.chapters.map((ch: DocumentSchema.ChapterOutline) => ({
      ...ch,
      estimatedWords: wordsPerChapter,
    }))
  }

  return {
    outline,
    styleGuide: template.styleGuide,
  }
}
