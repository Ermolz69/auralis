import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@/shared/api/tauri';
import { toCommandError, type PendingYoutubeImport } from '@/shared/api/contracts';
import {
  projectUpdated,
  subscribeYoutubeImports,
  subscribeProjectChanges,
  youtubeImportsChanged,
} from '@/entities/project';
import { Button } from '@/shared/ui/button';
import { toast } from '@/shared/ui/toast';

export function PendingYoutubeImports({ onCompleted }: { onCompleted: () => void }) {
  const [items, setItems] = useState<PendingYoutubeImport[]>([]);
  const [busy, setBusy] = useState<{ id: string; resume: boolean } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const mounted = useRef(false);
  const active = useRef(false);
  const generation = useRef(0);
  const reload = useCallback(async () => {
    const current = ++generation.current;
    try {
      const pending = await invoke('list_pending_youtube_imports_cmd');
      if (mounted.current && current === generation.current) {
        setItems(pending);
        setError(null);
      }
    } catch (error) {
      if (mounted.current && current === generation.current)
        setError(toCommandError(error).message);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void reload();
    const unsubscribeImports = subscribeYoutubeImports(() => void reload());
    const unsubscribeProjects = subscribeProjectChanges(() => void reload());
    return () => {
      mounted.current = false;
      generation.current += 1;
      unsubscribeImports();
      unsubscribeProjects();
    };
  }, [reload]);

  const run = async (item: PendingYoutubeImport, resume: boolean) => {
    if (active.current) return;
    active.current = true;
    setBusy({ id: item.projectId, resume });
    try {
      if (resume) {
        const project = await invoke('resume_youtube_import_cmd', { projectId: item.projectId });
        projectUpdated(project);
        onCompleted();
        toast.success('YouTube import completed');
      } else {
        await invoke('discard_youtube_import_cmd', { projectId: item.projectId });
        toast.success('Pending import discarded');
      }
    } catch (error) {
      toast.error(toCommandError(error).message);
    } finally {
      active.current = false;
      if (mounted.current) setBusy(null);
      youtubeImportsChanged();
    }
  };

  if (error)
    return (
      <div role="status">
        Pending YouTube imports: {error} <Button onClick={() => void reload()}>Retry</Button>
      </div>
    );
  if (!items.length) return null;
  return (
    <section aria-label="Pending YouTube imports" className="flex flex-col gap-2">
      <h3>Unfinished YouTube imports</h3>
      {items.map((item) => (
        <div key={item.projectId} className="flex items-center gap-2">
          <span className="min-w-0 flex-1 truncate">{item.title}</span>
          <span role="status">
            {busy?.id === item.projectId
              ? busy.resume
                ? 'Resuming…'
                : 'Discarding…'
              : item.state === 'Staged'
                ? 'Ready to save'
                : item.state === 'Failed'
                  ? 'Retry available'
                  : 'Pending'}
          </span>
          <Button disabled={busy !== null} onClick={() => void run(item, true)}>
            Resume
          </Button>
          <Button disabled={busy !== null} onClick={() => void run(item, false)}>
            Discard
          </Button>
        </div>
      ))}
    </section>
  );
}
