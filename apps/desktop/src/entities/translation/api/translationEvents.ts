import { listen } from '@/shared/api/tauri';

export async function subscribeTranslationChanges(
  projectId: string,
  refresh: () => void,
  signal?: AbortSignal,
): Promise<() => void> {
  const listeners = new Set<() => void>();
  const dispose = () => {
    signal?.removeEventListener('abort', dispose);
    for (const listener of listeners) listener();
    listeners.clear();
  };
  if (signal?.aborted) return dispose;
  signal?.addEventListener('abort', dispose, { once: true });
  await Promise.allSettled(
    [
      listen(
        'job-event',
        ({ payload }) => {
          if (payload.job.kind === 'translation' && payload.job.projectId === projectId) refresh();
        },
        { onInvalidPayload: refresh },
      ),
      listen('job-events-invalidated', refresh, { onInvalidPayload: refresh }),
      listen(
        'project-updated',
        ({ payload }) => {
          if (payload.projectId === projectId) refresh();
        },
        { onInvalidPayload: refresh },
      ),
    ].map(async (registration) => {
      const listener = await registration;
      if (signal?.aborted) listener();
      else listeners.add(listener);
    }),
  );
  return dispose;
}

export async function subscribeTranslationPublicationChanges(
  projectId: string,
  refresh: () => void,
  signal?: AbortSignal,
): Promise<() => void> {
  if (signal?.aborted) return () => {};
  const dispose = await listen(
    'project-updated',
    ({ payload }) => {
      if (payload.projectId === projectId) refresh();
    },
    { onInvalidPayload: refresh },
  );
  let disposed = false;
  const stop = () => {
    if (disposed) return;
    disposed = true;
    signal?.removeEventListener('abort', stop);
    dispose();
  };
  if (signal?.aborted) {
    stop();
    return stop;
  }
  signal?.addEventListener('abort', stop, { once: true });
  return stop;
}
