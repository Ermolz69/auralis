import { useState, useEffect } from 'react';

export type ToastType = 'default' | 'success' | 'warning' | 'danger';
export type ToastPhase = 'visible' | 'exiting';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface ToastProps {
  id: string;
  type: ToastType;
  phase: ToastPhase;
  title: string;
  description?: string;
  duration?: number;
  action?: ToastAction;
  dismissible?: boolean;
}

export type ToastOptions = Omit<ToastProps, 'id' | 'type' | 'phase' | 'title'>;

export const DEFAULT_TOAST_DURATION = 5000;
export const TOAST_EXIT_DURATION = 260;

let listeners: ((toasts: ToastProps[]) => void)[] = [];
let toasts: ToastProps[] = [];

const notify = () => {
  listeners.forEach((listener) => listener(toasts));
};

export const toast = {
  show: (type: ToastType, title: string, options?: ToastOptions) => {
    const id = Math.random().toString(36).slice(2, 9);
    toasts = [...toasts, { id, type, phase: 'visible', title, ...options }];
    notify();
    return id;
  },
  default: (title: string, options?: ToastOptions) => toast.show('default', title, options),
  success: (title: string, options?: ToastOptions) => toast.show('success', title, options),
  warning: (title: string, options?: ToastOptions) => toast.show('warning', title, options),
  danger: (title: string, options?: ToastOptions) => toast.show('danger', title, options),
  error: (title: string, options?: ToastOptions) => toast.show('danger', title, options),
  dismiss: (id: string) => {
    const target = toasts.find((item) => item.id === id);
    if (!target || target.phase === 'exiting') return;
    toasts = toasts.map((item) => (item.id === id ? { ...item, phase: 'exiting' } : item));
    notify();
    globalThis.setTimeout(() => {
      toasts = toasts.filter((item) => item.id !== id);
      notify();
    }, TOAST_EXIT_DURATION);
  },
  dismissAll: () => {
    for (const item of toasts) toast.dismiss(item.id);
  },
};

export const useToasts = () => {
  const [currentToasts, setCurrentToasts] = useState<ToastProps[]>(toasts);

  useEffect(() => {
    const listener = (newToasts: ToastProps[]) => {
      setCurrentToasts(newToasts);
    };
    listeners.push(listener);
    return () => {
      listeners = listeners.filter((l) => l !== listener);
    };
  }, []);

  return currentToasts;
};
