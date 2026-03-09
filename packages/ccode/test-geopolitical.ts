/**
 * 测试 ResearchLoop 对地缘政治问题的处理能力
 */

import { createResearchLoop, type ResearchProblem } from "./src/autonomous/execution/research-loop"

async function main() {
  console.log("=== CodeCoder 地缘政治分析能力测试 ===\n")

  const researchLoop = createResearchLoop({
    maxSources: 5,
    enableLearning: false,
    enableHandCreation: false,
  })

  const problem: ResearchProblem = {
    sessionId: `test-${Date.now()}`,
    topic: "美以和伊朗战争的未来发展",
    dimensions: ["局势分析", "经济影响", "可能走向"],
    timeRange: "week",
    sourceTypes: ["news"],
    maxSources: 5,
  }

  console.log("研究问题:", problem.topic)
  console.log("分析维度:", problem.dimensions?.join(", "))
  console.log("\n开始研究...\n")

  const startTime = Date.now()

  try {
    const result = await researchLoop.research(problem)

    console.log("\n=== 研究结果 ===")
    console.log("成功:", result.success)
    console.log("来源数:", result.sources.length)
    console.log("洞察数:", result.insights.length)
    console.log("耗时:", result.durationMs, "ms")
    console.log("\n摘要:")
    console.log(result.summary)
    console.log("\n洞察:")
    result.insights.forEach((insight, i) => {
      console.log(`  ${i + 1}. ${insight}`)
    })
    console.log("\n来源:")
    result.sources.forEach((source, i) => {
      console.log(`  ${i + 1}. [${source.credibility}] ${source.title}`)
      console.log(`     ${source.url}`)
    })

    if (result.outputPath) {
      console.log("\n报告保存路径:", result.outputPath)
    }

  } catch (error) {
    console.error("研究失败:", error)
  }

  await researchLoop.cleanup()

  console.log("\n=== 测试完成 ===")
  console.log("总耗时:", Date.now() - startTime, "ms")
}

main().catch(console.error)
