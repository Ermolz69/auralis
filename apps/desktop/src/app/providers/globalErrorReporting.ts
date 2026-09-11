export function installGlobalErrorReporting(
  target: Window = window,
  suppressDefault = import.meta.env.PROD,
): () => void {
  let windowStart = Date.now();
  let reported = 0;
  const report = (kind: 'error' | 'unhandledrejection') => {
    const now = Date.now();
    if (now - windowStart >= 60_000) {
      windowStart = now;
      reported = 0;
    }
    if (reported++ < 20)
      console.error('Auralis UI unhandled failure', { code: 'UI_UNHANDLED', kind });
  };
  const onError = (event: ErrorEvent) => {
    report('error');
    if (suppressDefault && event.cancelable) event.preventDefault();
  };
  const onRejection = (event: PromiseRejectionEvent) => {
    report('unhandledrejection');
    if (suppressDefault && event.cancelable) event.preventDefault();
  };
  target.addEventListener('error', onError);
  target.addEventListener('unhandledrejection', onRejection);
  return () => {
    target.removeEventListener('error', onError);
    target.removeEventListener('unhandledrejection', onRejection);
  };
}
