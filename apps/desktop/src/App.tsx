import { useState } from 'react';
import { HomePage } from './pages/home';
import { ProjectPage } from './pages/project';
import { SettingsPage } from './pages/settings';
import { Button } from './shared/ui/button';

type View = 'home' | 'project' | 'settings';

function App() {
  const [currentView, setCurrentView] = useState<View>('project');

  const cycleView = () => {
    setCurrentView((v) => {
      if (v === 'home') return 'project';
      if (v === 'project') return 'settings';
      return 'home';
    });
  };

  return (
    <>
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
      {currentView === 'home' && <HomePage />}
      {currentView === 'project' && <ProjectPage />}
      {currentView === 'settings' && <SettingsPage />}
    </>
  );
}

export default App;
