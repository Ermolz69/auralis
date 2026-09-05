import { useEffect, useState } from 'react';
import {
  getProjectPreferences,
  listProjects,
  subscribeProjectChanges,
  subscribeProjectPreferences,
  type Project,
} from '@/entities/project';

export function usePinnedProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  useEffect(() => {
    let cancelled = false;
    let generation = 0;
    const refresh = async () => {
      const sequence = ++generation;
      try {
        const items = await listProjects();
        if (!cancelled && sequence === generation) {
          setProjects(items.filter((item) => getProjectPreferences(item.id).pinned));
        }
      } catch {
        // Keep the last confirmed sidebar state when refresh is unavailable.
      }
    };
    const unsubscribe = subscribeProjectChanges((change) => {
      generation += 1;
      setProjects((current) =>
        change.type === 'removed'
          ? current.filter((project) => project.id !== change.projectId)
          : current.map((project) => (project.id === change.project.id ? change.project : project)),
      );
      void refresh();
    });
    void refresh();
    const unsubscribePreferences = subscribeProjectPreferences(() => void refresh());
    return () => {
      cancelled = true;
      unsubscribe();
      unsubscribePreferences();
    };
  }, []);
  return projects;
}
