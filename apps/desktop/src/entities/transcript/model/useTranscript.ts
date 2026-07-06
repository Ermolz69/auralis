import { useState, useEffect } from 'react';
import { listen } from '@/shared/api/tauri';
import { getTranscript } from '../api/transcriptApi';
import type { Transcript } from './types';

export function useTranscript(projectId: string | null) {
  const [transcript, setTranscript] = useState<Transcript | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const fetchTranscript = async (id: string) => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getTranscript(id);
      setTranscript(data);
    } catch (err: any) {
      setError(err?.toString() || 'Failed to load transcript');
    } finally {
      setIsLoading(false);
    }
  };

  // Initial fetch when project ID changes
  useEffect(() => {
    if (projectId) {
      fetchTranscript(projectId);
    } else {
      setTranscript(null);
    }
  }, [projectId]);

  // Listen to transcript-ready event
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen<{ projectId: string; jobId: string }>(
        'transcript-ready',
        (event) => {
          if (projectId && event.payload.projectId === projectId) {
            // Re-fetch the transcript because it is ready
            fetchTranscript(projectId);
          }
        }
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [projectId]);

  return { transcript, isLoading, error, refetch: () => projectId && fetchTranscript(projectId) };
}
