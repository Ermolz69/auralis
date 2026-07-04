export const TranscriptEditor = () => {
  return (
    <section className="flex-1 p-6 bg-bg flex flex-col gap-4">
      <h2 className="text-lg font-semibold text-text">Transcript</h2>
      <div className="flex-1 bg-surface border border-muted rounded p-6 text-muted overflow-auto shadow-sm">
        <p className="mb-4 hover:bg-bg p-2 rounded transition-colors">
          <span className="text-primary font-mono mr-2">[00:00]</span> Never gonna give you up...
        </p>
        <p className="mb-4 hover:bg-bg p-2 rounded transition-colors">
          <span className="text-primary font-mono mr-2">[00:03]</span> Never gonna let you down...
        </p>
      </div>
    </section>
  );
};
