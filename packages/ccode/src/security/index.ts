/**
 * Security Module
 * Security policies and access control for CodeCoder
 */

export { RemotePolicy, shouldRequireApproval, isDangerous, isSafe, riskLevel, describeApprovalReason } from "./remote-policy"

export {
  PromptInjectionScanner,
  getScanner,
  createScanner,
  scanForInjection,
  quickCheckInjection,
  sanitizeInput,
  type InjectionType,
  type InjectionSeverity,
  type InjectionPattern,
  type InjectionScanResult,
  type ScannerConfig,
} from "./prompt-injection"
