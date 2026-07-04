const fs = require('fs');
const path = require('path');

const files = {
  'apps/desktop/src/features/paste-youtube-link/ui/PasteYoutubeLink.tsx': `export const PasteYoutubeLink = () => {
  return (
    <div className="flex gap-2 w-full">
      <input type="text" placeholder="Paste YouTube link here..." className="bg-surface border border-muted rounded px-4 py-3 flex-1 text-text outline-none focus:border-primary transition-colors" />
      <button className="bg-primary hover:bg-primary/90 text-text px-6 py-3 rounded font-medium transition-colors cursor-pointer">Start Project</button>
    </div>
  );
};`,
  'apps/desktop/src/features/paste-youtube-link/index.ts': `export { PasteYoutubeLink } from './ui/PasteYoutubeLink';`,
  
  'apps/desktop/src/features/run-dubbing/ui/RunDubbing.tsx': `export const RunDubbing = () => {
  return (
    <button className="bg-primary hover:bg-primary/90 text-text px-4 py-2 rounded font-medium transition-colors cursor-pointer">
      Run Dubbing
    </button>
  );
};`,
  'apps/desktop/src/features/run-dubbing/index.ts': `export { RunDubbing } from './ui/RunDubbing';`,
  
  'apps/desktop/src/widgets/project-header/ui/ProjectHeader.tsx': `import { RunDubbing } from '../../../features/run-dubbing';

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
};`,
  'apps/desktop/src/widgets/project-header/index.ts': `export { ProjectHeader } from './ui/ProjectHeader';`,

  'apps/desktop/src/widgets/job-queue-panel/ui/JobQueuePanel.tsx': `export const JobQueuePanel = () => {
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
};`,
  'apps/desktop/src/widgets/job-queue-panel/index.ts': `export { JobQueuePanel } from './ui/JobQueuePanel';`,

  'apps/desktop/src/widgets/transcript-editor/ui/TranscriptEditor.tsx': `export const TranscriptEditor = () => {
  return (
    <section className="flex-1 p-6 bg-bg flex flex-col gap-4">
      <h2 className="text-lg font-semibold text-text">Transcript</h2>
      <div className="flex-1 bg-surface border border-muted rounded p-6 text-muted overflow-auto shadow-sm">
        <p className="mb-4 hover:bg-bg p-2 rounded transition-colors"><span className="text-primary font-mono mr-2">[00:00]</span> Never gonna give you up...</p>
        <p className="mb-4 hover:bg-bg p-2 rounded transition-colors"><span className="text-primary font-mono mr-2">[00:03]</span> Never gonna let you down...</p>
      </div>
    </section>
  );
};`,
  'apps/desktop/src/widgets/transcript-editor/index.ts': `export { TranscriptEditor } from './ui/TranscriptEditor';`,

  'apps/desktop/src/widgets/export-panel/ui/ExportPanel.tsx': `export const ExportPanel = () => {
  return (
    <div className="p-6 bg-surface border-t border-muted">
      <h3 className="font-semibold text-text mb-3">Export Settings</h3>
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted">Format: MP4</span>
        <button className="bg-primary hover:bg-primary/90 text-text px-4 py-2 rounded text-sm font-medium transition-colors cursor-pointer">Export Video</button>
      </div>
    </div>
  );
};`,
  'apps/desktop/src/widgets/export-panel/index.ts': `export { ExportPanel } from './ui/ExportPanel';`,

  'apps/desktop/src/pages/home/ui/HomePage.tsx': `import { PasteYoutubeLink } from '../../../features/paste-youtube-link';

export const HomePage = () => {
  return (
    <div className="min-h-screen bg-bg flex flex-col items-center justify-center p-8">
      <div className="w-full max-w-2xl text-center flex flex-col gap-8">
        <h1 className="text-5xl font-bold text-text bg-gradient-to-r from-primary to-danger bg-clip-text text-transparent pb-2">Auralis</h1>
        <p className="text-muted text-xl">AI-powered video dubbing straight from your desktop.</p>
        <div className="mt-4">
          <PasteYoutubeLink />
        </div>
      </div>
    </div>
  );
};`,
  'apps/desktop/src/pages/home/index.ts': `export { HomePage } from './ui/HomePage';`,

  'apps/desktop/src/pages/project/ui/ProjectPage.tsx': `import { ProjectHeader } from '../../../widgets/project-header';
import { TranscriptEditor } from '../../../widgets/transcript-editor';
import { JobQueuePanel } from '../../../widgets/job-queue-panel';
import { ExportPanel } from '../../../widgets/export-panel';

export const ProjectPage = () => {
  return (
    <div className="h-screen flex flex-col bg-bg text-text font-sans">
      <ProjectHeader />
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col min-w-0">
          <TranscriptEditor />
          <ExportPanel />
        </div>
        <JobQueuePanel />
      </div>
    </div>
  );
};`,
  'apps/desktop/src/pages/project/index.ts': `export { ProjectPage } from './ui/ProjectPage';`,

  'apps/desktop/src/App.tsx': `import { useState } from 'react';
import { HomePage } from './pages/home';
import { ProjectPage } from './pages/project';

function App() {
  const [currentView, setCurrentView] = useState<'home' | 'project'>('project');

  return (
    <>
      <div className="fixed bottom-4 left-4 z-50">
        <button 
          onClick={() => setCurrentView(v => v === 'home' ? 'project' : 'home')}
          className="bg-surface text-muted px-3 py-1 text-xs border border-muted rounded opacity-50 hover:opacity-100 transition-opacity cursor-pointer shadow-sm hover:shadow"
        >
          Toggle View (Dev)
        </button>
      </div>
      {currentView === 'home' ? <HomePage /> : <ProjectPage />}
    </>
  );
}

export default App;`
};

for (const [filepath, content] of Object.entries(files)) {
  const fullPath = path.resolve(__dirname, filepath);
  const dir = path.dirname(fullPath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  fs.writeFileSync(fullPath, content);
}
