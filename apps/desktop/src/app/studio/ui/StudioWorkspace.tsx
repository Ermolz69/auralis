import { describeCommandError } from '@/shared/api/contracts/error';
import { useEffect, useState } from 'react';
import { useLibrary, toolNames } from '@/entities/library';
import type { LibraryTool, LibraryWorkspace } from '@/shared/api/contracts/library';
import { LibraryCreateDialog } from '@/widgets/library-browser';
import { importFiles } from '@/entities/library';
import { button, field } from '@/widgets/library-browser';
import { StudioToolPanel } from './StudioToolPanel';
import { panelKey, workspacePanels } from '../model/panels';
import { parseDraft } from '../model/tabDraft';
export function StudioWorkspace({ workspace }: { workspace: LibraryWorkspace }) {
  const { state, projects, mutate, refresh, openTool } = useLibrary();
  const activeAssetId = workspace.tabs.find((tab) => tab.id === workspace.activeTab)?.assetId;
  const [assetId, setAssetId] = useState(activeAssetId ?? workspace.assetId ?? '');
  useEffect(() => {
    setAssetId(activeAssetId ?? workspace.assetId ?? '');
  }, [activeAssetId, workspace.assetId]);
  const [dialog, setDialog] = useState(false);
  const [error, setError] = useState('');
  const assets = state.assets.filter(
    (a) =>
      !state.hiddenAssets.includes(a.id) &&
      (workspace.projectId
        ? state.links.some((l) => l.assetId === a.id && l.projectId === workspace.projectId)
        : a.id === workspace.assetId || !state.links.some((l) => l.assetId === a.id)),
  );
  const title = workspace.projectId
    ? projects.find((p) => p.id === workspace.projectId)?.title
    : state.assets.find((a) => a.id === workspace.assetId)?.name;
  const close = async (id: string) => {
    const tab = workspace.tabs.find((t) => t.id === id)!;
    const draft = parseDraft(tab.draft);
    const asset = state.assets.find((a) => a.id === tab.assetId);
    const shared = workspace.tabs.some((t) => t.id !== id && panelKey(t) === panelKey(tab));
    if (
      !shared &&
      draft.text &&
      draft.text !== (asset?.text ?? '') &&
      !window.confirm(
        'Закрыть вкладку и удалить её черновик? Сохраните текст новой версией, чтобы оставить правки в библиотеке.',
      )
    )
      return;
    await mutate((state) => {
      const item = state.workspaces.find((w) => w.id === workspace.id)!;
      const index = item.tabs.findIndex((t) => t.id === id);
      item.tabs = item.tabs.filter((t) => t.id !== id);
      if (item.activeTab === id) item.activeTab = item.tabs[Math.max(0, index - 1)]?.id ?? null;
    });
  };
  const destination = { projectId: workspace.projectId, folderId: null };
  const act = async (work: () => Promise<unknown>) => {
    setError('');
    try {
      await work();
    } catch (e) {
      setError(describeCommandError(e));
    }
  };
  return (
    <section
      className="flex h-full min-h-0 flex-col"
      aria-label={`Рабочее пространство ${title ?? ''}`}
    >
      <div className="shrink-0 border-b border-border p-4">
        <h1 className="mb-3 text-xl font-semibold">
          {title ?? 'Рабочее пространство'}
          <span className="ml-3 text-xs font-normal text-muted">
            {workspace.projectId ? 'Проект' : 'Самостоятельный материал'}
          </span>
        </h1>
        <div className="flex flex-wrap items-center gap-2">
          <select
            aria-label="Материал для инструмента"
            className={`${field} !w-auto max-w-72`}
            value={assetId}
            onChange={(e) => setAssetId(e.target.value)}
          >
            <option value="">Новый ввод / контекст проекта</option>
            {assets.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          {(Object.entries(toolNames) as [LibraryTool, string][]).map(([tool, name]) => (
            <button
              className={button}
              key={tool}
              onClick={() => void act(() => openTool(tool, assetId || null))}
            >
              {name}
            </button>
          ))}
          <button className={button} onClick={() => setDialog(true)}>
            + Текст
          </button>
          <button
            className={button}
            onClick={() => void act(() => importFiles(destination, mutate, refresh))}
          >
            Импорт
          </button>
        </div>
        {error && (
          <p role="alert" className="mt-2 text-sm">
            {error}
          </p>
        )}
      </div>
      <div
        role="tablist"
        aria-label="Вкладки рабочего пространства"
        className="flex shrink-0 overflow-x-auto border-b border-border bg-surface"
      >
        {workspace.tabs.map((tab) => (
          <div key={tab.id} className="flex shrink-0 items-center border-r border-border">
            <button
              role="tab"
              id={`tab-${tab.id}`}
              aria-controls={`panel-${workspace.id}-${panelKey(tab)}`}
              aria-selected={workspace.activeTab === tab.id}
              tabIndex={workspace.activeTab === tab.id ? 0 : -1}
              onKeyDown={(event) => {
                const keys = ['ArrowLeft', 'ArrowRight', 'Home', 'End'];
                if (!keys.includes(event.key)) return;
                event.preventDefault();
                const index = workspace.tabs.findIndex((t) => t.id === tab.id);
                const next =
                  event.key === 'Home'
                    ? 0
                    : event.key === 'End'
                      ? workspace.tabs.length - 1
                      : (index + (event.key === 'ArrowLeft' ? -1 : 1) + workspace.tabs.length) %
                        workspace.tabs.length;
                const target = workspace.tabs[next].id;
                document.getElementById(`tab-${target}`)?.focus();
                void mutate((state) => {
                  state.workspaces.find((w) => w.id === workspace.id)!.activeTab = target;
                }).catch(() => {});
              }}
              className={`${button} border-0 ${workspace.activeTab === tab.id ? 'bg-surface-active font-semibold' : ''}`}
              onClick={() =>
                void mutate((state) => {
                  state.workspaces.find((w) => w.id === workspace.id)!.activeTab = tab.id;
                }).catch(() => {})
              }
            >
              {toolNames[tab.tool]}
              {tab.assetId
                ? ` · ${state.assets.find((a) => a.id === tab.assetId)?.name ?? ''}`
                : ''}
            </button>
            <button
              className="p-2 text-muted hover:text-text"
              aria-label={`Закрыть ${toolNames[tab.tool]}`}
              onClick={() => void act(() => close(tab.id))}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {workspacePanels(workspace.tabs, workspace.activeTab).map((panel) => (
          <div
            key={panel.key}
            role="tabpanel"
            id={`panel-${workspace.id}-${panel.key}`}
            aria-labelledby={`tab-${panel.tab.id}`}
            hidden={!panel.active}
            className="h-full"
          >
            <StudioToolPanel tab={panel.tab} workspace={workspace} />
          </div>
        ))}
        {!workspace.tabs.length && (
          <div className="p-6">
            <p className="mb-4 text-muted">Выберите материал и откройте нужный инструмент.</p>
            <div className="grid gap-2 sm:grid-cols-2">
              {assets.map((asset) => (
                <button
                  className={`${button} text-left`}
                  key={asset.id}
                  onClick={() => {
                    setAssetId(asset.id);
                    void act(() =>
                      openTool(
                        asset.kind === 'video'
                          ? 'source'
                          : asset.kind === 'audio'
                            ? 'results'
                            : 'text',
                        asset.id,
                      ),
                    );
                  }}
                >
                  {asset.name}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
      {dialog && (
        <LibraryCreateDialog
          kind="text"
          destination={destination}
          inWorkspace
          close={() => setDialog(false)}
        />
      )}
    </section>
  );
}
