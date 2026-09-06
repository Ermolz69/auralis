import { lazy, StrictMode, Suspense } from 'react';
import { createRoot } from 'react-dom/client';
import './app/styles/index.css';
import App from './App.tsx';
import { ProjectProvider } from './entities/project';

import { NavigationProvider } from './shared/router';

import { AppJobProvider } from './app/providers';
import { Toaster } from './shared/ui/toast';
import { initializeColorTheme, ThemeProvider } from './shared/theme';

initializeColorTheme();

const NativeE2ERunner = __NATIVE_E2E__
  ? lazy(() =>
      import('./app/native-e2e').then(({ NativeE2ERunner }) => ({
        default: NativeE2ERunner,
      })),
    )
  : null;

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <NavigationProvider>
        <ProjectProvider>
          <AppJobProvider>
            <App />
            <Toaster />
            {NativeE2ERunner ? (
              <Suspense fallback={null}>
                <NativeE2ERunner />
              </Suspense>
            ) : null}
          </AppJobProvider>
        </ProjectProvider>
      </NavigationProvider>
    </ThemeProvider>
  </StrictMode>,
);
