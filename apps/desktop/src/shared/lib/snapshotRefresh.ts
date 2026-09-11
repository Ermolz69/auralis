export function subscribeSnapshotRefresh(refresh: () => Promise<unknown>) {
  let disposed = false;
  let pending = false;
  let lastRefresh = 0;
  let timer: ReturnType<typeof setTimeout>;
  const run = async () => {
    if (disposed || pending || document.hidden || Date.now() - lastRefresh < 1000) return;
    pending = true;
    lastRefresh = Date.now();
    try {
      await refresh();
    } catch {
      /* The snapshot owner exposes its read error. */
    } finally {
      pending = false;
    }
  };
  const schedule = () => {
    timer = setTimeout(() => {
      void run().finally(() => {
        if (!disposed) schedule();
      });
    }, 30_000);
  };
  const focus = () => {
    void run();
  };
  schedule();
  window.addEventListener('focus', focus);
  document.addEventListener('visibilitychange', focus);
  return () => {
    disposed = true;
    clearTimeout(timer);
    window.removeEventListener('focus', focus);
    document.removeEventListener('visibilitychange', focus);
  };
}
