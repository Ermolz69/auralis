import { Card, CardContent } from '../../../shared/ui/card';
import { Button } from '../../../shared/ui/button';

export const ExportPanel = () => {
  return (
    <Card className="rounded-none border-x-0 border-b-0 flex-shrink-0">
      <CardContent className="p-6">
        <h3 className="font-semibold text-text mb-3">Export Settings</h3>
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted">Format: MP4</span>
          <Button variant="primary" size="sm">
            Export Video
          </Button>
        </div>
      </CardContent>
    </Card>
  );
};
