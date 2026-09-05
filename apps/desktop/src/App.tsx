import { lazy, Suspense, useCallback, useLayoutEffect, useRef } from 'react';
import { HomePage } from './pages/home';
import { AppShell } from './widgets/app-shell';
import { useNavigation } from './shared/router';
import { useProjectContext } from './entities/project';
import { AppUpdateNotifier, AppUpdateProvider } from './features/app-update';

const ProjectPage = lazy(() =>
  import('./pages/project').then((module) => ({ default: module.ProjectPage })),
);
const SettingsPage = lazy(() =>
  import('./pages/settings').then((module) => ({ default: module.SettingsPage })),
);
const JobQueuePanel = lazy(() =>
  import('./widgets/job-queue-panel').then((module) => ({ default: module.JobQueuePanel })),
);

function App() {
  return (
    <AppUpdateProvider>
      <AppContent />
    </AppUpdateProvider>
  );
}

function AppContent() {
  const { currentView, setCurrentView } = useNavigation();
  const { selection } = useProjectContext();
  const previousSelectionStatus = useRef(selection.status);
  const view = currentView === 'project' && selection.status === 'closed' ? 'home' : currentView;
  const openUpdateSettings = useCallback(() => setCurrentView('settings'), [setCurrentView]);

  useLayoutEffect(() => {
    const selectionClosed =
      previousSelectionStatus.current === 'open' && selection.status === 'closed';
    previousSelectionStatus.current = selection.status;
    if (selectionClosed || (currentView === 'project' && selection.status === 'closed')) {
      setCurrentView('home');
    }
  }, [currentView, selection.status, setCurrentView]);

  return (
    <>
      <AppUpdateNotifier onOpen={openUpdateSettings} />
      <AppShell
        jobQueue={
          <Suspense fallback={<WorkspaceRouteLoading />}>
            <JobQueuePanel className="h-[calc(100%-2.5rem)]" />
          </Suspense>
        }
      >
        {view === 'home' && <HomePage />}
        {view === 'project' && selection.status === 'open' && (
          <Suspense fallback={<WorkspaceRouteLoading />}>
            <ProjectPage />
          </Suspense>
        )}
        {view === 'settings' && (
          <Suspense fallback={<WorkspaceRouteLoading />}>
            <SettingsPage />
          </Suspense>
        )}
      </AppShell>
    </>
  );
}

function WorkspaceRouteLoading() {
  return (
    <div className="flex h-full items-center justify-center text-xs text-muted" role="status">
      Loading project workspace…
    </div>
  );
}

export default App;
