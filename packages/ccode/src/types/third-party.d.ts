/**
 * Type declarations for third-party modules without types
 */

// @parcel/watcher wrapper doesn't have types
declare module "@parcel/watcher/wrapper" {
  export function createWrapper(
    libc: string | undefined,
    options?: { workerPath?: string }
  ): Promise<typeof import("@parcel/watcher")>
}

// Optional snapshot module that may not exist
declare module "./models-snapshot" {
  export const snapshot: Record<string, unknown>
}
