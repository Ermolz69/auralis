import { lazy, Suspense, useEffect } from 'react';
import { HomePage } from './pages/home';
import { SettingsPage } from './pages/settings';
import { AppShell } from './widgets/app-shell';
import { JobQueuePanel } from './widgets/job-queue-panel';
import { useNavigation } from './shared/router';
import { useProjectContext } from './entities/project';

const ProjectPage = lazy(() =>
  import('./pages/project').then((module) => ({ default: module.ProjectPage })),
);

function App() {
  const { currentView, setCurrentView } = useNavigation();
  const { projectId } = useProjectContext();

  useEffect(() => {
    if (currentView === 'project' && !projectId) {
      setCurrentView('home');
    }
  }, [currentView, projectId, setCurrentView]);

  return (
    <AppShell jobQueue={<JobQueuePanel className="h-[calc(100%-2.5rem)]" />}>
      {currentView === 'home' && <HomePage />}
      {currentView === 'project' && projectId && (
        <Suspense fallback={<WorkspaceRouteLoading />}>
          <ProjectPage />
        </Suspense>
      )}
      {currentView === 'settings' && <SettingsPage />}
    </AppShell>
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
