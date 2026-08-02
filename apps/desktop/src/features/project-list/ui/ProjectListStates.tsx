import { Button } from '@/shared/ui/button';
import { Card, CardContent } from '@/shared/ui/card';
import { Icon } from '@/shared/ui/icon';

export function ProjectListLoadingState() {
  return (
    <Card variant="muted" aria-busy="true">
      <CardContent className="p-4">
        <p className="text-muted text-sm animate-pulse" role="status" aria-live="polite">
          Loading recent projects...
        </p>
      </CardContent>
    </Card>
  );
}

export function ProjectListErrorState({ error, onRetry }: { error: string; onRetry: () => void }) {
  return (
    <Card variant="muted" className="border-danger/40 text-left" role="alert">
      <CardContent className="p-4 flex flex-col gap-3">
        <div className="flex items-start gap-3">
          <Icon name="TriangleAlert" size="sm" color="danger" />
          <div>
            <p className="text-sm font-semibold text-text">Could not load recent projects</p>
            <p className="text-danger text-sm">{error}</p>
          </div>
        </div>
        <Button
          type="button"
          variant="secondary"
          size="sm"
          onClick={onRetry}
          className="self-start"
        >
          Retry
        </Button>
      </CardContent>
    </Card>
  );
}

export function ProjectListEmptyState() {
  return (
    <Card variant="muted" className="text-left">
      <CardContent className="p-4 flex items-start gap-3">
        <Icon name="FolderOpen" size="md" color="muted" />
        <div>
          <p className="text-sm font-semibold text-text">No projects yet</p>
          <p className="text-muted text-sm">
            Import a local video above to create a desktop project.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
