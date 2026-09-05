import { Badge } from '../../../shared/ui/badge';
import { Button } from '../../../shared/ui/button';
import { Icon } from '../../../shared/ui/icon';
import { Notice } from '../../../shared/ui/notice';
import { Progress } from '../../../shared/ui/progress';
import { type AppUpdatePhase, useAppUpdate } from '../model/appUpdateContext';

const status: Record<
  AppUpdatePhase,
  { label: string; variant: 'muted' | 'primary' | 'success' | 'warning' }
> = {
  idle: { label: 'Ready', variant: 'muted' },
  unsupported: { label: 'Installed builds', variant: 'muted' },
  checking: { label: 'Checking', variant: 'primary' },
  upToDate: { label: 'Up to date', variant: 'success' },
  available: { label: 'Update available', variant: 'primary' },
  downloading: { label: 'Installing', variant: 'primary' },
  restarting: { label: 'Restarting', variant: 'success' },
  error: { label: 'Action needed', variant: 'warning' },
};

export function AppUpdatePanel() {
  const updateState = useAppUpdate();
  const currentStatus = status[updateState.phase];
  const checking = updateState.phase === 'checking';
  const installing = updateState.phase === 'downloading';
  const available = updateState.phase === 'available' && updateState.update;

  return (
    <section
      aria-label="Application updates"
      className="rounded-md border border-border bg-surface-raised p-4"
    >
      <div className="flex items-start gap-3">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-border bg-surface text-primary">
          <Icon name="Download" size={15} />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold text-text">Application updates</h2>
          <p className="text-xs text-muted">Signed releases delivered through GitHub</p>
        </div>
        <Badge variant={currentStatus.variant} size="sm">
          {currentStatus.label}
        </Badge>
      </div>

      <div className="mt-4 space-y-3 border-t border-border/70 pt-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p className="text-xs text-subtle">Current version</p>
            <p className="font-mono text-sm font-semibold text-text">
              {updateState.currentVersion}
            </p>
          </div>

          {available ? (
            <Button
              size="sm"
              leftIcon={<Icon name="Download" size={14} />}
              onClick={() => void updateState.installUpdate()}
            >
              Download and install {updateState.update?.version}
            </Button>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              loading={checking || installing}
              disabled={
                checking ||
                installing ||
                updateState.phase === 'unsupported' ||
                updateState.phase === 'restarting'
              }
              leftIcon={<Icon name="RefreshCw" size={14} />}
              onClick={() => void updateState.checkForUpdates()}
            >
              {installing
                ? 'Installing update…'
                : updateState.phase === 'restarting'
                  ? 'Restarting…'
                  : 'Check for updates'}
            </Button>
          )}
        </div>

        {updateState.phase === 'upToDate' && (
          <Notice icon="CircleCheck" tone="accent" role="status">
            You are using the latest published version.
          </Notice>
        )}
        {updateState.phase === 'unsupported' && (
          <p className="text-xs text-subtle">
            Update checks are available in signed installed builds of Auralis.
          </p>
        )}
        {updateState.error && (
          <Notice icon="TriangleAlert" tone="warning" role="alert">
            {updateState.error}
          </Notice>
        )}
        {updateState.update && (
          <div className="rounded-md border border-primary/20 bg-primary-soft/35 p-3">
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <p className="text-sm font-semibold text-text">
                Version {updateState.update.version}
              </p>
              {updateState.update.date && (
                <time className="text-[11px] text-subtle" dateTime={updateState.update.date}>
                  {formatReleaseDate(updateState.update.date)}
                </time>
              )}
            </div>
            {updateState.update.notes && (
              <p className="mt-2 max-h-28 overflow-y-auto whitespace-pre-wrap text-xs leading-relaxed text-muted">
                {updateState.update.notes}
              </p>
            )}
          </div>
        )}
        {updateState.phase === 'downloading' && (
          <div className="space-y-1.5" aria-live="polite">
            <div className="flex justify-between text-[11px] text-subtle">
              <span>Downloading and verifying update</span>
              <span>{updateState.progress?.percent ?? '…'}%</span>
            </div>
            <Progress
              aria-label="Update download progress"
              value={updateState.progress?.percent ?? 0}
              indeterminate={updateState.progress?.percent === undefined}
            />
          </div>
        )}
      </div>
    </section>
  );
}

function formatReleaseDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(date);
}
