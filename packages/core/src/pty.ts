/**
 * PTY (Pseudo-Terminal) Shell Management
 *
 * Provides high-performance PTY-based shell sessions for interactive commands.
 */

// ============================================================================
// Type Definitions
// ============================================================================

export type PtyState = 'Running' | 'Exited' | 'Killed' | 'Error'

export interface PtyConfig {
  /** Initial terminal width in columns (default: 80) */
  cols?: number
  /** Initial terminal height in rows (default: 24) */
  rows?: number
  /** Shell to use (default: $SHELL or /bin/sh) */
  shell?: string
  /** Working directory */
  cwd?: string
  /** Environment variables */
  env?: Record<string, string>
  /** Inherit environment from parent (default: true) */
  inheritEnv?: boolean
}

export interface PtyInfo {
  /** Session ID */
  id: string
  /** Current state */
  state: PtyState
  /** Exit code (if exited) */
  exitCode: number | null
  /** Terminal width */
  cols: number
  /** Terminal height */
  rows: number
  /** Shell being used */
  shell: string
  /** Working directory */
  cwd: string
}

// ============================================================================
// Native Binding Check
// ============================================================================

let _nativeAvailable: boolean | null = null

async function checkNativeAvailable(): Promise<boolean> {
  if (_nativeAvailable !== null) return _nativeAvailable

  try {
    const bindings = (await import('./binding.js')) as any
    _nativeAvailable = typeof bindings.spawnPty === 'function'
  } catch {
    _nativeAvailable = false
  }

  return _nativeAvailable
}

export const isPtyNative = checkNativeAvailable

// ============================================================================
// PTY Session
// ============================================================================

export class PtySession {
  private handle: any

  private constructor(handle: any) {
    this.handle = handle
  }

  /**
   * Spawn a new shell PTY session
   */
  static async spawn(config: PtyConfig = {}): Promise<PtySession> {
    if (!(await checkNativeAvailable())) {
      throw new Error('PTY support not available. Native bindings not found.')
    }

    const bindings = (await import('./binding.js')) as any
    const handle = bindings.spawnPty({
      cols: config.cols,
      rows: config.rows,
      shell: config.shell,
      cwd: config.cwd,
      env: config.env,
      inheritEnv: config.inheritEnv,
    })
    return new PtySession(handle)
  }

  /**
   * Spawn a PTY session with a specific command
   */
  static async spawnCommand(
    command: string,
    args: string[] = [],
    config: PtyConfig = {}
  ): Promise<PtySession> {
    if (!(await checkNativeAvailable())) {
      throw new Error('PTY support not available. Native bindings not found.')
    }

    const bindings = (await import('./binding.js')) as any
    const handle = bindings.spawnPtyCommand(command, args, {
      cols: config.cols,
      rows: config.rows,
      shell: config.shell,
      cwd: config.cwd,
      env: config.env,
      inheritEnv: config.inheritEnv,
    })
    return new PtySession(handle)
  }

  /**
   * Get the session ID
   */
  get id(): string {
    return this.handle.id
  }

  /**
   * Check if the session is running
   */
  isRunning(): boolean {
    return this.handle.isRunning()
  }

  /**
   * Get session info
   */
  info(): PtyInfo {
    return this.handle.info()
  }

  /**
   * Read output from the PTY (non-blocking)
   */
  read(): Buffer {
    return this.handle.read()
  }

  /**
   * Read output as string
   */
  readString(): string {
    return this.handle.read().toString('utf-8')
  }

  /**
   * Read output with timeout
   */
  readWithTimeout(timeoutMs: number): Buffer {
    return this.handle.readWithTimeout(timeoutMs)
  }

  /**
   * Read output as string with timeout
   */
  readStringWithTimeout(timeoutMs: number): string {
    return this.handle.readWithTimeout(timeoutMs).toString('utf-8')
  }

  /**
   * Write data to the PTY
   */
  write(data: Buffer | string): void {
    const buffer = typeof data === 'string' ? Buffer.from(data) : data
    this.handle.write(buffer)
  }

  /**
   * Write a line (with newline) to the PTY
   */
  writeLine(line: string): void {
    this.handle.writeLine(line)
  }

  /**
   * Resize the terminal
   */
  resize(cols: number, rows: number): void {
    this.handle.resize(cols, rows)
  }

  /**
   * Kill the process
   */
  kill(): void {
    this.handle.kill()
  }

  /**
   * Wait for the process to exit
   */
  wait(): number {
    return this.handle.wait()
  }

  /**
   * Wait for the process with timeout
   */
  waitWithTimeout(timeoutMs: number): number | null {
    return this.handle.waitWithTimeout(timeoutMs)
  }

  /**
   * Get exit code (if exited)
   */
  exitCode(): number | null {
    return this.handle.exitCode()
  }
}

// ============================================================================
// PTY Manager
// ============================================================================

export class PtyManager {
  private handle: any

  private constructor(handle: any) {
    this.handle = handle
  }

  /**
   * Create a new PTY manager
   */
  static async create(): Promise<PtyManager> {
    if (!(await checkNativeAvailable())) {
      throw new Error('PTY support not available. Native bindings not found.')
    }

    const bindings = (await import('./binding.js')) as any
    const handle = new bindings.PtyManagerHandle()
    return new PtyManager(handle)
  }

  /**
   * Create a new shell session
   */
  createSession(config: PtyConfig = {}): string {
    return this.handle.create({
      cols: config.cols,
      rows: config.rows,
      shell: config.shell,
      cwd: config.cwd,
      env: config.env,
      inheritEnv: config.inheritEnv,
    })
  }

  /**
   * Create a new session with a specific command
   */
  createCommand(command: string, args: string[] = [], config: PtyConfig = {}): string {
    return this.handle.createCommand(command, args, {
      cols: config.cols,
      rows: config.rows,
      shell: config.shell,
      cwd: config.cwd,
      env: config.env,
      inheritEnv: config.inheritEnv,
    })
  }

  /**
   * List all session IDs
   */
  list(): string[] {
    return this.handle.list()
  }

  /**
   * List all session info
   */
  listInfo(): PtyInfo[] {
    return this.handle.listInfo()
  }

  /**
   * Clean up exited sessions
   */
  cleanup(): void {
    this.handle.cleanup()
  }

  /**
   * Kill all sessions
   */
  killAll(): void {
    this.handle.killAll()
  }
}
