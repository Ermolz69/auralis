// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationReview } from './TranslationReview';
import { TranslationReviewPanel } from './TranslationReviewPanel';

const listProjectTranslations = vi.hoisted(() => vi.fn());
const getTranslationComparison = vi.hoisted(() => vi.fn());
const listTranslationHistory = vi.hoisted(() => vi.fn());
const selectTranslationResult = vi.hoisted(() => vi.fn());
const getHistoricalTranslationEditContext = vi.hoisted(() => vi.fn());
const editHistoricalTranslationResult = vi.hoisted(() => vi.fn());
const publishTranslationResult = vi.hoisted(() => vi.fn());
const getTranslationPublication = vi.hoisted(() => vi.fn());

vi.mock('@/entities/project', () => ({
  useProjectContext: () => ({ project: { id: '00000000-0000-4000-8000-000000000001' } }),
}));
vi.mock('@/entities/translation', () => ({
  listProjectTranslations,
  getTranslationComparison,
  listTranslationHistory,
  selectTranslationResult,
  getHistoricalTranslationEditContext,
  editHistoricalTranslationResult,
  publishTranslationResult,
  getTranslationPublication,
}));

const projectId = '00000000-0000-4000-8000-000000000001';
const translationId = '00000000-0000-4000-8000-000000000002';
const resultId = '00000000-0000-4000-8000-000000000003';
const olderResultId = '00000000-0000-4000-8000-000000000007';
const link = {
  translationId,
  projectId,
  sourceArtifactId: '00000000-0000-4000-8000-000000000004',
  sourceSha256: 'a'.repeat(64),
  sourceFormat: 'srt',
  sourceLanguage: 'zh',
  targetLanguage: 'ru',
  activeRunId: null,
  selectedResultId: resultId,
  outputArtifactId: '00000000-0000-4000-8000-000000000005',
  revision: 2,
  createdAt: '2026-09-25T00:00:00Z',
  updatedAt: '2026-09-25T00:01:00Z',
};
const historyEntries = [resultId, olderResultId].map((id, index) => ({
  resultId: id,
  resultRevision: 2 - index,
  runId: '00000000-0000-4000-8000-000000000006',
  artifactId: link.outputArtifactId,
  needsReview: true,
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
beforeEach(() => {
  vi.clearAllMocks();
  listProjectTranslations.mockResolvedValue([link]);
  listTranslationHistory.mockResolvedValue({ link, entries: historyEntries, nextCursor: null });
  selectTranslationResult.mockResolvedValue({
    ...link,
    selectedResultId: olderResultId,
    revision: 3,
  });
  getTranslationComparison.mockImplementation(
    async (_projectId, _translationId, requestedResultId, offset) => ({
      resultId: requestedResultId,
      runId: '00000000-0000-4000-8000-000000000006',
      needsReview: true,
      totalSegments: 26,
      offset,
      segments: [
        {
          id: offset + 1,
          ordinal: offset,
          cueLabel: null,
          startMs: 1000,
          endMs: 2000,
          sourceLines: [offset ? 'Второй исходник' : '你好。'],
          translatedLines: [
            offset
              ? 'Второй перевод'
              : requestedResultId === resultId
                ? 'Здравствуйте.'
                : 'Новый перевод.',
          ],
        },
      ],
    }),
  );
});

describe('TranslationReview', () => {
  it('edits an older preview during another active run and retains publication status across preview changes', async () => {
    const editedId = '00000000-0000-4000-8000-000000000010';
    vi.stubGlobal('crypto', { randomUUID: () => editedId });
    const activeLink = { ...link, activeRunId: '00000000-0000-4000-8000-000000000011' };
    listProjectTranslations.mockResolvedValue([activeLink]);
    listTranslationHistory.mockResolvedValue({
      link: activeLink,
      entries: historyEntries,
      nextCursor: null,
    });
    getHistoricalTranslationEditContext.mockResolvedValue({
      projectId,
      translationId,
      baseResultId: olderResultId,
      runId: historyEntries[0].runId,
      baseRevision: 1,
      expectedHeadResultId: resultId,
      headRevision: 2,
      expectedLinkRevision: 2,
    });
    editHistoricalTranslationResult.mockResolvedValue(editedId);
    const pending = {
      translationId,
      resultId: editedId,
      runId: historyEntries[0].runId,
      artifactId: '00000000-0000-4000-8000-000000000012',
      sourceSha256: link.sourceSha256,
      needsReview: true,
      state: 'pending',
    };
    publishTranslationResult.mockResolvedValue(pending);
    getTranslationPublication.mockResolvedValue({ link: activeLink, publication: pending });
    render(<TranslationReview />);
    await screen.findByText('Здравствуйте.');
    await waitFor(() =>
      expect(
        screen.getByRole('combobox', { name: 'Версия для сравнения' }).hasAttribute('disabled'),
      ).toBe(false),
    );
    fireEvent.change(screen.getByRole('combobox', { name: 'Версия для сравнения' }), {
      target: { value: olderResultId },
    });
    await screen.findByText('Новый перевод.');
    fireEvent.click(screen.getByRole('button', { name: 'Исправить перевод' }));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Сохранить новую версию' }).hasAttribute('disabled'),
      ).toBe(false),
    );
    fireEvent.change(screen.getByRole('textbox', { name: 'Строка перевода 1' }), {
      target: { value: 'Историческая правка.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Сохранить новую версию' }));
    await screen.findByText(/Ожидаем готовности отдельного файла/);
    expect(editHistoricalTranslationResult).toHaveBeenCalledWith(
      expect.objectContaining({
        baseResultId: olderResultId,
        expectedHeadResultId: resultId,
        expectedLinkRevision: 2,
        lines: ['Историческая правка.'],
      }),
    );
    expect(selectTranslationResult).not.toHaveBeenCalled();
    const chosen = {
      ...activeLink,
      selectedResultId: editedId,
      outputArtifactId: pending.artifactId,
      revision: 3,
    };
    getTranslationPublication.mockResolvedValue({
      link: chosen,
      publication: { ...pending, state: 'ready' },
    });
    listProjectTranslations.mockResolvedValue([chosen]);
    listTranslationHistory.mockResolvedValue({
      link: chosen,
      entries: [
        {
          ...historyEntries[0],
          resultId: editedId,
          resultRevision: 3,
          artifactId: pending.artifactId,
        },
        ...historyEntries,
      ],
      nextCursor: null,
    });
    fireEvent.click(screen.getByRole('button', { name: 'Обновить переводы' }));
    await screen.findByRole('option', { name: /версия 3/ });
    await screen.findByText(/выбрана для проекта/);
    fireEvent.change(screen.getByRole('combobox', { name: 'Версия для сравнения' }), {
      target: { value: editedId },
    });
    await waitFor(() =>
      expect(getTranslationComparison).toHaveBeenCalledWith(
        projectId,
        translationId,
        editedId,
        0,
        25,
      ),
    );
    expect(screen.getByText(/выбрана для проекта/)).not.toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });
  it('shows a ready result beside its original and requests only the next page', async () => {
    render(<TranslationReview />);
    expect(await screen.findByText('Здравствуйте.')).not.toBeNull();
    expect(screen.getByText('你好。')).not.toBeNull();
    expect(screen.getByText('Требует проверки · 26 реплик')).not.toBeNull();
    expect(getTranslationComparison).toHaveBeenCalledWith(
      projectId,
      translationId,
      resultId,
      0,
      25,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Далее' }));
    expect(await screen.findByText('Второй перевод')).not.toBeNull();
    expect(getTranslationComparison).toHaveBeenCalledWith(
      projectId,
      translationId,
      resultId,
      25,
      25,
    );
    expect(screen.getByRole('button', { name: 'Далее' }).hasAttribute('disabled')).toBe(true);
  });

  it('reports an unavailable comparison without presenting an unverified result', async () => {
    getTranslationComparison.mockRejectedValue(new Error('Original changed'));
    render(<TranslationReview />);
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Original changed');
    expect(screen.queryByText('Здравствуйте.')).toBeNull();
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
  });

  it('loads the newly selected result after publication refresh', async () => {
    const nextResultId = '00000000-0000-4000-8000-000000000007';
    const { rerender } = render(<TranslationReview />);
    await screen.findByText('Здравствуйте.');
    listProjectTranslations.mockResolvedValue([
      { ...link, selectedResultId: nextResultId, revision: 3 },
    ]);
    rerender(<TranslationReview refreshRevision={1} />);
    await waitFor(() =>
      expect(getTranslationComparison).toHaveBeenCalledWith(
        projectId,
        translationId,
        nextResultId,
        0,
        25,
      ),
    );
    expect(await screen.findByText('Новый перевод.')).not.toBeNull();
    expect(screen.queryByText('Здравствуйте.')).toBeNull();
  });

  it('previews history without attachment, then explicitly selects at the observed revision', async () => {
    render(<TranslationReview />);
    await screen.findByText('Здравствуйте.');
    await waitFor(() =>
      expect(
        screen.getByRole('combobox', { name: 'Версия для сравнения' }).hasAttribute('disabled'),
      ).toBe(false),
    );
    fireEvent.change(screen.getByRole('combobox', { name: 'Версия для сравнения' }), {
      target: { value: olderResultId },
    });
    await screen.findByText('Новый перевод.');
    expect(screen.queryByText('Здравствуйте.')).toBeNull();
    expect(selectTranslationResult).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Исправить перевод' })).not.toBeNull();
    const chosen = { ...link, selectedResultId: olderResultId, revision: 3 };
    listProjectTranslations.mockResolvedValue([chosen]);
    listTranslationHistory.mockResolvedValue({
      link: chosen,
      entries: historyEntries,
      nextCursor: null,
    });
    fireEvent.click(screen.getByRole('button', { name: 'Использовать эту версию' }));
    await waitFor(() =>
      expect(selectTranslationResult).toHaveBeenCalledWith(
        projectId,
        translationId,
        olderResultId,
        2,
      ),
    );
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'Использовать эту версию' })).toBeNull(),
    );
    expect(screen.getByText('Новый перевод.')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Исправить перевод' })).not.toBeNull();
  });

  it('keeps the preview and attachment separate after a stale-selection conflict', async () => {
    selectTranslationResult.mockRejectedValue({
      code: 'CONFLICT',
      message: 'Refresh before choosing a version',
    });
    render(<TranslationReview />);
    await screen.findByText('Здравствуйте.');
    await waitFor(() =>
      expect(
        screen.getByRole('combobox', { name: 'Версия для сравнения' }).hasAttribute('disabled'),
      ).toBe(false),
    );
    fireEvent.change(screen.getByRole('combobox', { name: 'Версия для сравнения' }), {
      target: { value: olderResultId },
    });
    await screen.findByText('Новый перевод.');
    fireEvent.click(screen.getByRole('button', { name: 'Использовать эту версию' }));
    expect(await screen.findByText('Refresh before choosing a version')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Использовать эту версию' })).not.toBeNull();
    expect(screen.getByRole('option', { name: /версия 2 · Прикреплена/ })).not.toBeNull();
    expect(listProjectTranslations).toHaveBeenCalledTimes(1);
  });

  it('loads more history using the cursor without selecting or discarding earlier entries', async () => {
    listTranslationHistory
      .mockResolvedValueOnce({ link, entries: [historyEntries[0]], nextCursor: resultId })
      .mockResolvedValueOnce({ link, entries: [historyEntries[1]], nextCursor: null });
    render(<TranslationReview />);
    await screen.findByText('Здравствуйте.');
    fireEvent.click(await screen.findByRole('button', { name: 'Ещё версии' }));
    await waitFor(() =>
      expect(listTranslationHistory).toHaveBeenLastCalledWith(
        projectId,
        translationId,
        resultId,
        25,
      ),
    );
    await screen.findByRole('option', { name: /версия 1/ });
    expect(screen.getByRole('option', { name: /версия 2/ })).not.toBeNull();
    expect(selectTranslationResult).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Ещё версии' })).toBeNull();
  });

  it('discards late history and comparison responses after changing project', async () => {
    let resolveHistory!: (value: unknown) => void;
    let resolveComparison!: (value: unknown) => void;
    listTranslationHistory.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveHistory = resolve;
        }),
    );
    getTranslationComparison.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveComparison = resolve;
        }),
    );
    const { rerender } = render(<TranslationReviewPanel projectId={projectId} />);
    await waitFor(() => expect(getTranslationComparison).toHaveBeenCalledTimes(1));
    const otherProjectId = '00000000-0000-4000-8000-000000000008';
    listProjectTranslations.mockResolvedValue([]);
    rerender(<TranslationReviewPanel projectId={otherProjectId} />);
    await screen.findByText('Пока нет переводов для этого проекта.');
    resolveHistory({ link, entries: historyEntries, nextCursor: null });
    resolveComparison({
      resultId,
      runId: historyEntries[0].runId,
      totalSegments: 1,
      offset: 0,
      needsReview: true,
      segments: [
        {
          id: 1,
          ordinal: 0,
          startMs: 1000,
          endMs: 2000,
          sourceLines: ['Old source'],
          translatedLines: ['Old result'],
        },
      ],
    });
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
    expect(screen.queryByText('Old result')).toBeNull();
    expect(screen.queryByRole('combobox', { name: 'Версия для сравнения' })).toBeNull();
  });
});
