export type WindowEventChannel<T> = {
  emit(detail: T): void;
  subscribe(listener: (detail: T) => void): () => void;
};

export function createWindowEventChannel<T>(eventName: string): WindowEventChannel<T> {
  return {
    emit(detail) {
      window.dispatchEvent(new CustomEvent<T>(eventName, { detail }));
    },
    subscribe(listener) {
      const handle = (event: Event) => listener((event as CustomEvent<T>).detail);
      window.addEventListener(eventName, handle);
      return () => window.removeEventListener(eventName, handle);
    },
  };
}
