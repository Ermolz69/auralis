import { useEffect, useRef, useState, type RefCallback } from 'react';
import {
  getProjectPreferences,
  updateProjectPreferences,
  type Project,
} from '@/entities/project';
import {
  formatProjectStatus,
  formatProjectTitle,
  formatSourceLabel,
  getProjectStatusTone,
} from '@/entities/media';
import { Button } from '@/shared/ui/button';
import { Icon } from '@/shared/ui/icon';

type ProjectListRowProps = {
  project: Project;
  isDeleting: boolean;
  isAnyDeleting: boolean;
  openButtonRef: RefCallback<HTMLButtonElement>;
  deleteButtonRef: RefCallback<HTMLButtonElement>;
  onOpen: (project: Project) => void;
  onDelete: (project: Project) => void;
  onRename?: (project: Project, title: string) => void;
  onOpenFolder?: (project: Project) => void;
};

export const ProjectListRow = ({
  project,
  isDeleting,
  isAnyDeleting,
  openButtonRef,
  deleteButtonRef,
  onOpen,
  onDelete,
  onRename = () => undefined,
  onOpenFolder = () => undefined,
}: ProjectListRowProps) => {
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [preferences, setPreferences] = useState(() => getProjectPreferences(project.id));
  const avatarInputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener('click', close);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('blur', close);
    };
  }, [menu]);
  const displayTitle = formatProjectTitle(project.title, project.source);
  const sourceLabel = formatSourceLabel(project.source);
  const statusLabel = formatProjectStatus(project.status);
  const statusTone = getProjectStatusTone(project.status);
  const statusClass = {
    success: 'bg-success',
    danger: 'bg-danger',
    primary: 'animate-pulse bg-primary signal-glow-sm',
    warning: 'bg-warning',
    muted: 'bg-muted',
  }[statusTone];
  const updatedLabel = new Date(project.updatedAt).toLocaleDateString();

  return (
    <div
      className={`group flex min-w-0 items-center rounded-md transition-colors hover:bg-surface-raised ${isDeleting ? 'opacity-45' : ''}`}
      aria-busy={isDeleting}
      onContextMenu={(event) => {
        event.preventDefault();
        setMenu({ x: event.clientX, y: event.clientY });
      }}
    >
      <input
        ref={avatarInputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.currentTarget.value = '';
          if (!file) return;
          const reader = new FileReader();
          reader.onload = () => {
            if (typeof reader.result !== 'string') return;
            setPreferences(updateProjectPreferences(project.id, { avatar: reader.result }));
          };
          reader.readAsDataURL(file);
        }}
      />
      <button
        type="button"
        ref={openButtonRef}
        className="flex min-w-0 flex-1 items-center gap-3 rounded-md p-2.5 text-left focus:outline-none"
        onClick={() => onOpen(project)}
        disabled={isAnyDeleting}
        aria-label={`Open ${displayTitle}. Status: ${statusLabel}. Source: ${sourceLabel}`}
      >
        <span className="flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-sm border border-border bg-surface-raised text-muted transition-colors group-hover:bg-surface-hover">
          {preferences.avatar ? (
            <img src={preferences.avatar} alt="" className="h-full w-full object-cover" />
          ) : (
            <Icon
              name={
                project.source?.kind === 'youtubeUrl' || project.source?.kind === 'remoteUrl'
                  ? 'Video'
                  : 'Folder'
              }
              size={14}
            />
          )}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-sm font-medium text-text">{displayTitle}</span>
          <span className="mt-0.5 flex min-w-0 items-center gap-1.5 text-[11px] text-subtle">
            <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusClass}`} />
            <span>{statusLabel}</span>
            <span aria-hidden="true">·</span>
            <span className="truncate" aria-label={`Source: ${sourceLabel}`}>
              {sourceLabel}
            </span>
          </span>
        </span>
        <span className="hidden shrink-0 font-mono text-[10px] text-subtle sm:inline">
          {updatedLabel}
        </span>
      </button>

      <div className="shrink-0 pr-2">
        <Button
          ref={deleteButtonRef}
          variant="ghost"
          size="sm"
          className="!px-2 opacity-0 transition-opacity focus:opacity-100 group-focus-within:opacity-100 group-hover:opacity-100"
          loading={isDeleting}
          disabled={isAnyDeleting}
          onClick={() => onDelete(project)}
          title="Delete Project"
          aria-label={`Delete ${displayTitle}`}
          leftIcon={!isDeleting ? <Icon name="Trash2" size={14} /> : undefined}
        />
      </div>
      {menu && (
        <div
          role="menu"
          aria-label={`Actions for ${displayTitle}`}
          onClick={(event) => event.stopPropagation()}
          style={{ left: Math.min(menu.x, window.innerWidth - 190), top: Math.min(menu.y, window.innerHeight - 230) }}
          className="fixed z-50 w-48 rounded-md border border-border bg-surface p-1 shadow-lg"
        >
          <MenuItem
            icon="Pencil"
            label="Переименовать"
            onClick={() => {
              setMenu(null);
              const title = window.prompt('Новое название проекта', project.title)?.trim();
              if (title && title !== project.title) onRename(project, title);
            }}
          />
          <MenuItem icon="ImagePlus" label="Выбрать аватарку" onClick={() => { setMenu(null); avatarInputRef.current?.click(); }} />
          {preferences.avatar && (
            <MenuItem icon="ImageOff" label="Убрать аватарку" onClick={() => { setPreferences(updateProjectPreferences(project.id, { avatar: null })); setMenu(null); }} />
          )}
          <MenuItem
            icon={preferences.pinned ? 'PinOff' : 'Pin'}
            label={preferences.pinned ? 'Открепить' : 'Закрепить'}
            onClick={() => {
              setPreferences(updateProjectPreferences(project.id, { pinned: !preferences.pinned }));
              setMenu(null);
            }}
          />
          <MenuItem icon="FolderOpen" label="Открыть папку проекта" onClick={() => { setMenu(null); onOpenFolder(project); }} />
          <div className="my-1 h-px bg-border" />
          <MenuItem icon="Trash2" label="Удалить" danger onClick={() => { setMenu(null); onDelete(project); }} />
        </div>
      )}
    </div>
  );
};

function MenuItem({ icon, label, onClick, danger = false }: { icon: 'Pencil' | 'ImagePlus' | 'ImageOff' | 'Pin' | 'PinOff' | 'FolderOpen' | 'Trash2'; label: string; onClick: () => void; danger?: boolean }) {
  return (
    <button type="button" role="menuitem" onClick={onClick} className={`flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs hover:bg-surface-hover ${danger ? 'text-danger' : 'text-muted'}`}>
      <Icon name={icon} size={13} />
      {label}
    </button>
  );
}
