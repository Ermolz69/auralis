import { Card, CardContent } from '../../../shared/ui/card';
import { Badge } from '../../../shared/ui/badge';
import { Icon } from '../../../shared/ui/icon';
import { useTranscript } from '@/entities/transcript';
import { useProjectContext } from '@/entities/project';

export const TranscriptEditor = () => {
  const { projectId } = useProjectContext();
  const { transcript, isLoading, error } = useTranscript(projectId);

  return (
    <section className="flex-1 p-6 flex flex-col gap-4 overflow-hidden min-h-0">
      <h2 className="text-lg font-semibold text-text shrink-0">Transcript</h2>
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
            <div className="h-full flex flex-col items-center justify-center text-center">
              <p className="text-lg font-medium text-text animate-pulse">Loading transcript...</p>
            </div>
          ) : error ? (
            <div className="h-full flex flex-col items-center justify-center text-center">
              <Icon name="CircleAlert" size="lg" className="text-danger mb-4" />
              <p className="text-lg font-medium text-text">Error loading transcript</p>
              <p className="text-sm text-danger mt-2 max-w-sm">{error}</p>
            </div>
          ) : !transcript || transcript.segments.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-center">
              <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
              <p className="text-lg font-medium text-text">Waiting for transcript generation...</p>
              <p className="text-sm text-muted mt-2 max-w-sm">
                The mock pipeline is currently running. The transcript will appear here when ready.
              </p>
            </div>
          ) : (
            transcript.segments.map((line, idx) => (
              <p key={idx} className="mb-4 hover:bg-bg p-2 rounded transition-colors">
                <Badge variant="primary" size="sm" className="mr-2 font-mono">
                  [{Math.floor(line.startMs / 1000)}s - {Math.floor(line.endMs / 1000)}s]
                </Badge>{' '}
                {line.sourceText}
              </p>
            ))
          )}
        </CardContent>
      </Card>
    </section>
  );
};
