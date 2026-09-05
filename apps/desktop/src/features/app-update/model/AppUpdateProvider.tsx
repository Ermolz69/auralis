import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import {
  nativeAppUpdateClient,
  type AppUpdateCandidate,
  type AppUpdateClient,
} from '../api/appUpdateClient';
import { AppUpdateContext, type AppUpdateProgress, type AppUpdateState } from './appUpdateContext';

export function AppUpdateProvider({
  children,
  client = nativeAppUpdateClient,
  autoCheck = true,
}: {
  children: ReactNode;
  client?: AppUpdateClient;
  autoCheck?: boolean;
}) {
  const [state, setState] = useState<AppUpdateState>({
    phase: 'idle',
    currentVersion: __APP_VERSION__,
    update: null,
    progress: null,
    error: null,
  });
  const candidateRef = useRef<AppUpdateCandidate | null>(null);
  const installingRef = useRef(false);
  const mountedRef = useRef(false);
  const generationRef = useRef(0);

  const checkForUpdates = useCallback(async () => {
    if (installingRef.current) return;
    const generation = ++generationRef.current;
    const previous = candidateRef.current;
    candidateRef.current = null;
    if (previous) void previous.close().catch(() => undefined);
    setState((current) => ({
      ...current,
      phase: 'checking',
      update: null,
      progress: null,
      error: null,
    }));

    try {
      const currentVersion = await client.getCurrentVersion();
      if (!client.isSupported()) {
        if (mountedRef.current && generation === generationRef.current) {
          setState({
            phase: 'unsupported',
            currentVersion,
            update: null,
            progress: null,
            error: null,
          });
        }
        return;
      }

      const candidate = await client.check();
      if (!mountedRef.current || generation !== generationRef.current) {
        if (candidate) void candidate.close().catch(() => undefined);
        return;
      }
      candidateRef.current = candidate;
      setState({
        phase: candidate ? 'available' : 'upToDate',
        currentVersion,
        update: candidate
          ? { version: candidate.version, date: candidate.date, notes: candidate.body }
          : null,
        progress: null,
        error: null,
      });
    } catch {
      if (mountedRef.current && generation === generationRef.current) {
        setState((current) => ({
          ...current,
          phase: 'error',
          update: null,
          progress: null,
          error: 'Could not check for updates. Check your connection and try again.',
        }));
      }
    }
  }, [client]);

  const installUpdate = useCallback(async () => {
    const candidate = candidateRef.current;
    if (!candidate || installingRef.current) return;
    installingRef.current = true;
    let downloadedBytes = 0;
    let totalBytes: number | undefined;
    setState((current) => ({ ...current, phase: 'downloading', progress: null, error: null }));

    try {
      await candidate.downloadAndInstall((event) => {
        if (!mountedRef.current) return;
        if (event.event === 'Started') {
          totalBytes = event.data.contentLength;
          setState((current) => ({
            ...current,
            progress: progress(downloadedBytes, totalBytes),
          }));
        } else if (event.event === 'Progress') {
          downloadedBytes += event.data.chunkLength;
          setState((current) => ({
            ...current,
            progress: progress(downloadedBytes, totalBytes),
          }));
        } else {
          setState((current) => ({
            ...current,
            progress: progress(totalBytes ?? downloadedBytes, totalBytes),
          }));
        }
      });
      candidateRef.current = null;
      if (mountedRef.current) {
        setState((current) => ({ ...current, phase: 'restarting' }));
      }
      await client.relaunch();
    } catch {
      candidateRef.current = null;
      void candidate.close().catch(() => undefined);
      if (mountedRef.current) {
        setState((current) => ({
          ...current,
          phase: 'error',
          progress: null,
          error: 'Could not install the update. This version will continue to work.',
        }));
      }
    } finally {
      installingRef.current = false;
    }
  }, [client]);

  useEffect(() => {
    mountedRef.current = true;
    if (autoCheck) void checkForUpdates();
    return () => {
      mountedRef.current = false;
      generationRef.current += 1;
      const candidate = candidateRef.current;
      candidateRef.current = null;
      if (candidate) void candidate.close().catch(() => undefined);
    };
  }, [autoCheck, checkForUpdates]);

  const value = useMemo(
    () => ({ ...state, checkForUpdates, installUpdate }),
    [checkForUpdates, installUpdate, state],
  );

  return <AppUpdateContext.Provider value={value}>{children}</AppUpdateContext.Provider>;
}

function progress(downloadedBytes: number, totalBytes?: number): AppUpdateProgress {
  return {
    downloadedBytes,
    totalBytes,
    percent:
      totalBytes && totalBytes > 0
        ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100))
        : undefined,
  };
}
