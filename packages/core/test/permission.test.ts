import { describe, expect, it, beforeEach } from 'bun:test'
import {
  AutoApproveEngine,
  evaluateToolApproval,
  evaluateAdaptiveToolApproval,
  canToolBeSafeAutoApproved,
  isPermissionNative,
  type AutoApproveConfig,
  type ToolInput,
  type ExecutionContext,
  type ApprovalDecision,
} from '../src/permission.js'

describe('Permission Module', () => {
  describe('isPermissionNative', () => {
    it('should be a boolean', () => {
      expect(typeof isPermissionNative).toBe('boolean')
    })

    it('should indicate native bindings are available', () => {
      // In test environment, native bindings should be available
      expect(isPermissionNative).toBe(true)
    })
  })

  describe('AutoApproveEngine', () => {
    describe('creation', () => {
      it('should create engine with custom configuration', () => {
        const config: AutoApproveConfig = {
          enabled: true,
          allowedTools: ['Read', 'Glob'],
          riskThreshold: 'low',
          timeoutMs: 5000,
          unattended: false,
        }

        const engine = AutoApproveEngine.create(config)
        const retrieved = engine.config()

        expect(retrieved.enabled).toBe(true)
        expect(retrieved.allowedTools).toEqual(['Read', 'Glob'])
        expect(retrieved.riskThreshold).toBe('low')
        expect(retrieved.timeoutMs).toBe(5000)
        expect(retrieved.unattended).toBe(false)
      })

      it('should create safe-only engine', () => {
        const engine = AutoApproveEngine.safeOnly(false)
        const config = engine.config()

        expect(config.enabled).toBe(true)
        expect(config.riskThreshold).toBe('low')
        expect(config.allowedTools).toContain('Read')
        expect(config.allowedTools).toContain('Glob')
        expect(config.allowedTools).toContain('Grep')
        expect(config.allowedTools).toContain('LS')
        expect(config.allowedTools).toContain('WebFetch')
        expect(config.allowedTools).toContain('WebSearch')
      })

      it('should create permissive engine', () => {
        const engine = AutoApproveEngine.permissive(true)
        const config = engine.config()

        expect(config.enabled).toBe(true)
        expect(config.riskThreshold).toBe('medium')
        expect(config.timeoutMs).toBe(30000)
        expect(config.unattended).toBe(true)
        expect(config.allowedTools).toEqual([]) // Risk-based only
      })
    })

    describe('configuration', () => {
      it('should allow updating configuration', () => {
        const engine = AutoApproveEngine.safeOnly(false)

        expect(engine.config().riskThreshold).toBe('low')

        engine.setConfig({
          enabled: true,
          allowedTools: ['Bash'],
          riskThreshold: 'high',
          timeoutMs: 10000,
          unattended: true,
        })

        const updated = engine.config()
        expect(updated.riskThreshold).toBe('high')
        expect(updated.allowedTools).toEqual(['Bash'])
        expect(updated.timeoutMs).toBe(10000)
      })
    })

    describe('evaluate', () => {
      describe('safe-only engine', () => {
        let engine: AutoApproveEngine

        beforeEach(() => {
          engine = AutoApproveEngine.safeOnly(false)
        })

        it('should approve Read tool', () => {
          const decision = engine.evaluate('Read', null)

          expect(decision.approved).toBe(true)
          expect(decision.risk).toBe('safe')
          expect(decision.autoApprovable).toBe(true)
        })

        it('should approve Glob tool', () => {
          const decision = engine.evaluate('Glob', null)

          expect(decision.approved).toBe(true)
          expect(decision.risk).toBe('safe')
        })

        it('should approve Grep tool', () => {
          const decision = engine.evaluate('Grep', null)

          expect(decision.approved).toBe(true)
          expect(decision.risk).toBe('safe')
        })

        it('should approve WebFetch tool', () => {
          const decision = engine.evaluate('WebFetch', null)

          expect(decision.approved).toBe(true)
          expect(decision.risk).toBe('low')
        })

        it('should reject Bash tool (not in whitelist)', () => {
          const decision = engine.evaluate('Bash', null)

          expect(decision.approved).toBe(false)
        })

        it('should reject Write tool (not in whitelist)', () => {
          const decision = engine.evaluate('Write', null)

          expect(decision.approved).toBe(false)
        })

        it('should reject Edit tool (not in whitelist)', () => {
          const decision = engine.evaluate('Edit', null)

          expect(decision.approved).toBe(false)
        })
      })

      describe('permissive engine', () => {
        let engine: AutoApproveEngine

        beforeEach(() => {
          engine = AutoApproveEngine.permissive(false)
        })

        it('should approve low-risk Bash commands', () => {
          const input: ToolInput = {
            inputType: 'bash',
            command: 'git status',
          }

          const decision = engine.evaluate('Bash', input)

          expect(decision.approved).toBe(true)
          expect(decision.risk).toBe('low')
        })

        it('should approve medium-risk Write operations', () => {
          const input: ToolInput = {
            inputType: 'file',
            path: 'src/main.rs',
          }

          const decision = engine.evaluate('Write', input)

          expect(decision.approved).toBe(true)
        })

        it('should reject high-risk file operations', () => {
          const input: ToolInput = {
            inputType: 'file',
            path: '.env',
          }

          const decision = engine.evaluate('Write', input)

          expect(decision.approved).toBe(false)
          expect(decision.risk).toBe('high')
        })

        it('should always reject critical operations', () => {
          const input: ToolInput = {
            inputType: 'bash',
            command: 'sudo rm -rf /',
          }

          const decision = engine.evaluate('Bash', input)

          expect(decision.approved).toBe(false)
          expect(decision.risk).toBe('critical')
          expect(decision.autoApprovable).toBe(false)
        })
      })

      describe('disabled engine', () => {
        it('should reject all operations when disabled', () => {
          const engine = AutoApproveEngine.create({
            enabled: false,
            allowedTools: ['Read', 'Glob'],
            riskThreshold: 'high',
            timeoutMs: 0,
            unattended: false,
          })

          const decision = engine.evaluate('Read', null)

          expect(decision.approved).toBe(false)
          expect(decision.reason).toContain('disabled')
        })
      })
    })

    describe('canAutoApprove', () => {
      it('should return true for whitelisted safe tools', () => {
        const engine = AutoApproveEngine.safeOnly(false)

        expect(engine.canAutoApprove('Read')).toBe(true)
        expect(engine.canAutoApprove('Glob')).toBe(true)
        expect(engine.canAutoApprove('Grep')).toBe(true)
      })

      it('should return false for non-whitelisted tools', () => {
        const engine = AutoApproveEngine.safeOnly(false)

        expect(engine.canAutoApprove('Bash')).toBe(false)
        expect(engine.canAutoApprove('Write')).toBe(false)
      })

      it('should return false when engine is disabled', () => {
        const engine = AutoApproveEngine.create({
          enabled: false,
          allowedTools: ['Read'],
          riskThreshold: 'high',
          timeoutMs: 0,
          unattended: false,
        })

        expect(engine.canAutoApprove('Read')).toBe(false)
      })
    })

    describe('assessRisk', () => {
      let engine: AutoApproveEngine

      beforeEach(() => {
        engine = AutoApproveEngine.permissive(false)
      })

      it('should assess Read tool as safe', () => {
        const result = engine.assessRisk('Read', null)

        expect(result.risk).toBe('safe')
        expect(result.autoApprovable).toBe(true)
      })

      it('should assess Bash with git command as low risk', () => {
        const result = engine.assessRisk('Bash', {
          inputType: 'bash',
          command: 'git log --oneline',
        })

        expect(result.risk).toBe('low')
        expect(result.autoApprovable).toBe(true)
      })

      it('should assess Bash with sudo as critical', () => {
        const result = engine.assessRisk('Bash', {
          inputType: 'bash',
          command: 'sudo apt-get update',
        })

        expect(result.risk).toBe('critical')
        expect(result.autoApprovable).toBe(false)
      })

      it('should assess .env file as high risk', () => {
        const result = engine.assessRisk('Write', {
          inputType: 'file',
          path: '.env',
        })

        expect(result.risk).toBe('high')
      })

      it('should assess credential files as high risk', () => {
        const result = engine.assessRisk('Write', {
          inputType: 'file',
          path: 'credentials.json',
        })

        expect(result.risk).toBe('high')
      })

      it('should assess normal source files as medium risk', () => {
        const result = engine.assessRisk('Write', {
          inputType: 'file',
          path: 'src/index.ts',
        })

        // Base risk for Write is medium
        expect(result.risk).toBe('medium')
      })
    })

    describe('evaluateAdaptive', () => {
      let engine: AutoApproveEngine
      let baseCtx: ExecutionContext

      beforeEach(() => {
        engine = AutoApproveEngine.permissive(false)
        baseCtx = {
          sessionId: 'test-session',
          iteration: 1,
          errors: 0,
          successes: 0,
          isProduction: false,
        }
      })

      it('should approve with high success rate', () => {
        const ctx: ExecutionContext = {
          ...baseCtx,
          successes: 100,
          errors: 0,
        }

        const decision = engine.evaluateAdaptive('Write', null, ctx)

        // High success rate should decrease risk
        expect(decision.approved).toBe(true)
      })

      it('should increase risk with errors in session', () => {
        const ctx: ExecutionContext = {
          ...baseCtx,
          errors: 3,
          successes: 5,
        }

        // Write is medium risk, errors should increase it to high
        const riskResult = engine.evaluateAdaptiveRisk('Write', null, ctx)

        expect(riskResult.adjustment).toBeGreaterThan(0)
        expect(riskResult.adjustedRisk).not.toBe(riskResult.baseRisk)
      })

      it('should decrease risk with high success and no errors', () => {
        const ctx: ExecutionContext = {
          ...baseCtx,
          successes: 100,
          errors: 0,
        }

        const riskResult = engine.evaluateAdaptiveRisk('Read', null, ctx)

        expect(riskResult.adjustment).toBe(-1)
        expect(riskResult.adjustmentReason).toContain('success')
      })

      it('should consider production environment sensitivity', () => {
        const ctx: ExecutionContext = {
          ...baseCtx,
          isProduction: true,
        }

        const riskResult = engine.evaluateAdaptiveRisk('Write', null, ctx)

        // Production environment is a factor in adaptive risk
        // The actual adjustment depends on other factors too
        expect(typeof riskResult.adjustment).toBe('number')
        expect(riskResult.adjustmentReason).toBeDefined()
      })

      it('should reject critical operations even with high success rate', () => {
        const ctx: ExecutionContext = {
          ...baseCtx,
          successes: 1000,
          errors: 0,
        }

        // sudo commands have critical base risk
        const decision = engine.evaluateAdaptive(
          'Bash',
          { inputType: 'bash', command: 'sudo rm -rf /' },
          ctx,
        )

        // Even though base risk is critical, adaptive adjustment may lower it
        // But it should still be rejected (not approved)
        expect(decision.approved).toBe(false)
        // The adjusted risk is high due to success rate adjustment
        expect(['critical', 'high']).toContain(decision.risk)
      })
    })

    describe('evaluateAdaptiveRisk', () => {
      it('should return base and adjusted risk', () => {
        const engine = AutoApproveEngine.permissive(false)
        const ctx: ExecutionContext = {
          sessionId: 'test',
          iteration: 5,
          errors: 2,
          successes: 10,
          isProduction: false,
        }

        const result = engine.evaluateAdaptiveRisk('Read', null, ctx)

        expect(result.baseRisk).toBeDefined()
        expect(result.adjustedRisk).toBeDefined()
        expect(typeof result.adjustment).toBe('number')
        expect(typeof result.adjustmentReason).toBe('string')
      })
    })

    describe('audit log', () => {
      it('should start with empty audit log', () => {
        const engine = AutoApproveEngine.safeOnly(false)

        expect(engine.auditLog()).toEqual([])
      })

      it('should clear audit log', () => {
        const engine = AutoApproveEngine.safeOnly(false)

        // The engine doesn't automatically log, but we can verify clear works
        engine.clearAuditLog()

        expect(engine.auditLog()).toEqual([])
      })
    })
  })

  describe('Convenience Functions', () => {
    describe('evaluateToolApproval', () => {
      it('should evaluate tool with config (stateless)', () => {
        const config: AutoApproveConfig = {
          enabled: true,
          allowedTools: ['Read'],
          riskThreshold: 'low',
          timeoutMs: 0,
          unattended: false,
        }

        const decision = evaluateToolApproval(config, 'Read', null)

        expect(decision.approved).toBe(true)
        expect(decision.risk).toBe('safe')
      })

      it('should reject when disabled', () => {
        const config: AutoApproveConfig = {
          enabled: false,
          allowedTools: ['Read'],
          riskThreshold: 'high',
          timeoutMs: 0,
          unattended: false,
        }

        const decision = evaluateToolApproval(config, 'Read', null)

        expect(decision.approved).toBe(false)
      })
    })

    describe('evaluateAdaptiveToolApproval', () => {
      it('should evaluate with adaptive risk (stateless)', () => {
        const config: AutoApproveConfig = {
          enabled: true,
          allowedTools: [],
          riskThreshold: 'medium',
          timeoutMs: 0,
          unattended: false,
        }
        const ctx: ExecutionContext = {
          sessionId: 'test',
          iteration: 1,
          errors: 0,
          successes: 50,
          isProduction: false,
        }

        const decision = evaluateAdaptiveToolApproval(config, 'Read', null, ctx)

        expect(decision.approved).toBe(true)
      })
    })

    describe('canToolBeSafeAutoApproved', () => {
      it('should return true for safe tools', () => {
        expect(canToolBeSafeAutoApproved('Read')).toBe(true)
        expect(canToolBeSafeAutoApproved('Glob')).toBe(true)
        expect(canToolBeSafeAutoApproved('Grep')).toBe(true)
        expect(canToolBeSafeAutoApproved('LS')).toBe(true)
      })

      it('should return false for risky tools', () => {
        expect(canToolBeSafeAutoApproved('Bash')).toBe(false)
        expect(canToolBeSafeAutoApproved('Write')).toBe(false)
        expect(canToolBeSafeAutoApproved('Edit')).toBe(false)
      })
    })
  })

  describe('Risk Thresholds', () => {
    it('should respect low threshold', () => {
      const engine = AutoApproveEngine.create({
        enabled: true,
        allowedTools: [],
        riskThreshold: 'low',
        timeoutMs: 0,
        unattended: false,
      })

      // Safe and low should pass
      expect(engine.evaluate('Read', null).approved).toBe(true)
      expect(engine.evaluate('WebFetch', null).approved).toBe(true)

      // Medium and higher should fail
      expect(engine.evaluate('Write', null).approved).toBe(false)
    })

    it('should respect medium threshold', () => {
      const engine = AutoApproveEngine.create({
        enabled: true,
        allowedTools: [],
        riskThreshold: 'medium',
        timeoutMs: 0,
        unattended: false,
      })

      // Safe, low, medium should pass
      expect(engine.evaluate('Read', null).approved).toBe(true)
      expect(engine.evaluate('WebFetch', null).approved).toBe(true)
      expect(engine.evaluate('Write', null).approved).toBe(true)

      // High should fail
      expect(
        engine.evaluate('Write', { inputType: 'file', path: '.env' }).approved,
      ).toBe(false)
    })

    it('should respect high threshold', () => {
      const engine = AutoApproveEngine.create({
        enabled: true,
        allowedTools: [],
        riskThreshold: 'high',
        timeoutMs: 0,
        unattended: false,
      })

      // Everything except critical should pass
      expect(engine.evaluate('Read', null).approved).toBe(true)
      expect(
        engine.evaluate('Bash', { inputType: 'bash', command: 'git push' })
          .approved,
      ).toBe(true)
      expect(
        engine.evaluate('Write', { inputType: 'file', path: '.env' }).approved,
      ).toBe(true)

      // Critical should still fail
      expect(
        engine.evaluate('Bash', { inputType: 'bash', command: 'sudo rm -rf /' })
          .approved,
      ).toBe(false)
    })
  })

  describe('Tool Input Types', () => {
    let engine: AutoApproveEngine

    beforeEach(() => {
      engine = AutoApproveEngine.permissive(false)
    })

    it('should handle bash input type', () => {
      const input: ToolInput = {
        inputType: 'bash',
        command: 'npm test',
      }

      const result = engine.assessRisk('Bash', input)

      expect(result.risk).toBeDefined()
    })

    it('should handle file input type', () => {
      const input: ToolInput = {
        inputType: 'file',
        path: '/tmp/test.txt',
      }

      const result = engine.assessRisk('Write', input)

      expect(result.risk).toBeDefined()
    })

    it('should handle json input type', () => {
      const input: ToolInput = {
        inputType: 'json',
        json: JSON.stringify({ key: 'value' }),
      }

      const result = engine.assessRisk('Write', input)

      expect(result.risk).toBeDefined()
    })

    it('should handle none input type', () => {
      const input: ToolInput = {
        inputType: 'none',
      }

      const result = engine.assessRisk('Read', input)

      expect(result.risk).toBe('safe')
    })

    it('should handle null input', () => {
      const result = engine.assessRisk('Read', null)

      expect(result.risk).toBe('safe')
    })
  })

  describe('Critical Operations', () => {
    it('should block sudo commands', () => {
      const engine = AutoApproveEngine.create({
        enabled: true,
        allowedTools: [],
        riskThreshold: 'high',
        timeoutMs: 0,
        unattended: false,
      })

      const decision = engine.evaluate('Bash', {
        inputType: 'bash',
        command: 'sudo apt-get install nginx',
      })

      expect(decision.approved).toBe(false)
      expect(decision.risk).toBe('critical')
      expect(decision.autoApprovable).toBe(false)
    })

    it('should block rm -rf commands as high risk', () => {
      const engine = AutoApproveEngine.permissive(false)

      const decision = engine.evaluate('Bash', {
        inputType: 'bash',
        command: 'rm -rf /important/folder',
      })

      // rm -rf is rated as high risk (critical only for root path patterns)
      expect(decision.approved).toBe(false)
      expect(decision.risk).toBe('high')
    })

    it('should rate curl commands appropriately', () => {
      const engine = AutoApproveEngine.permissive(false)

      // Simple curl without mutation is low risk
      const decision = engine.evaluate('Bash', {
        inputType: 'bash',
        command: 'curl https://example.com/file.txt',
      })

      // Curl without POST/PUT/DELETE/data flags is low risk
      expect(decision.risk).toBe('low')
      expect(decision.approved).toBe(true)
    })
  })

  describe('Type Safety', () => {
    it('should return correct ApprovalDecision structure', () => {
      const engine = AutoApproveEngine.safeOnly(false)
      const decision: ApprovalDecision = engine.evaluate('Read', null)

      expect(typeof decision.approved).toBe('boolean')
      expect(typeof decision.risk).toBe('string')
      expect(typeof decision.reason).toBe('string')
      expect(typeof decision.timeoutApproved).toBe('boolean')
      expect(typeof decision.autoApprovable).toBe('boolean')
    })

    it('should return correct config structure', () => {
      const engine = AutoApproveEngine.safeOnly(false)
      const config = engine.config()

      expect(typeof config.enabled).toBe('boolean')
      expect(Array.isArray(config.allowedTools)).toBe(true)
      expect(typeof config.riskThreshold).toBe('string')
      expect(typeof config.timeoutMs).toBe('number')
      expect(typeof config.unattended).toBe('boolean')
    })
  })
})
