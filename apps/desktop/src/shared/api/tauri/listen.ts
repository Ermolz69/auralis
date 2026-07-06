import { listen as tauriListen } from '@tauri-apps/api/event';
import type { UnlistenFn, EventCallback } from '@tauri-apps/api/event';

/**
 * Type-safe wrapper around Tauri's listen.
 * Use this to subscribe to backend events.
 */
export async function listen<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
  return tauriListen<T>(event, handler);
}
