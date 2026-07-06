import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './app/styles/index.css';
import App from './App.tsx';
import { ProjectProvider } from './entities/project';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ProjectProvider>
      <App />
    </ProjectProvider>
  </StrictMode>,
);
