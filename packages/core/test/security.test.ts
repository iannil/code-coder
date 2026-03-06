import { describe, expect, it, beforeEach } from 'bun:test'
import {
  PermissionManager,
  Vault,
  createMemoryVault,
  isSecurityNative,
} from '../src/security.js'
import {
  FallbackPermissionManager,
  FallbackVault,
} from '../src/fallback.js'
import type { Permission, PermissionRule, SecretEntry } from '../src/types.js'

describe('Security Module', () => {
  describe('isSecurityNative', () => {
    it('should be a boolean', () => {
      expect(typeof isSecurityNative).toBe('boolean')
    })
  })

  describe('PermissionManager', () => {
    let manager: PermissionManager

    beforeEach(() => {
      manager = new PermissionManager()
    })

    it('should deny by default', () => {
      const permission: Permission = { tool: 'bash', action: 'execute' }
      expect(manager.check(permission)).toBe(false)
    })

    it('should allow after adding allow rule', () => {
      const rule: PermissionRule = {
        permission: { tool: 'file', action: 'read' },
        allow: true,
      }
      manager.addRule(rule)

      expect(manager.check({ tool: 'file', action: 'read' })).toBe(true)
      expect(manager.check({ tool: 'file', action: 'write' })).toBe(false)
    })

    it('should support wildcard patterns', () => {
      manager.addRule({
        permission: { tool: 'file', action: '*' },
        allow: true,
      })

      expect(manager.check({ tool: 'file', action: 'read' })).toBe(true)
      expect(manager.check({ tool: 'file', action: 'write' })).toBe(true)
      expect(manager.check({ tool: 'bash', action: 'execute' })).toBe(false)
    })

    it('should support resource patterns', () => {
      // Allow writes in general first
      manager.addRule({
        permission: { tool: 'file', action: 'write' },
        allow: true,
      })
      // Then deny writes to /etc/* (this rule is checked first due to reverse order)
      manager.addRule({
        permission: { tool: 'file', action: 'write', resource: '/etc/*' },
        allow: false,
      })

      // Last matching rule wins - /etc/* deny matches /etc/passwd
      expect(manager.check({ tool: 'file', action: 'write', resource: '/home/user/file.txt' })).toBe(true)
      expect(manager.check({ tool: 'file', action: 'write', resource: '/etc/passwd' })).toBe(false)
    })

    it('should allow after granting permission', () => {
      const permission: Permission = { tool: 'bash', action: 'execute' }
      expect(manager.check(permission)).toBe(false)

      manager.grant(permission)
      expect(manager.check(permission)).toBe(true)
    })

    it('should clear all rules and grants', () => {
      manager.addRule({
        permission: { tool: 'file', action: 'read' },
        allow: true,
      })
      manager.grant({ tool: 'bash', action: 'execute' })

      expect(manager.check({ tool: 'file', action: 'read' })).toBe(true)
      expect(manager.check({ tool: 'bash', action: 'execute' })).toBe(true)

      manager.clear()

      expect(manager.check({ tool: 'file', action: 'read' })).toBe(false)
      expect(manager.check({ tool: 'bash', action: 'execute' })).toBe(false)
    })

    it('should support deny rules', () => {
      manager.addRule({
        permission: { tool: '*', action: '*' },
        allow: true,
      })
      manager.addRule({
        permission: { tool: 'bash', action: 'execute' },
        allow: false,
        reason: 'Bash execution is dangerous',
      })

      expect(manager.check({ tool: 'file', action: 'read' })).toBe(true)
      expect(manager.check({ tool: 'bash', action: 'execute' })).toBe(false)
    })
  })

  describe('FallbackPermissionManager', () => {
    it('should work the same as PermissionManager', () => {
      const manager = new FallbackPermissionManager()

      expect(manager.check({ tool: 'bash', action: 'execute' })).toBe(false)

      manager.addRule({
        permission: { tool: 'file', action: 'read' },
        allow: true,
      })

      expect(manager.check({ tool: 'file', action: 'read' })).toBe(true)

      manager.clear()
      expect(manager.check({ tool: 'file', action: 'read' })).toBe(false)
    })
  })

  describe('Vault', () => {
    let vault: Vault

    beforeEach(() => {
      vault = createMemoryVault('test-password')
    })

    it('should store and retrieve secrets', () => {
      const entry: SecretEntry = {
        name: 'api_key',
        value: 'sk-test-12345',
        description: 'Test API key',
      }

      vault.set(entry)

      const retrieved = vault.get('api_key')
      expect(retrieved).not.toBeNull()
      expect(retrieved?.name).toBe('api_key')
      expect(retrieved?.value).toBe('sk-test-12345')
      expect(retrieved?.description).toBe('Test API key')
    })

    it('should return null for non-existent secrets', () => {
      expect(vault.get('non-existent')).toBeNull()
    })

    it('should get just the value', () => {
      vault.set({ name: 'secret', value: 'my-value' })

      expect(vault.getValue('secret')).toBe('my-value')
      expect(vault.getValue('non-existent')).toBeNull()
    })

    it('should delete secrets', () => {
      vault.set({ name: 'to-delete', value: 'value' })
      expect(vault.get('to-delete')).not.toBeNull()

      const deleted = vault.delete('to-delete')
      expect(deleted).toBe(true)
      expect(vault.get('to-delete')).toBeNull()
    })

    it('should return false when deleting non-existent secret', () => {
      expect(vault.delete('non-existent')).toBe(false)
    })

    it('should list all secret names', () => {
      vault.set({ name: 'secret1', value: 'value1' })
      vault.set({ name: 'secret2', value: 'value2' })
      vault.set({ name: 'secret3', value: 'value3' })

      const names = vault.list()
      expect(names.length).toBe(3)
      expect(names).toContain('secret1')
      expect(names).toContain('secret2')
      expect(names).toContain('secret3')
    })

    it('should overwrite existing secrets', () => {
      vault.set({ name: 'key', value: 'original' })
      expect(vault.getValue('key')).toBe('original')

      vault.set({ name: 'key', value: 'updated' })
      expect(vault.getValue('key')).toBe('updated')
    })

    it('should support secrets without description', () => {
      vault.set({ name: 'simple', value: 'value' })

      const retrieved = vault.get('simple')
      expect(retrieved?.description).toBeUndefined()
    })
  })

  describe('FallbackVault', () => {
    it('should work as an in-memory store', () => {
      const vault = new FallbackVault('/fake/path.enc', 'password')

      vault.set({ name: 'secret', value: 'value' })
      expect(vault.getValue('secret')).toBe('value')

      vault.delete('secret')
      expect(vault.get('secret')).toBeNull()
    })

    it('should have correct path', () => {
      const vault = new FallbackVault('/custom/path.enc', 'password')
      expect(vault.path).toBe('/custom/path.enc')
    })
  })

  describe('createMemoryVault', () => {
    it('should create an in-memory vault', () => {
      const vault = createMemoryVault('password')

      vault.set({ name: 'test', value: 'value' })
      expect(vault.getValue('test')).toBe('value')

      // Memory vault has :memory: path
      expect(vault.path).toBe(':memory:')
    })
  })
})
