import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { InvokeArgs } from '@tauri-apps/api/core';

/**
 * Type-safe wrapper around Tauri's invoke.
 * Use this to call backend commands with strongly typed return values.
 */
export async function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}
