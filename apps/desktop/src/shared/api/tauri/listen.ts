import { listen as tauriListen } from '@tauri-apps/api/event';
import type { UnlistenFn, EventCallback } from '@tauri-apps/api/event';
import type { EventMap } from '../contracts';
import { parseEventPayload } from '../contracts/runtimeValidation';

export async function listen<K extends keyof EventMap>(
  event: K,
  handler: EventCallback<EventMap[K]>,
): Promise<UnlistenFn> {
  return tauriListen<unknown>(event, (tauriEvent) => {
    handler({
      ...tauriEvent,
      payload: parseEventPayload(event, tauriEvent.payload),
    });
  });
}
