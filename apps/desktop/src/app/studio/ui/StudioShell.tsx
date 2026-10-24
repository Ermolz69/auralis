import { lazy, Suspense, useContext } from 'react';
import { JobContext } from '@/entities/job';
import { useLibrary } from '@/entities/library';
import { button } from '@/widgets/library-browser';
import { Icon } from '@/shared/ui/icon';
import { ProcessQueueStrip } from './ProcessQueueStrip';
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
  const { state, projects, view, setView, workspaceId, error, refresh } = useLibrary();
  const jobs = useContext(JobContext);
  const workspace = state.workspaces.find((item) => item.id === workspaceId);
  const workspaceName = workspace?.projectId
    ? projects.find((project) => project.id === workspace.projectId)?.title
    : state.assets.find((asset) => asset.id === workspace?.assetId)?.name;
  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden bg-canvas text-text">
      <header className="flex shrink-0 items-center justify-between gap-3 border-b border-border bg-surface px-4 py-2">
        <div className="flex min-w-0 flex-1 items-center gap-3">
          <button
            className="shrink-0 text-base font-bold"
            aria-label="Auralis — библиотека"
            title="Библиотека"
            aria-current={view === 'library' ? 'page' : undefined}
            onClick={() => {
              setView('library');
              void refresh().catch(() => {});
            }}
          >
            Auralis
          </button>
          {view === 'workspace' && workspaceName && (
            <>
              <span className="text-muted" aria-hidden="true">
                /
              </span>
              <span className="truncate text-sm" title={workspaceName}>
                {workspaceName}
              </span>
            </>
          )}
        </div>
        <button
          type="button"
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md text-muted hover:bg-surface-raised hover:text-text"
          aria-label="Настройки"
          title="Настройки"
          aria-pressed={view === 'settings'}
          onClick={() => setView(view === 'settings' ? 'library' : 'settings')}
        >
          <Icon name="Settings" size={20} />
        </button>
      </header>
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
