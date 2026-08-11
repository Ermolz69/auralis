import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './app/styles/index.css';
import App from './App.tsx';
import { ProjectProvider } from './entities/project';

import { NavigationProvider } from './shared/router';

import { AppJobProvider } from './app/providers';
import { Toaster } from './shared/ui/toast';
import { initializeColorTheme, ThemeProvider } from './shared/theme';

initializeColorTheme();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <NavigationProvider>
        <ProjectProvider>
          <AppJobProvider>
            <App />
            <Toaster />
          </AppJobProvider>
        </ProjectProvider>
      </NavigationProvider>
    </ThemeProvider>
  </StrictMode>,
);
