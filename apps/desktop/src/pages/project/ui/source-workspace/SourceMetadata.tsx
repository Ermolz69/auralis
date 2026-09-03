import { formatDuration } from '@/entities/media';
import type { MediaMetadata } from '@/shared/api/contracts/media';
import { Icon } from '@/shared/ui/icon';

export function SourceMetadata({
  metadata,
  sourceValue,
}: {
  metadata: MediaMetadata;
  sourceValue: string;
}) {
  return (
    <section
      className="overflow-hidden rounded-md border border-border bg-surface-raised"
      aria-label="Source metadata"
    >
      <dl className="grid grid-cols-[96px_minmax(0,1fr)] text-xs">
        <Row label="Source">
          <span className="truncate font-mono" title={sourceValue}>
            {sourceValue}
          </span>
        </Row>
        <Row label="Duration">{formatDuration(metadata.durationMs)}</Row>
        <Row label="Container">{(metadata.container || 'unknown').toUpperCase()}</Row>
        <Row label="Resolution">
          {metadata.width && metadata.height ? `${metadata.width}×${metadata.height}` : '—'}
        </Row>
        <Row label="Video">
          {metadata.videoCodec?.toUpperCase() || '—'}
          {metadata.fps ? ` · ${metadata.fps.toFixed(2)} fps` : ''}
        </Row>
      </dl>
      <div className="border-t border-border px-3 py-2.5">
        <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-subtle">
          Audio tracks
        </h3>
        {metadata.audioTracks.length ? (
          metadata.audioTracks.map((track) => (
            <p key={track.streamIndex} className="font-mono text-[11px] text-subtle">
              {track.codec?.toUpperCase() || 'AUDIO'} · Track #{track.streamIndex} ·{' '}
              {track.channels ?? '?'} ch · {(track.language || 'und').toUpperCase()}
            </p>
          ))
        ) : (
          <p className="text-[11px] text-subtle">Аудиодорожки не найдены</p>
        )}
      </div>
      <details className="group border-t border-border">
        <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-[11px] text-subtle">
          <Icon
            name="ChevronRight"
            size={12}
            className="transition-transform group-open:rotate-90"
          />
          More details
        </summary>
        <div className="space-y-1.5 px-3 pb-3">
          {metadata.streams.map((stream) => (
            <p
              key={stream.index}
              className="rounded-sm border border-border/70 bg-surface px-2 py-1.5 font-mono text-[11px] text-subtle"
            >
              {stream.codecType} stream #{stream.index} ·{' '}
              {stream.codecName?.toUpperCase() || 'UNKNOWN'} {stream.codecLongName || ''}
            </p>
          ))}
        </div>
      </details>
    </section>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <>
      <dt className="border-t border-border/70 px-3 py-1.5 text-subtle first:border-t-0">
        {label}
      </dt>
      <dd className="flex min-w-0 items-center border-t border-border/70 px-3 py-1.5 font-mono text-muted">
        {children}
      </dd>
    </>
  );
}
