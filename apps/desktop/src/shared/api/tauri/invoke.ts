import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { InvokeArgs } from '@tauri-apps/api/core';
import type { CommandMap } from '../contracts';
import { toCommandError } from '../contracts/error';
import { parseCommandResult } from '../contracts/runtimeValidation';

/**
 * Type-safe wrapper around Tauri's invoke.
 * Uses a CommandMap to ensure that command names, arguments, and return types are strongly typed.
 */
export async function invoke<K extends keyof CommandMap>(
  cmd: K,
  ...args: CommandMap[K]['args'] extends undefined ? [] : [CommandMap[K]['args']]
): Promise<CommandMap[K]['result']> {
  let result: unknown;
  try {
    result = await tauriInvoke<unknown>(cmd, args[0] as InvokeArgs);
  } catch (err) {
    throw toCommandError(err);
  }
  return parseCommandResult(cmd, result);
}
