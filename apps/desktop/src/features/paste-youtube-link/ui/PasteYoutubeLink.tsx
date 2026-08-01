import type { FormEvent } from 'react';
import { Input } from '../../../shared/ui/input';
import { Button } from '../../../shared/ui/button';
import { usePasteYoutubeLink } from '../model/usePasteYoutubeLink';

export const PasteYoutubeLink = () => {
  const { url, setUrl, startProject, isStarting, isBlockedByDeletion, error } =
    usePasteYoutubeLink();
  const errorId = 'youtube-link-error';
  const statusId = 'youtube-link-status';
  const describedBy = error ? errorId : isStarting ? statusId : undefined;

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void startProject();
  };

  return (
    <form className="flex flex-col gap-2 w-full" onSubmit={handleSubmit}>
      <div className="flex gap-2 w-full">
        <Input
          aria-label="YouTube video link"
          aria-describedby={describedBy}
          aria-invalid={Boolean(error)}
          placeholder="Paste a YouTube link..."
          className="flex-1"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          disabled={isStarting || isBlockedByDeletion}
        />
        <Button
          type="submit"
          variant="secondary"
          size="lg"
          disabled={isStarting || !url || isBlockedByDeletion}
          loading={isStarting}
        >
          {isStarting ? 'Creating project...' : 'Add YouTube source'}
        </Button>
      </div>
      {isStarting && (
        <p id={statusId} className="text-muted text-sm text-left" role="status" aria-live="polite">
          Creating YouTube project. The workspace will show subtitle import progress after this step.
        </p>
      )}
      {isBlockedByDeletion && (
        <p className="text-muted text-sm text-left">
          Finish the current delete action before adding a YouTube source.
        </p>
      )}
      {error && (
        <p id={errorId} className="text-danger text-sm text-left" role="alert">
          {error}
        </p>
      )}
    </form>
  );
};
