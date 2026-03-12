/**
 * Snapshot Type Stubs
 * @deprecated This module has been moved to Rust.
 */

export namespace Snapshot {
  export function init(): void {
    // Stub - snapshot cleanup is now handled by Rust
  }

  export async function track(): Promise<string | undefined> {
    return undefined
  }

  export async function patch(_hash: string): Promise<{ hash: string; files: string[] }> {
    return { hash: _hash, files: [] }
  }

  export async function diff(_hash: string): Promise<string> {
    return ""
  }
}
