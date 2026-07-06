import { Input } from '../../../shared/ui/input';
import { Button } from '../../../shared/ui/button';
import { usePasteYoutubeLink } from '../model/usePasteYoutubeLink';

export const PasteYoutubeLink = () => {
  const { url, setUrl, startProject, isStarting, error } = usePasteYoutubeLink();

  return (
    <div className="flex flex-col gap-2 w-full">
      <div className="flex gap-2 w-full">
        <Input
          aria-label="YouTube video link"
          placeholder="Paste YouTube link here..."
          className="flex-1"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          disabled={isStarting}
        />
        <Button variant="primary" size="lg" onClick={startProject} disabled={isStarting || !url}>
          {isStarting ? 'Starting...' : 'Start Project'}
        </Button>
      </div>
      {error && <p className="text-danger text-sm">{error}</p>}
    </div>
  );
};
