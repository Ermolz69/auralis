import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { JobContext, isActiveJobStatus, selectProjectJobs } from '@/entities/job';
import { formatProjectTitle } from '@/entities/media';
import { useProjectContext } from '@/entities/project';
import { useNavigation, type PipelineStep, type View } from '@/shared/router';
import { AppHeader } from './app-shell/AppHeader';
import { AppSidebar } from './app-shell/AppSidebar';
import { AppStatusBar } from './app-shell/AppStatusBar';
import { JobQueueDrawer } from './app-shell/JobQueueDrawer';
import { locationLabel } from './app-shell/model';
import { getPipelineStatus } from '../model/pipelineStatus';

export function AppShell({ children, jobQueue }: { children: ReactNode; jobQueue?: ReactNode }) {
  const { currentView, setCurrentView, pipelineStep, setPipelineStep } = useNavigation();
  const { projectId, project } = useProjectContext();
  const jobState = useContext(JobContext);
  const mainRef = useRef<HTMLElement>(null);
  const queueButtonRef = useRef<HTMLButtonElement>(null);
  const [settingsReturnView, setSettingsReturnView] = useState<View>('home');
  const [queueOpen, setQueueOpen] = useState(false);

  useEffect(() => {
    const main = mainRef.current;
    const heading = main?.querySelector('h1');
    if (heading instanceof HTMLElement) {
      heading.tabIndex = -1;
      heading.focus();
    } else main?.focus();
  }, [currentView, pipelineStep]);

  useEffect(() => {
    if (!projectId && settingsReturnView === 'project') setSettingsReturnView('home');
  }, [projectId, settingsReturnView]);

  const goHome = () => setCurrentView('home');
  const goProject = () => {
    if (projectId) setCurrentView('project');
  };
  const goStep = (step: PipelineStep) => {
    if (!projectId) return;
    setPipelineStep(step);
    setCurrentView('project');
  };
  const goSettings = () => {
    setSettingsReturnView(currentView === 'project' && projectId ? 'project' : 'home');
    setCurrentView('settings');
  };
  const returnFromSettings = () =>
    setCurrentView(settingsReturnView === 'project' && projectId ? 'project' : 'home');
  const closeQueue = useCallback(() => {
    setQueueOpen(false);
    requestAnimationFrame(() => queueButtonRef.current?.focus());
  }, []);

  const jobs = useMemo(() => Object.values(jobState?.jobs ?? {}), [jobState?.jobs]);
  const activeJobs = jobs.filter((job) => isActiveJobStatus(job.status));
  const pipelineStatus = getPipelineStatus(
    Boolean(project?.source),
    selectProjectJobs(jobState?.jobs ?? {}, projectId),
  );
  const projectTitle = project
    ? formatProjectTitle(project.title, project.source)
    : 'Проект не открыт';

  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden bg-canvas text-text">
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <AppSidebar
          currentView={currentView}
          pipelineStep={pipelineStep}
          hasProject={Boolean(projectId)}
          pipelineStatus={pipelineStatus}
          onHome={goHome}
          onProject={goProject}
          onSettings={goSettings}
          onStep={goStep}
        />
        <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden pb-14 lg:pb-0">
          <AppHeader
            currentView={currentView}
            pipelineStep={pipelineStep}
            projectTitle={projectTitle}
            hasProject={Boolean(projectId)}
            hasJobContext={Boolean(jobState)}
            activeJobs={activeJobs.length}
            queueOpen={queueOpen}
            queueButtonRef={queueButtonRef}
            onHome={goHome}
            onBackFromSettings={returnFromSettings}
            onToggleQueue={() => setQueueOpen((open) => !open)}
          />
          <main
            ref={mainRef}
            tabIndex={-1}
            aria-label={locationLabel[currentView]}
            className="min-h-0 flex-1 overflow-hidden bg-bg outline-none"
          >
            {children}
          </main>
          {queueOpen && jobState && (
            <JobQueueDrawer onClose={closeQueue}>{jobQueue}</JobQueueDrawer>
          )}
        </div>
      </div>
      <AppStatusBar
        projectTitle={projectId ? projectTitle : null}
        durationMs={project?.metadata?.durationMs}
        width={project?.metadata?.width}
        height={project?.metadata?.height}
        activeJobTitle={activeJobs[0]?.title}
        activeJobPercent={activeJobs[0]?.progress.percent}
      />
    </div>
  );
}
