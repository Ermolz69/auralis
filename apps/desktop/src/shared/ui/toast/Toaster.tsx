import { useToasts } from './toast';
import { ToastItem } from './ToastItem';

export const Toaster = () => {
  const toasts = useToasts();

  return (
    <div
      className="fixed bottom-4 right-4 z-50 flex w-[min(400px,calc(100vw-2rem))] flex-col gap-2 pointer-events-none"
      aria-live="polite"
      aria-atomic="false"
    >
      {toasts.map((currentToast) => (
        <ToastItem key={currentToast.id} toast={currentToast} />
      ))}
    </div>
  );
};
