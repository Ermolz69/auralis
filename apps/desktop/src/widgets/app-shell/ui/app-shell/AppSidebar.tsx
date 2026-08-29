import { useRef, useState } from 'react';
import type { PipelineStep, View } from '@/shared/router';
import { PipelinePanel } from './PipelinePanel';
import { PrimaryNavigation } from './PrimaryNavigation';
import { ProjectListPanel } from './ProjectListPanel';
import type { PipelineDisplayStatus } from './model';

type Props = {
  currentView: View;
  pipelineStep: PipelineStep;
  hasProject: boolean;
  pipelineStatus: { source: PipelineDisplayStatus; subtitles: PipelineDisplayStatus };
  onHome: () => void;
  onProject: () => void;
  onSettings: () => void;
  onStep: (step: PipelineStep) => void;
};

export function AppSidebar(props: Props) {
  const bodyRef = useRef<HTMLDivElement>(null);
  const [projectsHeight, setProjectsHeight] = useState(190);
  return (
    <aside className="fixed inset-x-0 bottom-0 z-40 flex h-14 border-t border-border bg-bg lg:static lg:h-full lg:w-[234px] lg:shrink-0 lg:flex-col lg:border-r lg:border-t-0">
      <div className="hidden h-12 shrink-0 items-center gap-2.5 border-b border-border px-3 lg:flex">
        <div className="relative flex h-7 w-7 items-center justify-center rounded-sm bg-primary text-sm font-bold text-primary-foreground">
          A
          <span className="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full border-2 border-bg bg-success" />
        </div>
        <span className="flex-1 text-sm font-semibold">Auralis</span>
      </div>
      <div ref={bodyRef} className="hidden min-h-0 flex-1 flex-col lg:flex">
        <ProjectListPanel height={projectsHeight} />
        <div
          role="separator"
          aria-label="Resize projects and pipeline panels"
          aria-orientation="horizontal"
          aria-valuenow={Math.round(projectsHeight)}
          tabIndex={0}
          title="Перетащите, чтобы изменить высоту панелей"
          onPointerDown={(event) => {
            event.preventDefault();
            const startY = event.clientY;
            const startHeight = projectsHeight;
            const maxHeight = Math.max(150, (bodyRef.current?.clientHeight ?? 0) - 120);
            const move = (moveEvent: PointerEvent) =>
              setProjectsHeight(
                Math.min(maxHeight, Math.max(120, startHeight + moveEvent.clientY - startY)),
              );
            const end = () => {
              window.removeEventListener('pointermove', move);
              window.removeEventListener('pointerup', end);
            };
            window.addEventListener('pointermove', move);
            window.addEventListener('pointerup', end);
          }}
          onKeyDown={(event) => {
            if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
            event.preventDefault();
            setProjectsHeight((height) =>
              Math.max(120, height + (event.key === 'ArrowUp' ? -12 : 12)),
            );
          }}
          className="group relative h-1.5 shrink-0 cursor-row-resize touch-none border-y border-border/60 bg-canvas outline-none hover:bg-primary/15 focus-visible:bg-primary/20"
        >
          <span className="absolute inset-x-0 top-1/2 h-px -translate-y-1/2 bg-border-strong transition-colors group-hover:bg-primary" />
        </div>
        <PipelinePanel
          currentView={props.currentView}
          pipelineStep={props.pipelineStep}
          hasProject={props.hasProject}
          status={props.pipelineStatus}
          onStep={props.onStep}
        />
      </div>
      <PrimaryNavigation
        currentView={props.currentView}
        hasProject={props.hasProject}
        onHome={props.onHome}
        onProject={props.onProject}
        onSettings={props.onSettings}
      />
    </aside>
  );
}
