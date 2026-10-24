import { useEffect, useState } from 'react';
import { invoke } from '@/shared/api/tauri';
import { describeCommandError } from '@/shared/api/contracts/error';
import type { SpeechRunView } from '@/shared/api/contracts/speech';
import { AudioAssemblyPanel } from './AudioAssemblyPanel';

export function AudioAssemblyWorkspace({ projectId }: { projectId: string }) {
  const [sources, setSources] = useState<SpeechRunView[]>([]);
  const [selected, setSelected] = useState('');
  const [revision, setRevision] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    void invoke('list_speech_runs_cmd', { projectId })
      .then((runs) => {
        if (!active) return;
        setSources(runs.filter((run) => run.result?.ready));
      })
      .catch((reason) => {
        if (active) setError(describeCommandError(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [projectId, revision]);
  const source = sources.find((run) => run.run_id === selected);
  return (
    <div className="mt-4 space-y-2 text-xs text-muted">
      <label>
        Готовый исходный синтез
        <select
          aria-label="Готовый исходный синтез"
          className="ml-2 max-w-full rounded border border-border bg-surface p-2 text-text"
          value={selected}
          onChange={(event) => setSelected(event.target.value)}
        >
          <option value="">Выберите источник итогового аудио</option>
          {sources.map((run) => (
            <option key={run.run_id} value={run.run_id}>
              {run.run_id.slice(0, 8)} · {run.profile_id} · {run.utterance_count} реплик
            </option>
          ))}
        </select>
      </label>
      <button
        className="rounded border border-border px-3 py-2 text-text disabled:opacity-40"
        disabled={loading}
        onClick={() => setRevision((value) => value + 1)}
      >
        Обновить готовую озвучку
      </button>
      {error && <p role="alert">{error}</p>}
      <AudioAssemblyPanel
        key={`${projectId}:${source?.run_id ?? ''}`}
        projectId={projectId}
        source={source}
      />
    </div>
  );
}
