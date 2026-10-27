import { useState } from 'react';
import { kindNames, useLibrary, useStudioActions, VideoPreview } from '@/entities/library';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { Icon } from '@/shared/ui/icon';
import { field } from '@/widgets/library-browser';

const icons = { text: 'FileText', subtitle: 'Subtitles', audio: 'Volume2', video: 'Film' } as const;
export function WorkspaceAssetBrowser({
  projectId,
  selectedId,
  onSelect,
  onOpen,
}: {
  projectId: string;
  selectedId: string | null;
  onSelect: (asset: LibraryAsset) => void;
  onOpen: (asset: LibraryAsset) => void;
}) {
  const { state, workspaceId } = useLibrary();
  const actions = useStudioActions();
  const [search, setSearch] = useState('');
  const links = state.links.filter((link) => link.projectId === projectId);
  const assets = state.assets.filter(
    (asset) =>
      !state.hiddenAssets.includes(asset.id) && links.some((link) => link.assetId === asset.id),
  );

  const query = search.trim().toLocaleLowerCase('ru');
  const item = (asset: LibraryAsset) => (
    <li key={asset.id}>
      <button
        className={`flex w-full min-w-0 items-start gap-2 rounded-md border px-2 py-2 text-left text-sm hover:bg-surface-raised ${selectedId === asset.id ? 'border-border-strong bg-surface-active' : 'border-transparent'}`}
        aria-current={selectedId === asset.id ? 'page' : undefined}
        title={asset.name}
        onClick={() => onSelect(asset)}
        onContextMenu={(e) => {
          onSelect(asset);
          actions.context(e, { projectId, assetId: asset.id, workspaceId });
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') onOpen(asset);
          else actions.context(e, { projectId, assetId: asset.id, workspaceId });
        }}
        onDoubleClick={() => onOpen(asset)}
      >
        {asset.kind === 'video' ? (
          <VideoPreview asset={asset} compact />
        ) : (
          <Icon name={icons[asset.kind]} size={16} className="mt-0.5 shrink-0" />
        )}
        <span className="min-w-0">
          <span className="block break-words">{asset.name}</span>
          <span className="block text-xs text-muted">{kindNames[asset.kind]}</span>
        </span>
      </button>
    </li>
  );
  return (
    <aside
      aria-label="Материалы текущего проекта"
      className="flex h-full min-h-0 w-52 shrink-0 flex-col border-r border-border bg-surface xl:w-64"
    >
      <div className="shrink-0 space-y-3 border-b border-border p-3">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-sm font-semibold">
            Материалы <span className="text-xs font-normal text-muted">{assets.length}</span>
          </h2>
        </div>
        <input
          type="search"
          className={`${field} text-xs`}
          aria-label="Поиск в материалах проекта"
          placeholder="Найти материал…"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
      </div>
      <div
        className="min-h-0 flex-1 overflow-y-auto p-2"
        onContextMenu={(e) => actions.context(e, { projectId, assetId: null, workspaceId })}
      >
        <ul aria-label="Содержимое проекта" className="space-y-1">
          {query ? (
            assets
              .filter((asset) =>
                `${asset.name} ${asset.tags.join(' ')}`.toLocaleLowerCase('ru').includes(query),
              )
              .map(item)
          ) : (
            <>
              {Object.entries(kindNames).map(([kind, name]) => (
                <li key={kind}>
                  <details open>
                    <summary className="cursor-pointer px-2 py-2 text-xs text-muted">
                      {name}
                    </summary>
                    <ul>{assets.filter((a) => a.kind === kind).map(item)}</ul>
                  </details>
                </li>
              ))}
            </>
          )}
        </ul>
        {!assets.length && <p className="p-2 text-xs text-muted">В проекте пока нет материалов.</p>}
        {query &&
          !assets.some((asset) =>
            `${asset.name} ${asset.tags.join(' ')}`.toLocaleLowerCase('ru').includes(query),
          ) && <p className="p-2 text-xs text-muted">Материалы не найдены.</p>}
      </div>
    </aside>
  );
}
