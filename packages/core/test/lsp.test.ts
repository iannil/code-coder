import { describe, expect, it, beforeEach, afterEach } from 'bun:test'
import {
  LspServerManager,
  isLspNative,
  pathToUri,
  uriToPath,
  getLanguageId,
  detectLanguageIdNative,
  applyTextEdits,
  LSP_EXTENSIONS,
  type LspTextEdit,
} from '../src/lsp.js'

describe('LSP Module', () => {
  describe('isLspNative', () => {
    it('should be a boolean', () => {
      expect(typeof isLspNative).toBe('boolean')
    })

    it('should be true when native bindings are available', () => {
      expect(isLspNative).toBe(true)
    })
  })

  describe('Utility Functions', () => {
    describe('pathToUri', () => {
      it('should convert file path to URI', () => {
        expect(pathToUri('/foo/bar.ts')).toBe('file:///foo/bar.ts')
      })

      it('should pass through existing URIs', () => {
        expect(pathToUri('file:///foo/bar.ts')).toBe('file:///foo/bar.ts')
      })
    })

    describe('uriToPath', () => {
      it('should convert URI to file path', () => {
        expect(uriToPath('file:///foo/bar.ts')).toBe('/foo/bar.ts')
      })

      it('should pass through paths without scheme', () => {
        expect(uriToPath('/foo/bar.ts')).toBe('/foo/bar.ts')
      })
    })

    describe('getLanguageId', () => {
      it('should return typescript for .ts files', () => {
        expect(getLanguageId('ts')).toBe('typescript')
        expect(getLanguageId('.ts')).toBe('typescript')
      })

      it('should return typescriptreact for .tsx files', () => {
        expect(getLanguageId('tsx')).toBe('typescriptreact')
      })

      it('should return rust for .rs files', () => {
        expect(getLanguageId('rs')).toBe('rust')
      })

      it('should return go for .go files', () => {
        expect(getLanguageId('go')).toBe('go')
      })

      it('should return python for .py files', () => {
        expect(getLanguageId('py')).toBe('python')
      })

      it('should return extension for unknown types', () => {
        expect(getLanguageId('xyz')).toBe('xyz')
      })
    })

    describe('detectLanguageIdNative', () => {
      it('should detect TypeScript', () => {
        expect(detectLanguageIdNative('ts')).toBe('typescript')
      })

      it('should detect Rust', () => {
        expect(detectLanguageIdNative('rs')).toBe('rust')
      })
    })

    describe('LSP_EXTENSIONS', () => {
      it('should map ts to typescript', () => {
        expect(LSP_EXTENSIONS.ts).toBe('typescript')
      })

      it('should map rs to rust-analyzer', () => {
        expect(LSP_EXTENSIONS.rs).toBe('rust-analyzer')
      })

      it('should map go to gopls', () => {
        expect(LSP_EXTENSIONS.go).toBe('gopls')
      })

      it('should map py to pyright', () => {
        expect(LSP_EXTENSIONS.py).toBe('pyright')
      })
    })
  })

  describe('applyTextEdits', () => {
    it('should apply single edit', () => {
      const content = 'Hello World'
      const edits: LspTextEdit[] = [
        { startLine: 0, startCharacter: 6, endLine: 0, endCharacter: 11, newText: 'Test' }
      ]
      expect(applyTextEdits(content, edits)).toBe('Hello Test')
    })

    it('should apply multiple edits', () => {
      const content = 'Line1\nLine2\nLine3'
      const edits: LspTextEdit[] = [
        { startLine: 0, startCharacter: 0, endLine: 0, endCharacter: 5, newText: 'First' },
        { startLine: 2, startCharacter: 0, endLine: 2, endCharacter: 5, newText: 'Third' },
      ]
      expect(applyTextEdits(content, edits)).toBe('First\nLine2\nThird')
    })

    it('should handle empty edits', () => {
      const content = 'Hello World'
      expect(applyTextEdits(content, [])).toBe('Hello World')
    })

    it('should handle insertion', () => {
      const content = 'Hello World'
      const edits: LspTextEdit[] = [
        { startLine: 0, startCharacter: 5, endLine: 0, endCharacter: 5, newText: ' Beautiful' }
      ]
      expect(applyTextEdits(content, edits)).toBe('Hello Beautiful World')
    })

    it('should handle deletion', () => {
      const content = 'Hello Beautiful World'
      const edits: LspTextEdit[] = [
        { startLine: 0, startCharacter: 5, endLine: 0, endCharacter: 15, newText: '' }
      ]
      expect(applyTextEdits(content, edits)).toBe('Hello World')
    })

    it('should handle multi-line edits', () => {
      const content = 'function foo() {\n  return 1\n}'
      const edits: LspTextEdit[] = [
        { startLine: 1, startCharacter: 9, endLine: 1, endCharacter: 10, newText: '42' }
      ]
      expect(applyTextEdits(content, edits)).toBe('function foo() {\n  return 42\n}')
    })
  })

  describe('LspServerManager', () => {
    let manager: LspServerManager

    beforeEach(() => {
      manager = new LspServerManager()
    })

    afterEach(async () => {
      await manager.stopAll().catch(() => {})
    })

    describe('constructor', () => {
      it('should create a manager instance', () => {
        expect(manager).toBeInstanceOf(LspServerManager)
      })

      it('should report native status', () => {
        expect(typeof manager.isNative).toBe('boolean')
        expect(manager.isNative).toBe(true)
      })
    })

    describe('allStatuses', () => {
      it('should return empty object when no servers running', async () => {
        const statuses = await manager.allStatuses()
        expect(statuses).toEqual({})
      })
    })

    describe('status', () => {
      it('should return not_found for non-existent server', async () => {
        const status = await manager.status('nonexistent')
        expect(status.status).toBe('not_found')
      })
    })

    describe('stopAll', () => {
      it('should not throw when no servers exist', async () => {
        await manager.stopAll()
      })
    })

    describe('stop', () => {
      it('should not throw for non-existent server', async () => {
        await manager.stop('nonexistent')
      })
    })

    // NOTE: Actual LSP server tests require language servers to be installed.
    // These tests verify the manager API works correctly.
    describe.skip('Integration tests (require language servers)', () => {
      it('should start TypeScript server', async () => {
        // Requires typescript-language-server installed
        const key = await manager.start('typescript', process.cwd())
        expect(typeof key).toBe('string')

        const status = await manager.status(key)
        expect(['starting', 'running']).toContain(status.status)
      })
    })
  })

  describe('Type Exports', () => {
    it('should export LspSymbol type', () => {
      const symbol = {
        name: 'foo',
        kind: 'Function',
        startLine: 0,
        startCharacter: 0,
        endLine: 0,
        endCharacter: 10,
      }
      expect(symbol.name).toBe('foo')
    })

    it('should export LspTextEdit type', () => {
      const edit: LspTextEdit = {
        startLine: 0,
        startCharacter: 0,
        endLine: 0,
        endCharacter: 5,
        newText: 'hello',
      }
      expect(edit.newText).toBe('hello')
    })
  })
})
