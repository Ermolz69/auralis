import { useCallback, useEffect, useMemo, useState } from 'react';
import { useLibrary } from '@/entities/library';
import type { LibraryTab } from '@/shared/api/contracts/library';
import type { SpeechDraft } from '@/features/speech-catalog';
export function parseDraft(value: string): SpeechDraft {
  try {
    return JSON.parse(value) as SpeechDraft;
  } catch {
    return {};
  }
}
export function useTabDraft(tab: LibraryTab) {
  const { mutate } = useLibrary();
  const initial = useMemo(() => parseDraft(tab.draft), [tab.draft]);
  const [draft, setDraft] = useState<SpeechDraft | null>(null);
  const change = useCallback(
    (next: SpeechDraft) =>
      setDraft((current) => (JSON.stringify(current) === JSON.stringify(next) ? current : next)),
    [],
  );
  useEffect(() => {
    if (!draft) return;
    const timer = setTimeout(
      () =>
        void mutate((state) => {
          for (const workspace of state.workspaces) {
            const stored = workspace.tabs.find((t) => t.id === tab.id);
            if (stored) stored.draft = JSON.stringify(draft);
          }
        }).catch(() => {}),
      400,
    );
    return () => clearTimeout(timer);
  }, [draft, mutate, tab.id]);
  return { initial, draft, change };
}
