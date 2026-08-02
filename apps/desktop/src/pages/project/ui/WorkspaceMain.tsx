import { TranscriptEditor } from '../../../widgets/transcript-editor';
import { ExportPanel } from '../../../widgets/export-panel';
import { CurrentStepSummary } from './CurrentStepSummary';

export function WorkspaceMain() {
  return (
    <section
      className="order-1 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
      aria-label="Current project work"
      data-testid="workspace-main"
    >
      <CurrentStepSummary />
      <TranscriptEditor />
      <ExportPanel />
    </section>
  );
}
