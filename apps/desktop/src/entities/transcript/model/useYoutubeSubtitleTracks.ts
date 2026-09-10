import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toCommandError } from '@/shared/api/contracts';
import type { SubtitleTrack } from '@/shared/api/contracts/subtitle';
import { listYoutubeSubtitleTracks } from '../api/transcriptApi';

type CachedTracks = {
  cachedAt: number;
  tracks: SubtitleTrack[];
};

const CACHE_PREFIX = 'auralis:youtube-subtitle-tracks:v1:';
const memoryCache = new Map<string, CachedTracks>();
const refreshedThisSession = new Set<string>();
const pendingLoads = new Map<string, Promise<SubtitleTrack[]>>();

type TracksScope = {
  generation: number;
  key: symbol;
  projectId: string | null;
};

type TracksState = {
  error: string | null;
  isLoading: boolean;
  scopeKey: symbol | null;
  tracks: SubtitleTrack[];
};

export function useYoutubeSubtitleTracks(projectId: string | null) {
  const [state, setState] = useState<TracksState>({
    error: null,
    isLoading: false,
    scopeKey: null,
    tracks: [],
  });
  const activeScopeRef = useRef<TracksScope | null>(null);
  const scopeGenerationRef = useRef(0);
  const requestIdRef = useRef(0);
  const renderedScopeKey = useMemo(() => Symbol(projectId ?? 'no-project'), [projectId]);

  const load = useCallback(async (scope: TracksScope, force = false) => {
    const { projectId: scopedProjectId } = scope;
    if (!scopedProjectId || !isActiveScope(activeScopeRef.current, scope)) return;

    const cached = readCache(scopedProjectId);
    if (!force && refreshedThisSession.has(scopedProjectId) && cached) {
      setState({
        error: null,
        isLoading: false,
        scopeKey: scope.key,
        tracks: cached.tracks,
      });
      return;
    }

    const requestId = ++requestIdRef.current;
    setState((current) => ({
      error: null,
      isLoading: true,
      scopeKey: scope.key,
      tracks: current.scopeKey === scope.key ? current.tracks : (cached?.tracks ?? []),
    }));

    try {
      const result = await loadShared(scopedProjectId);
      if (requestId !== requestIdRef.current || !isActiveScope(activeScopeRef.current, scope)) {
        return;
      }
      setState({
        error: null,
        isLoading: true,
        scopeKey: scope.key,
        tracks: result,
      });
    } catch (cause) {
      if (requestId !== requestIdRef.current || !isActiveScope(activeScopeRef.current, scope)) {
        return;
      }
      if (!cached) {
        setState({
          error: toCommandError(cause).message,
          isLoading: true,
          scopeKey: scope.key,
          tracks: [],
        });
      }
    } finally {
      if (requestId === requestIdRef.current && isActiveScope(activeScopeRef.current, scope)) {
        setState((current) =>
          current.scopeKey === scope.key ? { ...current, isLoading: false } : current,
        );
      }
    }
  }, []);

  useEffect(() => {
    const scope: TracksScope = {
      generation: ++scopeGenerationRef.current,
      key: renderedScopeKey,
      projectId,
    };
    activeScopeRef.current = scope;
    requestIdRef.current += 1;

    const cached = projectId ? readCache(projectId) : null;
    setState({
      error: null,
      isLoading: false,
      scopeKey: scope.key,
      tracks: cached?.tracks ?? [],
    });

    if (projectId) void load(scope);

    return () => {
      if (activeScopeRef.current?.generation === scope.generation) {
        activeScopeRef.current = null;
      }
      requestIdRef.current += 1;
    };
  }, [load, projectId, renderedScopeKey]);

  const visibleState = state.scopeKey === renderedScopeKey ? state : null;

  return {
    tracks: visibleState?.tracks ?? [],
    isLoading: visibleState?.isLoading ?? false,
    error: visibleState?.error ?? null,
    refresh: () => {
      const scope = activeScopeRef.current;
      return scope && scope.key === renderedScopeKey ? load(scope, true) : Promise.resolve();
    },
  };
}

function isActiveScope(active: TracksScope | null, expected: TracksScope): boolean {
  return active?.generation === expected.generation && active.key === expected.key;
}

function loadShared(projectId: string): Promise<SubtitleTrack[]> {
  const pending = pendingLoads.get(projectId);
  if (pending) return pending;

  const request = listYoutubeSubtitleTracks(projectId)
    .then((tracks) => {
      writeCache(projectId, tracks);
      refreshedThisSession.add(projectId);
      return tracks;
    })
    .finally(() => {
      if (pendingLoads.get(projectId) === request) pendingLoads.delete(projectId);
    });
  pendingLoads.set(projectId, request);
  return request;
}

function cacheKey(projectId: string): string {
  return `${CACHE_PREFIX}${projectId}`;
}

function readCache(projectId: string): CachedTracks | null {
  const memoryValue = memoryCache.get(projectId);
  if (memoryValue) return memoryValue;

  try {
    const raw = globalThis.localStorage?.getItem(cacheKey(projectId));
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (!isCachedTracks(parsed)) {
      globalThis.localStorage?.removeItem(cacheKey(projectId));
      return null;
    }
    memoryCache.set(projectId, parsed);
    return parsed;
  } catch {
    return null;
  }
}

function writeCache(projectId: string, tracks: SubtitleTrack[]) {
  const value: CachedTracks = { cachedAt: Date.now(), tracks };
  memoryCache.set(projectId, value);
  try {
    globalThis.localStorage?.setItem(cacheKey(projectId), JSON.stringify(value));
  } catch {
    // Persistent cache is an optimization; the in-memory cache remains available.
  }
}

function isCachedTracks(value: unknown): value is CachedTracks {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as { cachedAt?: unknown; tracks?: unknown };
  return (
    typeof candidate.cachedAt === 'number' &&
    Array.isArray(candidate.tracks) &&
    candidate.tracks.every(isSubtitleTrack)
  );
}

function isSubtitleTrack(value: unknown): value is SubtitleTrack {
  if (!value || typeof value !== 'object') return false;
  const track = value as Record<string, unknown>;
  return (
    typeof track.id === 'string' &&
    typeof track.language === 'string' &&
    (typeof track.label === 'string' || track.label === null) &&
    (typeof track.format === 'string' || track.format === null) &&
    typeof track.isAutoGenerated === 'boolean'
  );
}
