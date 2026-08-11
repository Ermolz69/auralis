import { useEffect, useRef, useState, type FormEvent } from 'react';
import { Input } from '../../../shared/ui/input';
import { Button } from '../../../shared/ui/button';
import { usePasteYoutubeLink } from '../model/usePasteYoutubeLink';

export const PasteYoutubeLink = () => {
  const { url, setUrl, startProject, isStarting, isBlockedByDeletion, error } =
    usePasteYoutubeLink();
  const [validationError, setValidationError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const statusId = 'youtube-link-status';
  const displayedError = validationError || error;
  const trimmedUrl = url.trim();

  useEffect(() => {
    if (trimmedUrl) setValidationError(null);
  }, [trimmedUrl]);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!trimmedUrl) {
      setValidationError('Paste a YouTube URL before adding a source.');
      inputRef.current?.focus();
      return;
    }
    void startProject();
  };

  return (
    <form
      className="flex flex-col gap-3 w-full text-left"
      aria-label="Add YouTube source"
      onSubmit={handleSubmit}
      noValidate
    >
      <div className="flex w-full flex-col gap-2 sm:flex-row sm:items-start">
        <Input
          id="youtube-link-url"
          ref={inputRef}
          label="YouTube URL"
          helperText="Supported source: a single YouTube video link."
          error={Boolean(displayedError)}
          errorText={displayedError ?? undefined}
          placeholder="https://youtube.com/watch?v=..."
          className="flex-1"
          type="url"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          disabled={isStarting || isBlockedByDeletion}
        />
        <Button
          type="submit"
          variant="secondary"
          size="lg"
          disabled={isStarting || !trimmedUrl || isBlockedByDeletion}
          loading={isStarting}
        >
          Add from YouTube
        </Button>
      </div>
      {isStarting && (
        <p id={statusId} className="text-muted text-sm text-left" role="status" aria-live="polite">
          Loading video metadata and downloading the source media. Subtitles start on step 2.
        </p>
      )}
      {isBlockedByDeletion && (
        <p className="text-muted text-sm text-left">
          Finish the current delete action before adding a YouTube source.
        </p>
      )}
    </form>
  );
};
