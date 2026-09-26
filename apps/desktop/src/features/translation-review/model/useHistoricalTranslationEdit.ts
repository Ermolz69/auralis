import { useEffect, useRef, useState } from 'react';
import {
  editHistoricalTranslationResult,
  getHistoricalTranslationEditContext,
  publishTranslationResult,
} from '@/entities/translation';
import type { HistoricalTranslationEditContext } from '@/shared/api/contracts/translationHistoricalEdit';
import type {
  ComparedTranslationSegment,
  TranslationPublication,
} from '@/shared/api/contracts/translation';
import { toCommandError, type CommandError } from '@/shared/api/contracts';
import type { HistoricalEditAttempt } from './historicalEditAttempt';

export function useHistoricalTranslationEdit(
  projectId: string,
  translationId: string,
  baseResultId: string,
  segment: ComparedTranslationSegment,
  onPublished: (publication: TranslationPublication) => void,
) {
  const scope = `${projectId}:${translationId}:${baseResultId}:${segment.id}`;
  const [openedScope, setOpenedScope] = useState<string | null>(null);
  const open = openedScope === scope;
  const [lines, setLines] = useState(segment.translatedLines);
  const [context, setContext] = useState<HistoricalTranslationEditContext | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [contextRevision, setContextRevision] = useState(0);
  const [attempt, setAttempt] = useState<HistoricalEditAttempt | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<CommandError | null>(null);
  const [publication, setPublication] = useState<TranslationPublication | null>(null);
  const generation = useRef(0);
  const savingRef = useRef(false);

  useEffect(() => {
    const invalidate = () => {
      ++generation.current;
    };
    invalidate();
    savingRef.current = false;
    setOpenedScope(null);
    setLines(segment.translatedLines);
    setAttempt(null);
    setPublication(null);
    setContext(null);
    setError(null);
    setSaving(false);
    return invalidate;
  }, [projectId, translationId, baseResultId, segment.id, segment.translatedLines]);

  useEffect(() => {
    if (!open) return;
    let current = true;
    setContext(null);
    setPreparing(true);
    setError(null);
    void getHistoricalTranslationEditContext(projectId, translationId, baseResultId)
      .then((found) => {
        if (current) setContext(found);
      })
      .catch((reason: unknown) => {
        if (current) setError(normalize(reason));
      })
      .finally(() => {
        if (current) setPreparing(false);
      });
    return () => {
      current = false;
    };
  }, [open, projectId, translationId, baseResultId, contextRevision]);

  const save = async () => {
    if (savingRef.current || !context || preparing || publication) return;
    const current = generation.current;
    const frozen = attempt ?? {
      input: {
        projectId,
        translationId,
        baseResultId,
        resultId: crypto.randomUUID(),
        expectedHeadResultId: context.expectedHeadResultId,
        expectedLinkRevision: context.expectedLinkRevision,
        segmentId: segment.id,
        lines: [...lines],
      },
      runId: context.runId,
      edited: false,
    };
    savingRef.current = true;
    setAttempt(frozen);
    setSaving(true);
    setError(null);
    try {
      if (!frozen.edited) {
        const savedId = await editHistoricalTranslationResult(frozen.input);
        if (savedId !== frozen.input.resultId)
          throw new Error('Сохранённая версия не совпадает с запросом');
        if (generation.current === current) setAttempt({ ...frozen, edited: true });
      }
      const published = await publishTranslationResult(
        frozen.input.projectId,
        frozen.input.translationId,
        frozen.input.resultId,
      );
      if (
        published.resultId !== frozen.input.resultId ||
        published.translationId !== frozen.input.translationId ||
        published.runId !== frozen.runId
      )
        throw new Error('Опубликованная версия не совпадает с сохранённой правкой');
      if (generation.current === current) {
        setPublication(published);
        onPublished(published);
      }
    } catch (reason) {
      if (generation.current === current) setError(normalize(reason));
    } finally {
      if (generation.current === current) {
        savingRef.current = false;
        setSaving(false);
      }
    }
  };

  const fresh = () => {
    if (savingRef.current) return;
    setAttempt(null);
    setContext(null);
    setContextRevision((value) => value + 1);
  };
  return {
    open,
    setOpen: (value: boolean) => setOpenedScope(value ? scope : null),
    lines,
    setLines,
    context,
    preparing,
    attempt,
    saving,
    error,
    publication,
    save,
    fresh,
  };
}

function normalize(reason: unknown): CommandError {
  return reason instanceof Error
    ? { code: 'INTERNAL', message: reason.message }
    : toCommandError(reason);
}
