import { useState } from 'react';
import { createProjectFromYoutube, useProjectContext } from '@/entities/project';
import type { Job } from '@/entities/job';
import type { Project } from '@/entities/project';

export function usePasteYoutubeLink() {
  const [url, setUrl] = useState('');
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setProjectId, setProject, setCurrentView } = useProjectContext();
  
  // We can return the created project/job if the component needs to redirect or update global state
  const startProject = async (): Promise<{ project: Project, job: Job } | null> => {
    if (!url) return null;
    
    setIsStarting(true);
    setError(null);
    try {
      const response = await createProjectFromYoutube(url);
      setUrl(''); // clear input
      setProjectId(response.project.id);
      setProject(response.project);
      setCurrentView('project');
      return response;
    } catch (err: any) {
      setError(err?.toString() || 'Failed to start project');
      return null;
    } finally {
      setIsStarting(false);
    }
  };

  return {
    url,
    setUrl,
    startProject,
    isStarting,

    error,
  };
}
