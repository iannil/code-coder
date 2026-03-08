/**
 * Responders Module
 *
 * Exports all responder components for the Observer Network.
 *
 * Responders transform observations into actions:
 * - Notifier: Sends notifications
 * - Analyzer: Triggers deep analysis
 * - Executor: Executes automated actions
 * - Historian: Records history
 *
 * @module observer/responders
 */

export {
  Notifier,
  createNotifier,
  type NotificationPriority,
  type NotificationChannel,
  type Notification,
  type NotificationRule,
  type NotifierConfig,
} from "./notifier"

export {
  Analyzer,
  createAnalyzer,
  type AnalysisType,
  type AnalysisStatus,
  type AnalysisRequest,
  type AnalysisResult,
  type AnalyzerConfig,
} from "./analyzer"

export {
  Executor,
  createExecutor,
  type ExecutionType,
  type ExecutionStatus,
  type ExecutionRequest,
  type ExecutionAction,
  type ExecutionResult,
  type ExecutorConfig,
} from "./executor"

export {
  Historian,
  createHistorian,
  type HistoryEventType,
  type HistoryEntry,
  type HistoryQuery,
  type HistoryStats,
  type HistorianConfig,
} from "./historian"
