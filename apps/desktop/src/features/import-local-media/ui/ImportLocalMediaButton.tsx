import { useEffect, useRef } from 'react';
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
    sourceLabel,
  } = useImportLocalMedia();
  const errorSummaryRef = useRef<HTMLDivElement>(null);
  const stageText =
    stage === 'selecting'
      ? 'Waiting for file selection'
      : stage === 'probing'
        ? `Checking media${sourceLabel ? `: ${sourceLabel}` : ''}`
        : stage === 'importing'
          ? `Importing into project${sourceLabel ? `: ${sourceLabel}` : ''}`
          : null;

  useEffect(() => {
    if (error) errorSummaryRef.current?.focus();
  }, [error]);

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
        <Card
          ref={errorSummaryRef}
          variant="muted"
          className="border-danger/40 text-left focus:outline-none focus:ring-2 focus:ring-danger focus:ring-offset-2 focus:ring-offset-bg"
          role="alert"
          tabIndex={-1}
        >
          <CardContent className="p-4 flex flex-col gap-3">
            <div className="flex items-start gap-3">
              <Icon name="TriangleAlert" size="sm" color="danger" />
              <div className="min-w-0">
                <p className="text-sm font-semibold text-text">Local import did not finish</p>
                <p className="text-danger text-sm break-words">{error}</p>
                {draftProject && (
                  <p className="text-muted text-sm mt-1">
                    A draft project was saved. You can open it, choose the file again, or delete the
                    draft from Recent Projects.
                  </p>
                )}
                {!draftProject && (
                  <p className="text-muted text-sm mt-1">
                    No project was created. Choose the file again when ready.
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
