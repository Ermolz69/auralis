import { useCallback, useEffect, useRef, useState } from 'react';
import { listProjects } from '@/entities/project';
import type { Project } from '@/entities/project';
import { toCommandError } from '@/shared/api/contracts';
import { useProjectListUpdates } from './useProjectListUpdates';

export function useProjectListData(deletingProjectIdRef: { current: string | null }) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const fetchGenerationRef = useRef(0);

  const fetchProjects = useCallback(async (showLoading = false) => {
    fetchGenerationRef.current += 1;
    const currentGen = fetchGenerationRef.current;
    if (showLoading) setIsLoading(true);

    try {
      const data = await listProjects();
      if (currentGen === fetchGenerationRef.current) {
        setProjects((current) =>
          data.map((incoming) => {
            const existing = current.find((item) => item.id === incoming.id);
            return existing && existing.revision > incoming.revision ? existing : incoming;
          }),
        );
        setListError(null);
        setIsLoading(false);
      }
    } catch (e) {
      if (currentGen === fetchGenerationRef.current) {
        const commandError = toCommandError(e);
        setListError(commandError.message);
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void fetchProjects(true);
  }, [fetchProjects]);

  useProjectListUpdates(fetchProjects, deletingProjectIdRef);

  return { projects, setProjects, isLoading, listError, fetchGenerationRef, fetchProjects };
}
