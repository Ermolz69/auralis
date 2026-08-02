import { Card, CardContent } from '../../../shared/ui/card';
import { Icon } from '../../../shared/ui/icon';

export const ExportPanel = () => {
  return (
    <Card className="rounded-none border-x-0 border-b-0 flex-shrink-0">
      <CardContent className="p-6">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <h3 className="font-semibold text-text">Export</h3>
            <p id="export-unavailable-note" className="text-sm text-muted">
              Export is not available in this version.
            </p>
          </div>
          <div
            className="inline-flex w-fit items-center gap-2 rounded-md border border-border bg-surface px-3 py-1.5 text-sm font-medium text-muted"
            role="status"
            aria-describedby="export-unavailable-note"
          >
            <Icon name="Lock" size="sm" />
            Export unavailable
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
