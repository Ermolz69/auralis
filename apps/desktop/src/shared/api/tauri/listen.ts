import { listen as tauriListen } from '@tauri-apps/api/event';
import type { UnlistenFn, EventCallback } from '@tauri-apps/api/event';
import type { EventMap } from '../contracts';
import { toCommandError } from '../contracts/error';
import { IpcContractError, parseEventPayload } from '../contracts/runtimeValidation';

type ListenOptions = {
  onInvalidPayload?: (error: IpcContractError) => void;
};

export async function listen<K extends keyof EventMap>(
  event: K,
  handler: EventCallback<EventMap[K]>,
  options: ListenOptions = {},
): Promise<UnlistenFn> {
  try {
    return await tauriListen<unknown>(event, (tauriEvent) => {
      let payload: EventMap[K];
      try {
        payload = parseEventPayload(event, tauriEvent.payload);
      } catch (cause) {
        const error =
          cause instanceof IpcContractError ? cause : new IpcContractError('event', String(event));
        if (options.onInvalidPayload) {
          options.onInvalidPayload(error);
        } else {
          console.error('Rejected invalid Tauri event payload', error);
        }
        return;
      }

      handler({ ...tauriEvent, payload });
    });
  } catch (error) {
    throw toCommandError(error);
  }
}
