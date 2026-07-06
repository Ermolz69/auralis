import { useImportLocalMedia } from '../model/useImportLocalMedia';

export function ImportLocalMediaButton() {
  const { handleImport, isImporting, error } = useImportLocalMedia();

  return (
    <div className="flex flex-col items-center">
      <button
        onClick={handleImport}
        disabled={isImporting}
        className="px-6 py-3 bg-secondary text-secondary-foreground font-medium rounded hover:bg-secondary/90 transition-colors disabled:opacity-50"
      >
        {isImporting ? 'Importing...' : 'Import local video'}
      </button>
      {error && (
        <p className="text-destructive text-sm mt-2">{error}</p>
      )}
    </div>
  );
}
