import { useEffect, useState, useRef } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { invoke } from '@/shared/api/tauri';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { describeCommandError } from '@/shared/api/contracts/error';
import { Icon } from '@/shared/ui/icon';
let queue: Promise<unknown> = Promise.resolve();
const cache = new Map<string, Promise<string | null>>();
function preview(asset: LibraryAsset) {
  const key = asset.artifactId ?? `youtube:${asset.ownerProjectId}`;
  let promise = cache.get(key);
  if (!promise) {
    promise = queue.then(async () =>
      asset.artifactId
        ? convertFileSrc(await invoke('get_video_preview_cmd', { artifactId: asset.artifactId }))
        : (await invoke('get_project_avatar_cmd', { projectId: asset.ownerProjectId })).dataUrl,
    );
    queue = promise.catch(() => {
      cache.delete(key);
    });
    cache.set(key, promise);
    if (cache.size > 256) cache.delete(cache.keys().next().value!);
  }
  return promise;
}
export function VideoPreview({
  asset,
  compact = false,
}: {
  asset: LibraryAsset;
  compact?: boolean;
}) {
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const element = useRef<HTMLSpanElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const observer = new IntersectionObserver(
      (items) => {
        if (items.some((item) => item.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: '160px' },
    );
    if (element.current) observer.observe(element.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!visible) return;
    let active = true;
    setUrl(null);
    setError(null);
    void preview(asset)
      .then((value) => {
        if (active) {
          setUrl(value);
          if (!value) setError('Миниатюра источника недоступна');
        }
      })
      .catch((reason) => {
        if (active) setError(describeCommandError(reason));
      });
    return () => {
      active = false;
    };
  }, [visible, asset.id, asset.artifactId, asset.ownerProjectId]);
  return (
    <span
      ref={element}
      className={`flex items-center justify-center overflow-hidden bg-canvas ${compact ? 'h-9 w-14 shrink-0 rounded' : 'h-full w-full'}`}
      title={error ?? asset.name}
    >
      {url ? (
        <img
          src={url}
          alt={`Превью: ${asset.name}`}
          className="h-full w-full object-cover"
          onError={() => {
            setUrl(null);
            setError('Миниатюра источника недоступна');
          }}
        />
      ) : (
        <span className="flex flex-col items-center gap-2 text-muted">
          <Icon name="Film" size={compact ? 16 : 28} />
          {!compact && (
            <span className="text-xs">{error ? 'Превью недоступно' : 'Загружаем превью…'}</span>
          )}
        </span>
      )}
    </span>
  );
}
