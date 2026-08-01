import { Button } from '../../../shared/ui/button';
import { Card, CardContent } from '../../../shared/ui/card';
import { Icon } from '../../../shared/ui/icon';
import { useImportLocalMedia } from '../model/useImportLocalMedia';

export function ImportLocalMediaButton() {
  const {
    handleImport,
    openDraftProject,
    isImporting,
    isBlockedByDeletion,
    stage,
    error,
    draftProject,
  } = useImportLocalMedia();
  const stageText =
    stage === 'selecting'
      ? 'Waiting for file selection'
      : stage === 'probing'
        ? 'Checking media'
        : stage === 'importing'
          ? 'Importing into project'
          : null;

  return (
    <div className="flex flex-col items-stretch gap-3">
      <Button
        onClick={handleImport}
        disabled={isImporting || isBlockedByDeletion}
        loading={isImporting}
        variant="primary"
        size="lg"
        leftIcon={<Icon name="Film" size="sm" />}
        fullWidth
      >
        Import local video
      </Button>
      {stageText && (
        <p className="text-muted text-sm" role="status" aria-live="polite">
          {stageText}
        </p>
      )}
      {isBlockedByDeletion && (
        <p className="text-muted text-sm">Finish the current delete action before importing.</p>
      )}
      {error && (
        <Card variant="muted" className="border-danger/40 text-left" role="alert">
          <CardContent className="p-4 flex flex-col gap-3">
            <div className="flex items-start gap-3">
              <Icon name="TriangleAlert" size="sm" color="danger" />
              <div className="min-w-0">
                <p className="text-sm font-semibold text-text">Local import did not finish</p>
                <p className="text-danger text-sm break-words">{error}</p>
                {draftProject && (
                  <p className="text-muted text-sm mt-1">
                    A draft project was saved. You can open it or choose the file again.
                  </p>
                )}
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={handleImport}
                disabled={isImporting || isBlockedByDeletion}
              >
                Choose file again
              </Button>
              {draftProject && (
                <Button type="button" variant="ghost" size="sm" onClick={openDraftProject}>
                  Open draft
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
