export const PasteYoutubeLink = () => {
  return (
    <div className="flex gap-2 w-full">
      <input
        type="text"
        placeholder="Paste YouTube link here..."
        className="bg-surface border border-muted rounded px-4 py-3 flex-1 text-text outline-none focus:border-primary transition-colors"
      />
      <button className="bg-primary hover:bg-primary/90 text-text px-6 py-3 rounded font-medium transition-colors cursor-pointer">
        Start Project
      </button>
    </div>
  );
};
