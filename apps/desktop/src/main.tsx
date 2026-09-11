import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './app/styles/index.css';
import App from './App.tsx';
import { ProjectProvider } from './entities/project';

import { NavigationProvider } from './shared/router';

import { AppErrorBoundary, AppJobProvider, reportReactError } from './app/providers';
import { Toaster } from './shared/ui/toast';
import { initializeColorTheme } from './shared/theme';
import { NativeThemeProvider as ThemeProvider, installGlobalErrorReporting } from './app/providers';
import { NativeE2ERunner } from './app/native-e2e';

initializeColorTheme();
const stopGlobalErrorReporting = installGlobalErrorReporting();
if (import.meta.hot) import.meta.hot.dispose(stopGlobalErrorReporting);

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('Auralis root element is missing');

const application = __NATIVE_E2E__ ? (
  <NativeE2ERunner />
) : (
  <AppErrorBoundary>
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
  </AppErrorBoundary>
);

createRoot(rootElement, {
  onCaughtError: (error) => reportReactError('caught', error),
  onUncaughtError: (error) => reportReactError('uncaught', error),
  onRecoverableError: (error) => reportReactError('recoverable', error),
}).render(<StrictMode>{application}</StrictMode>);
