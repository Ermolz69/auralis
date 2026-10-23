import { describeCommandError } from '@/shared/api/contracts/error';
import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { invoke } from '@/shared/api/tauri';
import type { Artifact } from '@/shared/api/contracts/artifact';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { useLibrary } from '@/entities/library';
import { button } from '@/widgets/library-browser';
export function ResultsTool({ projectId, asset }: { projectId: string; asset?: LibraryAsset }) {
  const { refresh } = useLibrary();
  const [files, setFiles] = useState<Artifact[]>([]);
  const [error, setError] = useState('');
  const [audio, setAudio] = useState('');
  const [busy, setBusy] = useState(false);
  const load = async () => {
    setBusy(true);
    try {
      setFiles(await invoke('list_project_artifacts_cmd', { projectId }));
      await refresh();
      setError('');
    } catch (e) {
      setError(describeCommandError(e));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    let active = true;
    void invoke('list_project_artifacts_cmd', { projectId })
      .then((items) => {
        if (active) setFiles(items);
      })
      .catch((e) => {
        if (active) setError(describeCommandError(e));
      });
    return () => {
      active = false;
    };
  }, [projectId]);
  const play = async (id: string) => {
    try {
      setAudio(convertFileSrc(await invoke('resolve_artifact_path_cmd', { artifactId: id })));
    } catch (e) {
      setError(describeCommandError(e));
    }
  };
  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Результаты{asset ? ` · ${asset.name}` : ''}</h2>
        <button className={button} disabled={busy} onClick={() => void load()}>
          Обновить результаты
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
      {audio && <audio aria-label="Прослушивание результата" controls autoPlay src={audio} />}
      {!files.length && (
        <p className="text-muted">Готовые результаты появятся после выполнения операции.</p>
      )}
      <ul className="space-y-2">
        {files
          .filter((f) => f.state === 'ready')
          .map((file) => (
            <li
              key={file.id}
              className="flex flex-wrap items-center justify-between gap-3 border border-border bg-surface p-3"
            >
              <div>
                <p>{file.kind}</p>
                <p className="break-all text-xs text-muted">
                  {file.location.value.split(/[\\/]/).pop()} · {file.sizeBytes ?? '—'} байт
                </p>
              </div>
              {(file.kind === 'generatedSpeech' ||
                file.kind === 'generatedSpeechSegment' ||
                file.kind === 'extractedAudio' ||
                (file.kind === 'sourceVideo' && asset?.kind === 'audio')) && (
                <button className={button} onClick={() => void play(file.id)}>
                  Прослушать
                </button>
              )}
            </li>
          ))}
      </ul>
    </section>
  );
}
