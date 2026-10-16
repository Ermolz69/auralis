import { getProjectTranslationSnapshot, subscribeTranslationChanges } from '@/entities/translation';
import type { TranslationLink, TranslationRunSnapshot } from '@/shared/api/contracts/translation';

const RECOVERY_INTERVAL_MS = 3000;

type Handlers = {
  onSnapshot: (links: TranslationLink[], snapshots: Record<string, TranslationRunSnapshot>) => void;
  onError: (reason: unknown) => void;
};

export function observeTranslationRuns(projectId: string, handlers: Handlers): () => void {
  let active = true;
  let reading = false;
  let pending = false;
  let unsubscribe: (() => void) | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const subscriptions = new AbortController();

  const scheduleRecovery = () => {
    if (timer) clearTimeout(timer);
    if (active) timer = setTimeout(refresh, RECOVERY_INTERVAL_MS);
  };

  async function read() {
    try {
      const snapshot = await getProjectTranslationSnapshot(projectId);
      if (active && !pending)
        handlers.onSnapshot(
          snapshot.entries.map((entry) => entry.link),
          snapshot.entries.reduce<Record<string, TranslationRunSnapshot>>((runs, entry) => {
            if (entry.run) runs[entry.link.translationId] = entry.run;
            return runs;
          }, {}),
        );
    } catch (reason) {
      if (active && !pending) handlers.onError(reason);
    } finally {
      reading = false;
      if (active && pending) {
        pending = false;
        refresh();
      } else {
        scheduleRecovery();
      }
    }
  }

  function refresh() {
    if (!active) return;
    if (timer) clearTimeout(timer);
    if (reading) {
      pending = true;
      return;
    }
    reading = true;
    void read();
  }

  scheduleRecovery();
  void subscribeTranslationChanges(projectId, refresh, subscriptions.signal).then((dispose) => {
    if (!active) {
      dispose();
      return;
    }
    unsubscribe = dispose;
    refresh();
  }, refresh);
  return () => {
    active = false;
    subscriptions.abort();
    if (timer) clearTimeout(timer);
    unsubscribe?.();
  };
}
