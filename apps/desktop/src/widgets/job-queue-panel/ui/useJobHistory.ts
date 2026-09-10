import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { listJobHistoryPage, type JobDto, type JobStoreState } from '@/entities/job';
import type { JobHistoryCursor } from '@/shared/api/contracts/job';
import { toCommandError } from '@/shared/api/contracts';

const HISTORY_PAGE_SIZE = 100;

export function useJobHistory(liveCompletedJobs: readonly JobDto[], phase: JobStoreState['phase']) {
  const [pagedJobs, setPagedJobs] = useState<JobDto[]>([]);
  const [nextCursor, setNextCursor] = useState<JobHistoryCursor | null>(null);
  const [hasLoadedPage, setHasLoadedPage] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestGeneration = useRef(0);

  const loadPage = useCallback(async (cursor: JobHistoryCursor | null, replace: boolean) => {
    const request = ++requestGeneration.current;
    setIsLoading(true);
    setError(null);
    try {
      const page = await listJobHistoryPage(cursor, HISTORY_PAGE_SIZE);
      if (request !== requestGeneration.current) return;
      setPagedJobs((current) => (replace ? page.jobs : mergeJobs(current, page.jobs)));
      setNextCursor(page.nextCursor);
      setHasLoadedPage(true);
    } catch (reason) {
      if (request !== requestGeneration.current) return;
      setError(toCommandError(reason).message);
    } finally {
      if (request === requestGeneration.current) setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (phase !== 'ready') return;
    void loadPage(null, true);
    return () => {
      requestGeneration.current += 1;
    };
  }, [loadPage, phase]);

  const jobs = useMemo(
    () => sortJobs(mergeJobs(pagedJobs, liveCompletedJobs)),
    [liveCompletedJobs, pagedJobs],
  );

  return {
    jobs,
    isLoading,
    error,
    hasMore: hasLoadedPage && nextCursor !== null,
    loadMore: () => {
      if (!isLoading && nextCursor) void loadPage(nextCursor, false);
    },
    retry: () => {
      if (!isLoading) void loadPage(hasLoadedPage ? nextCursor : null, !hasLoadedPage);
    },
  };
}

function mergeJobs(first: readonly JobDto[], second: readonly JobDto[]): JobDto[] {
  const merged = new Map(first.map((job) => [job.id, job]));
  for (const job of second) {
    const current = merged.get(job.id);
    if (!current || job.revision > current.revision) merged.set(job.id, job);
  }
  return [...merged.values()];
}

function sortJobs(jobs: JobDto[]): JobDto[] {
  return jobs.sort(
    (left, right) =>
      right.createdAt.localeCompare(left.createdAt) || right.id.localeCompare(left.id),
  );
}
