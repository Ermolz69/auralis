import { lazy, Suspense, useContext, useState } from 'react';
import { JobContext, isActiveJobStatus } from '@/entities/job';
import { useLibrary } from '@/entities/library';
import { JobQueueDrawer } from '@/widgets/app-shell';
import { button } from '@/widgets/library-browser';
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
const JobQueuePanel = lazy(() =>
  import('@/widgets/job-queue-panel').then((m) => ({ default: m.JobQueuePanel })),
);
export function StudioShell() {
  const { state, projects, view, setView, workspaceId, activateWorkspace, error, refresh } =
    useLibrary();
  const jobs = useContext(JobContext);
  const [queueOpen, setQueueOpen] = useState(false);
  const active = Object.values(jobs?.jobs ?? {}).filter((job) => isActiveJobStatus(job.status));
  const workspaceName = (id: string) => {
    const workspace = state.workspaces.find((w) => w.id === id);
    return workspace?.projectId
      ? projects.find((p) => p.id === workspace.projectId)?.title
      : state.assets.find((a) => a.id === workspace?.assetId)?.name;
  };
  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden bg-canvas text-text">
      <header className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-border bg-surface px-4 py-2">
        <div className="flex min-w-0 items-center gap-3">
          <button
            className="text-base font-bold"
            aria-label="Auralis — библиотека"
            onClick={() => setView('library')}
          >
            Auralis
          </button>
          <span className="text-muted">/</span>
          <button
            className={button}
            onClick={() => {
              setView('library');
              void refresh().catch(() => {});
            }}
            aria-current={view === 'library' ? 'page' : undefined}
          >
            Библиотека
          </button>
          {state.workspaces.length > 0 && (
            <select
              aria-label="Открытые рабочие пространства"
              className="max-w-64 border border-border bg-surface p-2 text-sm"
              value={view === 'workspace' ? (workspaceId ?? '') : ''}
              onChange={(e) => {
                if (e.target.value) activateWorkspace(e.target.value);
              }}
            >
              <option value="">Рабочие пространства…</option>
              {state.workspaces.map((w) => (
                <option key={w.id} value={w.id}>
                  {workspaceName(w.id) ?? 'Рабочее пространство'}
                </option>
              ))}
            </select>
          )}
        </div>
        <div className="flex gap-2">
          <button
            className={button}
            aria-label="Settings"
            onClick={() => setView(view === 'settings' ? 'library' : 'settings')}
          >
            Настройки
          </button>
          <button
            className={button}
            aria-expanded={queueOpen}
            aria-controls="global-job-queue"
            onClick={() => setQueueOpen((value) => !value)}
          >
            Очередь{active.length ? ` · ${active.length}` : ''}
          </button>
        </div>
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
          {queueOpen && jobs && (
            <JobQueueDrawer onClose={() => setQueueOpen(false)}>
              <JobQueuePanel className="h-[calc(100%-2.5rem)]" />
            </JobQueueDrawer>
          )}
        </Suspense>
      </main>
      <ProcessQueueStrip jobs={Object.values(jobs?.jobs ?? {})} />
    </div>
  );
}
