import type { MediaSource } from '@/shared/api/contracts/media';
import { Icon } from '@/shared/ui/icon';
import { ImportLocalMediaButton } from '@/features/import-local-media';
import { PasteYoutubeLink } from '@/features/paste-youtube-link';

export function SourceSelector({
  source,
  sourceValue,
}: {
  source: MediaSource | null;
  sourceValue: string;
}) {
  if (!source)
    return (
      <div className="space-y-3">
        <div className="rounded-md border border-border bg-surface-raised p-4">
          <PasteYoutubeLink />
        </div>
        <div className="flex items-center gap-3" aria-hidden="true">
          <span className="h-px flex-1 bg-border/70" />
          <span className="text-[11px] text-subtle">или</span>
          <span className="h-px flex-1 bg-border/70" />
        </div>
        <div className="rounded-md border border-border bg-surface-raised p-4">
          <ImportLocalMediaButton />
        </div>
      </div>
    );
  const remote = source.kind === 'youtubeUrl' || source.kind === 'remoteUrl';
  return (
    <section
      className="rounded-md border border-border bg-surface-raised p-4"
      aria-label="Connected video source"
    >
      <div className="flex items-center gap-3">
        <span className="flex h-8 w-8 items-center justify-center rounded-sm bg-surface-active text-primary">
          <Icon name={remote ? 'Video' : 'Film'} size={15} />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="text-xs font-semibold text-muted">
            {remote ? 'Video URL' : 'Локальный видеофайл'}
          </h2>
          <p className="truncate font-mono text-[11px] text-subtle" title={sourceValue}>
            {sourceValue}
          </p>
        </div>
      </div>
      {remote && (
        <input
          aria-label="Video URL"
          type="url"
          value={sourceValue}
          readOnly
          className="mt-3 h-9 w-full rounded-md border border-primary/45 bg-surface px-3 text-xs text-text outline-none"
        />
      )}
    </section>
  );
}
