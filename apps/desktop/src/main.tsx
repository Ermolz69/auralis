import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './app/styles/index.css';
import App from './App.tsx';
import { ProjectProvider } from './entities/project';
import { NavigationProvider } from './app/router';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <NavigationProvider>
      <ProjectProvider>
        <App />
      </ProjectProvider>
    </NavigationProvider>
  </StrictMode>,
);
