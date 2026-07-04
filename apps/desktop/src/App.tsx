import { useState } from 'react';
import { HomePage } from './pages/home';
import { ProjectPage } from './pages/project';

function App() {
  const [currentView, setCurrentView] = useState<'home' | 'project'>('project');

  return (
    <>
      <div className="fixed bottom-4 left-4 z-50">
        <button 
          onClick={() => setCurrentView(v => v === 'home' ? 'project' : 'home')}
          className="bg-surface text-muted px-3 py-1 text-xs border border-muted rounded opacity-50 hover:opacity-100 transition-opacity cursor-pointer shadow-sm hover:shadow"
        >
          Toggle View (Dev)
        </button>
      </div>
      {currentView === 'home' ? <HomePage /> : <ProjectPage />}
    </>
  );
}

export default App;
