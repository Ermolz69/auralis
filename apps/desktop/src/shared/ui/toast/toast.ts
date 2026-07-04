import { useState, useEffect } from 'react';

type ToastType = 'default' | 'success' | 'warning' | 'danger';

export interface ToastProps {
  id: string;
  type: ToastType;
  title: string;
  description?: string;
  duration?: number;
}

type ToastOptions = Omit<ToastProps, 'id' | 'type'>;

let listeners: ((toasts: ToastProps[]) => void)[] = [];
let toasts: ToastProps[] = [];

const notify = () => {
  listeners.forEach((listener) => listener(toasts));
};

export const toast = {
  show: (type: ToastType, title: string, options?: ToastOptions) => {
    const id = Math.random().toString(36).slice(2, 9);
    toasts = [...toasts, { id, type, title, ...options }];
    notify();
    return id;
  },
  default: (title: string, options?: ToastOptions) => toast.show('default', title, options),
  success: (title: string, options?: ToastOptions) => toast.show('success', title, options),
  warning: (title: string, options?: ToastOptions) => toast.show('warning', title, options),
  danger: (title: string, options?: ToastOptions) => toast.show('danger', title, options),
  error: (title: string, options?: ToastOptions) => toast.show('danger', title, options),
  dismiss: (id: string) => {
    toasts = toasts.filter((t) => t.id !== id);
    notify();
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
