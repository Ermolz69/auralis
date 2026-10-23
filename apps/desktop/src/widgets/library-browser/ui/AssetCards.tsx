import type { LibraryAsset } from '@/shared/api/contracts/library';
import { kindNames } from '@/entities/library';
import { Icon } from '@/shared/ui/icon';
type Props = {
  assets: LibraryAsset[];
  selectedId: string | null;
  select: (id: string) => void;
  open: (asset: LibraryAsset) => void;
};
const icons = { text: 'FileText', subtitle: 'Subtitles', audio: 'Volume2', video: 'Film' } as const;
export function AssetCards({ assets, selectedId, select, open }: Props) {
  return (
    <div
      className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-3"
      role="list"
      aria-label="Материалы библиотеки"
    >
      {assets.map((asset) => (
        <div key={asset.id} role="listitem">
          <button
            className={`flex h-48 w-full flex-col overflow-hidden border bg-surface text-left hover:bg-surface-raised ${selectedId === asset.id ? 'border-border-strong ring-1 ring-focus' : 'border-border'}`}
            aria-pressed={selectedId === asset.id}
            draggable
            onDragStart={(e) => e.dataTransfer.setData('application/x-auralis-asset', asset.id)}
            onClick={() => select(asset.id)}
            onDoubleClick={() => open(asset)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                open(asset);
              }
            }}
          >
            <div className="flex min-h-0 flex-1 items-center justify-center gap-2 overflow-hidden p-4 text-muted">
              <Icon name={icons[asset.kind]} size={28} />
              {asset.text !== null && (
                <p className="line-clamp-3 min-w-0 whitespace-pre-wrap text-xs">
                  {asset.text.slice(0, 140) || 'Пустой текст'}
                </p>
              )}
            </div>
            <div className="w-full border-t border-border p-3">
              <p className="truncate font-semibold" title={asset.name}>
                {asset.name}
              </p>
              <p className="text-xs text-muted">
                {kindNames[asset.kind]}
                {asset.format ? ` · ${asset.format.toUpperCase()}` : ''}
              </p>
              <p className="truncate text-xs text-subtle">{asset.tags.join(' · ')}</p>
            </div>
          </button>
        </div>
      ))}
    </div>
  );
}
