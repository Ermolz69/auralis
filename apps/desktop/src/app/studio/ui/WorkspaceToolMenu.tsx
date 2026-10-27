import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { toolNames } from '@/entities/library';
import type { LibraryTool } from '@/shared/api/contracts/library';
import { Icon } from '@/shared/ui/icon';

export function WorkspaceToolMenu({
  materialName,
  onTool,
  onCreateText,
  onImport,
}: {
  materialName?: string;
  onTool: (tool: LibraryTool) => void;
  onCreateText: () => void;
  onImport: () => void;
}) {
  const [position, setPosition] = useState<{ left: number; top: number } | null>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const dismiss = (restoreFocus = false) => {
    setPosition(null);
    if (restoreFocus) trigger.current?.focus();
  };
  useEffect(() => {
    if (!position) return;
    menu.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
    const outside = (event: PointerEvent) => {
      if (
        !menu.current?.contains(event.target as Node) &&
        !trigger.current?.contains(event.target as Node)
      )
        setPosition(null);
    };
    const close = () => setPosition(null);
    document.addEventListener('pointerdown', outside);
    window.addEventListener('resize', close);
    window.addEventListener('blur', close);
    return () => {
      document.removeEventListener('pointerdown', outside);
      window.removeEventListener('resize', close);
      window.removeEventListener('blur', close);
    };
  }, [position]);
  const run = (action: () => void) => {
    dismiss(true);
    action();
  };
  const itemClass =
    'w-full rounded px-3 py-2 text-left text-sm hover:bg-surface-raised focus:bg-surface-active';
  return (
    <>
      <button
        ref={trigger}
        type="button"
        aria-label="Открыть инструмент"
        title="Открыть инструмент для выбранного материала"
        aria-haspopup="menu"
        aria-expanded={!!position}
        aria-controls={position ? menuId : undefined}
        className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[6px] border border-border bg-surface hover:bg-surface-raised"
        onClick={() => {
          if (position) return dismiss();
          const rect = trigger.current!.getBoundingClientRect();
          setPosition({
            left: Math.max(8, Math.min(rect.left, window.innerWidth - 296)),
            top: rect.bottom + 6,
          });
        }}
      >
        <Icon name="Plus" size={18} />
      </button>
      {position &&
        createPortal(
          <div
            ref={menu}
            id={menuId}
            role="menu"
            aria-label="Инструменты для материала"
            style={{ ...position, maxHeight: `calc(100vh - ${position.top + 8}px)` }}
            className="fixed z-50 w-72 overflow-y-auto rounded-lg border border-border bg-surface p-1 shadow-lg"
            onBlur={(event) => {
              if (
                !event.currentTarget.contains(event.relatedTarget as Node) &&
                event.relatedTarget !== trigger.current
              )
                dismiss();
            }}
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                event.preventDefault();
                event.stopPropagation();
                dismiss(true);
                return;
              }
              const items = Array.from(
                event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'),
              );
              const index = items.indexOf(document.activeElement as HTMLButtonElement);
              const next =
                event.key === 'Home'
                  ? 0
                  : event.key === 'End'
                    ? items.length - 1
                    : event.key === 'ArrowDown'
                      ? (index + 1) % items.length
                      : event.key === 'ArrowUp'
                        ? (index - 1 + items.length) % items.length
                        : null;
              if (next !== null) {
                event.preventDefault();
                items[next]?.focus();
              }
            }}
          >
            <p className="break-words border-b border-border px-3 py-2 text-xs text-muted">
              {materialName ?? 'Новый ввод'}
            </p>
            {(Object.entries(toolNames) as [LibraryTool, string][]).map(([tool, name]) => (
              <button
                key={tool}
                type="button"
                role="menuitem"
                tabIndex={-1}
                className={itemClass}
                onClick={() => run(() => onTool(tool))}
              >
                {name}
              </button>
            ))}
            <div role="separator" className="my-1 border-t border-border" />
            <button
              type="button"
              role="menuitem"
              tabIndex={-1}
              className={itemClass}
              onClick={() => run(onCreateText)}
            >
              Новый текст
            </button>
            <button
              type="button"
              role="menuitem"
              tabIndex={-1}
              className={itemClass}
              onClick={() => run(onImport)}
            >
              Импорт файлов
            </button>
          </div>,
          document.body,
        )}
    </>
  );
}
