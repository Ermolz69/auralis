import { useEffect } from 'react';
import { toast, useToasts, type ToastProps } from './toast';
import { Icon } from '../icon';

export const Toaster = () => {
  const toasts = useToasts();

  return (
    <div
      className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-[400px] w-full pointer-events-none"
      aria-live="polite"
    >
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  );
};

const ToastItem = ({ toast: t }: { toast: ToastProps }) => {
  useEffect(() => {
    const duration = t.duration || 4000;
    const timer = setTimeout(() => {
      toast.dismiss(t.id);
    }, duration);
    return () => clearTimeout(timer);
  }, [t.id, t.duration]);

  const typeStyles = {
    default: 'border-muted/30 bg-surface text-text',
    success: 'border-success/30 bg-surface text-text',
    warning: 'border-warning/30 bg-surface text-text',
    danger: 'border-danger/30 bg-surface text-text',
  };

  const typeIcon = {
    default: <Icon name="Info" size="md" color="muted" />,
    success: <Icon name="CircleCheck" size="md" color="success" />,
    warning: <Icon name="TriangleAlert" size="md" color="warning" />,
    danger: <Icon name="OctagonX" size="md" color="danger" />,
  };

  return (
    <div
      className={`pointer-events-auto flex items-start gap-3 rounded-lg border p-4 shadow-lg shadow-black/50 transition-all animate-toast-slide-in ${
        typeStyles[t.type]
      }`}
      role="alert"
    >
      <div className="shrink-0 mt-0.5">{typeIcon[t.type]}</div>
      <div className="flex-1 flex flex-col gap-1 min-w-0">
        <h4 className="text-sm font-semibold leading-none">{t.title}</h4>
        {t.description && <p className="text-sm text-muted break-words line-clamp-3">{t.description}</p>}
      </div>
      <button
        type="button"
        onClick={() => toast.dismiss(t.id)}
        className="shrink-0 opacity-70 hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-primary rounded-sm transition-opacity"
      >
        <Icon name="X" size="sm" ariaLabel="Close toast" />
      </button>
    </div>
  );
};
