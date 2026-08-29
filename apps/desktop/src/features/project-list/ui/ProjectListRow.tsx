import { useRef, useState, type RefCallback } from 'react';
import {
  getProjectPreferences,
  normalizeProjectAvatar,
  ProjectAvatarError,
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
import { toast } from '@/shared/ui/toast';
import { ProjectListContextMenu } from './ProjectListContextMenu';

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
  const openButtonElementRef = useRef<HTMLButtonElement>(null);
  const menuTriggerRef = useRef<HTMLElement | null>(null);
  const closeMenu = (restoreFocus: boolean) => {
    const trigger = menuTriggerRef.current;
    setMenu(null);
    if (restoreFocus) {
      queueMicrotask(() => {
        if (trigger?.isConnected) trigger.focus();
      });
    }
  };
  const openMenu = (x: number, y: number, target: EventTarget | null) => {
    menuTriggerRef.current =
      (target instanceof HTMLElement ? target.closest<HTMLElement>('button') : null) ??
      openButtonElementRef.current;
    setMenu({ x, y });
  };
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
        openMenu(event.clientX, event.clientY, event.target);
      }}
      onKeyDown={(event) => {
        const opensMenu =
          (event.shiftKey && event.key === 'F10') ||
          event.key === 'ContextMenu' ||
          event.key === 'Menu';
        if (!opensMenu) return;
        event.preventDefault();
        const target = event.target instanceof HTMLElement ? event.target : event.currentTarget;
        const bounds = target.getBoundingClientRect();
        openMenu(bounds.left, bounds.bottom, target);
      }}
    >
      <input
        ref={avatarInputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={async (event) => {
          const file = event.target.files?.[0];
          event.currentTarget.value = '';
          if (!file) return;
          try {
            const avatar = await normalizeProjectAvatar(file);
            setPreferences(updateProjectPreferences(project.id, { avatar }));
          } catch (error) {
            toast.error(
              error instanceof ProjectAvatarError
                ? error.message
                : 'Не удалось обработать аватарку',
            );
          }
        }}
      />
      <button
        type="button"
        ref={(element) => {
          openButtonElementRef.current = element;
          openButtonRef(element);
        }}
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
        <ProjectListContextMenu
          position={menu}
          projectLabel={displayTitle}
          hasAvatar={preferences.avatar !== null}
          pinned={preferences.pinned}
          onDismiss={closeMenu}
          onRename={() => {
            const title = window.prompt('Новое название проекта', project.title)?.trim();
            if (title && title !== project.title) onRename(project, title);
          }}
          onChooseAvatar={() => avatarInputRef.current?.click()}
          onRemoveAvatar={() => {
            setPreferences(updateProjectPreferences(project.id, { avatar: null }));
          }}
          onTogglePinned={() => {
            setPreferences(updateProjectPreferences(project.id, { pinned: !preferences.pinned }));
          }}
          onOpenFolder={() => onOpenFolder(project)}
          onDelete={() => onDelete(project)}
        />
      )}
    </div>
  );
};
