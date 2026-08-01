import { Card, CardContent } from '../../../shared/ui/card';
import { Badge } from '../../../shared/ui/badge';
import { Icon } from '../../../shared/ui/icon';
import { Button } from '../../../shared/ui/button';
import { useTranscript } from '@/entities/transcript';
import { useProjectContext } from '@/entities/project';

export const TranscriptEditor = () => {
  const { projectId, project } = useProjectContext();
  const { transcript, isLoading, error, refetch } = useTranscript(projectId);
  const sourceKind = project?.source?.kind;
  const isLocalSource = sourceKind === 'managedLocalFile' || sourceKind === 'externalLocalFile';

  return (
    <section className="flex-1 p-6 flex flex-col gap-4 overflow-hidden min-h-0" aria-label="Transcript viewer">
      <div className="flex shrink-0 items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold text-text">Transcript</h2>
          <p className="text-sm text-muted">
            Read-only subtitle text from the current pipeline.
          </p>
        </div>
        <Badge variant="muted" size="sm">Read-only</Badge>
      </div>
      <Card className="flex-1 overflow-hidden flex flex-col shadow-sm">
        <CardContent className="flex-1 p-6 overflow-y-auto min-h-0">
          {!projectId ? (
            <div className="h-full flex flex-col items-center justify-center text-center">
              <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
              <p className="text-lg font-medium text-text">No project selected</p>
              <p className="text-sm text-muted mt-2 max-w-sm">
                Paste a YouTube link and start a project to generate a transcript automatically.
              </p>
            </div>
          ) : isLoading ? (
            <div className="h-full flex flex-col items-center justify-center text-center" role="status" aria-live="polite">
              <p className="text-lg font-medium text-text animate-pulse">Loading transcript...</p>
              <p className="text-sm text-muted mt-2 max-w-sm">
                Checking whether subtitle text is available for this project.
              </p>
            </div>
          ) : error ? (
            <div className="h-full flex flex-col items-center justify-center text-center" role="alert">
              <Icon name="CircleAlert" size="lg" className="text-danger mb-4" />
              <p className="text-lg font-medium text-text">Error loading transcript</p>
              <p className="text-sm text-danger mt-2 max-w-sm">{error}</p>
              <Button variant="secondary" size="sm" className="mt-4" onClick={refetch}>
                Retry
              </Button>
            </div>
          ) : !transcript || transcript.segments.length === 0 ? (
            isLocalSource ? (
              <div className="h-full flex flex-col items-center justify-center text-center">
                <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
                <p className="text-lg font-medium text-text">Transcript Unavailable</p>
                <p className="text-sm text-muted mt-2 max-w-sm">
                  Local media import is complete, but automatic transcription for local files is
                  not supported in this version.
                </p>
              </div>
            ) : (
              <div className="h-full flex flex-col items-center justify-center text-center" role="status" aria-live="polite">
                <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
                <p className="text-lg font-medium text-text">
                  Waiting for transcript generation...
                </p>
                <p className="text-sm text-muted mt-2 max-w-sm">
                  The mock pipeline is currently running. The transcript will appear here when
                  ready.
                </p>
              </div>
            )
          ) : (
            <div className="flex flex-col gap-3" aria-label="Read-only transcript segments">
              {transcript.segments.map((line, idx) => (
                <p key={idx} className="rounded border border-muted/30 bg-bg p-3">
                  <Badge variant="primary" size="sm" className="mr-2 font-mono">
                    [{Math.floor(line.startMs / 1000)}s - {Math.floor(line.endMs / 1000)}s]
                  </Badge>{' '}
                  {line.sourceText}
                </p>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
};
