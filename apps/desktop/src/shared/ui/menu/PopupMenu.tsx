import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
export type MenuEntry = {
  id: string;
  label: string;
  disabled?: string;
  run?: () => void;
  children?: MenuEntry[];
};
export function PopupMenu({
  items,
  x,
  y,
  label,
  close,
  returnFocus,
}: {
  items: MenuEntry[];
  x: number;
  y: number;
  label: string;
  close: () => void;
  returnFocus?: HTMLElement | null;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [stack, setStack] = useState<{ label: string; items: MenuEntry[] }[]>([]);
  const current = stack.at(-1)?.items ?? items;
  const top = Math.max(
    4,
    Math.min(y, window.innerHeight - Math.min(current.length * 48 + 16, 400)),
  );
  useLayoutEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
  }, [current]);
  useEffect(() => {
    const previous = returnFocus ?? (document.activeElement as HTMLElement | null);
    const outside = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) close();
    };
    document.addEventListener('pointerdown', outside);
    window.addEventListener('blur', close);
    return () => {
      document.removeEventListener('pointerdown', outside);
      window.removeEventListener('blur', close);
      previous?.focus();
    };
  }, [close, returnFocus]);
  return createPortal(
    <div
      ref={ref}
      role="menu"
      aria-label={stack.at(-1)?.label ?? label}
      className="fixed z-[100] min-w-64 max-w-80 overflow-y-auto rounded-md border border-border-strong bg-surface p-1 text-sm text-text shadow-lg [app-region:no-drag]"
      style={{
        left: Math.max(4, Math.min(x, window.innerWidth - 324)),
        top,
        maxHeight: Math.max(64, window.innerHeight - top - 8),
      }}
      onKeyDown={(event) => {
        const buttons = [...ref.current!.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
          event.preventDefault();
          buttons[
            event.key === 'Home'
              ? 0
              : event.key === 'End'
                ? buttons.length - 1
                : (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length
          ]?.focus();
        }
        if (event.key === 'Escape') {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
        if (event.key === 'ArrowLeft' && stack.length) {
          event.preventDefault();
          setStack(stack.slice(0, -1));
        }
        if (event.key === 'ArrowRight') {
          event.preventDefault();
          (document.activeElement as HTMLButtonElement)?.click();
        }
      }}
    >
      {stack.length > 0 && (
        <button
          role="menuitem"
          className="mb-1 w-full border-b border-border p-2 text-left"
          onClick={() => setStack(stack.slice(0, -1))}
        >
          ← {stack.at(-1)?.label}
        </button>
      )}
      {current.map((item) => (
        <button
          key={item.id}
          role="menuitem"
          aria-label={item.label}
          aria-disabled={!!item.disabled}
          aria-haspopup={item.children ? 'menu' : undefined}
          title={item.disabled}
          className={`flex w-full items-center justify-between gap-5 rounded px-3 py-2 text-left focus:bg-surface-active focus:outline-none ${item.disabled ? 'text-muted' : 'hover:bg-surface-raised'}`}
          onClick={() => {
            if (item.disabled) return;
            if (item.children) setStack([...stack, { label: item.label, items: item.children }]);
            else {
              close();
              item.run?.();
            }
          }}
        >
          <span>
            {item.label}
            {item.disabled && <span className="mt-0.5 block text-xs">{item.disabled}</span>}
          </span>
          {item.children && <span aria-hidden>›</span>}
        </button>
      ))}
    </div>,
    document.body,
  );
}
