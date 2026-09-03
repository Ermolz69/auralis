import { useLayoutEffect, useRef } from 'react';
import { HomePage } from './pages/home';
import { ProjectPage } from './pages/project';
import { SettingsPage } from './pages/settings';
import { AppShell } from './widgets/app-shell';
import { JobQueuePanel } from './widgets/job-queue-panel';
import { useNavigation } from './shared/router';
import { useProjectContext } from './entities/project';

function App() {
  const { currentView, setCurrentView } = useNavigation();
  const { selection } = useProjectContext();
  const previousSelectionStatus = useRef(selection.status);
  const view = currentView === 'project' && selection.status === 'closed' ? 'home' : currentView;

  useLayoutEffect(() => {
    const selectionClosed =
      previousSelectionStatus.current === 'open' && selection.status === 'closed';
    previousSelectionStatus.current = selection.status;
    if (selectionClosed || (currentView === 'project' && selection.status === 'closed')) {
      setCurrentView('home');
    }
  }, [currentView, selection.status, setCurrentView]);

  return (
    <AppShell jobQueue={<JobQueuePanel className="h-[calc(100%-2.5rem)]" />}>
      {view === 'home' && <HomePage />}
      {view === 'project' && selection.status === 'open' && <ProjectPage />}
      {view === 'settings' && <SettingsPage />}
    </AppShell>
  );
}

export default App;
