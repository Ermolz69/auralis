import { useEffect } from 'react';
import { JobContext, type JobStoreState } from '@/entities/job';
import { ProjectProvider, useProjectContext } from '@/entities/project';
import { NavigationProvider, useNavigation } from '@/shared/router';
import { AppShell } from '@/widgets/app-shell';
import { ProjectPage } from './ProjectPage';
import { project, runningJob } from './ProjectPage.storyData';

export function projectWorkspaceBoundary() {
  const internal = (
    window as unknown as {
      __TAURI_INTERNALS__: { invoke: (command: string, args?: unknown) => Promise<unknown> };
    }
  ).__TAURI_INTERNALS__;
  const previous = internal.invoke;
  internal.invoke = async (command, args) => {
    if (command === 'get_project_cmd' && (args as { projectId?: string })?.projectId === project.id)
      return structuredClone(project);
    return previous(command, args);
  };
  return () => {
    internal.invoke = previous;
  };
}

export function ProjectWorkspaceStory({
  initialStep = 'source',
}: {
  initialStep?: 'source' | 'subtitles';
}) {
  const state: JobStoreState = {
    phase: 'ready',
    jobs: { [runningJob.id]: runningJob },
    buffer: [],
    pendingRefetch: false,
    generation: 1,
  };
  return (
    <NavigationProvider>
      <InitialProjectView step={initialStep} />
      <ProjectProvider>
        <SelectWorkspaceProject />
        <JobContext.Provider value={state}>
          <AppShell>
            <ProjectPage />
          </AppShell>
        </JobContext.Provider>
      </ProjectProvider>
    </NavigationProvider>
  );
}
function SelectWorkspaceProject() {
  const { setProject } = useProjectContext();
  useEffect(() => {
    setProject(project);
  }, []);
  return null;
}
function InitialProjectView({ step }: { step: 'source' | 'subtitles' }) {
  const { setCurrentView, setPipelineStep } = useNavigation();
  useEffect(() => {
    setPipelineStep(step);
    setCurrentView('project');
  }, [setCurrentView, setPipelineStep, step]);
  return null;
}
