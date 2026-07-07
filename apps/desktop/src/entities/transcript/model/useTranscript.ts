import { useState, useEffect, useRef, useCallback } from 'react';
import { listen } from '@/shared/api/tauri';
import { getTranscript } from '../api/transcriptApi';
import type { Transcript } from './types';

export function useTranscript(projectId: string | null) {
  const [transcript, setTranscript] = useState<Transcript | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const activeProjectId = useRef(projectId);

  useEffect(() => {
    activeProjectId.current = projectId;
  }, [projectId]);

  const fetchTranscript = useCallback(async (id: string) => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getTranscript(id);
      if (activeProjectId.current === id) {
        setTranscript(data);
      }
    } catch (err: any) {
      if (activeProjectId.current === id) {
        setError(err?.toString() || 'Failed to load transcript');
      }
    } finally {
      if (activeProjectId.current === id) {
        setIsLoading(false);
      }
    }
  }, []);

  // Initial fetch when project ID changes
  useEffect(() => {
    if (projectId) {
      fetchTranscript(projectId);
    } else {
      setTranscript(null);
    }
  }, [projectId, fetchTranscript]);

  // Listen to transcript-ready event
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        const fn = await listen<{ projectId: string; jobId: string }>('transcript-ready', (event) => {
          if (projectId && event.payload.projectId === projectId) {
            // Re-fetch the transcript because it is ready
            fetchTranscript(projectId);
          }
        });

        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      } catch (err) {
        console.warn('Failed to listen to transcript-ready event (Tauri might not be available):', err);
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [projectId, fetchTranscript]);

  return { transcript, isLoading, error, refetch: () => projectId && fetchTranscript(projectId) };
}
