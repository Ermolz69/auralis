import { useState, useEffect, useRef, useMemo } from 'react';
import type { ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
// eslint-disable-next-line boundaries/dependencies
import { useProjectContext } from '@/entities/project';
import { JobContext } from './context';
import type { Job, JobEvent } from './types';

export function JobProvider({ children }: { children: ReactNode }) {
  const { projectId } = useProjectContext();
  const [jobs, setJobs] = useState<Record<string, Job>>({});

  const scopeGenerationRef = useRef<number>(0);

  useEffect(() => {
    if (!projectId) {
      setJobs({});
      return;
    }

    scopeGenerationRef.current += 1;
    const currentGen = scopeGenerationRef.current;

    let cancelled = false;
    let unlistenEvent: (() => void) | undefined;
    let unlistenInvalidated: (() => void) | undefined;

    const setup = async () => {
      try {
        const snapshot = await invoke('list_jobs_snapshot_cmd', { projectId });

        if (cancelled || currentGen !== scopeGenerationRef.current) return;

        const jobsMap: Record<string, Job> = {};
        for (const job of snapshot) {
          jobsMap[job.id] = job;
        }
        setJobs(jobsMap);
      } catch (err) {
        console.error('Failed to load jobs snapshot:', err);
      }

      try {
        const fn = await listen<JobEvent>('job-event', (event) => {
          if (cancelled || currentGen !== scopeGenerationRef.current) return;

          const payload = event.payload;
          if (payload.projectId !== projectId) return;

          setJobs((prev) => {
            const existing = prev[payload.jobId];
            if (existing && existing.revision >= payload.revision) {
              return prev; // Ignore older events
            }

            return {
              ...prev,
              [payload.jobId]: {
                id: payload.jobId,
                projectId: payload.projectId,
                revision: payload.revision,
                title: existing?.title ?? 'Unknown',
                status: payload.status,
                stage: payload.stage,
                progress: payload.progress,
                error: payload.error,
                createdAt: existing?.createdAt ?? new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              },
            };
          });
        });

        if (cancelled) {
          fn();
        } else {
          unlistenEvent = fn;
        }
      } catch (err) {
        console.warn('Failed to listen to job events:', err);
      }

      try {
        const fnInvalidated = await listen('job-events-invalidated', async () => {
          if (cancelled || currentGen !== scopeGenerationRef.current) return;
          try {
            const snapshot = await invoke('list_jobs_snapshot_cmd', { projectId });
            if (cancelled || currentGen !== scopeGenerationRef.current) return;
            const jobsMap: Record<string, Job> = {};
            for (const job of snapshot) {
              jobsMap[job.id] = job;
            }
            setJobs(jobsMap);
          } catch (err) {
            console.error('Failed to reload jobs snapshot on invalidation:', err);
          }
        });

        if (cancelled) {
          fnInvalidated();
        } else {
          unlistenInvalidated = fnInvalidated;
        }
      } catch (err) {
        console.warn('Failed to listen to job-events-invalidated:', err);
      }
    };

    setup();

    return () => {
      cancelled = true;
      if (unlistenEvent) unlistenEvent();
      if (unlistenInvalidated) unlistenInvalidated();
    };
  }, [projectId]);

  const activeJobs = useMemo(() => {
    const list = Object.values(jobs).filter(
      (j) => j.status === 'pending' || j.status === 'running',
    );
    list.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
    return list;
  }, [jobs]);

  const completedJobs = useMemo(() => {
    const list = Object.values(jobs).filter(
      (j) => j.status !== 'pending' && j.status !== 'running',
    );
    list.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
    return list;
  }, [jobs]);

  return (
    <JobContext.Provider value={{ jobs, activeJobs, completedJobs }}>
      {children}
    </JobContext.Provider>
  );
}
