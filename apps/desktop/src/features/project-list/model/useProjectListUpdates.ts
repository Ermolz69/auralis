import { useEffect } from 'react';
import { listen } from '@/shared/api/tauri';
import { toCommandError } from '@/shared/api/contracts';

export function useProjectListUpdates(
  refresh: () => Promise<void>,
  deletingProjectId: { current: string | null },
) {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      try {
        const registered = await listen(
          'project-updated',
          (event) => {
            if (event.payload.projectId !== deletingProjectId.current) void refresh();
          },
          { onInvalidPayload: () => void refresh() },
        );
        if (cancelled) registered();
        else unlisten = registered;
      } catch (error) {
        if (!cancelled) {
          console.warn('Failed to setup Tauri listeners:', toCommandError(error));
        }
      }
    };

    void setup();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [deletingProjectId, refresh]);
}
