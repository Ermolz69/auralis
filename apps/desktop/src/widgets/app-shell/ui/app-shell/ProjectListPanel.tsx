import { useRef, useState } from 'react';
import { createProject, useProjectContext } from '@/entities/project';
import { toCommandError } from '@/shared/api/contracts';
import { useNavigation } from '@/shared/router';
import { Icon } from '@/shared/ui/icon';
import { toast } from '@/shared/ui/toast';
import { SectionLabel } from './SectionLabel';
import { usePinnedProjects } from '../../model/usePinnedProjects';

export function ProjectListPanel({ height }: { height: number }) {
  const { currentView, setCurrentView, setPipelineStep } = useNavigation();
  const { projectId, setProject, captureToken, validateToken } = useProjectContext();
  const inputRef = useRef<HTMLInputElement>(null);
  const [name, setName] = useState('');
  const [required, setRequired] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const pinned = usePinnedProjects();

  const create = async () => {
    const title = name.trim();
    if (!title) {
      setRequired(true);
      inputRef.current?.focus();
      toast.warning('Укажите название проекта');
      return;
    }
    if (isCreating) return;
    const token = captureToken();
    if (!validateToken(token)) return;
    setIsCreating(true);
    try {
      const project = await createProject(title);
      if (!validateToken(token)) return;
      setProject(project);
      setPipelineStep('source');
      setCurrentView('project');
      setName('');
      setRequired(false);
      toast.success('Project created');
    } catch (error) {
      if (!validateToken(token)) return;
      toast.error(toCommandError(error).message);
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <section className="shrink-0 overflow-y-auto pb-2" style={{ height }}>
      <SectionLabel label="Проекты" />
      <form
        className="flex items-center gap-1.5 px-1.5"
        onSubmit={(event) => {
          event.preventDefault();
          void create();
        }}
      >
        <button
          type="submit"
          aria-label="Create project"
          title="Создать проект"
          disabled={isCreating}
          className="motion-control flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-border bg-surface-raised text-subtle hover:border-border-strong hover:text-primary"
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
            ref={inputRef}
            type="text"
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              if (event.target.value.trim()) setRequired(false);
            }}
            aria-label="Project name"
            aria-invalid={required}
            placeholder="Название проекта..."
            className={`h-8 w-full rounded-sm border bg-surface-raised pl-7 pr-7 text-[11px] text-text outline-none placeholder:text-subtle focus:ring-1 ${required ? 'border-danger focus:border-danger focus:ring-danger/30' : 'border-border focus:border-primary focus:ring-primary/30'}`}
          />
          {name && (
            <button
              type="button"
              aria-label="Clear project name"
              onClick={() => {
                setName('');
                inputRef.current?.focus();
              }}
              className="motion-icon absolute right-1.5 top-1/2 -translate-y-1/2 rounded-xs p-1 text-subtle hover:text-text"
            >
              <Icon name="X" size={11} />
            </button>
          )}
        </div>
      </form>
      <p className="px-2.5 pb-1 pt-2 text-[9px] font-semibold uppercase tracking-wider text-subtle">
        Закреплённые
      </p>
      {pinned.length === 0 ? (
        <p className="px-3 py-1.5 text-[11px] text-subtle">Нет закреплённых проектов</p>
      ) : (
        pinned.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => {
              setProject(item);
              setPipelineStep('source');
              setCurrentView('project');
            }}
            className={`motion-control mx-1 flex w-[calc(100%_-_0.5rem)] items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs ${currentView === 'project' && projectId === item.id ? 'bg-primary/7 text-text' : 'text-muted hover:bg-surface hover:text-text'}`}
          >
            <Icon name="Folder" size={12} color={projectId === item.id ? 'primary' : 'muted'} />
            <span className="min-w-0 flex-1 truncate font-semibold">{item.title}</span>
          </button>
        ))
      )}
      <p className="px-2.5 pt-2 text-[9px] font-semibold uppercase tracking-wider text-subtle">
        Проекты
      </p>
    </section>
  );
}
