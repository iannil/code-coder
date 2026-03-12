/**
 * Causal Recorder Hooks Stub
 * @deprecated Hooks are now implemented in Rust.
 */

export interface ToolActionParams {
  sessionId: string
  toolName: string
  toolInput: Record<string, unknown>
  toolOutput?: string
}

export const CausalRecorder = {
  register: () => {},
  unregister: () => {},
  recordToolAction: async (_params: ToolActionParams): Promise<void> => {
    // Stub - causal recording is now handled by Rust
  },
}

export const CausalRecorderHooks = CausalRecorder

export default CausalRecorder
