import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import {
  StudioActionsContext,
  useLibrary,
  defaultTool,
  importFiles,
  type ActionTarget,
  type StudioAction,
} from '@/entities/library';
import { renameProject } from '@/entities/project';
import type { LibraryTool, MediaOperation } from '@/shared/api/contracts/library';
import { invoke } from '@/shared/api/tauri';
import { describeCommandError } from '@/shared/api/contracts/error';
import { PopupMenu, type MenuEntry } from '@/shared/ui/menu';
import { Dialog, DialogTitle, DialogClose } from '@/shared/ui/dialog';
import { LibraryCreateDialog, button, field } from '@/widgets/library-browser';
import { SettingsPage } from '@/pages/settings';
import { commandMenus, toolUnavailable } from '../model/commands';
import { useWindowChrome } from '../model/useWindowChrome';

const MenuContext = createContext<
  (name: 'file' | 'project' | 'tools', anchor: HTMLElement) => void
>(() => {});
type Modal = { kind: StudioAction | 'select-tool'; target: ActionTarget; tool?: LibraryTool };
export function StudioCommands({ children }: { children: ReactNode }) {
  const library = useLibrary();
  const { state, projects, view, workspaceId, mutate, refresh, openProject, openAsset } = library;
  const chrome = useWindowChrome();
  const [selection, setSelection] = useState<ActionTarget | null>(null);
  const [menu, setMenu] = useState<{
    items: MenuEntry[];
    x: number;
    y: number;
    label: string;
    returnFocus: HTMLElement | null;
  } | null>(null);
  const [modal, setModal] = useState<Modal | null>(null);
  const [settings, setSettings] = useState(false);
  const [value, setValue] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [operation, setOperation] = useState<MediaOperation | null>(null);
  const copying = useRef(false);
  useEffect(() => {
    const open = () => setSettings(true);
    window.addEventListener('auralis-open-settings', open);
    return () => window.removeEventListener('auralis-open-settings', open);
  }, []);
  const workspace = state.workspaces.find((w) => w.id === workspaceId);
  const fallback: ActionTarget =
    view === 'library'
      ? {
          projectId: state.browser.scope === 'project' ? state.browser.projectId : null,
          assetId: state.browser.selectedId,
        }
      : {
          projectId: workspace?.projectId ?? null,
          assetId:
            workspace?.tabs.find((t) => t.id === workspace.activeTab)?.assetId ??
            workspace?.assetId ??
            null,
          workspaceId,
        };
  const target =
    selection &&
    (view === 'library' ? !selection.workspaceId : selection.workspaceId === workspaceId)
      ? selection
      : fallback;
  const act = async (work: () => Promise<unknown>) => {
    setError('');
    setBusy(true);
    try {
      await work();
    } catch (e) {
      setError(describeCommandError(e));
    } finally {
      setBusy(false);
    }
  };
  const openTool = async (tool: LibraryTool, context: ActionTarget) => {
    const asset = state.assets.find((a) => a.id === context.assetId);
    if ((tool === 'voices' || (tool === 'results' && !asset)) && context.projectId) {
      await openProject(context.projectId);
      await mutate((s) => {
        const w = s.workspaces.find((w) => w.projectId === context.projectId)!;
        let t = w.tabs.find((t) => t.tool === tool && t.assetId === null);
        if (!t) {
          t = { id: crypto.randomUUID(), tool, assetId: null, draft: '{}' };
          w.tabs.push(t);
        }
        w.activeTab = t.id;
      });
      return;
    }
    if (asset) {
      if (toolUnavailable(tool, asset)) return;
      await openAsset(asset, tool, context.projectId);
      return;
    }
    setModal({ kind: 'select-tool', target: context, tool });
  };
  const run = (action: StudioAction, context: ActionTarget = target) => {
    setError('');
    if (action.startsWith('tool:')) {
      void act(() => openTool(action.slice(5) as LibraryTool, context));
      return;
    }
    if (action === 'settings') {
      setSettings(true);
      return;
    }
    if (action === 'exit') {
      void chrome.run('close');
      return;
    }
    if (action === 'open-project' && context.projectId) {
      void act(() => openProject(context.projectId!));
      return;
    }
    const asset = state.assets.find((a) => a.id === context.assetId);
    if (action === 'open-asset' && asset) {
      void act(() => openAsset(asset, defaultTool(asset), context.projectId));
      return;
    }
    setValue(
      action === 'rename-project'
        ? (projects.find((p) => p.id === context.projectId)?.title ?? '')
        : action === 'rename-asset'
          ? (asset?.name ?? '')
          : '',
    );
    setModal({ kind: action, target: context });
  };
  const closeMenu = useCallback(() => setMenu(null), []);
  const display = (items: MenuEntry[], x: number, y: number, label: string) =>
    setMenu({ items, x, y, label, returnFocus: document.activeElement as HTMLElement | null });
  const attach = async (context: ActionTarget, projectId: string) => {
    if (!context.assetId || copying.current) return;
    copying.current = true;
    const id = crypto.randomUUID();
    setOperation({
      id,
      projectId,
      assetId: context.assetId,
      status: 'copying',
      completedBytes: 0,
      totalBytes: 0,
      error: null,
    });
    let timer: ReturnType<typeof setInterval> | undefined;
    try {
      timer = setInterval(() => {
        void invoke('get_media_operation_cmd', { operationId: id })
          .then((op) => {
            if (op) setOperation(op);
          })
          .catch(() => {});
      }, 500);
      await invoke('attach_library_asset_cmd', {
        operationId: id,
        projectId,
        assetId: context.assetId,
        duplicate: false,
      });
      const deadline = Date.now() + 120_000;
      for (;;) {
        if (Date.now() > deadline)
          throw new Error(
            'Добавление ещё не завершено. Состояние сохранено; библиотека обновится после публикации.',
          );
        await refresh();
        const op = await invoke('get_media_operation_cmd', { operationId: id });
        if (!op || op.status === 'ready') break;
        if (['failed', 'cancelled'].includes(op.status)) {
          if (op.error) throw new Error(op.error);
          break;
        }
        await new Promise((r) => setTimeout(r, 500));
      }
      setModal(null);
    } finally {
      clearInterval(timer);
      copying.current = false;
      setOperation(null);
    }
  };
  const submit = async () => {
    if (!modal) return;
    const context = modal.target;
    await act(async () => {
      if (modal.kind === 'import')
        await importFiles({ projectId: context.projectId, folderId: null }, mutate, refresh);
      if (modal.kind === 'rename-project') {
        const project = projects.find((p) => p.id === context.projectId)!;
        await renameProject(project.id, value.trim(), project.revision);
        await refresh();
      }
      if (modal.kind === 'rename-asset')
        await mutate((s) => {
          const a = s.assets.find((a) => a.id === context.assetId);
          if (a) a.name = value.trim();
        });
      if (modal.kind === 'delete-project')
        await mutate((s) => {
          s.removedProjects.push(context.projectId!);
          s.links = s.links.filter((l) => l.projectId !== context.projectId);
          s.workspaces = s.workspaces.filter((w) => w.projectId !== context.projectId);
        });
      if (modal.kind === 'remove-asset')
        await mutate((s) => {
          if (context.projectId)
            s.links = s.links.filter(
              (l) => l.projectId !== context.projectId || l.assetId !== context.assetId,
            );
          else {
            s.hiddenAssets.push(context.assetId!);
            s.links = s.links.filter((l) => l.assetId !== context.assetId);
          }
        });
      if (modal.kind === 'attach') {
        await attach(context, value);
        return;
      }
      setModal(null);
    });
  };
  const projectName =
    projects.find((p) => p.id === (modal?.kind === 'attach' ? value : modal?.target.projectId))
      ?.title ?? (modal?.kind === 'attach' ? 'Выберите проект' : 'Без проекта');
  const createKind =
    modal?.kind === 'new-text'
      ? 'text'
      : modal?.kind === 'new-project'
        ? 'project'
        : modal?.kind === 'youtube'
          ? 'youtube'
          : null;
  return (
    <StudioActionsContext.Provider
      value={{
        select: setSelection,
        run,
        drop: (assetId, projectId) =>
          void act(() => attach({ assetId, projectId: null }, projectId)),
        context: (event, context) => {
          if (
            'key' in event &&
            event.key !== 'ContextMenu' &&
            !(event.shiftKey && event.key === 'F10')
          )
            return;
          event.preventDefault();
          event.stopPropagation();
          setSelection(context);
          const rect = event.currentTarget.getBoundingClientRect();
          const entries = commandMenus(
            context,
            state.assets.find((a) => a.id === context.assetId),
            run,
          );
          display(
            entries.context,
            'clientX' in event ? event.clientX : rect.left,
            'clientY' in event ? event.clientY : rect.bottom,
            'Действия',
          );
        },
      }}
    >
      <MenuContext.Provider
        value={(name, anchor) => {
          const rect = anchor.getBoundingClientRect();
          display(
            commandMenus(
              target,
              state.assets.find((a) => a.id === target.assetId),
              run,
            )[name],
            rect.left,
            rect.bottom,
            { file: 'Файл', project: 'Проект', tools: 'Инструменты' }[name],
          );
        }}
      >
        {children}
        {menu && <PopupMenu {...menu} close={closeMenu} />}
        {error && (
          <div
            role="alert"
            className="fixed bottom-12 left-4 z-50 max-w-lg rounded border border-danger bg-surface p-3 text-sm"
          >
            {error}
            <button className={`${button} ml-3`} onClick={() => setError('')}>
              Закрыть
            </button>
          </div>
        )}
        {operation && (
          <div
            role="status"
            className="fixed bottom-0 right-0 z-40 flex h-10 items-center gap-3 border-l border-border bg-surface px-3 text-xs"
          >
            Добавление в проект ·{' '}
            {operation.totalBytes
              ? Math.round((operation.completedBytes / operation.totalBytes) * 100)
              : 0}
            %
            <button
              onClick={() =>
                void invoke('cancel_media_operation_cmd', { operationId: operation.id })
              }
            >
              Отменить
            </button>
          </div>
        )}
        <Dialog open={settings} onOpenChange={setSettings}>
          <DialogTitle>Настройки</DialogTitle>
          <DialogClose aria-label="Закрыть настройки" />
          <div className="mt-4 max-h-[75vh] overflow-auto">
            {settings && <SettingsPage embedded />}
          </div>
        </Dialog>
        {createKind && modal && (
          <LibraryCreateDialog
            kind={createKind}
            destination={{ projectId: modal.target.projectId, folderId: null }}
            close={() => setModal(null)}
          />
        )}
        <Dialog
          open={!!modal && !createKind}
          onOpenChange={(open) => {
            if (!open && !busy) setModal(null);
          }}
        >
          <DialogTitle>
            {modal?.kind === 'select-tool'
              ? 'Выберите материал'
              : modal?.kind === 'attach'
                ? 'Добавить копию в проект'
                : modal?.kind === 'import'
                  ? 'Импорт файлов'
                  : modal?.kind === 'properties'
                    ? 'Свойства материала'
                    : modal?.kind.startsWith('rename')
                      ? 'Переименовать'
                      : 'Удаление'}
          </DialogTitle>
          <DialogClose aria-label="Закрыть диалог" disabled={busy} />
          <p className="my-4 text-sm text-muted">Назначение: {projectName}</p>
          {modal?.kind === 'select-tool' ? (
            <div className="max-h-80 space-y-1 overflow-auto">
              {state.assets
                .filter(
                  (a) =>
                    !state.hiddenAssets.includes(a.id) &&
                    !toolUnavailable(modal.tool!, a) &&
                    (!modal.target.projectId ||
                      state.links.some(
                        (l) => l.assetId === a.id && l.projectId === modal.target.projectId,
                      )),
                )
                .map((a) => (
                  <button
                    key={a.id}
                    className={`${button} w-full text-left`}
                    onClick={() =>
                      void act(async () => {
                        await openAsset(a, modal.tool, modal.target.projectId);
                        setModal(null);
                      })
                    }
                  >
                    {a.name}
                  </button>
                ))}
            </div>
          ) : modal?.kind === 'properties' ? (
            <p>
              {state.assets.find((a) => a.id === modal.target.assetId)?.name} ·{' '}
              {state.assets.find((a) => a.id === modal.target.assetId)?.tags.join(', ')}
            </p>
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void submit();
              }}
            >
              {modal?.kind.startsWith('rename') && (
                <input
                  className={field}
                  aria-label="Новое название"
                  value={value}
                  onChange={(e) => setValue(e.target.value)}
                  required
                  maxLength={200}
                />
              )}
              {modal?.kind === 'attach' && (
                <select
                  className={field}
                  aria-label="Проект назначения"
                  value={value}
                  onChange={(e) => setValue(e.target.value)}
                  required
                >
                  <option value="">Выберите проект</option>
                  {projects
                    .filter(
                      (p) =>
                        !state.standaloneOwners.includes(p.id) &&
                        !state.removedProjects.includes(p.id),
                    )
                    .map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.title}
                      </option>
                    ))}
                </select>
              )}
              {(modal?.kind === 'delete-project' || modal?.kind === 'remove-asset') && (
                <p className="text-sm">
                  Объект будет убран из текущего раздела. Сохранённые файлы останутся для
                  восстановления.
                </p>
              )}
              <div className="mt-5 flex justify-end gap-2">
                <button
                  type="button"
                  className={button}
                  disabled={busy}
                  onClick={() => setModal(null)}
                >
                  Отмена
                </button>
                <button className={button} disabled={busy}>
                  {busy
                    ? 'Выполняем…'
                    : modal?.kind === 'import'
                      ? 'Выбрать файлы…'
                      : 'Подтвердить'}
                </button>
              </div>
            </form>
          )}
        </Dialog>
      </MenuContext.Provider>
    </StudioActionsContext.Provider>
  );
}
export function StudioMenuBar() {
  const open = useContext(MenuContext);
  return (
    <nav
      role="menubar"
      aria-label="Главное меню"
      className="flex h-full shrink-0 items-center [app-region:no-drag]"
    >
      {(
        [
          ['file', 'Файл'],
          ['project', 'Проект'],
          ['tools', 'Инструменты'],
        ] as const
      ).map(([id, label]) => (
        <button
          key={id}
          role="menuitem"
          aria-haspopup="menu"
          className="h-full px-3 text-xs hover:bg-surface-raised focus:bg-surface-active"
          onClick={(e) => open(id, e.currentTarget)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              open(id, e.currentTarget);
            }
            if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
              e.preventDefault();
              const buttons = [...e.currentTarget.parentElement!.querySelectorAll('button')];
              buttons[
                (buttons.indexOf(e.currentTarget) + (e.key === 'ArrowRight' ? 1 : 2)) % 3
              ]?.focus();
            }
          }}
        >
          {label}
        </button>
      ))}
    </nav>
  );
}
