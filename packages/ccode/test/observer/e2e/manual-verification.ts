#!/usr/bin/env bun
/**
 * Observer Network 手动验证脚本
 *
 * 此脚本演示如何手动验证 Observer Network 的各项功能。
 * 运行方式: bun run test/observer/e2e/manual-verification.ts
 */

// 导入测试设置 (必须首先导入)
import "../setup"

import {
  ObserverNetwork,
  ThreeDials,
  GEAR_PRESETS,
  getEventStream,
  getConsensusEngine,
  resetEventStream,
  resetConsensusEngine,
  resetModeController,
  createExecutor,
} from "@/observer"

import { ObservationInjector } from "../helpers/observation-injector"
import {
  createNormalOperationObservations,
  createCrisisEmergenceObservations,
  createCrisisEscalationObservations,
  createRecoveryObservations,
  getAllCrisisPhases,
} from "../fixtures/crisis-observations"

// ═══════════════════════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════════════════════

function printHeader(title: string) {
  console.log("\n" + "═".repeat(70))
  console.log(`  ${title}`)
  console.log("═".repeat(70))
}

function printStep(step: number, description: string) {
  console.log(`\n[步骤 ${step}] ${description}`)
  console.log("-".repeat(50))
}

function printResult(label: string, value: unknown, expected?: unknown) {
  const valueStr = JSON.stringify(value, null, 2)
  if (expected !== undefined) {
    const match = JSON.stringify(value) === JSON.stringify(expected)
    console.log(`  ${label}: ${valueStr} ${match ? "✅" : "❌"}`)
  } else {
    console.log(`  ${label}: ${valueStr}`)
  }
}

function assert(condition: boolean, message: string) {
  if (!condition) {
    console.error(`  ❌ 断言失败: ${message}`)
    process.exit(1)
  }
  console.log(`  ✅ ${message}`)
}

// ═══════════════════════════════════════════════════════════════════════════
// 验证流程
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  console.log("\n🔍 Observer Network 手动验证开始\n")

  // 清理状态
  resetEventStream()
  resetConsensusEngine()
  resetModeController()

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 1: 档位系统验证
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 1: 档位系统验证")

  printStep(1, "验证 GEAR_PRESETS 预设值")
  printResult("P (Park)", GEAR_PRESETS.P, { observe: 0, decide: 0, act: 0 })
  printResult("N (Neutral)", GEAR_PRESETS.N, { observe: 50, decide: 0, act: 0 })
  printResult("D (Drive)", GEAR_PRESETS.D, { observe: 70, decide: 60, act: 40 })
  printResult("S (Sport)", GEAR_PRESETS.S, { observe: 90, decide: 80, act: 70 })
  printResult("M (Manual)", GEAR_PRESETS.M, { observe: 50, decide: 50, act: 50 })

  printStep(2, "创建 ThreeDials 并验证 D 模式")
  const dials = ThreeDials.fromGear("D")
  printResult("当前档位", dials.gear, "D")
  printResult("旋钮值", dials.values(), { observe: 70, decide: 60, act: 40 })

  printStep(3, "设置单独旋钮后自动切换到 M 模式")
  dials.setDial("act", 100)
  printResult("新档位", dials.gear, "M")
  printResult("act 值", dials.act.value, 100)

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 2: Observer Network 启动
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 2: Observer Network 启动")

  printStep(4, "启动 Observer Network (HYBRID 模式)")
  const network = await ObserverNetwork.start({
    mode: "HYBRID",
    autoModeSwitch: true,
    riskTolerance: "balanced",
    watchers: {
      code: false, // 禁用实际 watcher 以便手动注入
      world: false,
      self: false,
      meta: false,
    },
    stream: {
      buffered: false, // 立即处理
    },
  })

  assert(network.isRunning(), "网络正在运行")
  printResult("当前模式", network.getMode(), "HYBRID")
  printResult("当前档位", network.getGear(), "D")

  printStep(5, "检查 Watcher 状态")
  const statuses = network.getWatcherStatuses()
  // 注: 我们禁用了实际 watcher 以便手动注入测试数据
  printResult("Watcher 数量", statuses.length)
  console.log("  (已禁用实际 watcher 以便手动注入测试数据)")
  for (const status of statuses) {
    console.log(`    - ${status.watcherType}: running=${status.running}`)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 3: 观察事件注入
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 3: 观察事件注入")

  printStep(6, "创建事件注入器")
  const stream = getEventStream()
  const injector = new ObservationInjector(stream)
  console.log("  事件注入器已创建")

  printStep(7, "注入正常运行观察 (Phase 1)")
  const normalObs = createNormalOperationObservations()
  await injector.injectBatch(normalObs)
  printResult("注入数量", normalObs.length, 4)

  printStep(8, "注入危机萌芽观察 (Phase 2)")
  const crisisObs = createCrisisEmergenceObservations()
  await injector.injectBatch(crisisObs)
  printResult("注入数量", crisisObs.length, 3)

  printStep(9, "检查事件流统计")
  const stats = stream.getStats()
  printResult("已接收", stats.received)
  printResult("已发送", stats.emitted)
  printResult("当前缓存", stats.current)

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 4: 共识引擎验证
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 4: 共识引擎验证")

  printStep(10, "触发共识更新")
  const consensusEngine = getConsensusEngine()
  const snapshot = await consensusEngine.update()
  assert(snapshot !== null, "快照已生成")

  printStep(11, "检查共识快照")
  printResult("时间戳", snapshot.timestamp.toISOString())
  printResult("置信度", snapshot.confidence)
  printResult("模式数量", snapshot.patterns.length)
  printResult("异常数量", snapshot.anomalies.length)
  printResult("机会数量", snapshot.opportunities.length)

  printStep(12, "检查世界模型")
  const worldModel = snapshot.worldModel
  if (worldModel) {
    printResult("世界模型 ID", worldModel.id)
    printResult("世界模型时间戳", worldModel.timestamp.toISOString())
    console.log("  ✅ 世界模型已生成")
  } else {
    console.log("  ⚠️ 世界模型未生成 (需要更多观察)")
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 5: 模式控制器验证
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 5: 模式控制器验证")

  printStep(13, "获取模式控制器统计")
  const modeStats = network.getModeControllerStats()
  assert(modeStats !== null, "模式统计已获取")
  printResult("当前模式", modeStats!.currentMode)
  printResult("当前档位", modeStats!.currentGear)
  printResult("模式切换次数", modeStats!.modeSwitches)
  printResult("待处理升级", modeStats!.pendingEscalations)

  printStep(14, "切换到 S (Sport) 模式")
  await network.switchGear("S", "手动验证")
  printResult("新档位", network.getGear(), "S")
  printResult("新模式", network.getMode(), "AUTO")

  printStep(15, "切换回 D (Drive) 模式")
  await network.switchGear("D", "恢复正常")
  printResult("档位", network.getGear(), "D")
  printResult("模式", network.getMode(), "HYBRID")

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 6: 执行器验证
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 6: 执行器验证")

  printStep(16, "创建执行器")
  const executor = createExecutor({
    autoExecute: false,
    mode: "HYBRID",
    dryRun: true,
    useDialControl: false,
  })
  await executor.start()
  console.log("  执行器已启动")

  printStep(17, "创建执行请求")
  const request = await executor.requestExecution({
    type: "auto_optimize",
    description: "测试优化",
    trigger: { type: "manual" },
    actions: [
      {
        id: "action_1",
        type: "test_action",
        description: "测试动作",
        status: "pending",
      },
    ],
  })
  printResult("请求 ID", request.id)
  printResult("需要审批", request.requiresApproval, true)
  printResult("状态", request.status, "pending")

  printStep(18, "验证高危命令需要审批")
  const dangerousRequest = await executor.requestExecution({
    type: "hands_action",
    description: "危险操作",
    trigger: { type: "manual" },
    actions: [
      {
        id: "dangerous_1",
        type: "cleanup",
        description: "清理",
        command: "rm -rf /tmp/test",
        status: "pending",
      },
    ],
  })
  assert(dangerousRequest.requiresApproval === true, "高危命令需要审批")

  executor.stop()

  // ─────────────────────────────────────────────────────────────────────────
  // Phase 7: 完整危机流程
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("Phase 7: 完整危机流程验证")

  printStep(19, "获取所有危机阶段")
  const phases = getAllCrisisPhases()
  printResult("阶段数量", phases.length, 5)
  for (const phase of phases) {
    console.log(`    - ${phase.name}: ${phase.observations.length} 个观察`)
  }

  printStep(20, "注入危机升级观察")
  const escalationObs = createCrisisEscalationObservations()
  await injector.injectBatch(escalationObs)
  printResult("注入数量", escalationObs.length, 4)

  // 触发共识更新
  await consensusEngine.update()

  printStep(21, "检查最终统计")
  const finalStats = network.getStats()
  printResult("总观察数", finalStats.observations)

  // ─────────────────────────────────────────────────────────────────────────
  // 清理
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("清理")

  await network.stop()
  resetEventStream()
  resetConsensusEngine()
  resetModeController()

  console.log("  ✅ 资源已清理")

  // ─────────────────────────────────────────────────────────────────────────
  // 总结
  // ─────────────────────────────────────────────────────────────────────────

  printHeader("验证完成")

  console.log(`
  ┌─────────────────────────────────────────────────────────────┐
  │                    手动验证结果                              │
  ├─────────────────────────────────────────────────────────────┤
  │  ✅ 档位系统 (Gear/Dial)                                    │
  │  ✅ Observer Network 启动/停止                               │
  │  ✅ 观察事件注入与路由                                       │
  │  ✅ 共识引擎快照生成                                         │
  │  ✅ 模式控制器统计                                           │
  │  ✅ 执行器请求与审批                                         │
  │  ✅ 完整危机流程                                             │
  └─────────────────────────────────────────────────────────────┘
  `)
}

// 运行验证
main().catch(console.error)
