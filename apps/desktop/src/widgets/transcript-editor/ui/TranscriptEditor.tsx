import { Card, CardContent } from '../../../shared/ui/card';
import { Badge } from '../../../shared/ui/badge';
import { Icon } from '../../../shared/ui/icon';

export const TranscriptEditor = () => {
  const transcript: { time: string; text: string }[] = []; // Simulating empty state

  return (
    <section className="flex-1 p-6 flex flex-col gap-4 overflow-hidden min-h-0">
      <h2 className="text-lg font-semibold text-text shrink-0">Transcript</h2>
      <Card className="flex-1 overflow-hidden flex flex-col shadow-sm">
        <CardContent className="flex-1 p-6 overflow-y-auto min-h-0">
          {transcript.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-center">
              <Icon name="FileText" size="lg" className="text-muted/50 mb-4" />
              <p className="text-lg font-medium text-text">No transcript available</p>
              <p className="text-sm text-muted mt-2 max-w-sm">
                Paste a YouTube link and start a project to generate a transcript automatically.
              </p>
            </div>
          ) : (
            transcript.map((line, idx) => (
              <p key={idx} className="mb-4 hover:bg-bg p-2 rounded transition-colors">
                <Badge variant="primary" size="sm" className="mr-2 font-mono">
                  [{line.time}]
                </Badge>{' '}
                {line.text}
              </p>
            ))
          )}
        </CardContent>
      </Card>
    </section>
  );
};
