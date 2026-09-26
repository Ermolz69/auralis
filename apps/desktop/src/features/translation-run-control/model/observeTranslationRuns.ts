import {
  getTranslationRunStatus,
  listProjectTranslations,
  subscribeTranslationChanges,
} from '@/entities/translation';
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
      const links = await listProjectTranslations(projectId);
      const entries = await Promise.all(
        links.map(async (link) => {
          if (!link.activeRunId) return null;
          return [
            link.translationId,
            await getTranslationRunStatus(projectId, link.translationId),
          ] as const;
        }),
      );
      if (active)
        handlers.onSnapshot(links, Object.fromEntries(entries.filter((item) => item !== null)));
    } catch (reason) {
      if (active) handlers.onError(reason);
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
