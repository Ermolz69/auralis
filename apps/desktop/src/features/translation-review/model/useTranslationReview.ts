import { useEffect, useState } from 'react';
import { getTranslationComparison, listProjectTranslations } from '@/entities/translation';
import type {
  TranslationComparisonPage,
  TranslationLink,
} from '@/shared/api/contracts/translation';

export const COMPARISON_PAGE_SIZE = 25;

export function useTranslationReview(
  projectId: string | null,
  refreshRevision = 0,
  sourceArtifactId?: string,
) {
  const [links, setLinks] = useState<TranslationLink[]>([]);
  const [selectedTranslationId, setSelectedTranslationId] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ translationId: string; resultId: string } | null>(null);
  const [offset, setOffset] = useState(0);
  const [page, setPage] = useState<TranslationComparisonPage | null>(null);
  const [loadingLinks, setLoadingLinks] = useState(false);
  const [loadingPage, setLoadingPage] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    setLinks([]);
    setSelectedTranslationId(null);
    setPreview(null);
    setOffset(0);
    setPage(null);
    setError(null);
  }, [projectId, sourceArtifactId]);

  useEffect(() => {
    if (!projectId) {
      setLoadingLinks(false);
      return;
    }
    let current = true;
    setLoadingLinks(true);
    setError(null);
    void listProjectTranslations(projectId)
      .then((all) => {
        if (!current) return;
        const found = sourceArtifactId
          ? all.filter((link) => link.sourceArtifactId === sourceArtifactId)
          : all;
        setLinks(found);
        setSelectedTranslationId((selected) =>
          found.some((link) => link.translationId === selected && link.selectedResultId)
            ? selected
            : (found.find((link) => link.selectedResultId)?.translationId ?? null),
        );
        setLoadingLinks(false);
      })
      .catch((reason: unknown) => {
        if (!current) return;
        setError(reason instanceof Error ? reason.message : 'Не удалось загрузить переводы');
        setLoadingLinks(false);
      });
    return () => {
      current = false;
    };
  }, [projectId, sourceArtifactId, revision, refreshRevision]);

  const projectLinks = links.filter((link) => link.projectId === projectId);
  const selected =
    projectLinks.find((link) => link.translationId === selectedTranslationId) ?? null;
  const verifiedTranslationId = selected?.translationId ?? null;
  const selectedResultId =
    preview?.translationId === selectedTranslationId
      ? preview.resultId
      : (selected?.selectedResultId ?? null);
  useEffect(() => {
    if (!projectId || !verifiedTranslationId || !selectedTranslationId || !selectedResultId) {
      setPage(null);
      setLoadingPage(false);
      return;
    }
    let current = true;
    setLoadingPage(true);
    setPage(null);
    setError(null);
    void getTranslationComparison(
      projectId,
      selectedTranslationId,
      selectedResultId,
      offset,
      COMPARISON_PAGE_SIZE,
    )
      .then((found) => {
        if (!current) return;
        setPage(found);
        setLoadingPage(false);
      })
      .catch((reason: unknown) => {
        if (!current) return;
        setError(reason instanceof Error ? reason.message : 'Не удалось загрузить сравнение');
        setLoadingPage(false);
      });
    return () => {
      current = false;
    };
  }, [projectId, verifiedTranslationId, selectedTranslationId, selectedResultId, offset]);

  return {
    links: projectLinks,
    selectedTranslationId,
    selected,
    previewResultId: selectedResultId,
    historyRevision: revision + refreshRevision,
    preview: (resultId: string) => {
      if (!selectedTranslationId) return;
      setPreview({ translationId: selectedTranslationId, resultId });
      setOffset(0);
      if (selectedResultId !== resultId || offset !== 0) setPage(null);
    },
    offset,
    page,
    loadingLinks,
    loadingPage,
    error,
    refresh: () => setRevision((value) => value + 1),
    select: (translationId: string) => {
      setSelectedTranslationId(translationId);
      setPreview(null);
      setOffset(0);
      setPage(null);
    },
    previous: () => setOffset((value) => Math.max(0, value - COMPARISON_PAGE_SIZE)),
    next: () => setOffset((value) => value + COMPARISON_PAGE_SIZE),
  };
}
