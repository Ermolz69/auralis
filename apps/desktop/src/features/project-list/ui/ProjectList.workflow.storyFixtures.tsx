import { ProjectProvider, type Project } from '@/entities/project';
import { NavigationProvider } from '@/shared/router';
import { Toaster } from '@/shared/ui/toast';
import { ProjectList } from './ProjectList';

export const workflowProjects: Project[] = ['Chapter one', 'Chapter two'].map((title, index) => ({
  id: `workflow-project-${index}`,
  title,
  revision: 1,
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-10-23T10:00:00Z',
  updatedAt: '2026-10-23T10:00:00Z',
}));

export function ProjectListWorkflow() {
  return (
    <NavigationProvider>
      <ProjectProvider>
        <div className="w-[520px]">
          <ProjectList />
          <Toaster />
        </div>
      </ProjectProvider>
    </NavigationProvider>
  );
}

export function projectListBoundary(failFetch = false) {
  let projects = structuredClone(workflowProjects);
  let fetches = 0;
  let deletions = 0;
  let settle: ((error?: string) => void) | undefined;
  const internal = (
    window as unknown as {
      __TAURI_INTERNALS__: { invoke: (command: string, args?: unknown) => Promise<unknown> };
    }
  ).__TAURI_INTERNALS__;
  const previous = internal.invoke;
  internal.invoke = async (command, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    if (command === 'list_projects_cmd') {
      if (failFetch && fetches++ === 0) throw { code: 'IO', message: 'Project index unavailable' };
      return structuredClone(projects);
    }
    if (command === 'delete_project_cmd') {
      if (!projects.some((project) => project.id === args.projectId))
        throw new Error('Unexpected deletion identity');
      deletions++;
      return new Promise((resolve, reject) => {
        settle = (error) => {
          if (error) reject({ code: 'IO', message: error });
          else {
            projects = projects.filter((project) => project.id !== args.projectId);
            resolve(null);
          }
        };
      });
    }
    if (command === 'list_pending_youtube_imports_cmd' || command === 'list_artifact_recovery_cmd')
      return [];
    return previous(command, payload);
  };
  return {
    deletions: () => deletions,
    finishDeletion: (error?: string) => settle?.(error),
    restore: () => {
      internal.invoke = previous;
    },
  };
}
