import { useEffect, useRef, type KeyboardEvent } from 'react';
import { Icon, type IconName } from '@/shared/ui/icon';

type ProjectListContextMenuProps = {
  position: { x: number; y: number };
  projectLabel: string;
  hasAvatar: boolean;
  pinned: boolean;
  onDismiss: (restoreFocus: boolean) => void;
  onRename: () => void;
  onChooseAvatar: () => void;
  onRemoveAvatar: () => void;
  onTogglePinned: () => void;
  onOpenFolder: () => void;
  onDelete: () => void;
};

export function ProjectListContextMenu({
  position,
  projectLabel,
  hasAvatar,
  pinned,
  onDismiss,
  onRename,
  onChooseAvatar,
  onRemoveAvatar,
  onTogglePinned,
  onOpenFolder,
  onDelete,
}: ProjectListContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
    const close = () => onDismiss(false);
    window.addEventListener('click', close);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('blur', close);
    };
  }, [onDismiss]);

  const run = (action: () => void) => {
    onDismiss(true);
    action();
  };

  return (
    <div
      ref={menuRef}
      role="menu"
      aria-label={`Actions for ${projectLabel}`}
      onClick={(event) => event.stopPropagation()}
      onKeyDown={(event) => handleMenuKeyDown(event, () => onDismiss(true))}
      style={{
        left: Math.min(position.x, window.innerWidth - 190),
        top: Math.min(position.y, window.innerHeight - 230),
      }}
      className="fixed z-50 w-48 rounded-md border border-border bg-surface p-1 shadow-lg"
    >
      <MenuItem icon="Pencil" label="Переименовать" onClick={() => run(onRename)} />
      <MenuItem icon="ImagePlus" label="Выбрать аватарку" onClick={() => run(onChooseAvatar)} />
      {hasAvatar && (
        <MenuItem icon="ImageOff" label="Убрать аватарку" onClick={() => run(onRemoveAvatar)} />
      )}
      <MenuItem
        icon={pinned ? 'PinOff' : 'Pin'}
        label={pinned ? 'Открепить' : 'Закрепить'}
        onClick={() => run(onTogglePinned)}
      />
      <MenuItem icon="FolderOpen" label="Открыть папку проекта" onClick={() => run(onOpenFolder)} />
      <div className="my-1 h-px bg-border" />
      <MenuItem icon="Trash2" label="Удалить" danger onClick={() => run(onDelete)} />
    </div>
  );
}

function handleMenuKeyDown(event: KeyboardEvent<HTMLDivElement>, onEscape: () => void) {
  const items = Array.from(
    event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'),
  );
  const currentIndex = items.indexOf(document.activeElement as HTMLButtonElement);
  let nextIndex: number | null = null;
  if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % items.length;
  if (event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + items.length) % items.length;
  if (event.key === 'Home') nextIndex = 0;
  if (event.key === 'End') nextIndex = items.length - 1;
  if (nextIndex !== null) {
    event.preventDefault();
    items[nextIndex]?.focus();
  }
  if (event.key === 'Escape') {
    event.preventDefault();
    event.stopPropagation();
    onEscape();
  }
}

function MenuItem({
  icon,
  label,
  onClick,
  danger = false,
}: {
  icon: IconName;
  label: string;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      tabIndex={-1}
      onClick={onClick}
      className={`flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs hover:bg-surface-hover ${danger ? 'text-danger' : 'text-muted'}`}
    >
      <Icon name={icon} size={13} />
      {label}
    </button>
  );
}
