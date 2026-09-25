// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationReview } from './TranslationReview';

const listProjectTranslations = vi.hoisted(() => vi.fn());
const getTranslationComparison = vi.hoisted(() => vi.fn());

vi.mock('@/entities/project', () => ({
  useProjectContext: () => ({ project: { id: '00000000-0000-4000-8000-000000000001' } }),
}));
vi.mock('@/entities/translation/api/translationApi', () => ({
  listProjectTranslations,
  getTranslationComparison,
}));

const projectId = '00000000-0000-4000-8000-000000000001';
const translationId = '00000000-0000-4000-8000-000000000002';
const resultId = '00000000-0000-4000-8000-000000000003';
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

afterEach(() => cleanup());
beforeEach(() => {
  vi.clearAllMocks();
  listProjectTranslations.mockResolvedValue([link]);
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
  it('shows a ready result beside its original and requests only the next page', async () => {
    render(<TranslationReview />);
    expect(await screen.findByText('Здравствуйте.')).not.toBeNull();
    expect(screen.getByText('你好。')).not.toBeNull();
    expect(screen.getByText(/Требует проверки/)).not.toBeNull();
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
});
