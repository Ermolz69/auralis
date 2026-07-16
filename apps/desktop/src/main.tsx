import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './app/styles/index.css';
import App from './App.tsx';
import { ProjectProvider } from './entities/project';
import { JobProvider } from './entities/job';
import { NavigationProvider } from './shared/router';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <NavigationProvider>
      <ProjectProvider>
        <JobProvider>
          <App />
        </JobProvider>
      </ProjectProvider>
    </NavigationProvider>
  </StrictMode>,
);
