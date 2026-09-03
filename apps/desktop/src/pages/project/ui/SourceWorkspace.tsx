import { useMemo, type ReactNode } from 'react';
import { useProjectJobs } from '@/entities/job';
import { formatDuration } from '@/entities/media';
import { useProjectContext } from '@/entities/project';
import type { MediaSource } from '@/shared/api/contracts/media';
import { Icon } from '@/shared/ui/icon';
import { ImportLocalMediaButton } from '@/features/import-local-media';
import { PasteYoutubeLink } from '@/features/paste-youtube-link';

export function SourceWorkspace() {
  const { project } = useProjectContext();
  const { jobs } = useProjectJobs(project?.id ?? null);
  const source = project?.source ?? null;
  const metadata = project?.metadata ?? null;
  const sourceValue = getSourceValue(source);
  const isRemoteSource = source?.kind === 'youtubeUrl' || source?.kind === 'remoteUrl';
  const activity = useMemo(() => {
    if (!project) return [];

    const projectJobs = [...jobs].sort((left, right) =>
      left.createdAt.localeCompare(right.createdAt),
    );
    const items: Array<{
      timestamp: string;
      time: string;
      text: string;
      tone?: 'success' | 'danger';
    }> = [
      {
        timestamp: project.createdAt,
        time: formatTime(project.createdAt),
        text: 'Проект создан',
      },
    ];

    if (source) {
      items.push({
        timestamp: project.updatedAt,
        time: formatTime(project.updatedAt),
        text: `Источник подключён: ${sourceValue}`,
        tone: 'success',
      });
    }

    if (metadata) {
      const resolution =
        metadata.width && metadata.height ? ` ${metadata.width}×${metadata.height}` : '';
      items.push({
        timestamp: project.updatedAt,
        time: formatTime(project.updatedAt),
        text: `Метаданные загружены: ${(metadata.container || 'media').toUpperCase()}${resolution}`,
        tone: 'success',
      });
    }

    items.push({
      timestamp: project.updatedAt,
      time: formatTime(project.updatedAt),
      text: `Статус проекта: ${formatProjectState(project.status)}`,
      tone: project.status === 'failed' ? 'danger' : undefined,
    });

    for (const job of projectJobs) {
      items.push({
        timestamp: job.updatedAt,
        time: formatTime(job.updatedAt),
        text: `${job.title}: ${
          job.status === 'failed'
            ? job.error || formatJobState(job.status)
            : job.progress.message || formatJobState(job.status)
        }`,
        tone:
          job.status === 'failed' ? 'danger' : job.status === 'completed' ? 'success' : undefined,
      });
    }

    return items.sort((left, right) => left.timestamp.localeCompare(right.timestamp));
  }, [jobs, metadata, project, source, sourceValue]);

  return (
    <section
      className="h-full min-h-0 overflow-y-auto px-4 py-5 sm:px-6"
      aria-label="Video source configuration"
      data-testid="source-workspace"
    >
      <div className="space-y-5">
        <div className="space-y-3">
          {!source && (
            <div className="rounded-md border border-border bg-surface-raised p-4">
              <PasteYoutubeLink />
            </div>
          )}
          {source && (
            <div className="flex flex-col gap-1.5">
              <label htmlFor="project-source-url" className="text-xs font-medium text-muted">
                YouTube URL
              </label>
              <div className="relative">
                <input
                  id="project-source-url"
                  type="url"
                  value={isRemoteSource ? sourceValue : ''}
                  readOnly
                  placeholder="https://www.youtube.com/watch?v=..."
                  className="h-[43px] w-full rounded-md border border-primary/45 bg-surface-raised px-3 pr-10 text-[13px] font-medium text-text outline-none transition-colors placeholder:text-subtle focus:border-primary focus:ring-2 focus:ring-primary/20"
                />
                {isRemoteSource && (
                  <Icon
                    name="X"
                    size={13}
                    color="muted"
                    className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 opacity-55"
                    aria-hidden="true"
                  />
                )}
              </div>
              <p className="text-[11px] text-subtle">Только для личного локального использования</p>
            </div>
          )}

          <div className="flex items-center gap-3" aria-hidden="true">
            <span className="h-px flex-1 bg-border/70" />
            <span className="text-[11px] font-medium text-subtle">или</span>
            <span className="h-px flex-1 bg-border/70" />
          </div>

          {!source ? (
            <div className="rounded-md border border-border bg-surface-raised p-4">
              <ImportLocalMediaButton />
            </div>
          ) : (
            <div
              aria-disabled="true"
              className="flex min-h-[68px] items-center gap-4 rounded-md border border-border/60 px-4 py-3.5 opacity-45"
            >
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-sm bg-surface-active text-subtle">
                <Icon name="Film" size={15} />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[13px] font-semibold text-muted">
                  {source && !isRemoteSource ? sourceValue : 'Выбрать локальный MP4'}
                </span>
                <span className="mt-0.5 block text-[11px] text-subtle">
                  {source && !isRemoteSource
                    ? 'Локальный источник уже подключён'
                    : 'Нажмите или перетащите файл'}
                </span>
              </span>
              <span className="shrink-0 font-mono text-[11px] text-subtle">.mp4</span>
            </div>
          )}
        </div>

        {metadata && (
          <div className="overflow-hidden rounded-md border border-border bg-surface-raised">
            <div className="grid grid-cols-[84px_minmax(0,1fr)] text-xs sm:grid-cols-[96px_minmax(0,1fr)]">
              <MetadataRow label="Source" first>
                <span className="truncate font-mono text-muted" title={sourceValue}>
                  {sourceValue}
                </span>
              </MetadataRow>
              <MetadataRow label="Duration">
                <span className="font-mono text-muted">{formatDuration(metadata.durationMs)}</span>
              </MetadataRow>
              <MetadataRow label="Container">
                <span className="rounded-xs border border-border-strong bg-surface-active px-1.5 py-px font-mono text-[11px] text-muted">
                  {(metadata.container || 'unknown').toUpperCase()}
                  {metadata.videoCodec ? ` · ${metadata.videoCodec.toUpperCase()}` : ''}
                </span>
              </MetadataRow>
              <MetadataRow label="Resolution">
                <span className="font-mono text-muted">
                  {metadata.width && metadata.height ? `${metadata.width}×${metadata.height}` : '—'}
                </span>
              </MetadataRow>
              <MetadataRow label="Video">
                <span className="font-mono text-muted">
                  {metadata.videoCodec?.toUpperCase() || '—'}
                  {metadata.fps ? ` · ${metadata.fps.toFixed(2)} fps` : ''}
                </span>
              </MetadataRow>
            </div>

            <div className="border-t border-border px-3 py-2.5">
              <p className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-subtle">
                Audio tracks
              </p>
              {metadata.audioTracks.length > 0 ? (
                <div className="space-y-1.5">
                  {metadata.audioTracks.map((track) => (
                    <div
                      key={track.streamIndex}
                      className="flex min-w-0 flex-wrap items-center gap-2 font-mono text-[11px] text-subtle"
                    >
                      <span className="rounded-xs border border-border-strong bg-surface-active px-1.5 py-px text-muted">
                        {track.codec?.toUpperCase() || 'AUDIO'}
                      </span>
                      <span>Track #{track.streamIndex}</span>
                      <span aria-hidden="true">·</span>
                      <span>{track.channels ?? '?'} ch</span>
                      <span aria-hidden="true">·</span>
                      <span>{track.sampleRate ?? '?'} Hz</span>
                      <span aria-hidden="true">·</span>
                      <span>{(track.language || 'und').toUpperCase()}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="font-mono text-[11px] text-subtle">Аудиодорожки не найдены</p>
              )}
            </div>

            <details className="group border-t border-border">
              <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-[11px] text-subtle transition-colors hover:bg-surface-hover hover:text-muted focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus">
                <Icon
                  name="ChevronRight"
                  size={12}
                  className="transition-transform group-open:rotate-90"
                />
                More details
              </summary>
              <div className="space-y-1.5 px-3 pb-3">
                {metadata.streams.map((stream) => (
                  <div
                    key={stream.index}
                    className="flex min-w-0 flex-wrap items-center gap-2 rounded-sm border border-border/70 bg-surface px-2 py-1.5 font-mono text-[11px] text-subtle"
                  >
                    <span className="text-muted">
                      {stream.codecType} stream #{stream.index}
                    </span>
                    <span aria-hidden="true">·</span>
                    <span>{stream.codecName?.toUpperCase() || 'UNKNOWN'}</span>
                    {stream.codecLongName && (
                      <span className="truncate">{stream.codecLongName}</span>
                    )}
                  </div>
                ))}
              </div>
            </details>
          </div>
        )}

        <div>
          <h2 className="mb-2 text-xs font-semibold text-muted">Лог</h2>
          <div className="min-h-[132px] rounded-md border border-border bg-canvas px-3 py-2.5 font-mono text-[11px] leading-5">
            {activity.length > 0 ? (
              activity.map((item, index) => (
                <div key={`${item.time}-${index}`} className="flex min-w-0 gap-3">
                  <span className="shrink-0 text-subtle">{item.time}</span>
                  <span
                    className={`min-w-0 break-words ${
                      item.tone === 'danger'
                        ? 'text-danger'
                        : item.tone === 'success'
                          ? 'text-text'
                          : 'text-muted'
                    }`}
                  >
                    {item.text}
                  </span>
                </div>
              ))
            ) : (
              <p className="text-subtle">Событий пока нет</p>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

function MetadataRow({
  label,
  first = false,
  children,
}: {
  label: string;
  first?: boolean;
  children: ReactNode;
}) {
  const borderClass = first ? '' : 'border-t border-border/70';

  return (
    <>
      <div className={`px-3 py-1.5 text-subtle ${borderClass}`}>{label}</div>
      <div className={`flex min-w-0 items-center px-3 py-1.5 ${borderClass}`}>{children}</div>
    </>
  );
}

function getSourceValue(source: MediaSource | null): string {
  if (!source) return '—';
  if (source.kind === 'youtubeUrl' || source.kind === 'remoteUrl') return source.url;
  if (source.kind === 'externalLocalFile') return source.path;
  return source.originalFilename;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '--:--:--';
  return date.toLocaleTimeString('ru-RU', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function formatJobState(status: string): string {
  switch (status) {
    case 'pending':
      return 'ожидает запуска';
    case 'running':
      return 'выполняется';
    case 'completed':
      return 'завершено';
    case 'failed':
      return 'ошибка';
    case 'cancelled':
      return 'отменено';
    default:
      return status;
  }
}

function formatProjectState(status: string): string {
  switch (status) {
    case 'draft':
      return 'черновик';
    case 'source_imported':
      return 'источник импортирован';
    case 'ready_for_processing':
      return 'готов к обработке';
    case 'processing':
      return 'обработка';
    case 'completed':
      return 'завершён';
    case 'failed':
      return 'ошибка';
    case 'cancelled':
      return 'отменён';
    default:
      return status;
  }
}
