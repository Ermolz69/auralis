export const JobQueuePanel = () => {
  return (
    <aside className="p-6 bg-surface border-l border-muted w-80 h-full flex flex-col gap-4">
      <h2 className="text-lg font-semibold text-text">Job Queue</h2>
      <div className="flex-1 flex flex-col gap-3">
        <div className="bg-bg p-4 rounded border border-muted">
          <p className="text-sm text-text font-medium">Downloading audio...</p>
          <div className="w-full bg-surface h-1.5 mt-3 rounded overflow-hidden">
            <div className="bg-primary w-1/2 h-full"></div>
          </div>
        </div>
      </div>
    </aside>
  );
};
