import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useLibrary } from '@/entities/library';
import type { LibraryTab } from '@/shared/api/contracts/library';
import type { SpeechDraft } from '@/features/speech-catalog';
import { describeCommandError } from '@/shared/api/contracts/error';
import { panelKey } from './panels';
const liveDrafts = new WeakMap<object, Map<string, SpeechDraft>>();
export function readTabDraft(tab: LibraryTab, owner: object) {
  return liveDrafts.get(owner)?.get(tab.id) ?? parseDraft(tab.draft);
}
export function parseDraft(value: string): SpeechDraft {
  try {
    const parsed: unknown = JSON.parse(value);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};
    const candidate = parsed as SpeechDraft;
    if (candidate.mode !== undefined && !['text', 'subtitles'].includes(candidate.mode)) return {};
    if (
      ['text', 'subtitleId', 'translationId', 'selected'].some((key) => {
        const field = candidate[key as keyof SpeechDraft];
        return field !== undefined && typeof field !== 'string';
      })
    )
      return {};
    return candidate;
  } catch {
    return {};
  }
}
export function useTabDraft(tab: LibraryTab) {
  const { mutate } = useLibrary();
  if (!liveDrafts.has(mutate)) liveDrafts.set(mutate, new Map());
  const initial = useMemo(() => parseDraft(tab.draft), [tab.draft]);
  const [edit, setEdit] = useState<{ assetId: string | null; value: SpeechDraft } | null>(null);
  const draft = edit?.assetId === tab.assetId ? edit.value : null;
  const [error, setError] = useState('');
  const pending = useRef<{ id: string; value: SpeechDraft } | null>(null);
  const mounted = useRef(false);
  const change = useCallback(
    (next: SpeechDraft) => {
      liveDrafts.get(mutate)!.set(tab.id, next);
      setEdit((current) =>
        current?.assetId === tab.assetId && JSON.stringify(current.value) === JSON.stringify(next)
          ? current
          : { assetId: tab.assetId, value: next },
      );
    },
    [tab.assetId, tab.id, mutate],
  );
  const flush = useCallback(async () => {
    const next = pending.current;
    if (!next) return;
    pending.current = null;
    try {
      await mutate((state) => {
        for (const workspace of state.workspaces) {
          const stored = workspace.tabs.find((item) => item.id === next.id);
          if (stored)
            for (const item of workspace.tabs) {
              if (panelKey(item) === panelKey(stored)) item.draft = JSON.stringify(next.value);
            }
        }
      });
      if (mounted.current) setError('');
    } catch (reason) {
      pending.current ??= next;
      if (mounted.current) setError(describeCommandError(reason));
    }
  }, [mutate]);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(
    () => () => {
      liveDrafts.get(mutate)?.delete(tab.id);
    },
    [mutate, tab.id],
  );
  useEffect(
    () => () => {
      void flush();
    },
    [flush, tab.assetId],
  );
  useEffect(() => {
    if (!draft) return;
    pending.current = { id: tab.id, value: draft };
    const timer = setTimeout(() => void flush(), 400);
    return () => clearTimeout(timer);
  }, [draft, flush, tab.id]);
  return { initial, draft, change, error, retry: flush };
}
