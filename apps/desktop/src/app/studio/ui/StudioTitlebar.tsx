import { useLibrary, useStudioActions } from '@/entities/library';
import { StudioMenuBar } from './StudioCommands';
import { Icon } from '@/shared/ui/icon';
import { useWindowChrome } from '../model/useWindowChrome';

export function StudioTitlebar() {
  const { state, projects, view, setView, workspaceId, refresh } = useLibrary();
  const windowChrome = useWindowChrome();
  const actions = useStudioActions();
  const workspace = state.workspaces.find((item) => item.id === workspaceId);
  const workspaceName = workspace?.projectId
    ? projects.find((project) => project.id === workspace.projectId)?.title
    : state.assets.find((asset) => asset.id === workspace?.assetId)?.name;
  const control =
    'flex h-full w-11 shrink-0 items-center justify-center text-muted [app-region:no-drag] hover:bg-surface-raised hover:text-text disabled:opacity-40';
  const maximizeLabel = windowChrome.maximized ? 'Восстановить окно' : 'Развернуть окно';
  return (
    <>
      <header
        aria-label="Панель окна"
        data-tauri-drag-region
        className="flex h-9 shrink-0 select-none items-center justify-between border-b border-border bg-surface pl-4 [app-region:drag]"
      >
        <div data-tauri-drag-region className="flex h-full min-w-0 flex-1 items-center gap-3">
          <button
            type="button"
            className="shrink-0 text-sm font-bold [app-region:no-drag]"
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
          <StudioMenuBar />
          {view === 'workspace' && workspaceName && (
            <>
              <span data-tauri-drag-region className="text-muted" aria-hidden="true">
                /
              </span>
              <span data-tauri-drag-region className="truncate text-sm" title={workspaceName}>
                {workspaceName}
              </span>
            </>
          )}
        </div>
        <div className="flex h-full shrink-0 items-center [app-region:no-drag]">
          <button
            type="button"
            className={control}
            aria-label="Настройки"
            title="Настройки"
            onClick={() => actions.run('settings')}
          >
            <Icon name="Settings" size={16} />
          </button>
          <button
            type="button"
            className={control}
            aria-label="Свернуть окно"
            title="Свернуть"
            disabled={!windowChrome.available}
            onClick={() => void windowChrome.run('minimize')}
          >
            <Icon name="Minus" size={14} strokeWidth={1.5} />
          </button>
          <button
            type="button"
            className={control}
            aria-label={maximizeLabel}
            title={maximizeLabel}
            disabled={!windowChrome.available}
            onClick={() => void windowChrome.run('toggleMaximize')}
          >
            <Icon name={windowChrome.maximized ? 'Copy' : 'Square'} size={13} strokeWidth={1.5} />
          </button>
          <button
            type="button"
            className={`${control} hover:bg-danger/15 hover:text-danger`}
            aria-label="Закрыть окно"
            title="Закрыть"
            disabled={!windowChrome.available}
            onClick={() => void windowChrome.run('close')}
          >
            <Icon name="X" size={16} strokeWidth={1.5} />
          </button>
        </div>
      </header>
      {windowChrome.error && (
        <p
          role="alert"
          className="shrink-0 border-b border-border bg-surface px-4 py-2 text-sm text-danger"
        >
          Не удалось управлять окном: {windowChrome.error}
        </p>
      )}
    </>
  );
}
