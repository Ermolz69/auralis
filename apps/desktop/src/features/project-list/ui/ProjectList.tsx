import { useEffect, useState, useCallback } from 'react';
import { deleteProject, listProjects, useProjectContext } from '@/entities/project';
import { listen } from '@/shared/api/tauri';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { Card } from '@/shared/ui/card';
import { Icon } from '@/shared/ui/icon';
import { Button } from '@/shared/ui/button';
import { toast } from '@/shared/ui/toast';
import { isCommandError } from '@/shared/api/contracts';
import {
  Dialog,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from '@/shared/ui/dialog';

export const ProjectList = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [projectToDelete, setProjectToDelete] = useState<Project | null>(null);
  const { projectId: currentProjectId, setProjectId, setProject } = useProjectContext();
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

  const handleDeleteClick = (e: React.MouseEvent, project: Project) => {
    e.stopPropagation();
    if (deletingId) return;
    setProjectToDelete(project);
  };

  const executeDelete = async () => {
    if (!projectToDelete) return;
    const project = projectToDelete;
    
    setDeletingId(project.id);
    setProjectToDelete(null);

    try {
      await deleteProject(project.id);
      
      setProjects((prev) => prev.filter((p) => p.id !== project.id));
      if (currentProjectId === project.id) {
        setProjectId(null);
        setProject(null);
        setCurrentView('home');
      }
      toast.success('Project deleted successfully');
    } catch (error) {
      const errorMessage = isCommandError(error) ? error.message : String(error);
      toast.error(errorMessage);
    } finally {
      setDeletingId(null);
    }
  };

  return (
    <div className="w-full flex flex-col gap-3 mt-8">
      <Dialog open={!!projectToDelete} onOpenChange={(open) => !open && setProjectToDelete(null)}>
        <DialogHeader>
          <DialogTitle>Delete Project</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete the project "{projectToDelete?.title}"? This action cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={() => setProjectToDelete(null)}>
            Cancel
          </Button>
          <Button variant="danger" onClick={executeDelete}>
            Confirm Delete
          </Button>
        </DialogFooter>
        <DialogClose />
      </Dialog>
      <h3 className="text-sm font-semibold text-muted uppercase tracking-wider mb-2 text-left">
        Recent Projects
      </h3>
      <div className="flex flex-col gap-2 max-h-[40vh] overflow-y-auto pr-2 custom-scrollbar">
        {projects.map((project) => (
          <Card
            key={project.id}
            className="group p-4 hover:bg-bg/50 cursor-pointer transition-colors flex items-center justify-between shadow-sm border border-secondary"
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
            <div className="flex items-center gap-4">
              <div className="text-muted text-xs">
                {new Date(project.updatedAt).toLocaleDateString()}
              </div>
              <Button
                variant="ghost"
                size="sm"
                className="opacity-0 group-hover:opacity-100 transition-opacity"
                loading={deletingId === project.id}
                disabled={deletingId !== null}
                onClick={(e) => handleDeleteClick(e, project)}
                title="Delete Project"
                aria-label={`Delete ${project.title}`}
                leftIcon={deletingId !== project.id ? <Icon name="Trash2" size="sm" /> : undefined}
              />
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
};
