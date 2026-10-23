import { describeCommandError } from '@/shared/api/contracts/error';
import { useEffect, useState } from 'react';
import { SpeechCatalog } from '@/features/speech-catalog';
import { useLibrary } from '@/entities/library';
import { invoke } from '@/shared/api/tauri';
import { createText } from '@/entities/library';
import type { LibraryAsset, LibraryTab, LibraryWorkspace } from '@/shared/api/contracts/library';
import type { SpeechSourceRequest } from '@/shared/api/contracts/speech';
import { useTabDraft } from '../../model/tabDraft';
import { button } from '@/widgets/library-browser';
export function SpeechTool({
  projectId,
  asset,
  tab,
  workspace,
}: {
  projectId: string;
  asset?: LibraryAsset;
  tab: LibraryTab;
  workspace?: LibraryWorkspace;
}) {
  const { initial, draft, change } = useTabDraft(tab);
  const { mutate, openTool, refresh } = useLibrary();
  const [input, setInput] = useState<SpeechSourceRequest | null>(null);
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    let active = true;
    const source: SpeechSourceRequest =
      asset?.kind === 'subtitle'
        ? {
            kind: 'subtitles',
            text: null,
            translation_id: null,
            artifact_id: asset.artifactId,
            format: asset.format === 'vtt' ? 'vtt' : 'srt',
          }
        : { kind: 'text', text: asset?.text ?? '', translation_id: null };
    void (
      asset?.kind === 'subtitle'
        ? invoke('list_project_translations_cmd', { projectId })
        : Promise.resolve([])
    )
      .then(async (links) => {
        const link = links.find((l) => l.outputArtifactId === asset?.artifactId);
        if (asset?.kind === 'text' && asset.text === null && asset.artifactId) {
          const path = await invoke('resolve_artifact_path_cmd', { artifactId: asset.artifactId });
          source.text = await invoke('read_library_text_cmd', { path });
        }
        if (active)
          setInput(
            link ? { kind: 'translation', text: null, translation_id: link.translationId } : source,
          );
      })
      .catch((e) => {
        if (active) setError(describeCommandError(e));
      });
    return () => {
      active = false;
    };
  }, [projectId, asset?.id, asset?.kind, asset?.format, asset?.artifactId, asset?.text]);
  const saveVersion = async () => {
    const text = draft?.text ?? asset?.text ?? '';
    if (!text.trim()) return;
    setSaving(true);
    try {
      const version = await createText(
        `${asset?.name ?? 'Текст'} · версия`,
        text,
        { projectId: workspace?.projectId ?? null, folderId: null },
        mutate,
        asset?.id ?? null,
      );
      await refresh();
      await openTool('text', version.id);
    } catch (e) {
      setError(describeCommandError(e));
    } finally {
      setSaving(false);
    }
  };
  return (
    <div className="space-y-3">
      {error && <p role="alert">{error}</p>}
      {tab.tool === 'text' && (
        <div className="flex flex-wrap items-center gap-3">
          <button
            className={button}
            disabled={saving || asset?.kind === 'subtitle' || !(draft?.text ?? asset?.text)?.trim()}
            onClick={() => void saveVersion()}
          >
            {saving ? 'Сохраняем…' : 'Сохранить текст новой версией'}
          </button>
          <span className="text-xs text-muted">
            Черновик вкладки сохраняется автоматически. Проверка текста подтверждается отдельно.
          </span>
        </div>
      )}
      {input ? (
        <SpeechCatalog
          projectId={projectId}
          initialSource={input}
          draft={initial}
          onDraftChange={change}
          stage={tab.tool === 'voices' ? 'voices' : tab.tool === 'text' ? 'text' : 'speech'}
        />
      ) : (
        !error && <p role="status">Проверяем источник текста…</p>
      )}
    </div>
  );
}
