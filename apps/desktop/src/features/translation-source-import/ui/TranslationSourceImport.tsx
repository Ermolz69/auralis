import { useEffect, useRef, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import {
  beginTranslationRun,
  importTranslationSource,
  listOriginalSubtitles,
  listProjectTranslations,
} from '@/entities/translation';
import type { Artifact } from '@/shared/api/contracts/artifact';
import type { TranslationFormat, TranslationLink } from '@/shared/api/contracts/translation';
import { Button } from '@/shared/ui/button';

type Props = { projectId: string; sourceArtifactId?: string; onTranslationPrepared?: () => void };

export function TranslationSourceImport({
  projectId,
  sourceArtifactId,
  onTranslationPrepared,
}: Props) {
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [links, setLinks] = useState<TranslationLink[]>([]);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [preparing, setPreparing] = useState<string | null>(null);
  const beginAttempts = useRef(
    new Map<
      string,
      {
        artifactId: string;
        translationId: string;
        runId: string;
      }
    >(),
  );
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    void Promise.all([listOriginalSubtitles(projectId), listProjectTranslations(projectId)])
      .then(([found, foundLinks]) => {
        if (!active) return;
        setArtifacts(
          sourceArtifactId ? found.filter((item) => item.id === sourceArtifactId) : found,
        );
        setLinks(foundLinks);
        setLoading(false);
      })
      .catch((reason: unknown) => {
        if (!active) return;
        setError(reason instanceof Error ? reason.message : 'Не удалось загрузить исходники');
        setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [projectId, sourceArtifactId, revision]);

  const importFile = async () => {
    if (importing) return;
    setImporting(true);
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Subtitle file', extensions: ['srt', 'vtt'] }],
      });
      if (typeof selected !== 'string') return;
      const format: TranslationFormat | null = selected.toLowerCase().endsWith('.srt')
        ? 'srt'
        : selected.toLowerCase().endsWith('.vtt')
          ? 'vtt'
          : null;
      if (!format) throw new Error('Выберите файл SRT или VTT');
      await importTranslationSource(projectId, selected, format);
      setRevision((value) => value + 1);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Не удалось импортировать субтитры');
    } finally {
      setImporting(false);
    }
  };

  const prepareTranslation = async (artifact: Artifact, format: TranslationFormat) => {
    if (preparing) return;
    const current = beginAttempts.current.get(artifact.id) ?? {
      artifactId: artifact.id,
      translationId: crypto.randomUUID(),
      runId: crypto.randomUUID(),
    };
    beginAttempts.current.set(artifact.id, current);
    setPreparing(artifact.id);
    setError(null);
    try {
      const link = await beginTranslationRun({
        projectId,
        sourceArtifactId: artifact.id,
        format,
        sourceLanguage: 'zh',
        translationId: current.translationId,
        runId: current.runId,
      });
      setLinks((existing) =>
        existing.some((item) => item.translationId === link.translationId)
          ? existing
          : [...existing, link],
      );
      setRevision((value) => value + 1);
      onTranslationPrepared?.();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Не удалось подготовить перевод');
    } finally {
      setPreparing(null);
    }
  };

  return (
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Исходные субтитры"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-text">Исходные субтитры</h2>
        <div className="flex gap-2">
          <Button size="sm" onClick={() => void importFile()} loading={importing}>
            Импортировать SRT/VTT
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setRevision((value) => value + 1)}
            loading={loading}
          >
            Обновить исходники
          </Button>
        </div>
      </div>
      <p className="mt-2 text-xs text-muted">
        Auralis сохранит отдельную копию. Выбранный файл останется без изменений.
      </p>
      {error && (
        <p role="alert" className="mt-2 text-xs text-danger">
          {error}
        </p>
      )}
      {loading && (
        <p role="status" className="mt-2 text-xs text-muted">
          Загружаем исходники…
        </p>
      )}
      {!loading && artifacts.length === 0 && (
        <p className="mt-2 text-xs text-muted">Исходников пока нет.</p>
      )}
      {artifacts.length > 0 && (
        <ul className="mt-3 space-y-1 text-xs text-text">
          {artifacts.map((artifact) => {
            const format =
              artifact.location.kind === 'storageKey' &&
              artifact.location.value.toLowerCase().endsWith('.srt')
                ? 'srt'
                : artifact.location.kind === 'storageKey' &&
                    artifact.location.value.toLowerCase().endsWith('.vtt')
                  ? 'vtt'
                  : null;
            const linked = links.some(
              (link) => link.sourceArtifactId === artifact.id && link.sourceLanguage === 'zh',
            );
            return (
              <li
                key={artifact.id}
                className="flex flex-wrap items-center justify-between gap-2 rounded-sm border border-border px-2 py-1.5"
              >
                <span className="font-mono">
                  {artifact.id.slice(0, 8)} · {format?.toUpperCase() ?? 'unknown'}
                </span>
                <span>
                  {artifact.state === 'ready'
                    ? linked
                      ? 'Перевод подготовлен'
                      : 'Готов к переводу'
                    : artifact.state === 'pendingFinalize'
                      ? 'Сохраняется…'
                      : 'Требует внимания'}
                </span>
                {artifact.state === 'ready' && format && !linked && (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => void prepareTranslation(artifact, format)}
                    loading={preparing === artifact.id}
                  >
                    Подготовить перевод с китайского
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
