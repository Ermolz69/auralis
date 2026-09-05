import { useEffect, type ReactNode } from 'react';
import { Icon } from '@/shared/ui/icon';

export function JobQueueDrawer({
  children,
  onClose,
}: {
  children?: ReactNode;
  onClose: () => void;
}) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  return (
    <div
      id="global-job-queue"
      className="absolute inset-y-9 right-0 z-30 w-full max-w-80 animate-drawer-in border-l border-border bg-surface shadow-lg"
    >
      <div className="flex h-10 items-center justify-between border-b border-border px-4">
        <span className="text-xs font-semibold text-muted">Очередь задач</span>
        <button
          type="button"
          aria-label="Close job queue"
          onClick={onClose}
          className="motion-control rounded-xs p-1 text-subtle hover:bg-surface-hover hover:text-text"
        >
          <Icon name="X" size={14} />
        </button>
      </div>
      {children}
    </div>
  );
}
