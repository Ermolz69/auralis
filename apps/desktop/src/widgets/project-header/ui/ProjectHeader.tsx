import { RunDubbing } from '../../../features/run-dubbing';

export const ProjectHeader = () => {
  return (
    <header className="flex items-center justify-between px-6 py-4 bg-surface border-b border-muted">
      <div>
        <h1 className="text-xl font-bold text-text">Project Title</h1>
        <p className="text-sm text-muted mt-1">Video ID: dQw4w9WgXcQ</p>
      </div>
      <RunDubbing />
    </header>
  );
};
