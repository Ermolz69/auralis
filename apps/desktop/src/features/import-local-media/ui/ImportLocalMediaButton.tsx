import { Button } from '../../../shared/ui/button';
import { useImportLocalMedia } from '../model/useImportLocalMedia';

export function ImportLocalMediaButton() {
  const { handleImport, isImporting, error } = useImportLocalMedia();

  return (
    <div className="flex flex-col items-center">
      <Button
        onClick={handleImport}
        disabled={isImporting}
        loading={isImporting}
        variant="secondary"
        size="lg"
      >
        Import local video
      </Button>
      {error && <p className="text-danger text-sm mt-2">{error}</p>}
    </div>
  );
}
