import { useLibrary } from '@/entities/library';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { button } from './controls';
export function AssetVersions({ asset }: { asset: LibraryAsset }) {
  const { state, openAsset } = useLibrary();
  const history: LibraryAsset[] = [];
  const visited = new Set<string>();
  let parent = asset.parentId;
  while (parent && !visited.has(parent)) {
    visited.add(parent);
    const item = state.assets.find((a) => a.id === parent);
    if (!item) break;
    history.push(item);
    parent = item.parentId;
  }
  const children = state.assets.filter(
    (a) => a.parentId === asset.id && !state.hiddenAssets.includes(a.id),
  );
  if (!history.length && !children.length) return null;
  return (
    <details className="mt-4 border-t border-border pt-3">
      <summary className="cursor-pointer text-sm">Версии и производные материалы</summary>
      <div className="mt-2 space-y-2">
        {[...history.reverse(), ...children].map((item) => (
          <button
            key={item.id}
            className={`${button} w-full text-left`}
            onClick={() => void openAsset(item).catch(() => {})}
          >
            {item.name}
          </button>
        ))}
      </div>
    </details>
  );
}
