import { useCallback, useEffect, useRef, useState } from 'react';
import { subscribeProjectChanges, type Project } from '@/entities/project';
import { formatProjectTitle } from '@/entities/media';
import { invoke } from '@/shared/api/tauri';
import { toCommandError } from '@/shared/api/contracts';
import { Button } from '@/shared/ui/button';

export function ArtifactRecovery({ projects }: { projects: Project[] }) {
  const [ids, setIds] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queued, setQueued] = useState(false);
  const active = useRef(false);
  const mounted = useRef(false);
  const generation = useRef(0);
  const reload = useCallback(async () => {
    const request = ++generation.current;
    try {
      const result = await invoke('list_artifact_recovery_cmd');
      if (mounted.current && request === generation.current) {
        setIds(result);
        setError(null);
      }
    } catch (error) {
      if (mounted.current && request === generation.current)
        setError(toCommandError(error).message);
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    void reload();
    const refresh = () => {
      void reload();
    };
    const unsubscribe = subscribeProjectChanges(refresh);
    window.addEventListener('focus', refresh);
    return () => {
      mounted.current = false;
      generation.current += 1;
      unsubscribe();
      window.removeEventListener('focus', refresh);
    };
  }, [reload]);
  const retry = async (projectId: string) => {
    if (active.current) return;
    active.current = true;
    setBusy(true);
    try {
      await invoke('retry_artifact_finalization_cmd', { projectId });
      if (mounted.current) {
        setQueued(true);
        await reload();
      }
    } catch (error) {
      if (mounted.current) setError(toCommandError(error).message);
    } finally {
      active.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  if (!ids.length && !error && !queued) return null;
  return (
    <section aria-label="Artifact recovery" className="flex flex-col gap-2">
      {error && <p role="alert">File recovery: {error}</p>}
      {queued && <p role="status">File finalization queued. Existing downloads will be reused.</p>}
      {ids.map((id) => {
        const project = projects.find((project) => project.id === id);
        return (
          <div key={id} className="flex items-center gap-2">
            <span>
              {project ? formatProjectTitle(project.title, project.source) : 'Project'}: file
              finalization stopped
            </span>
            <Button disabled={busy} onClick={() => void retry(id)}>
              Retry file finalization
            </Button>
          </div>
        );
      })}
      <Button disabled={busy} onClick={() => void reload()}>
        Refresh file recovery
      </Button>
    </section>
  );
}
