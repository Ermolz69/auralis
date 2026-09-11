import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { listen } from '@/shared/api/tauri';
import { subscribeSnapshotRefresh } from '@/shared/lib';
import { toCommandError } from '@/shared/api/contracts';
import { getTranscript } from '../api/transcriptApi';
import type { Transcript } from './types';

type TranscriptScope = {
  projectId: string | null;
  generation: number;
  key: symbol;
};

type TranscriptState = {
  scopeKey: symbol;
  transcript: Transcript | null;
  isLoading: boolean;
  error: string | null;
};

export function useTranscript(projectId: string | null) {
  const renderedScope = useMemo(
    () => ({ projectId, key: Symbol('transcript-scope') }),
    [projectId],
  );
  const [state, setState] = useState<TranscriptState>(() => ({
    scopeKey: renderedScope.key,
    transcript: null,
    isLoading: false,
    error: null,
  }));
  const activeScope = useRef<TranscriptScope | null>(null);
  const scopeSequence = useRef(0);
  const requestSequence = useRef(0);

  const fetchTranscript = useCallback(async (requestScope: TranscriptScope) => {
    const id = requestScope.projectId;
    if (!id || activeScope.current?.generation !== requestScope.generation) return;

    const requestId = ++requestSequence.current;
    setState((current) => ({
      scopeKey: requestScope.key,
      transcript: current.scopeKey === requestScope.key ? current.transcript : null,
      isLoading: true,
      error: null,
    }));

    const isCurrentRequest = () =>
      activeScope.current?.generation === requestScope.generation &&
      requestSequence.current === requestId;

    try {
      const transcript = await getTranscript(id);
      if (!isCurrentRequest()) return;
      setState((current) =>
        current.scopeKey === requestScope.key ? { ...current, transcript } : current,
      );
    } catch (caught: unknown) {
      if (!isCurrentRequest()) return;
      const error = toCommandError(caught).message;
      setState((current) =>
        current.scopeKey === requestScope.key ? { ...current, error } : current,
      );
    } finally {
      if (isCurrentRequest()) {
        setState((current) =>
          current.scopeKey === requestScope.key ? { ...current, isLoading: false } : current,
        );
      }
    }
  }, []);

  useEffect(() => {
    const scope: TranscriptScope = {
      ...renderedScope,
      generation: ++scopeSequence.current,
    };
    activeScope.current = scope;
    requestSequence.current += 1;
    setState({
      scopeKey: scope.key,
      transcript: null,
      isLoading: false,
      error: null,
    });
    if (scope.projectId) void fetchTranscript(scope);

    return () => {
      if (activeScope.current?.generation === scope.generation) {
        activeScope.current = null;
      }
      requestSequence.current += 1;
    };
  }, [fetchTranscript, renderedScope]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const scope = activeScope.current;
    if (!scope || scope.key !== renderedScope.key) return;

    const isCurrentScope = () => !cancelled && activeScope.current?.generation === scope.generation;

    const setupListener = async () => {
      try {
        const fn = await listen(
          'transcript-ready',
          (event) => {
            if (
              isCurrentScope() &&
              scope.projectId &&
              event.payload.projectId === scope.projectId
            ) {
              void fetchTranscript(scope);
            }
          },
          {
            onInvalidPayload: () => {
              if (isCurrentScope() && scope.projectId) void fetchTranscript(scope);
            },
          },
        );

        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      } catch (caught: unknown) {
        if (!cancelled) {
          console.warn('Failed to listen to transcript-ready event:', toCommandError(caught));
        }
      }
    };

    void setupListener();
    const stopSnapshotRefresh = subscribeSnapshotRefresh(() => fetchTranscript(scope));

    return () => {
      cancelled = true;
      stopSnapshotRefresh();
      unlisten?.();
    };
  }, [fetchTranscript, renderedScope]);

  const ownsVisibleState = state.scopeKey === renderedScope.key;
  const refetch = useCallback(() => {
    const scope = activeScope.current;
    if (!scope?.projectId || scope.key !== renderedScope.key) return null;
    return fetchTranscript(scope);
  }, [fetchTranscript, renderedScope]);

  return {
    transcript: ownsVisibleState ? state.transcript : null,
    isLoading: ownsVisibleState ? state.isLoading : false,
    error: ownsVisibleState ? state.error : null,
    refetch,
  };
}
