---
id: "daily-digest"
name: "Daily Digest"
version: "1.0.0"
schedule: "0 0 8 * * *"
agent: "writer"
enabled: true
memory_path: "hands/daily-digest/{date}.md"
params:
  sources:
    - "hackernews"
    - "github-trending"
    - "arxiv-cs"
  topics:
    - "AI"
    - "Rust"
    - "distributed-systems"
  max_items: 10
autonomy:
  level: "wild"
  unattended: true
  max_iterations: 2
decision:
  use_close: true
  web_search: true
  evolution: false
  auto_continue: true
resources:
  max_tokens: 30000
  max_cost_usd: 1.0
  max_duration_sec: 180
---

# Daily Digest

每日技术新闻摘要，聚合多个来源的热门内容。

## 数据来源

- **Hacker News**: Top 10 stories
- **GitHub Trending**: Today's top repos
- **arXiv CS**: Recent papers in AI/ML

## 过滤规则

- 只保留与配置主题相关的内容
- 去重跨平台的相同文章
- 优先中文或有中文翻译的内容

## 输出格式

```markdown
# Daily Digest - {date}

## 🔥 热门头条
...

## 💻 GitHub 趋势
...

## 📚 学术论文
...

## 📝 每日一思
...
```
