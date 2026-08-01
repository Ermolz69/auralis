import { useEffect } from 'react';
import { HomePage } from './pages/home';
import { ProjectPage } from './pages/project';
import { SettingsPage } from './pages/settings';
import { AppShell } from './widgets/app-shell';
import { useNavigation } from './shared/router';
import { useProjectContext } from './entities/project';

function App() {
  const { currentView, setCurrentView } = useNavigation();
  const { projectId } = useProjectContext();

  useEffect(() => {
    if (currentView === 'project' && !projectId) {
      setCurrentView('home');
    }
  }, [currentView, projectId, setCurrentView]);

  return (
    <AppShell>
      {currentView === 'home' && <HomePage />}
      {currentView === 'project' && projectId && <ProjectPage />}
      {currentView === 'settings' && <SettingsPage />}
    </AppShell>
  );
}

export default App;
