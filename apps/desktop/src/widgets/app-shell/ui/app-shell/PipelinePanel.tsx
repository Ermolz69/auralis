import type { PipelineStep, View } from '@/shared/router';
import { SectionLabel } from './SectionLabel';
import type { PipelineDisplayStatus } from './model';

type Props = {
  currentView: View;
  pipelineStep: PipelineStep;
  hasProject: boolean;
  status: { source: PipelineDisplayStatus; subtitles: PipelineDisplayStatus };
  onStep: (step: PipelineStep) => void;
};

export function PipelinePanel(props: Props) {
  return (
    <section className="min-h-0 flex-1 overflow-y-auto">
      <SectionLabel label="Pipeline" />
      <ol className="space-y-px">
        <PipelineItem
          index={1}
          label="Источник видео"
          status={props.status.source}
          active={props.currentView === 'project' && props.pipelineStep === 'source'}
          disabled={!props.hasProject}
          onClick={() => props.onStep('source')}
        />
        <PipelineItem
          index={2}
          label="Субтитры"
          status={props.status.subtitles}
          active={props.currentView === 'project' && props.pipelineStep === 'subtitles'}
          disabled={!props.hasProject}
          onClick={() => props.onStep('subtitles')}
        />
        <PipelineItem index={3} label="Перевод" status="unavailable" disabled />
        <PipelineItem index={4} label="Подготовка TTS" status="unavailable" disabled />
        <PipelineItem index={5} label="Синтез речи" status="unavailable" disabled />
        <PipelineItem index={6} label="Рендер MP4" status="unavailable" disabled />
      </ol>
    </section>
  );
}

function PipelineItem({
  index,
  label,
  status,
  active = false,
  disabled = false,
  onClick,
}: {
  index: number;
  label: string;
  status: PipelineDisplayStatus;
  active?: boolean;
  disabled?: boolean;
  onClick?: () => void;
}) {
  const statusClass: Record<PipelineDisplayStatus, string> = {
    idle: 'bg-border-strong',
    unavailable: 'bg-surface-active',
    pending: 'animate-pulse bg-muted',
    running: 'animate-pulse bg-primary signal-glow-sm',
    completed: 'bg-success',
    failed: 'bg-danger',
    cancelled: 'bg-subtle',
  };
  return (
    <li>
      <button
        type="button"
        disabled={disabled}
        onClick={onClick}
        className={`motion-control flex w-full items-center gap-3 px-4 py-2 text-left text-xs ${active ? 'bg-primary/7 text-text' : disabled ? 'cursor-not-allowed text-subtle opacity-55' : 'text-muted hover:bg-surface hover:text-text'}`}
      >
        <span className="w-3 text-right font-mono text-[10px] text-subtle">{index}</span>
        <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
        <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusClass[status]}`} />
      </button>
    </li>
  );
}
