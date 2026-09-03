import { useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { JobContext, isActiveJobStatus, selectProjectJobs } from '@/entities/job';
import { formatDuration, formatProjectTitle } from '@/entities/media';
import { createProject, useProjectContext } from '@/entities/project';
import { toCommandError } from '@/shared/api/contracts';
import { useNavigation, type PipelineStep, type View } from '@/shared/router';
import { Button } from '@/shared/ui/button';
import { Icon, type IconName } from '@/shared/ui/icon';
import { toast } from '@/shared/ui/toast';
import { usePinnedProjects } from '../model/usePinnedProjects';
import { getPipelineStatus, type PipelineDisplayStatus } from '../model/pipelineStatus';

const locationLabel: Record<View, string> = {
  home: 'Проекты',
  project: 'Рабочее пространство',
  settings: 'Настройки',
};

const stepLabel: Record<PipelineStep, string> = {
  source: 'Загрузка видео',
  subtitles: 'Субтитры',
};

export function AppShell({ children, jobQueue }: { children: ReactNode; jobQueue?: ReactNode }) {
  const { currentView, setCurrentView, pipelineStep, setPipelineStep } = useNavigation();
  const { projectId, project, setProject, captureToken, validateToken } = useProjectContext();
  const jobState = useContext(JobContext);
  const mainRef = useRef<HTMLElement>(null);
  const projectNameRef = useRef<HTMLInputElement>(null);
  const sidebarBodyRef = useRef<HTMLDivElement>(null);
  const [settingsReturnView, setSettingsReturnView] = useState<View>('home');
  const [queueOpen, setQueueOpen] = useState(false);
  const [projectName, setProjectName] = useState('');
  const [projectNameRequired, setProjectNameRequired] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const pinnedProjects = usePinnedProjects();
  const [projectsPaneHeight, setProjectsPaneHeight] = useState(190);

  useEffect(() => {
    const main = mainRef.current;
    const heading = main?.querySelector('h1');

    if (heading instanceof HTMLElement) {
      heading.tabIndex = -1;
      heading.focus();
      return;
    }

    main?.focus();
  }, [currentView, pipelineStep]);

  useEffect(() => {
    if (!projectId && settingsReturnView === 'project') {
      setSettingsReturnView('home');
    }
  }, [projectId, settingsReturnView]);

  const goToHome = () => setCurrentView('home');
  const goToProject = () => {
    if (projectId) setCurrentView('project');
  };
  const goToStep = (step: PipelineStep) => {
    if (!projectId) return;
    setPipelineStep(step);
    setCurrentView('project');
  };
  const goToSettings = () => {
    setSettingsReturnView(currentView === 'project' && projectId ? 'project' : 'home');
    setCurrentView('settings');
  };
  const returnFromSettings = () => {
    setCurrentView(settingsReturnView === 'project' && projectId ? 'project' : 'home');
  };
  const handleCreateProject = async () => {
    const title = projectName.trim();
    if (!title) {
      setProjectNameRequired(true);
      projectNameRef.current?.focus();
      toast.warning('Укажите название проекта');
      return;
    }
    if (isCreating) return;
    const token = captureToken();
    if (!validateToken(token)) return;
    setIsCreating(true);
    try {
      const created = await createProject(title);
      if (!validateToken(token)) return;
      setProject(created);
      setPipelineStep('source');
      setCurrentView('project');
      setProjectName('');
      setProjectNameRequired(false);
      toast.success('Project created');
    } catch (error) {
      if (!validateToken(token)) return;
      toast.error(toCommandError(error).message);
    } finally {
      setIsCreating(false);
    }
  };

  const jobs = useMemo(() => Object.values(jobState?.jobs ?? {}), [jobState?.jobs]);
  const activeJobs = jobs.filter((job) => isActiveJobStatus(job.status));
  const pipelineStatus = getPipelineStatus(
    project?.source != null,
    selectProjectJobs(jobState?.jobs ?? {}, projectId),
  );
  const projectTitle = project
    ? formatProjectTitle(project.title, project.source)
    : 'Проект не открыт';

  return (
    <div className="flex h-screen min-h-0 flex-col overflow-hidden bg-canvas text-text">
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <aside className="fixed inset-x-0 bottom-0 z-40 flex h-14 border-t border-border bg-bg lg:static lg:h-full lg:w-[234px] lg:shrink-0 lg:flex-col lg:border-r lg:border-t-0">
          <div className="hidden h-12 shrink-0 items-center gap-2.5 border-b border-border px-3 lg:flex">
            <div className="relative flex h-7 w-7 items-center justify-center rounded-sm bg-primary text-sm font-bold text-primary-foreground">
              A
              <span className="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full border-2 border-bg bg-success" />
            </div>
            <span className="flex-1 text-sm font-semibold">Auralis</span>
          </div>

          <div ref={sidebarBodyRef} className="hidden min-h-0 flex-1 flex-col lg:flex">
            <section
              className="shrink-0 overflow-y-auto pb-2"
              style={{ height: projectsPaneHeight }}
            >
              <SectionLabel label="Проекты" />
              <form
                className="flex items-center gap-1.5 px-1.5"
                onSubmit={(event) => {
                  event.preventDefault();
                  void handleCreateProject();
                }}
              >
                <button
                  type="submit"
                  aria-label="Create project"
                  title="Создать проект"
                  disabled={isCreating}
                  className="flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-border bg-surface-raised text-subtle transition-colors hover:border-border-strong hover:text-primary"
                >
                  <Icon name="FolderPlus" size={13} />
                </button>
                <div className="relative min-w-0 flex-1">
                  <Icon
                    name="Search"
                    size={12}
                    color="muted"
                    className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 opacity-55"
                  />
                  <input
                    ref={projectNameRef}
                    type="text"
                    value={projectName}
                    onChange={(event) => {
                      setProjectName(event.target.value);
                      if (event.target.value.trim()) setProjectNameRequired(false);
                    }}
                    aria-label="Project name"
                    aria-invalid={projectNameRequired}
                    placeholder="Название проекта..."
                    className={`h-8 w-full rounded-sm border bg-surface-raised pl-7 pr-7 text-[11px] text-text outline-none placeholder:text-subtle focus:ring-1 ${
                      projectNameRequired
                        ? 'border-danger focus:border-danger focus:ring-danger/30'
                        : 'border-border focus:border-primary focus:ring-primary/30'
                    }`}
                  />
                  {projectName && (
                    <button
                      type="button"
                      aria-label="Clear project name"
                      onClick={() => {
                        setProjectName('');
                        projectNameRef.current?.focus();
                      }}
                      className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded-xs p-1 text-subtle hover:text-text"
                    >
                      <Icon name="X" size={11} />
                    </button>
                  )}
                </div>
              </form>

              <p className="px-2.5 pb-1 pt-2 text-[9px] font-semibold uppercase tracking-wider text-subtle">
                Закреплённые
              </p>
              {pinnedProjects.length > 0 ? (
                pinnedProjects.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => {
                      setProject(item);
                      setPipelineStep('source');
                      setCurrentView('project');
                    }}
                    className={`mx-1 flex w-[calc(100%_-_0.5rem)] items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs transition-colors ${
                      currentView === 'project' && projectId === item.id
                        ? 'bg-primary/7 text-text'
                        : 'text-muted hover:bg-surface hover:text-text'
                    }`}
                  >
                    <Icon
                      name="Folder"
                      size={12}
                      color={projectId === item.id ? 'primary' : 'muted'}
                    />
                    <span className="min-w-0 flex-1 truncate font-semibold">{item.title}</span>
                  </button>
                ))
              ) : (
                <p className="px-3 py-1.5 text-[11px] text-subtle">Нет закреплённых проектов</p>
              )}
              <p className="px-2.5 pt-2 text-[9px] font-semibold uppercase tracking-wider text-subtle">
                Проекты
              </p>
            </section>

            <div
              role="separator"
              aria-label="Resize projects and pipeline panels"
              aria-orientation="horizontal"
              aria-valuenow={Math.round(projectsPaneHeight)}
              tabIndex={0}
              title="Перетащите, чтобы изменить высоту панелей"
              onPointerDown={(event) => {
                event.preventDefault();
                const startY = event.clientY;
                const startHeight = projectsPaneHeight;
                const bodyHeight = sidebarBodyRef.current?.clientHeight ?? 0;
                const maxHeight = Math.max(150, bodyHeight - 120);
                const handleMove = (moveEvent: PointerEvent) => {
                  setProjectsPaneHeight(
                    Math.min(maxHeight, Math.max(120, startHeight + moveEvent.clientY - startY)),
                  );
                };
                const handleEnd = () => {
                  window.removeEventListener('pointermove', handleMove);
                  window.removeEventListener('pointerup', handleEnd);
                };
                window.addEventListener('pointermove', handleMove);
                window.addEventListener('pointerup', handleEnd);
              }}
              onKeyDown={(event) => {
                if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
                event.preventDefault();
                const bodyHeight = sidebarBodyRef.current?.clientHeight ?? 0;
                const maxHeight = Math.max(150, bodyHeight - 120);
                setProjectsPaneHeight((height) =>
                  Math.min(maxHeight, Math.max(120, height + (event.key === 'ArrowUp' ? -12 : 12))),
                );
              }}
              className="group relative h-1.5 shrink-0 cursor-row-resize touch-none border-y border-border/60 bg-canvas outline-none hover:bg-primary/15 focus-visible:bg-primary/20"
            >
              <span className="absolute inset-x-0 top-1/2 h-px -translate-y-1/2 bg-border-strong transition-colors group-hover:bg-primary" />
            </div>

            <section className="min-h-0 flex-1 overflow-y-auto">
              <SectionLabel label="Pipeline" />
              <ol className="space-y-px">
                <PipelineItem
                  index={1}
                  label="Источник видео"
                  status={pipelineStatus.source}
                  active={currentView === 'project' && pipelineStep === 'source'}
                  disabled={!projectId}
                  onClick={() => goToStep('source')}
                />
                <PipelineItem
                  index={2}
                  label="Субтитры"
                  status={pipelineStatus.subtitles}
                  active={currentView === 'project' && pipelineStep === 'subtitles'}
                  disabled={!projectId}
                  onClick={() => goToStep('subtitles')}
                />
                <PipelineItem index={3} label="Перевод" status="unavailable" disabled />
                <PipelineItem index={4} label="Подготовка TTS" status="unavailable" disabled />
                <PipelineItem index={5} label="Синтез речи" status="unavailable" disabled />
                <PipelineItem index={6} label="Рендер MP4" status="unavailable" disabled />
              </ol>
            </section>
          </div>

          <nav
            aria-label="Primary"
            className="grid h-full w-full grid-cols-3 items-stretch lg:h-auto lg:grid-cols-1 lg:border-t lg:border-border lg:py-1.5"
          >
            <ShellDestination
              active={currentView === 'home'}
              icon="LayoutGrid"
              label="Projects"
              visualLabel="Проекты"
              onClick={goToHome}
            />
            <ShellDestination
              active={currentView === 'project'}
              disabled={!projectId}
              icon="MonitorPlay"
              label="Workspace"
              visualLabel="Рабочее пространство"
              onClick={goToProject}
              desktopHidden
            />
            <ShellDestination
              active={currentView === 'settings'}
              icon="Settings"
              label="Settings"
              visualLabel="Настройки"
              onClick={goToSettings}
            />
          </nav>
        </aside>

        <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden pb-14 lg:pb-0">
          <header className="flex h-9 shrink-0 items-center justify-between border-b border-border bg-surface px-3">
            <div className="flex min-w-0 items-center gap-1.5">
              <div className="flex h-6 w-6 items-center justify-center rounded-xs bg-primary text-xs font-bold text-primary-foreground lg:hidden">
                A
              </div>
              {currentView === 'project' && (
                <>
                  <button
                    type="button"
                    onClick={goToHome}
                    aria-label="Back to projects"
                    className="rounded-xs p-1 text-subtle transition-colors hover:bg-surface-raised hover:text-muted"
                  >
                    <Icon name="ChevronLeft" size={14} />
                  </button>
                  <button
                    type="button"
                    disabled
                    aria-label="Forward unavailable"
                    className="rounded-xs p-1 text-subtle opacity-35"
                  >
                    <Icon name="ChevronRight" size={14} />
                  </button>
                  <span className="mx-1 h-4 w-px bg-border" aria-hidden="true" />
                </>
              )}
              {currentView === 'settings' && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={returnFromSettings}
                  leftIcon={<Icon name="ArrowLeft" size={14} />}
                  className="!px-2"
                  aria-label="Back"
                >
                  Назад
                </Button>
              )}
              <button
                type="button"
                onClick={goToHome}
                aria-label="Project list breadcrumb"
                className="rounded-xs px-1 py-0.5 text-xs text-subtle transition-colors hover:bg-surface-raised hover:text-muted"
              >
                Проекты
              </button>
              {currentView === 'project' && (
                <>
                  <span className="text-border-strong">/</span>
                  <span className="max-w-48 truncate px-1 text-xs font-medium text-muted">
                    {projectTitle}
                  </span>
                  <span className="text-border-strong">/</span>
                  <span className="truncate px-1 text-xs text-subtle">
                    {stepLabel[pipelineStep]}
                  </span>
                </>
              )}
              {currentView === 'settings' && (
                <>
                  <span className="text-border-strong">/</span>
                  <span className="truncate px-1 text-xs font-medium text-muted">
                    {locationLabel.settings}
                  </span>
                </>
              )}
            </div>

            {jobState && (
              <button
                type="button"
                aria-expanded={queueOpen}
                aria-controls="global-job-queue"
                onClick={() => setQueueOpen((open) => !open)}
                className="relative flex items-center gap-1.5 rounded-sm px-3 py-1 text-xs text-subtle transition-colors hover:bg-surface-raised hover:text-muted"
              >
                <Icon name="ListFilter" size={14} />
                <span className="hidden sm:inline">Очередь</span>
                {activeJobs.length > 0 && (
                  <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[9px] font-bold text-primary-foreground">
                    {activeJobs.length}
                  </span>
                )}
              </button>
            )}
          </header>

          <main
            ref={mainRef}
            tabIndex={-1}
            aria-label={locationLabel[currentView]}
            className="min-h-0 flex-1 overflow-hidden bg-bg outline-none"
          >
            {children}
          </main>

          {queueOpen && jobState && (
            <div
              id="global-job-queue"
              className="absolute inset-y-9 right-0 z-30 w-full max-w-80 border-l border-border bg-surface shadow-lg"
            >
              <div className="flex h-10 items-center justify-between border-b border-border px-4">
                <span className="text-xs font-semibold text-muted">Очередь задач</span>
                <button
                  type="button"
                  aria-label="Close job queue"
                  onClick={() => setQueueOpen(false)}
                  className="rounded-xs p-1 text-subtle transition-colors hover:bg-surface-hover hover:text-text"
                >
                  <Icon name="X" size={14} />
                </button>
              </div>
              {jobQueue}
            </div>
          )}
        </div>
      </div>

      <StatusBar
        projectTitle={projectId ? projectTitle : null}
        durationMs={project?.metadata?.durationMs}
        width={project?.metadata?.width}
        height={project?.metadata?.height}
        activeJobTitle={activeJobs[0]?.title}
        activeJobPercent={activeJobs[0]?.progress.percent}
      />
    </div>
  );
}

function SectionLabel({ label }: { label: string }) {
  return (
    <div className="flex h-7 items-center px-2">
      <span className="flex-1 text-[10px] font-semibold uppercase tracking-wider text-subtle">
        {label}
      </span>
      <Icon name="ChevronDown" size={12} color="muted" />
    </div>
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
        className={`flex w-full items-center gap-3 px-4 py-2 text-left text-xs transition-colors ${
          active
            ? 'bg-primary/7 text-text'
            : disabled
              ? 'cursor-not-allowed text-subtle opacity-55'
              : 'text-muted hover:bg-surface hover:text-text'
        }`}
      >
        <span className="w-3 text-right font-mono text-[10px] text-subtle">{index}</span>
        <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
        <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusClass[status]}`} />
      </button>
    </li>
  );
}

function StatusBar({
  projectTitle,
  durationMs,
  width,
  height,
  activeJobTitle,
  activeJobPercent,
}: {
  projectTitle: string | null;
  durationMs?: number;
  width?: number;
  height?: number;
  activeJobTitle?: string;
  activeJobPercent?: number;
}) {
  return (
    <footer className="hidden h-7 shrink-0 items-center justify-between border-t border-border/70 bg-canvas font-mono text-[10px] text-subtle lg:flex">
      <div className="flex h-full items-center">
        <span className="flex h-full items-center border-r border-border/70 px-2">
          v0.1.0 · local
        </span>
        <span className="max-w-72 truncate border-r border-border/70 px-3">
          {projectTitle ?? 'Auralis Signal'}
        </span>
        {projectTitle && width && height && (
          <span className="border-r border-border/70 px-3">
            {width}×{height}
          </span>
        )}
        {projectTitle && durationMs !== undefined && (
          <span className="px-3">{formatDuration(durationMs)}</span>
        )}
      </div>
      <div className="flex h-full items-center">
        {activeJobTitle && (
          <span className="flex h-full items-center gap-2 border-l border-border/70 px-3 text-primary">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-primary" />
            <span className="max-w-56 truncate">{activeJobTitle}</span>
            {Number.isFinite(activeJobPercent) && <span>{activeJobPercent}%</span>}
          </span>
        )}
        <span className="flex h-full items-center border-l border-border/70 px-3">MP4</span>
        <span className="flex h-full items-center border-l border-border/70 px-3">UTF-8</span>
        <span className="flex h-full items-center border-l border-border/70 px-3">
          Рабочее пространство
        </span>
      </div>
    </footer>
  );
}

type ShellDestinationProps = {
  active: boolean;
  disabled?: boolean;
  icon: IconName;
  label: string;
  visualLabel: string;
  onClick: () => void;
  desktopHidden?: boolean;
};

function ShellDestination({
  active,
  disabled,
  icon,
  label,
  visualLabel,
  onClick,
  desktopHidden = false,
}: ShellDestinationProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      aria-current={active ? 'page' : undefined}
      aria-label={disabled ? `${label} unavailable without an active project` : label}
      title={disabled ? 'Open a project first' : undefined}
      onClick={onClick}
      className={`flex min-w-0 items-center justify-center gap-2 border-primary px-3 py-2 text-xs font-medium transition-colors lg:justify-start lg:border-l-2 lg:px-3 ${
        desktopHidden ? 'lg:hidden' : ''
      } ${
        active
          ? 'border-t-2 bg-primary/7 text-text lg:border-t-0'
          : 'border-t-2 border-transparent text-subtle hover:bg-surface hover:text-muted lg:border-t-0'
      } disabled:cursor-not-allowed disabled:opacity-45`}
    >
      <Icon name={icon} size={14} color={active ? 'primary' : 'muted'} />
      <span className="truncate">{visualLabel}</span>
    </button>
  );
}
