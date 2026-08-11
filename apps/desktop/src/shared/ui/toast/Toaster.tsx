import { useEffect, useRef, useState } from 'react';
import { DEFAULT_TOAST_DURATION, toast, useToasts, type ToastProps } from './toast';
import { Icon } from '../icon';

export const Toaster = () => {
  const toasts = useToasts();

  return (
    <div
      className="fixed bottom-4 right-4 z-50 flex w-[min(400px,calc(100vw-2rem))] flex-col gap-2 pointer-events-none"
      aria-live="polite"
      aria-atomic="false"
    >
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  );
};

const ToastItem = ({ toast: t }: { toast: ToastProps }) => {
  const duration = t.duration ?? DEFAULT_TOAST_DURATION;
  const remainingRef = useRef(duration);
  const startedAtRef = useRef(Date.now());
  const timerRef = useRef<number | null>(null);
  const dragXRef = useRef(0);
  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    startedAt: number;
  } | null>(null);
  const [paused, setPaused] = useState(false);
  const [dragX, setDragX] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [snappingBack, setSnappingBack] = useState(false);
  const [swipeDismissed, setSwipeDismissed] = useState(false);

  useEffect(() => {
    if (t.phase !== 'visible' || paused || duration <= 0) return;
    startedAtRef.current = Date.now();
    timerRef.current = window.setTimeout(() => {
      toast.dismiss(t.id);
    }, remainingRef.current);
    return () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    };
  }, [duration, paused, t.id, t.phase]);

  const pauseTimer = () => {
    if (paused || t.phase !== 'visible' || duration <= 0) return;
    remainingRef.current = Math.max(
      0,
      remainingRef.current - (Date.now() - startedAtRef.current),
    );
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    setPaused(true);
  };

  const finishDrag = (cancelled = false) => {
    const drag = dragRef.current;
    if (!drag) return;
    const elapsed = Math.max(1, Date.now() - drag.startedAt);
    const distance = dragXRef.current;
    const velocity = distance / elapsed;
    const shouldDismiss = !cancelled && (distance >= 84 || (distance >= 28 && velocity >= 0.55));
    dragRef.current = null;
    setDragging(false);

    if (shouldDismiss) {
      setSwipeDismissed(true);
      dragXRef.current = window.innerWidth + 420;
      setDragX(window.innerWidth + 420);
      toast.dismiss(t.id);
      return;
    }

    setSnappingBack(true);
    dragXRef.current = 0;
    setDragX(0);
    setPaused(false);
  };

  const typeStyles = {
    default: 'border-muted/35 from-surface-raised to-surface text-muted',
    success: 'border-success/35 from-success-soft/75 to-surface text-success',
    warning: 'border-warning/35 from-warning-soft/75 to-surface text-warning',
    danger: 'border-danger/35 from-danger-soft/75 to-surface text-danger',
  };

  const typeLabels = {
    default: 'Информация',
    success: 'Готово',
    warning: 'Обратите внимание',
    danger: 'Ошибка',
  };

  const typeIcon = {
    default: <Icon name="Info" size="md" color="muted" />,
    success: <Icon name="CircleCheck" size="md" color="success" />,
    warning: <Icon name="TriangleAlert" size="md" color="warning" />,
    danger: <Icon name="OctagonX" size="md" color="danger" />,
  };

  return (
    <div
      className={`group pointer-events-auto relative touch-pan-y select-none overflow-hidden rounded-xl border bg-gradient-to-br shadow-lg shadow-shadow-tint backdrop-blur-md ${
        swipeDismissed
          ? 'transition-[transform,opacity] duration-200 ease-out'
          : dragging
            ? 'cursor-grabbing'
            : t.phase === 'exiting'
              ? 'animate-toast-exit'
              : snappingBack
                ? 'cursor-grab transition-[transform,opacity] duration-200 ease-out'
                : 'cursor-grab animate-toast-enter'
      } ${
        typeStyles[t.type]
      }`}
      role={t.type === 'danger' || t.type === 'warning' ? 'alert' : 'status'}
      onMouseEnter={pauseTimer}
      onMouseLeave={() => {
        if (!dragRef.current) setPaused(false);
      }}
      onPointerDown={(event) => {
        if (t.phase !== 'visible' || (event.target as HTMLElement).closest('button')) return;
        event.currentTarget.setPointerCapture?.(event.pointerId);
        dragRef.current = {
          pointerId: event.pointerId,
          startX: event.clientX,
          startedAt: Date.now(),
        };
        setSnappingBack(false);
        setDragging(true);
        pauseTimer();
      }}
      onPointerMove={(event) => {
        const drag = dragRef.current;
        if (!drag || drag.pointerId !== event.pointerId) return;
        const nextDragX = Math.max(0, event.clientX - drag.startX);
        dragXRef.current = nextDragX;
        setDragX(nextDragX);
      }}
      onPointerUp={(event) => {
        if (dragRef.current?.pointerId !== event.pointerId) return;
        event.currentTarget.releasePointerCapture?.(event.pointerId);
        finishDrag();
      }}
      onPointerCancel={() => finishDrag(true)}
      style={
        dragX > 0
          ? {
              transform: `translate3d(${dragX}px, 0, 0) rotate(${Math.min(4, dragX / 35)}deg)`,
              opacity: Math.max(0, 1 - dragX / 420),
            }
          : undefined
      }
      data-toast-phase={t.phase}
      data-toast-dragging={dragging ? 'true' : 'false'}
    >
      <div className="absolute inset-y-0 left-0 w-0.5 bg-current opacity-85" aria-hidden="true" />
      <div className="flex items-start gap-3 p-3.5 pb-4 text-text">
        <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-current/20 bg-canvas/35 shadow-inner">
          {typeIcon[t.type]}
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="font-mono text-[9px] font-semibold uppercase tracking-[0.16em] text-current">
            {typeLabels[t.type]}
          </span>
          <h4 className="text-sm font-semibold leading-tight text-text">{t.title}</h4>
          {t.description && (
            <p className="line-clamp-3 break-words text-xs leading-relaxed text-muted">
              {t.description}
            </p>
          )}
          {t.action && (
            <button
              type="button"
              className="mt-1 w-fit rounded-sm border border-current/30 bg-current/10 px-2 py-1 text-[11px] font-semibold text-current transition-colors hover:bg-current/15 focus:outline-none focus:ring-2 focus:ring-current/40"
              onClick={() => {
                toast.dismiss(t.id);
                t.action?.onClick();
              }}
            >
              {t.action.label}
            </button>
          )}
        </div>
        {t.dismissible !== false && (
          <button
            type="button"
            aria-label="Close toast"
            onClick={() => toast.dismiss(t.id)}
            className="shrink-0 rounded-md p-1 text-muted opacity-65 transition-all hover:bg-surface-hover hover:text-text hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-primary"
          >
            <Icon name="X" size={14} />
          </button>
        )}
      </div>
      {duration > 0 && (
        <div className="absolute inset-x-0 bottom-0 h-1 bg-canvas/45" aria-hidden="true">
          <div
            className="toast-countdown h-full origin-left bg-current shadow-[0_0_10px_currentColor]"
            style={{
              animationDuration: `${duration}ms`,
              animationPlayState: paused ? 'paused' : 'running',
            }}
          />
        </div>
      )}
    </div>
  );
};
