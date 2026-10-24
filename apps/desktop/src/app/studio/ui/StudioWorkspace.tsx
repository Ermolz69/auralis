import { describeCommandError } from '@/shared/api/contracts/error';
import { useEffect, useState } from 'react';
import { useLibrary, defaultTool } from '@/entities/library';
import type { LibraryWorkspace } from '@/shared/api/contracts/library';
import { LibraryCreateDialog } from '@/widgets/library-browser';
import { importFiles } from '@/entities/library';
import { button, field } from '@/widgets/library-browser';
import { StudioToolPanel } from './StudioToolPanel';
import { panelKey, workspacePanels } from '../model/panels';
import { parseDraft } from '../model/tabDraft';
import { WorkspaceAssetBrowser } from './WorkspaceAssetBrowser';
import { WorkspaceTabs } from './WorkspaceTabs';
import { WorkspaceToolMenu } from './WorkspaceToolMenu';
export function StudioWorkspace({ workspace }: { workspace: LibraryWorkspace }) {
  const { state, projects, mutate, refresh, openTool, view, workspaceId } = useLibrary();
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
  const selectedAsset = assets.find((asset) => asset.id === assetId);
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
    <section className="flex h-full min-h-0" aria-label={`Рабочее пространство ${title ?? ''}`}>
      {workspace.projectId && (
        <WorkspaceAssetBrowser
          projectId={workspace.projectId}
          selectedId={selectedAsset?.id ?? null}
          onSelect={(asset) => setAssetId(asset.id)}
          onOpen={(asset) => {
            setAssetId(asset.id);
            void act(() => openTool(defaultTool(asset), asset.id));
          }}
        />
      )}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {!workspace.projectId && (
          <div className="shrink-0 px-3 pt-3">
            <select
              aria-label="Материал для инструмента"
              className={`${field} !w-auto max-w-72`}
              value={assetId}
              onChange={(e) => setAssetId(e.target.value)}
            >
              <option value="">Новый ввод</option>
              {assets.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
          </div>
        )}
        {error && (
          <p role="alert" className="px-3 pt-3 text-sm">
            {error}
          </p>
        )}
        <WorkspaceTabs
          workspace={workspace}
          close={(id) => void act(() => close(id))}
          add={
            view === 'workspace' &&
            workspaceId === workspace.id && (
              <WorkspaceToolMenu
                materialName={selectedAsset?.name}
                onTool={(tool) => void act(() => openTool(tool, selectedAsset?.id ?? null))}
                onCreateText={() => setDialog(true)}
                onImport={() => void act(() => importFiles(destination, mutate, refresh))}
              />
            )
          }
        />
        <div className="mx-3 mb-3 min-h-0 flex-1 overflow-auto rounded-lg border border-border bg-bg">
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
              <p className="mb-4 text-muted">
                {workspace.projectId
                  ? 'Выберите материал слева и нажмите «+» рядом с вкладками, чтобы открыть инструмент.'
                  : 'Нажмите «+» рядом с вкладками и выберите инструмент.'}
              </p>
              {!workspace.projectId && (
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
              )}
            </div>
          )}
        </div>
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
