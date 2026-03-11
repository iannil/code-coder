/**
 * Security Module
 * Security policies and access control for CodeCoder
 *
 * @deprecated This module is scheduled for removal.
 * Security policies are now enforced by the Rust daemon.
 *
 * **Migration Guide:**
 * - Remote policy: Use SDK's HttpClient with built-in auth
 * - Prompt injection scanning: Handled by Rust security layer
 * - Permission checks: Enforced at the daemon API level
 *
 * **Rust implementation:** `services/zero-cli/src/security/mod.rs`
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
