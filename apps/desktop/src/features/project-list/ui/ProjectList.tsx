import { useEffect, useState, useCallback } from 'react';
import { listProjects, useProjectContext } from '@/entities/project';
import { listen } from '@/shared/api/tauri';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { Card } from '@/shared/ui/card';
import { Icon } from '@/shared/ui/icon';

export const ProjectList = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const { setProjectId, setProject } = useProjectContext();
  const { setCurrentView } = useNavigation();

  const fetchProjects = useCallback(async () => {
    const data = await listProjects();
    setProjects(data);
    setIsLoading(false);
  }, []);

  useEffect(() => {
    fetchProjects();

    let unlistenProject: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        unlistenProject = await listen('project-updated', () => fetchProjects());
      } catch (e) {
        console.warn('Failed to setup Tauri listeners:', e);
      }
    };

    setupListeners();

    return () => {
      if (unlistenProject) unlistenProject();
    };
  }, [fetchProjects]);

  const handleOpenProject = (project: Project) => {
    setProjectId(project.id);
    setProject(project);
    setCurrentView('project');
  };

  if (isLoading) {
    return (
      <div className="text-muted text-sm text-center py-4 animate-pulse">Loading projects...</div>
    );
  }



  if (projects.length === 0) {
    return null;
  }

  return (
    <div className="w-full flex flex-col gap-3 mt-8">
      <h3 className="text-sm font-semibold text-muted uppercase tracking-wider mb-2 text-left">
        Recent Projects
      </h3>
      <div className="flex flex-col gap-2 max-h-[40vh] overflow-y-auto pr-2 custom-scrollbar">
        {projects.map((project) => (
          <Card
            key={project.id}
            className="p-4 hover:bg-bg/50 cursor-pointer transition-colors flex items-center justify-between shadow-sm border border-secondary"
            onClick={() => handleOpenProject(project)}
          >
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary shrink-0">
                <Icon name={project.source?.kind === 'remoteUrl' ? 'Video' : 'Film'} size="md" />
              </div>
              <div className="flex flex-col text-left">
                <span
                  className="text-text font-medium truncate max-w-[250px]"
                  title={project.title}
                >
                  {project.title || 'Untitled Project'}
                </span>
                <span className="text-muted text-xs capitalize flex items-center gap-1.5 mt-0.5">
                  <span
                    className={`w-2 h-2 rounded-full ${project.status === 'completed' ? 'bg-success' : project.status === 'failed' ? 'bg-danger' : project.status === 'processing' ? 'bg-primary animate-pulse' : 'bg-muted'}`}
                  ></span>
                  {project.status.replace(/_/g, ' ')}
                </span>
              </div>
            </div>
            <div className="text-muted text-xs">
              {new Date(project.updatedAt).toLocaleDateString()}
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
};
