// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { TranslationReviewDecision } from './TranslationReviewDecision';

const getTranslationReview = vi.hoisted(() => vi.fn());
const recordTranslationReview = vi.hoisted(() => vi.fn());
vi.mock('@/entities/translation', () => ({ getTranslationReview, recordTranslationReview }));

const projectId = '00000000-0000-4000-8000-000000000001';
const translationId = '00000000-0000-4000-8000-000000000002';
const firstResult = '00000000-0000-4000-8000-000000000003';
const editedResult = '00000000-0000-4000-8000-000000000004';

beforeEach(() => {
  vi.clearAllMocks();
  getTranslationReview.mockImplementation(async (_projectId, _translationId, resultId) => ({
    resultId,
    resultSha256: resultId === firstResult ? 'a'.repeat(64) : 'b'.repeat(64),
    linkRevision: resultId === firstResult ? 2 : 3,
    needsReview: true,
    selected: true,
    latest: null,
  }));
  recordTranslationReview.mockImplementation(
    async (_projectId, _translationId, resultId, view) => ({
      ...view,
      resultId,
      latest: {
        eventId: '00000000-0000-4000-8000-000000000005',
        decision: 'approved',
        reviewerLabel: 'Мария',
        note: 'Сверено с оригиналом',
        createdAt: '2026-10-10T00:00:00Z',
      },
    }),
  );
});
afterEach(cleanup);

it('requires an explicit digest-bound decision and leaves an edited result unapproved', async () => {
  const rendered = render(
    <TranslationReviewDecision
      projectId={projectId}
      translationId={translationId}
      resultId={firstResult}
      refreshRevision={0}
    />,
  );
  await screen.findByText('Решение ещё не принято');
  const approve = screen.getByRole('button', { name: 'Одобрить для речи' });
  expect(approve.hasAttribute('disabled')).toBe(true);
  fireEvent.change(screen.getByLabelText('Проверяющий'), { target: { value: 'Мария' } });
  fireEvent.change(screen.getByLabelText('Основание решения'), {
    target: { value: 'Сверено с оригиналом' },
  });
  fireEvent.click(approve);
  await screen.findByText('Одобрено: Мария');
  expect(recordTranslationReview).toHaveBeenCalledWith(
    projectId,
    translationId,
    firstResult,
    expect.objectContaining({ resultSha256: 'a'.repeat(64), linkRevision: 2 }),
    'approved',
    'Мария',
    'Сверено с оригиналом',
  );

  rendered.rerender(
    <TranslationReviewDecision
      projectId={projectId}
      translationId={translationId}
      resultId={editedResult}
      refreshRevision={1}
    />,
  );
  await waitFor(() =>
    expect(getTranslationReview).toHaveBeenCalledWith(projectId, translationId, editedResult),
  );
  await screen.findByText('Решение ещё не принято');
});
