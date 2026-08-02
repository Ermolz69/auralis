import { Card, CardContent } from '../../../shared/ui/card';
import { Badge } from '../../../shared/ui/badge';
import { Icon } from '../../../shared/ui/icon';
import { Button } from '../../../shared/ui/button';
import type { MediaSourceKind } from '@/entities/media';
import type { Transcript } from '@/entities/transcript';

type TranscriptPanelViewProps = {
  projectId: string | null;
  sourceKind: MediaSourceKind | null;
  transcript: Transcript | null;
  isLoading: boolean;
  error: string | null;
  activeJobStatus: string | null;
  onRefetch: () => void;
};

export function TranscriptPanelView({
  projectId,
  sourceKind,
  transcript,
  isLoading,
  error,
  activeJobStatus,
  onRefetch,
}: TranscriptPanelViewProps) {
  const isLocalSource = sourceKind === 'managedLocalFile' || sourceKind === 'externalLocalFile';

  return (
    <section
      className="flex-1 p-6 flex flex-col gap-4 overflow-hidden min-h-0"
      aria-label="Transcript viewer"
    >
      <div className="flex shrink-0 items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold text-text">Transcript</h2>
          <p className="text-sm text-muted">Read-only subtitle text from the current project.</p>
        </div>
        <Badge variant="muted" size="sm">
          Read-only
        </Badge>
      </div>
      <Card className="flex-1 overflow-hidden flex flex-col shadow-sm">
        <CardContent
          className="flex-1 p-6 overflow-y-auto min-h-0"
          tabIndex={0}
          aria-label="Transcript content"
        >
          <TranscriptBody
            projectId={projectId}
            isLoading={isLoading}
            error={error}
            transcript={transcript}
            isLocalSource={isLocalSource}
            activeJobStatus={activeJobStatus}
            onRefetch={onRefetch}
          />
        </CardContent>
      </Card>
    </section>
  );
}

type TranscriptBodyProps = Pick<
  TranscriptPanelViewProps,
  'projectId' | 'isLoading' | 'error' | 'transcript' | 'activeJobStatus' | 'onRefetch'
> & {
  isLocalSource: boolean;
};

function TranscriptBody({
  projectId,
  isLoading,
  error,
  transcript,
  isLocalSource,
  activeJobStatus,
  onRefetch,
}: TranscriptBodyProps) {
  if (!projectId) return <NoProjectState />;
  if (isLoading) return <LoadingState />;
  if (error) return <ErrorState error={error} onRefetch={onRefetch} />;
  if (!transcript || transcript.segments.length === 0) {
    return isLocalSource ? (
      <LocalUnavailableState />
    ) : (
      <WaitingState activeJobStatus={activeJobStatus} />
    );
  }
  return <ReadOnlySegments transcript={transcript} />;
}

function NoProjectState() {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center">
      <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
      <p className="text-lg font-medium text-text">No project selected</p>
      <p className="text-sm text-muted mt-2 max-w-sm">
        Open a project to view transcript availability.
      </p>
    </div>
  );
}

function LoadingState() {
  return (
    <div
      className="h-full flex flex-col items-center justify-center text-center"
      role="status"
      aria-live="polite"
    >
      <p className="text-lg font-medium text-text animate-pulse">Loading transcript...</p>
      <p className="text-sm text-muted mt-2 max-w-sm">
        Checking whether subtitle text is available for this project.
      </p>
    </div>
  );
}

function ErrorState({ error, onRefetch }: { error: string; onRefetch: () => void }) {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center" role="alert">
      <Icon name="CircleAlert" size="lg" className="text-danger mb-4" />
      <p className="text-lg font-medium text-text">Error loading transcript</p>
      <p className="text-sm text-danger mt-2 max-w-sm">{error}</p>
      <Button variant="secondary" size="sm" className="mt-4" onClick={onRefetch}>
        Retry loading
      </Button>
    </div>
  );
}

function LocalUnavailableState() {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center">
      <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
      <p className="text-lg font-medium text-text">Transcript unavailable</p>
      <p className="text-sm text-muted mt-2 max-w-sm">
        Local media import is complete, but automatic transcription for local files is not supported
        in this version.
      </p>
    </div>
  );
}

function WaitingState({ activeJobStatus }: { activeJobStatus: string | null }) {
  return (
    <div
      className="h-full flex flex-col items-center justify-center text-center"
      role="status"
      aria-live="polite"
    >
      <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
      <p className="text-lg font-medium text-text">Waiting for subtitles</p>
      <p className="text-sm text-muted mt-2 max-w-sm">
        {activeJobStatus
          ? `Linked operation: ${activeJobStatus}. The transcript will appear here when ready.`
          : 'Start subtitle import for a supported source. The transcript will appear here when ready.'}
      </p>
    </div>
  );
}

function ReadOnlySegments({ transcript }: { transcript: Transcript }) {
  return (
    <div
      className="mx-auto flex w-full max-w-3xl flex-col gap-3"
      aria-label="Read-only transcript segments"
    >
      {transcript.segments.map((line) => (
        <div
          key={line.id}
          className="rounded border border-muted/30 bg-bg p-3 leading-relaxed break-words"
        >
          <Badge variant="primary" size="sm" className="mr-2 font-mono">
            [{Math.floor(line.startMs / 1000)}s - {Math.floor(line.endMs / 1000)}s]
          </Badge>{' '}
          {line.sourceText}
        </div>
      ))}
    </div>
  );
}
