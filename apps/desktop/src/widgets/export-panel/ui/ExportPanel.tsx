import { Card, CardContent } from '../../../shared/ui/card';
import { Button } from '../../../shared/ui/button';
import { Icon } from '../../../shared/ui/icon';

export const ExportPanel = () => {
  return (
    <Card className="rounded-none border-x-0 border-b-0 flex-shrink-0">
      <CardContent className="p-6">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <h3 className="font-semibold text-text">Export</h3>
            <p id="export-unavailable-note" className="text-sm text-muted">
              Video export is not available in the current app contract.
            </p>
          </div>
          <Button
            variant="secondary"
            size="sm"
            disabled
            aria-describedby="export-unavailable-note"
            leftIcon={<Icon name="Lock" size="sm" />}
          >
            Export unavailable
          </Button>
        </div>
      </CardContent>
    </Card>
  );
};
