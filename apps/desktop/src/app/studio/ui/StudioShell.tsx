import { lazy, Suspense, useContext } from 'react';
import { JobContext } from '@/entities/job';
import { useLibrary } from '@/entities/library';
import { button } from '@/widgets/library-browser';
import { ProcessQueueStrip } from './ProcessQueueStrip';
import { StudioTitlebar } from './StudioTitlebar';
const LibraryBrowser = lazy(() =>
  import('@/widgets/library-browser').then((m) => ({ default: m.LibraryBrowser })),
);
const StudioWorkspace = lazy(() =>
  import('./StudioWorkspace').then((m) => ({ default: m.StudioWorkspace })),
);
const SettingsPage = lazy(() =>
  import('@/pages/settings').then((m) => ({ default: m.SettingsPage })),
);
export function StudioShell() {
  const { state, view, workspaceId, error, refresh } = useLibrary();
  const jobs = useContext(JobContext);
  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden bg-canvas text-text">
      <StudioTitlebar />
      {error && (
        <div role="alert" className="border-b border-border bg-surface p-3 text-sm">
          {error}
          <button className={`${button} ml-3`} onClick={() => void refresh().catch(() => {})}>
            Повторить подключение
          </button>
        </div>
      )}
      <main
        className="relative min-h-0 flex-1 overflow-hidden"
        aria-label={
          view === 'library'
            ? 'Библиотека'
            : view === 'workspace'
              ? 'Рабочее пространство'
              : 'Настройки'
        }
      >
        <Suspense
          fallback={
            <p role="status" className="p-5">
              Загрузка рабочего пространства…
            </p>
          }
        >
          <div hidden={view !== 'library'} className="h-full">
            <LibraryBrowser />
          </div>
          {state.workspaces.map((workspace) => (
            <div
              key={workspace.id}
              hidden={view !== 'workspace' || workspace.id !== workspaceId}
              className="h-full"
            >
              <StudioWorkspace workspace={workspace} />
            </div>
          ))}
          {view === 'settings' && <SettingsPage />}
        </Suspense>
      </main>
      <ProcessQueueStrip jobs={Object.values(jobs?.jobs ?? {})} />
    </div>
  );
}
