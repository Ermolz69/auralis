import { AppUpdateNotifier, AppUpdateProvider } from './features/app-update';
import { LibraryProvider, useLibrary } from './entities/library';
import { StudioShell } from './app/studio';
import { lazy, Suspense, useState } from 'react';

const GuidedWorkspace = lazy(() =>
  import('./app/workflow').then((module) => ({ default: module.GuidedWorkspace })),
);

export default function App() {
  return (
    <AppUpdateProvider>
      <LibraryProvider>
        <Studio />
      </LibraryProvider>
    </AppUpdateProvider>
  );
}
function Studio() {
  const [guided, setGuided] = useState(false);
  const { refresh } = useLibrary();
  return (
    <>
      <AppUpdateNotifier
        onOpen={() =>
          window.dispatchEvent(
            new Event(guided ? 'auralis-workflow-settings' : 'auralis-open-settings'),
          )
        }
      />
      {guided ? (
        <Suspense fallback={<p role="status">Загружаем проекты…</p>}>
          <GuidedWorkspace
            onOpenLibrary={() => {
              setGuided(false);
              void refresh().catch(() => {});
            }}
          />
        </Suspense>
      ) : (
        <StudioShell onOpenWorkflow={() => setGuided(true)} />
      )}
    </>
  );
}
