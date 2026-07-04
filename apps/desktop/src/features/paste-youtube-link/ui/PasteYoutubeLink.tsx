import { Input } from '../../../shared/ui/input';
import { Button } from '../../../shared/ui/button';

export const PasteYoutubeLink = () => {
  return (
    <div className="flex gap-2 w-full">
      <Input
        aria-label="YouTube video link"
        placeholder="Paste YouTube link here..."
        className="flex-1"
      />
      <Button variant="primary" size="lg">
        Start Project
      </Button>
    </div>
  );
};
