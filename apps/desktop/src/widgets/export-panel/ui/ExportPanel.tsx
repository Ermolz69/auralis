export const ExportPanel = () => {
  return (
    <div className="p-6 bg-surface border-t border-muted">
      <h3 className="font-semibold text-text mb-3">Export Settings</h3>
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted">Format: MP4</span>
        <button className="bg-primary hover:bg-primary/90 text-text px-4 py-2 rounded text-sm font-medium transition-colors cursor-pointer">
          Export Video
        </button>
      </div>
    </div>
  );
};
