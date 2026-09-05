import { useState, type ReactNode } from 'react';
import { MediaPanel } from '../../../widgets/media-panel';
import { JobQueuePanel } from '../../../widgets/job-queue-panel';
import { Button } from '../../../shared/ui/button';
import { Dialog, DialogDescription, DialogTitle } from '../../../shared/ui/dialog';

const panelSession = { mediaOpen: true };

export function WorkspaceSecondaryPanels() {
  const [mediaOpen, setMediaOpen] = useState(panelSession.mediaOpen);

  const setPanelOpen = (open: boolean) => {
    panelSession.mediaOpen = open;
    setMediaOpen(open);
  };

  return (
    <>
      <div
        className="order-2 hidden min-h-0 overflow-hidden border-l border-muted bg-bg xl:flex"
        data-testid="workspace-wide-panels"
      >
        <WidePanel
          id="workspace-media-panel"
          label="Media"
          expanded={mediaOpen}
          onToggle={() => setPanelOpen(!mediaOpen)}
        >
          <MediaPanel className="w-80" />
        </WidePanel>
      </div>
      <div className="order-2 flex shrink-0 gap-2 border-t border-border bg-surface p-2 xl:hidden">
        <PanelDialog label="Media" description="Media details and stream information">
          <MediaPanel className="h-[70vh]" />
        </PanelDialog>
        <PanelDialog label="Jobs" description="Active and completed project jobs">
          <JobQueuePanel className="h-[70vh]" />
        </PanelDialog>
      </div>
    </>
  );
}

function WidePanel({
  id,
  label,
  expanded,
  onToggle,
  children,
}: {
  id: string;
  label: string;
  expanded: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <section className="flex min-h-0 min-w-0 flex-col" aria-label={`${label} panel controls`}>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        aria-expanded={expanded}
        aria-controls={id}
        onClick={onToggle}
        className="m-2"
      >
        {expanded ? `Hide ${label}` : `Show ${label}`}
      </Button>
      {expanded && (
        <div id={id} className="min-h-0 flex-1 animate-drawer-in overflow-hidden">
          {children}
        </div>
      )}
    </section>
  );
}

function PanelDialog({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Dialog
      trigger={
        <Button type="button" variant="secondary" size="sm" fullWidth>
          Open {label}
        </Button>
      }
    >
      <DialogTitle>{label}</DialogTitle>
      <DialogDescription>{description}</DialogDescription>
      <div className="mt-4 overflow-hidden rounded-md border border-muted">{children}</div>
    </Dialog>
  );
}
