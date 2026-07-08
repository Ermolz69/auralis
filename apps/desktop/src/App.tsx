import { useEffect } from 'react';
import { HomePage } from './pages/home';
import { ProjectPage } from './pages/project';
import { SettingsPage } from './pages/settings';
import { Button } from './shared/ui/button';
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

  const cycleView = () => {
    setCurrentView(
      currentView === 'home' ? 'project' : currentView === 'project' ? 'settings' : 'home',
    );
  };

  return (
    <>
      {import.meta.env.DEV && (
        <div className="fixed bottom-4 left-4 z-50 flex gap-2">
          <Button
            size="sm"
            variant="secondary"
            onClick={cycleView}
            className="shadow-sm hover:shadow"
          >
            Toggle View: {currentView}
          </Button>
        </div>
      )}
      {currentView === 'home' && <HomePage />}
      {currentView === 'project' && projectId && <ProjectPage />}
      {currentView === 'settings' && <SettingsPage />}
    </>
  );
}

export default App;
